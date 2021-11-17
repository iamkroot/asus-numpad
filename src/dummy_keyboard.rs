use evdev_rs::{
    enums::{EventCode, EV_KEY, EV_SYN},
    DeviceWrapper, InputEvent, TimeVal, UInputDevice, UninitDevice,
};

use crate::numpad_layout::NumpadLayout;

pub(crate) struct DummyKeyboard {
    pub(crate) udev: UInputDevice,
}

impl std::fmt::Debug for DummyKeyboard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DummyKeyboard")
            .field("udev", &self.udev.devnode())
            .finish()
    }
}

impl DummyKeyboard {
    pub(crate) fn new<Layout: NumpadLayout>(layout: &Layout) -> Self {
        let dev = UninitDevice::new().expect("No libevdev");
        dev.set_name("asus_touchpad");
        let default_keys = [EV_KEY::KEY_LEFTSHIFT, EV_KEY::KEY_NUMLOCK, EV_KEY::KEY_CALC];
        for key in default_keys {
            dev.enable(&EventCode::EV_KEY(key))
                .expect("Unable to enable key");
        }
        for row in layout.keys().iter() {
            for key in row {
                dev.enable(&EventCode::EV_KEY(*key))
                    .expect("Unable to enable key");
            }
        }
        Self {
            udev: UInputDevice::create_from_device(&dev).expect("Unable to create UInput"),
        }
    }
}

pub(crate) trait KeyEvents {
    fn keydown(&self, key: EV_KEY);
    fn keyup(&self, key: EV_KEY);
    fn multi_keydown(&self, keys: &[EV_KEY]);
    fn multi_keyup(&self, keys: &[EV_KEY]);

    fn keypress(&self, key: EV_KEY) {
        self.keydown(key);
        self.keyup(key);
    }
    fn multi_keypress(&self, keys: &[EV_KEY]) {
        self.multi_keydown(keys);
        self.multi_keyup(keys);
    }
    const KEYDOWN: i32 = 1;
    const KEYUP: i32 = 0;
    const DUMMY_TIMEVAL: TimeVal = TimeVal {
        tv_sec: 0,
        tv_usec: 0,
    };
}

impl KeyEvents for DummyKeyboard {
    fn keydown(&self, key: EV_KEY) {
        self.udev
            .write_event(&InputEvent::new(
                &Self::DUMMY_TIMEVAL,
                &EventCode::EV_KEY(key),
                Self::KEYDOWN,
            ))
            .expect("Couldn't send keydown");
        self.udev
            .write_event(&InputEvent::new(
                &Self::DUMMY_TIMEVAL,
                &EventCode::EV_SYN(EV_SYN::SYN_REPORT),
                0,
            ))
            .expect("No syn");
    }

    fn keyup(&self, key: EV_KEY) {
        self.udev
            .write_event(&InputEvent::new(
                &Self::DUMMY_TIMEVAL,
                &EventCode::EV_KEY(key),
                Self::KEYUP,
            ))
            .expect("Couldn't send keyup");
        self.udev
            .write_event(&InputEvent::new(
                &Self::DUMMY_TIMEVAL,
                &EventCode::EV_SYN(EV_SYN::SYN_REPORT),
                0,
            ))
            .expect("No syn");
    }

    fn multi_keydown(&self, keys: &[EV_KEY]) {
        for key in keys {
            self.udev
                .write_event(&InputEvent::new(
                    &Self::DUMMY_TIMEVAL,
                    &EventCode::EV_KEY(*key),
                    Self::KEYDOWN,
                ))
                .expect("Couldn't send keydown");
        }
        self.udev
            .write_event(&InputEvent::new(
                &Self::DUMMY_TIMEVAL,
                &EventCode::EV_SYN(EV_SYN::SYN_REPORT),
                0,
            ))
            .expect("No syn");
    }

    fn multi_keyup(&self, keys: &[EV_KEY]) {
        for key in keys {
            self.udev
                .write_event(&InputEvent::new(
                    &Self::DUMMY_TIMEVAL,
                    &EventCode::EV_KEY(*key),
                    Self::KEYUP,
                ))
                .expect("Couldn't send keyup");
        }
        self.udev
            .write_event(&InputEvent::new(
                &Self::DUMMY_TIMEVAL,
                &EventCode::EV_SYN(EV_SYN::SYN_REPORT),
                0,
            ))
            .expect("No syn");
    }
}
