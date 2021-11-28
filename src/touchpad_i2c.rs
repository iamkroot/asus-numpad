use i2cdev::core::I2CDevice;
use i2cdev::linux::LinuxI2CDevice;

use crate::Brightness;

pub struct TouchpadI2C {
    dev: LinuxI2CDevice,
}

impl TouchpadI2C {
    pub fn new(i2c_id: u32) -> Self {
        const TOUCHPAD_ADDR: u16 = 0x15;
        let dev =
            unsafe { LinuxI2CDevice::force_new(format!("/dev/i2c-{}", i2c_id), TOUCHPAD_ADDR) };
        let dev = dev.expect("Failed to open Touchpad I2C. Is i2c-dev kernel module loaded?");
        Self { dev }
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
