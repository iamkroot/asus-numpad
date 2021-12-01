#![feature(iter_advance_by)]

mod devices;
mod dummy_keyboard;
mod numpad_layout;
mod touchpad_i2c;
mod util;

use std::hint::unreachable_unchecked;
use std::os::unix::io::AsRawFd;

use crate::devices::{get_touchpad_bbox, open_input_evdev, read_proc_input};
use crate::dummy_keyboard::{DummyKeyboard, KeyEvents};
use crate::numpad_layout::{NumpadLayout, LAYOUT_NAMES};
use crate::touchpad_i2c::{Brightness, TouchpadI2C};
use crate::util::ElapsedSince;
use clap::{App, Arg};
use evdev_rs::{
    enums::{EventCode, EV_ABS, EV_KEY, EV_MSC},
    Device, GrabMode, InputEvent, ReadFlag, TimeVal,
};
use log::{debug, trace};

#[derive(PartialEq, Debug, Clone, Copy)]
enum FingerState {
    Lifted,
    Touching,
    Tapping,
}

impl Default for FingerState {
    fn default() -> Self {
        FingerState::Lifted
    }
}

#[derive(Debug, Clone, Copy)]
struct TouchpadState {
    posx: f32,
    posy: f32,
    finger_state: FingerState,
    numlock: bool,
    cur_key: Option<EV_KEY>,
    tap_started_at: TimeVal,
    tapped_outside_numlock_bbox: bool,
    brightness: Brightness,
}

impl TouchpadState {
    fn toggle_numlock(&mut self) -> bool {
        self.numlock = !self.numlock;
        self.numlock
    }
}

