#![feature(iter_advance_by)]

mod devices;
mod dummy_keyboard;
mod numpad_layout;
mod touchpad_i2c;
mod util;

use std::fmt::Display;
use std::hint::unreachable_unchecked;
use std::os::unix::io::AsRawFd;

use crate::devices::{get_touchpad_bbox, open_input_evdev, read_proc_input};
use crate::dummy_keyboard::{DummyKeyboard, KeyEvents};
use crate::numpad_layout::{NumpadLayout, LAYOUT_NAMES};
use crate::touchpad_i2c::{Brightness, TouchpadI2C};
use crate::util::{CustomDuration, ElapsedSince};
use anyhow::{anyhow, Context, Result};
use clap::{App, Arg};
use evdev_rs::{
    enums::{EventCode, EV_ABS, EV_KEY, EV_LED, EV_MSC},
    Device, DeviceWrapper, InputEvent, ReadFlag, TimeVal,
};
use log::{debug, trace, warn};

#[derive(PartialEq, Debug, Clone, Copy)]
enum FingerState {
    Lifted,
    TouchStart,
    Touching,
}

impl Default for FingerState {
    fn default() -> Self {
        FingerState::Lifted
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd)]
pub struct Point {
    x: i32,
    y: i32,
}

impl Display for Point {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("<{}, {}>", self.x, self.y))
    }
}

impl Point {
    fn dist_sq(&self, other: Self) -> i32 {
        (self.x - other.x).pow(2) + (self.y - other.y).pow(2)
    }
}

/// Represents the key being pressed currently
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CurKey {
    None,
    Numlock,
    Calc,
    /// A key on the actuall numpad bbox
    Numpad(EV_KEY),
}

impl CurKey {
    #[inline]
    fn reset(&mut self) {
        *self = Self::None;
    }
}

impl Default for CurKey {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, Copy)]
struct TouchpadState {
    pos: Point,
    finger_state: FingerState,
    numlock: bool,
    cur_key: CurKey,
    tap_started_at: TimeVal,
    tap_start_pos: Point,
    tapped_outside_numlock_bbox: bool,
    finger_dragged_too_much: bool,
    dragged_finger_lifted_at: TimeVal,
    brightness: Brightness,
}

impl TouchpadState {
    #[inline]
    fn toggle_numlock(&mut self) -> bool {
        self.numlock = !self.numlock;
        self.numlock
    }
}

impl Default for TouchpadState {
    fn default() -> Self {
        Self {
            pos: Default::default(),
            finger_state: Default::default(),
            numlock: false,
            cur_key: Default::default(),
            tap_started_at: TimeVal {
                tv_sec: 0,
                tv_usec: 0,
            },
            tap_start_pos: Default::default(),
            tapped_outside_numlock_bbox: false,
            finger_dragged_too_much: false,
            dragged_finger_lifted_at: TimeVal {
                tv_sec: 0,
                tv_usec: 0,
            },
            brightness: Default::default(),
        }
    }
}

struct Numpad {
    evdev: Device,
    keyboard_evdev: Device,
    touchpad_i2c: TouchpadI2C,
    dummy_kb: DummyKeyboard,
    layout: NumpadLayout,
    state: TouchpadState,
    config: Config,
}

impl std::fmt::Debug for Numpad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Numpad")
            .field("evdev", &self.evdev.file())
            .field("keyboard_evdev", &self.keyboard_evdev.file())
            .field("dummy_keyboard", &self.dummy_kb)
            .field("touchpad_i2c", &self.touchpad_i2c)
            .field("state", &self.state)
            .field("layout", &self.layout)
            .finish()
    }
}

impl Numpad {
    const HOLD_DURATION: CustomDuration = CustomDuration::from_millis(250);

    /// Min Euclidean distance (squared) that a finger needs to move for a tap
    /// to be changed into a drag.  
    const TAP_JITTER_DIST: i32 = 10000;

    /// Min Euclidean distance (squared) that a finger needs to be dragged to
    /// trigger the calculator key when numlock isn't active.
    const CALC_DRAG_DIST: i32 = 90000;

