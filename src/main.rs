#![feature(iter_advance_by)]
#![feature(with_options)]

mod devices;
mod dummy_keyboard;
mod numpad_layout;

use dummy_keyboard::KeyEvents;
use evdev_rs::{
    enums::{EventCode, EV_ABS, EV_KEY},
    Device, DeviceWrapper, GrabMode, ReadFlag,
};
use numpad_layout::NumpadLayout;

use crate::devices::{open_input_evdev, read_proc_input};

#[derive(PartialEq)]
enum FingerState {
    Lifted,
    Touching,
    Tapping,
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

fn main() {
    let (_, _, i2c) = devices::read_proc_input().expect("Couldn't get proc input devices");
    let layout = NumpadLayout::um425();
    let kb = dummy_keyboard::DummyKeyboard::new(&layout);
    let (_, touchpad_ev_id, _) = read_proc_input().expect("ADSF");
    let mut touchpad_dev = open_input_evdev(touchpad_ev_id);
    fn get_minmax(dev: &Device, code: EV_ABS) -> (f32, f32) {
        let abs = dev.abs_info(&EventCode::EV_ABS(code)).expect("MAX");
        (abs.minimum as f32, abs.maximum as f32)
    }
    let (_minx, maxx) = get_minmax(&touchpad_dev, EV_ABS::ABS_X);
    let (_miny, maxy) = get_minmax(&touchpad_dev, EV_ABS::ABS_Y);

    let mut pos_x = 0.0;
    let mut pos_y = 0.0;
    let mut numlock: bool = false;
    let mut cur_key: Option<EV_KEY> = None;
    let mut finger_state = FingerState::Lifted;
    // TODO: Support percentage key
    // TODO: Support calc key (top left)
    loop {
        let ev = touchpad_dev
            .next_event(ReadFlag::NORMAL | ReadFlag::BLOCKING)
            .map(|val| val.1);
        if let Ok(ev) = ev {
            match ev.event_code {
                evdev_rs::enums::EventCode::EV_ABS(evdev_rs::enums::EV_ABS::ABS_MT_POSITION_X) => {
                    pos_x = ev.value as f32;
                    continue;
                }
                evdev_rs::enums::EventCode::EV_ABS(evdev_rs::enums::EV_ABS::ABS_MT_POSITION_Y) => {
                    pos_y = ev.value as f32;
                    continue;
                }
                evdev_rs::enums::EventCode::EV_KEY(evdev_rs::enums::EV_KEY::BTN_TOOL_FINGER) => {
                    if ev.value == 0 {
                        // end of tap
                        finger_state = FingerState::Lifted;
                        if let Some(key) = cur_key {
                            kb.keyup(key);
                            cur_key = None;
                        }
                    } else if ev.value == 1 {
                        if finger_state == FingerState::Lifted {
                            // start of tap
                            finger_state = FingerState::Touching;
                        }
                        if pos_x > 0.95 * (maxx) && pos_y < 0.09 * maxy {
                            finger_state = FingerState::Lifted;
                            toggle_numlock(&mut numlock, &mut touchpad_dev);
                        }
                    }
                }
                _ => (),
            }
            if !numlock {
                continue;
            }
            if finger_state == FingerState::Touching {
                finger_state = FingerState::Tapping;
                let rows = layout.keys.len() as f32;
                let cols = layout.keys.first().unwrap().len() as f32;
                let col = (cols * pos_x / maxx) as isize;
                let row = ((rows * pos_y / maxy) - layout.top_offset) as isize;
                if row < 0 {
                    continue;
                }
                cur_key = Some(layout.keys[row as usize][col as usize]);
                kb.keydown(cur_key.unwrap());
            }
        }
    }
}
