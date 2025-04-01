use anyhow::{Context, Result, anyhow};
use evdev_rs::{
    Device, DeviceWrapper,
    enums::{EV_ABS, EventCode},
};
use std::{fs::OpenOptions, os::unix::prelude::OpenOptionsExt};

use crate::numpad_layout::BBox;

fn parse_id(line: &str, search_str: &str) -> Result<u32> {
    let pos = line
        .find(search_str)
        .ok_or_else(|| anyhow!("Can't find token {} in {}", search_str, line))?;
    let start_idx = pos + search_str.len();

    if start_idx >= line.len() {
        return Err(anyhow!("Search string is at the end of the line"));
    }

    let end_idx = start_idx
        + line[start_idx..]
            .chars()
            .position(|c| !c.is_numeric())
            .unwrap_or(line.len() - start_idx);

    if start_idx == end_idx {
        return Err(anyhow!("No numeric characters found"));
    }

    Ok(line[start_idx..end_idx].parse()?)
}

/// Parse `/proc/bus/input/devices` to find the keyboard and touchpad devices.
/// Returns the evdev handles for keybard and touchpad, along with I2C ID of touchpad.
pub(crate) fn read_proc_input() -> Result<(u32, u32, u32)> {
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

    let data = std::fs::read_to_string("/proc/bus/input/devices")
        .context("Could not read devices file")?;

    for line in data.lines() {
        match touchpad_detection {
            // FIXME: Should give priority to ASUE,ASUP etc. before ELAN
            // In general, need to parse the entire thing, and use proper filtering
            Detection::NotDetected => {
                if line.starts_with("N:")
                    && (line.contains("Name=\"ASUE")
                        || line.contains("Name=\"ELAN")
                        || line.contains("Name=\"ASUP")
                        || line.contains("Name=\"ASCP"))
                    && line.contains("Touchpad")
                {
                    touchpad_detection = Detection::Parsing;
                    continue;
                }
            }
            Detection::Parsing => {
                if line.starts_with("S:") {
                    touchpad_i2c_id =
                        Some(parse_id(line, "i2c-").context("Could not parse u32 ID")?);
                    continue;
                } else if line.starts_with("H:") {
                    touchpad_ev_id =
                        Some(parse_id(line, "event").context("Could not parse u32 ID")?);
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
                if line.starts_with("N:")
                    && (line.contains("Name=\"AT Translated Set 2 keyboard")
                        || ((line.contains("Name=\"ASUE") || line.contains("Name=\"Asus"))
                            && line.contains("Keyboard")))
                {
                    keyboard_detection = Detection::Parsing;
                    continue;
                }
            }
            Detection::Parsing => {
                if line.starts_with("H:") {
                    keyboard_ev_id =
                        Some(parse_id(line, "event").context("Could not parse u32 ID")?);
                    // TODO: We should verify that the device actually supports KEY_NUMLOCK using evdev
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
        keyboard_ev_id.ok_or(anyhow!("Can't find keyboard evdev"))?,
        touchpad_ev_id.ok_or(anyhow!("Can't find touchpad evdev"))?,
        touchpad_i2c_id.ok_or(anyhow!("Can't find touchpad I2C ID"))?,
    ))
}

pub(crate) fn open_input_evdev(evdev_id: u32) -> Result<Device> {
    let path = format!("/dev/input/event{}", evdev_id);
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(&path)
        .with_context(|| path.clone())
        .context("Couldn't open device event handle")?;
    Device::new_from_file(file)
        .with_context(|| path)
        .context("Unable to open evdev device")
}

pub(crate) fn get_touchpad_bbox(touchpad_evdev: &Device) -> Result<BBox> {
    let absx = touchpad_evdev
        .abs_info(&EventCode::EV_ABS(EV_ABS::ABS_X))
        .ok_or(anyhow!("Could not get touchpad max x"))?;
    let absy = touchpad_evdev
        .abs_info(&EventCode::EV_ABS(EV_ABS::ABS_Y))
        .ok_or(anyhow!("Could not get touchpad max y"))?;
    Ok(BBox::new(
        absx.minimum,
        absx.maximum,
        absy.minimum,
        absy.maximum,
    ))
}