    fn new(
        evdev: Device,
        keyboard_evdev: Device,
        touchpad_i2c: TouchpadI2C,
        dummy_kb: DummyKeyboard,
        layout: NumpadLayout,
        config: Config,
    ) -> Self {
        Self {
            evdev,
            keyboard_evdev,
            touchpad_i2c,
            dummy_kb,
            layout,
            state: TouchpadState::default(),
            config,
        }
    }

    /// Toggle numlock when user presses the numlock bbox on touchpad.
    fn toggle_numlock(&mut self) -> Result<()> {
        if self.state.toggle_numlock() {
            self.touchpad_i2c.set_brightness(self.state.brightness)?;
            // don't grab touchpad - allow moving pointer even if active
        } else {
            self.touchpad_i2c.set_brightness(Brightness::Zero)?;
            // we might still be grabbing the touchpad. release it.
            self.ungrab();
        }
        // Tell the system that we want to toggle the numlock
        self.dummy_kb.keypress(EV_KEY::KEY_NUMLOCK);
        Ok(())
    }

    /// Handle numlock pressed *from an external keyboard*.
    ///
    /// This is to keep the touchpad state in sync with system's numlock.
    fn handle_numlock_pressed(&mut self, val: i32) -> Result<()> {
        if val == 0 {
            debug!("setting off");
            self.state.numlock = false;
            // we might still be grabbing the touchpad. release it.
            self.ungrab();
            self.touchpad_i2c.set_brightness(Brightness::Zero)
        } else {
            debug!("setting on {}", self.state.brightness);
            self.state.numlock = true;
            self.touchpad_i2c.set_brightness(self.state.brightness)
        }
        // The numlock has already been toggled on the system- no need to press
        // the Num_Lock evkey.
    }

    /// Query the initial state of numlock led from the system.
    fn initialize_numlock(&mut self) -> Result<()> {
        let init_numlock = self
            .keyboard_evdev
            .event_value(&EventCode::EV_LED(EV_LED::LED_NUML))
            .ok_or(anyhow!("Failed to get initial numlock state"))?;

        if init_numlock != 0 {
            if self.config.disable_numlock_on_start {
                self.dummy_kb.keypress(EV_KEY::KEY_NUMLOCK);
            } else {
                self.handle_numlock_pressed(init_numlock)?;
            }
        }
        Ok(())
    }

    fn grab(&mut self) {
        debug!("Grabbing");
        self.evdev
            .grab(evdev_rs::GrabMode::Grab)
            .unwrap_or_else(|err| warn!("Failed to grab {}", err));
    }

    fn ungrab(&mut self) {
        self.evdev
            .grab(evdev_rs::GrabMode::Ungrab)
            .unwrap_or_else(|err| warn!("Failed to ungrab {}", err));
    }

    fn on_lift(&mut self) {
        // end of tap
        debug!("End tap");
        if !self.state.numlock
            && self.state.cur_key == CurKey::Calc
            && self.state.pos.dist_sq(self.state.tap_start_pos) >= Self::CALC_DRAG_DIST
        {
            // Start calculator
            debug!("Dragged to start calc");
            self.dummy_kb.keypress(EV_KEY::KEY_CALC);
        }

        if self.state.finger_state == FingerState::Touching {
            if let CurKey::Numpad(key) = self.state.cur_key {
                debug!("Keyup {:?}", key);

                if self.layout.needs_multikey(key) {
                    self.dummy_kb.multi_keyup(&self.layout.multikeys(key));
                } else {
                    self.dummy_kb.keyup(key);
                }
                // if we ungrab here, it causes the pointer to jump
                // so we only ungrab when finger is dragged
            }
        }
        self.state.cur_key.reset();
        self.state.finger_state = FingerState::Lifted;
    }

