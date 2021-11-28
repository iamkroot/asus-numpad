#![feature(iter_advance_by)]

mod devices;
mod dummy_keyboard;
mod numpad_layout;
mod touchpad_i2c;
mod util;

use crate::devices::{open_input_evdev, read_proc_input};
use crate::dummy_keyboard::{DummyKeyboard, KeyEvents};
use crate::numpad_layout::NumpadLayout;
use crate::touchpad_i2c::TouchpadI2C;
use crate::util::ElapsedSince;
use evdev_rs::{
    enums::{EventCode, EV_ABS, EV_KEY, EV_MSC},
    Device, DeviceWrapper, GrabMode, ReadFlag, TimeVal,
};
use numpad_layout::BBox;

#[derive(Debug, Clone, Copy)]
pub enum Brightness {
    Zero = 0,
    Low = 31,
    Half = 24,
    Full = 1,
}

impl Default for Brightness {
    fn default() -> Self {
        Brightness::Half
    }
}

impl std::fmt::Display for Brightness {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Brightness::*;
        let level = match self {
            Zero => "Zero",
            Low => "Low",
            Half => "Half",
            Full => "Full",
        };
        f.write_str(level)
    }
}

impl Brightness {
    fn next(&self) -> Self {
        use Brightness::*;
        match self {
            Zero => Default::default(), // Jump to default
            Low => Half,
            Half => Full,
            Full => Low,
        }
    }
    fn cycle(&mut self) -> Self {
        *self = self.next();
        *self
    }
}

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

fn get_minmax(dev: &Device, code: EV_ABS) -> (f32, f32) {
    let abs = dev.abs_info(&EventCode::EV_ABS(code)).expect("MAX");
    (abs.minimum as f32, abs.maximum as f32)
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
    // TODO: allow changing brightness
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
    touchpad_i2c: TouchpadI2C,
    dummy_kb: DummyKeyboard,
    layout: NumpadLayout,
    state: TouchpadState,
}

impl std::fmt::Debug for Numpad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Numpad")
            .field("evdev", &self.evdev.file())
            .field("keyboard", &self.dummy_kb)
            .field("state", &self.state)
            .field("layout", &self.layout)
            .finish()
    }
}

impl Numpad {
    fn new(
        evdev: Device,
        touchpad_i2c: TouchpadI2C,
        dummy_kb: DummyKeyboard,
        layout: NumpadLayout,
    ) -> Self {
        Self {
            evdev,
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

    fn process(&mut self) {
        // 1 second
        const HOLD_DURATION: TimeVal = TimeVal {
            tv_sec: 1,
            tv_usec: 0,
        };
        loop {
            let ev = self
                .evdev
                .next_event(ReadFlag::NORMAL | ReadFlag::BLOCKING)
                .map(|val| val.1);
            if let Ok(ev) = ev {
                match ev.event_code {
                    EventCode::EV_ABS(EV_ABS::ABS_MT_POSITION_X) => {
                        // what happens when it goes outside bbox of cur_key while dragging?
                        // should we move to new key?
                        // TODO: Check official Windows driver behaviour
                        self.state.posx = ev.value as f32;
                        continue;
                    }
                    EventCode::EV_ABS(EV_ABS::ABS_MT_POSITION_Y) => {
                        self.state.posy = ev.value as f32;
                        continue;
                    }
                    EventCode::EV_KEY(EV_KEY::BTN_TOOL_FINGER) => {
                        if ev.value == 0 {
                            // end of tap
                            self.state.finger_state = FingerState::Lifted;
                            if self.layout.in_calc_bbox(self.state.posx, self.state.posy) {
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
                                if self.layout.needs_multikey(key) {
                                    self.dummy_kb.multi_keyup(&self.layout.multikeys(key));
                                } else {
                                    self.dummy_kb.keyup(key);
                                }
                                self.state.cur_key = None;
                            }
                        } else if ev.value == 1 {
                            if self.state.finger_state == FingerState::Lifted {
                                // start of tap
                                self.state.finger_state = FingerState::Touching;
                                self.state.tap_started_at = ev.time;
                                self.state.tapped_outside_numlock_bbox = false;
                            }
                            if self
                                .layout
                                .in_numlock_bbox(self.state.posx, self.state.posy)
                            {
                                self.state.finger_state = FingerState::Tapping;
                            } else {
                                if self.layout.in_calc_bbox(self.state.posx, self.state.posy) {
                                    self.state.finger_state = FingerState::Tapping;
                                }
                                self.state.tapped_outside_numlock_bbox = true
                            }
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
                                if ev.time.elapsed_since(self.state.tap_started_at) >= HOLD_DURATION
                                {
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
                        if self.layout.needs_multikey(key) {
                            self.dummy_kb.multi_keydown(&self.layout.multikeys(key));
                        } else {
                            self.dummy_kb.keydown(key);
                        }
                    }
                }
            }
        }
    }
}

fn main() {
    let (_, touchpad_ev_id, i2c_id) = read_proc_input().expect("Couldn't get proc input devices");
    let touchpad_dev = open_input_evdev(touchpad_ev_id);
    let (minx, maxx) = get_minmax(&touchpad_dev, EV_ABS::ABS_X);
    let (miny, maxy) = get_minmax(&touchpad_dev, EV_ABS::ABS_Y);
    let bbox = BBox::new(minx, maxx, miny, maxy);
    let layout = NumpadLayout::m433ia(bbox);
    let kb = DummyKeyboard::new(&layout);
    let touchpad_i2c = TouchpadI2C::new(i2c_id);
    let mut numpad = Numpad::new(touchpad_dev, touchpad_i2c, kb, layout);
    numpad.process();
}
