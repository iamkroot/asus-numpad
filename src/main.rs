#![feature(iter_advance_by)]

mod devices;
mod dummy_keyboard;
mod numpad_layout;

use crate::devices::{open_input_evdev, read_proc_input};
use crate::dummy_keyboard::{DummyKeyboard, KeyEvents};
use crate::numpad_layout::NumpadLayout;
use evdev_rs::{
    enums::{EventCode, EV_ABS, EV_KEY, EV_MSC},
    Device, DeviceWrapper, GrabMode, ReadFlag, TimeVal,
};

#[derive(PartialEq, Debug, Clone, Copy)]
enum FingerState {
    Lifted,
    Touching,
    Tapping,
}

fn get_minmax(dev: &Device, code: EV_ABS) -> (f32, f32) {
    let abs = dev.abs_info(&EventCode::EV_ABS(code)).expect("MAX");
    (abs.minimum as f32, abs.maximum as f32)
}

fn deactivate_numlock() {
    std::process::Command::new("i2ctransfer")
        .args(
            "-f -y 0 w13@0x15 0x05 0x00 0x3d 0x03 0x06 0x00 0x07 0x00 0x0d 0x14 0x03 0x00 0xad"
                .split(' '),
        )
        .status()
        .expect("Numlock");
}

fn activate_numlock() {
    std::process::Command::new("i2ctransfer")
        .args(
            "-f -y 0 w13@0x15 0x05 0x00 0x3d 0x03 0x06 0x00 0x07 0x00 0x0d 0x14 0x03 0x1f 0xad"
                .split(' '),
        )
        .status()
        .expect("Numlock");
}

fn toggle_numlock(numlock: &mut bool, touchpad_dev: &mut Device) {
    *numlock = if *numlock {
        deactivate_numlock();
        touchpad_dev.grab(GrabMode::Ungrab).expect("UNGRAB");
        false
    } else {
        activate_numlock();
        touchpad_dev.grab(GrabMode::Grab).expect("GRAB");
        true
    };
}

trait ElapsedSince {
    /// Calculate time elapsed since `other`.
    ///
    /// Assumes that self >= other
    fn elapsed_since(&self, other: Self) -> Self;
}

impl ElapsedSince for TimeVal {
    fn elapsed_since(&self, other: Self) -> Self {
        const USEC_PER_SEC: u32 = 1_000_000;
        let (secs, nsec) = if self.tv_usec >= other.tv_usec {
            (
                (self.tv_sec - other.tv_sec) as i64,
                (self.tv_usec - other.tv_usec) as u32,
            )
        } else {
            (
                (self.tv_sec - other.tv_sec - 1) as i64,
                self.tv_usec as u32 + (USEC_PER_SEC as u32) - other.tv_usec as u32,
            )
        };

        Self {
            tv_sec: secs,
            tv_usec: nsec as i64,
        }
    }
}

fn main() {
    // TODO: Use i2cdev crate- wait for `force_new` release
    let (_, touchpad_ev_id, i2c) = read_proc_input().expect("Couldn't get proc input devices");
    let mut touchpad_dev = open_input_evdev(touchpad_ev_id);
    let (minx, maxx) = get_minmax(&touchpad_dev, EV_ABS::ABS_X);
    let (miny, maxy) = get_minmax(&touchpad_dev, EV_ABS::ABS_Y);
    let layout = NumpadLayout::m433ia(minx, maxx, miny, maxy);
    let kb = DummyKeyboard::new(&layout);

    let mut pos_x = 0.0;
    let mut pos_y = 0.0;
    let mut numlock: bool = false;
    let mut cur_key: Option<EV_KEY> = None;
    let mut finger_state = FingerState::Lifted;
    let mut tap_started_at = TimeVal {
        tv_sec: 0,
        tv_usec: 0,
    };
    let mut tapped_outside_numlock_bbox: bool = false;
    /// 1sec
    const HOLD_DURATION: TimeVal = TimeVal {
        tv_sec: 0,
        tv_usec: 750_000,
    };
    // TODO: Read I2C brightness while starting up to see if numlock is enabled
    loop {
        let ev = touchpad_dev
            .next_event(ReadFlag::NORMAL | ReadFlag::BLOCKING)
            .map(|val| val.1);
        if let Ok(ev) = ev {
            match ev.event_code {
                EventCode::EV_ABS(EV_ABS::ABS_MT_POSITION_X) => {
                    // what happens when it goes outside bbox of cur_key while dragging?
                    // should we move to new key?
                    // TODO: Check official Windows driver behaviour
                    pos_x = ev.value as f32;
                    continue;
                }
                EventCode::EV_ABS(EV_ABS::ABS_MT_POSITION_Y) => {
                    pos_y = ev.value as f32;
                    continue;
                }
                EventCode::EV_KEY(EV_KEY::BTN_TOOL_FINGER) => {
                    if ev.value == 0 {
                        // end of tap
                        finger_state = FingerState::Lifted;
                        if let Some(key) = cur_key {
                            if layout.needs_multikey(key) {
                                kb.multi_keyup(&layout.multikeys(key));
                            } else {
                                kb.keyup(key);
                            }
                            cur_key = None;
                        }
                    } else if ev.value == 1 {
                        if finger_state == FingerState::Lifted {
                            // start of tap
                            finger_state = FingerState::Touching;
                            tap_started_at = ev.time;
                            tapped_outside_numlock_bbox = false;
                        }
                        // TODO: Support calc key (top left)
                        if layout.in_numpad_bbox(pos_x, pos_y) {
                            finger_state = FingerState::Tapping;
                        } else {
                            tapped_outside_numlock_bbox = true
                        }
                    }
                }
                EventCode::EV_MSC(EV_MSC::MSC_TIMESTAMP) => {
                    // The toggle should happen automatically after HOLD_DURATION, even if user is
                    // still touching the numpad bbox.
                    if finger_state == FingerState::Tapping && !tapped_outside_numlock_bbox {
                        if layout.in_numpad_bbox(pos_x, pos_y) {
                            if ev.time.elapsed_since(tap_started_at) >= HOLD_DURATION {
                                toggle_numlock(&mut numlock, &mut touchpad_dev);
                                // If user doesn't lift the finger quickly, we don't want to keep
                                // toggling, so assume finger was moved. 
                                // Can't do finger_state = Lifted, since that would start another tap
                                // Can't do finger_state = Touching, since that would cause numpad
                                // keypresses (we don't check for margins in layout.get_key yet)
                                tapped_outside_numlock_bbox = true;
                            }
                        } else {
                            tapped_outside_numlock_bbox = true;
                        }
                    }
                }
                _ => (),
            }
            if numlock && finger_state == FingerState::Touching {
                cur_key = layout.get_key(pos_x, pos_y);
                if let Some(key) = cur_key {
                    finger_state = FingerState::Tapping;
                    if layout.needs_multikey(key) {
                        kb.multi_keydown(&layout.multikeys(key));
                    } else {
                        kb.keydown(key);
                    }
                }
            }
        }
    }
}
