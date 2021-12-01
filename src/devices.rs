use evdev_rs::{
    enums::{EventCode, EV_ABS},
    Device, DeviceWrapper,
};
use std::{fs::OpenOptions, os::unix::prelude::OpenOptionsExt};

use crate::numpad_layout::BBox;

fn parse_id(line: &str, search_str: &str) -> Result<u32, String> {
    let pos = line.find(search_str).ok_or("Can't find token")?;
    let start_idx = pos + search_str.len();
    let mut chars = line.chars();
    // we know chars is at least as long as start_idx
    unsafe { chars.advance_by(start_idx).unwrap_unchecked() };
    let end_idx = start_idx
        + chars
            .position(|c| !c.is_numeric())
            .expect("Reached end of line");
    let digits = line[start_idx..end_idx].parse();
    digits.map_err(|_| "No ID".to_string())
}

/// Parse `/proc/bus/input/devices` to find the keyboard and touchpad devices.
/// Returns the evdev handles for keybard and touchpad, along with I2C ID of touchpad.
pub(crate) fn read_proc_input() -> Result<(u32, u32, u32), String> {
    #[derive(Debug, PartialEq, Eq)]
    enum Detection {
        NotDetected,
        Parsing,
        Finished,
    }
    let mut touchpad_detection = Detection::NotDetected;
    let mut keyboard_detection = Detection::NotDetected;

    let mut touchpad_i2c_id: Option<u32> = None;
    let mut touchpad_ev_id: Option<u32> = None;
    let mut keyboard_ev_id: Option<u32> = None;

    let data = std::fs::read_to_string("/proc/bus/input/devices").map_err(|e| e.to_string())?;

    for line in data.lines() {
        match touchpad_detection {
            Detection::NotDetected => {
                if (line.contains("Name=\"ASUE") || line.contains("Name=\"ELAN"))
                    && line.contains("Touchpad")
                {
                    touchpad_detection = Detection::Parsing;
                    continue;
                }
            }
            Detection::Parsing => {
                if line.starts_with("S:") {
                    touchpad_i2c_id = Some(parse_id(line, "i2c-")?);
                    continue;
                } else if line.starts_with("H:") {
                    touchpad_ev_id = Some(parse_id(line, "event")?);
                    continue;
                } else if line.is_empty() {
                    // reset since we reached end of device info
                    touchpad_detection = Detection::NotDetected;
                }
                if touchpad_i2c_id.is_some() && touchpad_ev_id.is_some() {
                    touchpad_detection = Detection::Finished;
                }
            }
            _ => (),
        }

        match keyboard_detection {
            Detection::NotDetected => {
                if line.contains("Name=\"AT Translated Set 2 keyboard")
                    || line.contains("Name=\"Asus Keyboard")
                {
                    keyboard_detection = Detection::Parsing;
                    continue;
                }
            }
            Detection::Parsing => {
                if line.starts_with("H:") {
                    keyboard_ev_id = Some(parse_id(line, "event")?);
                    keyboard_detection = Detection::Finished;
                    continue;
                } else if line.is_empty() {
                    // reset since we reached end of device info
                    keyboard_detection = Detection::NotDetected;
                }
            }
            _ => (),
        }
        if touchpad_detection == Detection::Finished && keyboard_detection == Detection::Finished {
            break;
        }
    }
    Ok((
        keyboard_ev_id.ok_or("Can't find keyboard")?,
        touchpad_ev_id.ok_or("Can't find touchpad")?,
        touchpad_i2c_id.ok_or("Can't find touchpad I2C ID")?,
    ))
}

pub(crate) fn open_input_evdev(evdev_id: u32) -> Device {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(format!("/dev/input/event{}", evdev_id))
        .expect("Couldn't open device event handle");
    Device::new_from_file(file).expect("Unable to open evdev device")
}

pub(crate) fn get_touchpad_bbox(touchpad_evdev: &Device) -> BBox {
    let absx = touchpad_evdev
        .abs_info(&EventCode::EV_ABS(EV_ABS::ABS_X))
        .expect("MAX");
    let absy = touchpad_evdev
        .abs_info(&EventCode::EV_ABS(EV_ABS::ABS_Y))
        .expect("MAX");
    BBox::new(
        absx.minimum as f32,
        absx.maximum as f32,
        absy.minimum as f32,
        absy.maximum as f32,
    )
}