    fn on_tap(&mut self, time: TimeVal) {
        if self.state.finger_state == FingerState::Lifted {
            // start of tap
            debug!("Start tap");
            self.state.finger_state = FingerState::TouchStart;
            self.state.tap_started_at = time;
            self.state.tap_start_pos = self.state.pos;
            self.state.tapped_outside_numlock_bbox = false;
            self.state.finger_dragged_too_much = false;
            if self.state.numlock {
                self.state.cur_key = match self.layout.get_key(self.state.pos) {
                    Some(key) => {
                        self.grab();
                        self.state.finger_state = FingerState::Touching;

                        debug!("Keydown {:?}", key);
                        if self.layout.needs_multikey(key) {
                            self.dummy_kb.multi_keydown(&self.layout.multikeys(key));
                        } else {
                            self.dummy_kb.keydown(key);
                        }
                        CurKey::Numpad(key)
                    }
                    None => CurKey::None,
                };
            }
        }
        if self.layout.in_numlock_bbox(self.state.pos) {
            debug!("In numlock - start");
            self.state.finger_state = FingerState::Touching;
            self.state.cur_key = CurKey::Numlock;
        } else {
            if self.layout.in_calc_bbox(self.state.pos) {
                debug!("In calc - start");
                self.state.finger_state = FingerState::Touching;
                self.state.cur_key = CurKey::Calc;
            }
            self.state.tapped_outside_numlock_bbox = true
        }
    }

    fn handle_touchpad_event(&mut self, ev: InputEvent) -> Result<()> {
        // TODO: Double-taps when numpad is active should not be propagated.
        //       Need to grab/ungrab the device intelligently.
        // TODO: Dragging after a hold does not cause pointer to move, even
        //       if ungrabbed. Investigate this.

        // no need to trace timestamp events - too noisy
        if !matches!(ev.event_code, EventCode::EV_MSC(EV_MSC::MSC_TIMESTAMP)) {
            trace!("TP{:?} {}", ev.event_code, ev.value);
        }
        match ev.event_code {
            EventCode::EV_ABS(EV_ABS::ABS_MT_POSITION_X) => {
                self.state.pos.x = ev.value;
            }
            EventCode::EV_ABS(EV_ABS::ABS_MT_POSITION_Y) => {
                self.state.pos.y = ev.value;
            }
            EventCode::EV_KEY(EV_KEY::BTN_TOOL_FINGER) if ev.value == 0 => {
                if !self.state.finger_dragged_too_much {
                    // only call on_lift if we did not already call it as a result of finger drag
                    self.on_lift();
                } else {
                    self.state.dragged_finger_lifted_at = ev.time;
                }
            }
            EventCode::EV_KEY(EV_KEY::BTN_TOOL_FINGER) if ev.value == 1 => {
                if !self.state.finger_dragged_too_much
                    || ev.time.elapsed_since(self.state.dragged_finger_lifted_at)
                        >= Self::HOLD_DURATION
                {
                    self.on_tap(ev.time);
                }
            }
            EventCode::EV_MSC(EV_MSC::MSC_TIMESTAMP) => {
                // The toggle should happen automatically after HOLD_DURATION, even if user is
                // still touching the numpad bbox.
                if self.state.finger_state == FingerState::TouchStart {
                    trace!("Touch {}", self.state.pos);
                }

                if self.state.finger_state == FingerState::Touching
                    && !self.state.tapped_outside_numlock_bbox
                {
                    if self.layout.in_numlock_bbox(self.state.pos) {
                        if ev.time.elapsed_since(self.state.tap_started_at) >= Self::HOLD_DURATION {
                            debug!("Hold finish - toggle numlock");
                            self.toggle_numlock()?;
                            // If user doesn't lift the finger quickly, we don't want to keep
                            // toggling, so assume finger was moved.
                            // Can't do finger_state = Lifted, since that would start another tap
                            self.state.finger_state = FingerState::TouchStart;
                        }
                    } else {
                        self.state.tapped_outside_numlock_bbox = true;
                    }
                }
                if self.state.numlock
                    && self.state.cur_key == CurKey::Calc
                    && ev.time.elapsed_since(self.state.tap_started_at) >= Self::HOLD_DURATION
                {
                    debug!("Hold finish - cycle brightness");
                    self.touchpad_i2c
                        .set_brightness(self.state.brightness.cycle())?;
                    self.state.cur_key.reset();
                }
            }
            _ => (),
        }

        // if the finger drags too much, stop the tap
        if self.state.numlock
            && self.state.finger_state == FingerState::Touching
            && self.state.tap_start_pos.dist_sq(self.state.pos) > Self::TAP_JITTER_DIST
        {
            debug!("Moved too much");
            self.state.finger_dragged_too_much = true;
            self.ungrab();
            self.on_lift();
        }
        Ok(())
    }

