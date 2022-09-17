#![feature(iter_advance_by)]

mod config;
mod devices;
mod dummy_keyboard;
mod numpad_layout;
mod touchpad_i2c;
mod util;

use std::fmt::Display;
use std::hint::unreachable_unchecked;
use std::os::unix::io::AsRawFd;
use std::process::Command;

use crate::config::{Config, CustomCommand};
use crate::devices::{get_touchpad_bbox, open_input_evdev, read_proc_input};
use crate::dummy_keyboard::{DummyKeyboard, KeyEvents};
use crate::numpad_layout::NumpadLayout;
use crate::touchpad_i2c::{Brightness, TouchpadI2C};
use crate::util::{CustomDuration, ElapsedSince};
use anyhow::{Context, Result};
use evdev_rs::{
    enums::{EventCode, EV_ABS, EV_KEY, EV_LED, EV_MSC},
    Device, DeviceWrapper, InputEvent, ReadFlag, TimeVal,
};
use log::{debug, error, info, trace, warn};

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
    /// A key on the actual numpad bbox
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

#[derive(Debug)]
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
    calc_open: bool,
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
            calc_open: false,
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
            .event_value(&EventCode::EV_LED(EV_LED::LED_NUML));
        match init_numlock {
            Some(init_numlock) => {
                if init_numlock != 0 {
                    if self.config.disable_numlock_on_start() {
                        self.dummy_kb.keypress(EV_KEY::KEY_NUMLOCK);
                    } else {
                        self.handle_numlock_pressed(init_numlock)?;
                    }
                }
            }
            None => error!(
                "Failed to get initial numlock state. \
                There might be something wrong with evdev keyboard detection. \
                {}",
                self.keyboard_evdev.name().map_or_else(
                    || "Unknown device".to_owned(),
                    |n| format!("Using device: {}", n)
                )
            ),
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

    fn start_calc(&mut self) {
        debug!("Starting calc");
        match self.config.calc_start_command() {
            CustomCommand::Keys(keys) => self.dummy_kb.multi_keypress(keys.as_slice()),
            CustomCommand::Command { cmd, args } => {
                debug!("Running command {} with args {:?}", cmd, args);
                let cmd = cmd.clone();
                let args = args.clone();
                // spawn a thread that waits for the proc to end
                // ensures that all procs are reaped
                std::thread::spawn(|| {
                    match Command::new(cmd).args(args).spawn() {
                        Ok(mut child) => {
                            debug!("Started child proc: {}", child.id());
                            if let Err(err) = child.wait() {
                                warn!("Error while starting: {}", err);
                            } else {
                                trace!("Process ended");
                            }
                        }
                        Err(err) => warn!("Error while starting: {}", err),
                    };
                });
            }
        }
    }

    fn stop_calc(&mut self) {
        if let Some(stop_cmd) = self.config.calc_stop_command() {
            debug!("Stopping calc");

            match stop_cmd {
                CustomCommand::Keys(keys) => self.dummy_kb.multi_keypress(keys.as_slice()),
                CustomCommand::Command { cmd, args } => {
                    debug!("Running command {} with args {:?}", cmd, args);
                    match Command::new(cmd).args(args).spawn() {
                        Ok(mut child) => {
                            debug!("Started child proc: {}", child.id());
                            if let Err(err) = child.wait() {
                                warn!("Error while stopping: {}", err);
                            } else {
                                trace!("Process ended");
                            }
                        }
                        Err(err) => warn!("Error while stopping: {}", err),
                    };
                }
            }
        } else {
            // if no stop command given, we re-run the start cmd
            self.start_calc();
        }
    }

    fn on_lift(&mut self) {
        // end of tap
        debug!("End tap");
        if self.state.cur_key == CurKey::Calc
            && self.state.pos.dist_sq(self.state.tap_start_pos) >= Self::CALC_DRAG_DIST
        {
            if !self.state.calc_open {
                self.start_calc();
            } else {
                self.stop_calc();
            }
            self.state.calc_open = !self.state.calc_open;
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

        // no need to trace timestamp events - too noisy
        if !matches!(ev.event_code, EventCode::EV_MSC(EV_MSC::MSC_TIMESTAMP)) {
            trace!("TP {:?} {}", ev.event_code, ev.value);
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
                    && self.layout.in_calc_bbox(self.state.pos)
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
        // TODO: Use the same logic for numlock bbox instead of `tapped_outside_numlock_bbox`
        if self.state.numlock
            && self.state.finger_state == FingerState::Touching
            && self.state.cur_key != CurKey::Calc // we are fine if finger drags on calc box
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

fn main() -> Result<()> {
    env_logger::init();

    // Follows XDG Base Dir Spec
    const CONFIG_PATH: &str = "/etc/xdg/asus_numpad.toml";

    let config: Config = toml::from_slice(&std::fs::read(CONFIG_PATH)?)?;
    info!("Config: {:?}", config);
    let layout_name = config.layout();

    let (keyboard_ev_id, touchpad_ev_id, i2c_id) =
        read_proc_input().context("Couldn't get proc input devices")?;
    let touchpad_dev = open_input_evdev(touchpad_ev_id)?;
    let keyboard_dev = open_input_evdev(keyboard_ev_id)?;
    let bbox = get_touchpad_bbox(&touchpad_dev)?;
    let layout = NumpadLayout::from_supported_layout(layout_name, bbox)?;
    let kb = DummyKeyboard::new(&layout)?;
    let touchpad_i2c = TouchpadI2C::new(i2c_id)?;
    let mut numpad = Numpad::new(touchpad_dev, keyboard_dev, touchpad_i2c, kb, layout, config);
    numpad.process()?;
    Ok(())
}
