use std::fmt::Debug;

use i2cdev::core::I2CDevice;
use i2cdev::linux::LinuxI2CDevice;

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

    pub fn cycle(&mut self) -> Self {
        *self = self.next();
        *self
    }
}

pub struct TouchpadI2C {
    dev: LinuxI2CDevice,
    i2c_id: u32,
}

impl TouchpadI2C {
    pub fn new(i2c_id: u32) -> Self {
        const TOUCHPAD_ADDR: u16 = 0x15;
        let dev =
            unsafe { LinuxI2CDevice::force_new(format!("/dev/i2c-{}", i2c_id), TOUCHPAD_ADDR) };
        let dev = dev.expect("Failed to open Touchpad I2C. Is i2c-dev kernel module loaded?");
        Self { dev, i2c_id }
    }

    pub fn set_brightness(&mut self, brightness: Brightness) {
        let msg = [
            0x05,
            0x00,
            0x3d,
            0x03,
            0x06,
            0x00,
            0x07,
            0x00,
            0x0d,
            0x14,
            0x03,
            brightness as u8,
            0xad,
        ];
        self.dev.write(&msg).expect("could not set brightness");
    }
}

impl Debug for TouchpadI2C {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("TouchpadI2C: /dev/i2c-{}", self.i2c_id))
    }
}