    fn process(&mut self) -> Result<()> {
        self.initialize_numlock()?;

        let tp_fd = libc::pollfd {
            fd: self.evdev.file().as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        let kb_fd = libc::pollfd {
            fd: self.keyboard_evdev.file().as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        let mut fds = [tp_fd, kb_fd];

        loop {
            match unsafe { libc::poll(fds.as_mut_ptr(), 2, -1) } {
                0 => (), // timeout, TODO: disable numpad if idle (no touches) for 1 minute
                1 | 2 => {
                    if fds[0].revents & libc::POLLIN != 0 {
                        // read until no more events
                        while let Ok((_, ev)) = self.evdev.next_event(ReadFlag::NORMAL) {
                            self.handle_touchpad_event(ev)?;
                        }
                    }
                    if fds[1].revents & libc::POLLIN != 0 {
                        while let Ok((_, ev)) = self.keyboard_evdev.next_event(ReadFlag::NORMAL) {
                            // Note: We only listen to the LED event, and not the numlock event.
                            // While most environments keep them in sync, it is technically possible
                            // to change the led state without changing the numlock state.
                            //
                            // But there is no simple way for us to figure out the actual numlock
                            // state. We would need to bring in Xlib (and equivalent for wayland)
                            // and query it to get the numlock state.
                            //
                            // So, we only listen for LED changes, hoping that it reflects numlock state
                            if let EventCode::EV_LED(EV_LED::LED_NUML) = ev.event_code {
                                self.handle_numlock_pressed(ev.value)?;
                            }
                            trace!("KB {}, {}", ev.event_code, ev.value);
                        }
                    }
                }
                // we have only given 2 fds, so max return val of poll can be 2
                _ => unsafe { unreachable_unchecked() },
            }
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq, Hash)]
struct Config {
    pub disable_numlock_on_start: bool,
}

fn main() -> Result<()> {
    env_logger::init();
    let matches = App::new("asus-numpad")
        .arg(
            Arg::with_name("layout")
                .long("layout")
                .short("l")
                .takes_value(true)
                .required(true)
                .possible_values(&LAYOUT_NAMES),
        )
        .arg(
            Arg::with_name("disable_numlock_on_start")
                .long("disable_numlock_on_start")
                .takes_value(true)
                .default_value("true")
                .possible_values(&["true", "false"]),
        )
        .get_matches();
    let layout_name = matches.value_of("layout").expect("Expected layout");
    let config = Config {
        disable_numlock_on_start: matches
            .value_of("disable_numlock_on_start")
            .unwrap()
            .parse()?,
    };

    let (keyboard_ev_id, touchpad_ev_id, i2c_id) =
        read_proc_input().context("Couldn't get proc input devices")?;
    let touchpad_dev = open_input_evdev(touchpad_ev_id)?;
    let keyboard_dev = open_input_evdev(keyboard_ev_id)?;
    let bbox = get_touchpad_bbox(&touchpad_dev)?;
    let layout = NumpadLayout::from_model_name(layout_name, bbox)?;
    let kb = DummyKeyboard::new(&layout)?;
    let touchpad_i2c = TouchpadI2C::new(i2c_id)?;
    let mut numpad = Numpad::new(touchpad_dev, keyboard_dev, touchpad_i2c, kb, layout, config);
    numpad.process()?;
    Ok(())
}