impl Default for TouchpadState {
    fn default() -> Self {
        Self {
            posx: 0.0,
            posy: 0.0,
            finger_state: Default::default(),
            numlock: false,
            cur_key: None,
            tap_started_at: TimeVal {
                tv_sec: 0,
                tv_usec: 0,
            },
            tapped_outside_numlock_bbox: false,
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
    fn new(
        evdev: Device,
        keyboard_evdev: Device,
        touchpad_i2c: TouchpadI2C,
        dummy_kb: DummyKeyboard,
        layout: NumpadLayout,
    ) -> Self {
        Self {
            evdev,
            keyboard_evdev,
            touchpad_i2c,
            dummy_kb,
            layout,
            state: TouchpadState::default(),
        }
    }

    fn toggle_numlock(&mut self) {
        if self.state.toggle_numlock() {
            self.touchpad_i2c.set_brightness(self.state.brightness);
            self.evdev.grab(GrabMode::Grab).expect("GRAB");
        } else {
            self.touchpad_i2c.set_brightness(Brightness::Zero);
            self.evdev.grab(GrabMode::Ungrab).expect("UNGRAB");
        }
    }

    fn handle_touchpad_event(&mut self, ev: InputEvent) {
        // 1 second
        const HOLD_DURATION: TimeVal = TimeVal {
            tv_sec: 1,
            tv_usec: 0,
        };

        // no need to trace timestamp events - too noisy
        if !matches!(ev.event_code, EventCode::EV_MSC(EV_MSC::MSC_TIMESTAMP)) {
            trace!("TP{:?} {}", ev.event_code, ev.value);
        }
        match ev.event_code {
            EventCode::EV_ABS(EV_ABS::ABS_MT_POSITION_X) => {
                // what happens when it goes outside bbox of cur_key while dragging?
                // should we move to new key?
                // TODO: Check official Windows driver behaviour
                self.state.posx = ev.value as f32;
                return;
            }
            EventCode::EV_ABS(EV_ABS::ABS_MT_POSITION_Y) => {
                self.state.posy = ev.value as f32;
                return;
            }
            EventCode::EV_KEY(EV_KEY::BTN_TOOL_FINGER) if ev.value == 0 => {
                // end of tap
                debug!("End tap");
                self.state.finger_state = FingerState::Lifted;
                if self.layout.in_calc_bbox(self.state.posx, self.state.posy) {
                    debug!("In calc - end");
                    if self.state.numlock {
                        self.touchpad_i2c
                            .set_brightness(self.state.brightness.cycle());
                    } else {
                        // Start calculator
                        self.dummy_kb.keypress(EV_KEY::KEY_CALC);
                        // TODO: Should only start calc when dragged
                    }
                }
                if let Some(key) = self.state.cur_key {
                    debug!("Keyup {:?}", key);

                    if self.layout.needs_multikey(key) {
                        self.dummy_kb.multi_keyup(&self.layout.multikeys(key));
                    } else {
                        self.dummy_kb.keyup(key);
                    }
                    self.state.cur_key = None;
                }
            }
            EventCode::EV_KEY(EV_KEY::BTN_TOOL_FINGER) if ev.value == 1 => {
                if self.state.finger_state == FingerState::Lifted {
                    // start of tap
                    debug!("Start tap");
                    self.state.finger_state = FingerState::Touching;
                    self.state.tap_started_at = ev.time;
                    self.state.tapped_outside_numlock_bbox = false;
                }
                if self
                    .layout
                    .in_numlock_bbox(self.state.posx, self.state.posy)
                {
                    debug!("In numlock - start");
                    self.state.finger_state = FingerState::Tapping;
                } else {
                    if self.layout.in_calc_bbox(self.state.posx, self.state.posy) {
                        self.state.finger_state = FingerState::Tapping;
                        debug!("In calc - start");
                    }
                    self.state.tapped_outside_numlock_bbox = true
                }
            }

            EventCode::EV_MSC(EV_MSC::MSC_TIMESTAMP) => {
                // The toggle should happen automatically after HOLD_DURATION, even if user is
                // still touching the numpad bbox.
                if self.state.finger_state == FingerState::Tapping
                    && !self.state.tapped_outside_numlock_bbox
                {
                    if self
                        .layout
                        .in_numlock_bbox(self.state.posx, self.state.posy)
                    {
                        if ev.time.elapsed_since(self.state.tap_started_at) >= HOLD_DURATION {
                            debug!("Hold finish - toggle numlock");
                            self.toggle_numlock();
                            // If user doesn't lift the finger quickly, we don't want to keep
                            // toggling, so assume finger was moved.
                            // Can't do finger_state = Lifted, since that would start another tap
                            // Can't do finger_state = Touching, since that would cause numpad
                            // keypresses (we don't check for margins in layout.get_key yet)
                            self.state.tapped_outside_numlock_bbox = true;
                        }
                    } else {
                        self.state.tapped_outside_numlock_bbox = true;
                    }
                }
            }
            _ => (),
        }
        if self.state.numlock && self.state.finger_state == FingerState::Touching {
            self.state.cur_key = self.layout.get_key(self.state.posx, self.state.posy);
            if let Some(key) = self.state.cur_key {
                self.state.finger_state = FingerState::Tapping;
                debug!("Keydown {:?}", key);
                if self.layout.needs_multikey(key) {
                    self.dummy_kb.multi_keydown(&self.layout.multikeys(key));
                } else {
                    self.dummy_kb.keydown(key);
                }
            }
        }
    }

    fn process(&mut self) {
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
        // TODO: Initialize with numlock state of system
        // TODO: Remember the last used brightness and restore on start
        loop {
            match unsafe { libc::poll(fds.as_mut_ptr(), 2, -1) } {
                0 => (), // timeout, TODO: disable numpad if idle (no touches) for 1 minute
                1 | 2 => {
                    if fds[0].revents & libc::POLLIN != 0 {
                        // read until no more events
                        while let Ok((_, ev)) = self.evdev.next_event(ReadFlag::NORMAL) {
                            self.handle_touchpad_event(ev);
                        }
                    }
                    if fds[1].revents & libc::POLLIN != 0 {
                        while let Ok((_, ev)) = self.keyboard_evdev.next_event(ReadFlag::NORMAL) {
                            // TODO: Check for numlock
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

fn main() {
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
        .get_matches();
    let layout_name = matches.value_of("layout").expect("Expected layout");

    let (keyboard_ev_id, touchpad_ev_id, i2c_id) =
        read_proc_input().expect("Couldn't get proc input devices");
    let touchpad_dev = open_input_evdev(touchpad_ev_id);
    let keyboard_dev = open_input_evdev(keyboard_ev_id);
    let bbox = get_touchpad_bbox(&touchpad_dev);
    let layout = match layout_name {
        "ux433fa" => NumpadLayout::ux433fa(bbox),
        "m433ia" => NumpadLayout::m433ia(bbox),
        "ux581" => NumpadLayout::ux581(bbox),
        "gx701" => NumpadLayout::gx701(bbox),
        _ => unreachable!(),
    };
    let kb = DummyKeyboard::new(&layout);
    let touchpad_i2c = TouchpadI2C::new(i2c_id);
    let mut numpad = Numpad::new(touchpad_dev, keyboard_dev, touchpad_i2c, kb, layout);
    numpad.process();
}
