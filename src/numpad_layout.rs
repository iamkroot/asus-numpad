use evdev_rs::enums::EV_KEY;

#[derive(Debug)]
pub(crate) struct NumpadLayout {
    pub(crate) keys: Vec<Vec<EV_KEY>>,
    pub(crate) top_offset: f32,
}

impl NumpadLayout {
    pub(crate) fn ux433fa() -> Self {
        Self {
            keys: vec![
                vec![
                    EV_KEY::KEY_KP7,
                    EV_KEY::KEY_KP8,
                    EV_KEY::KEY_KP9,
                    EV_KEY::KEY_KPSLASH,
                    EV_KEY::KEY_BACKSPACE,
                ],
                vec![
                    EV_KEY::KEY_KP4,
                    EV_KEY::KEY_KP5,
                    EV_KEY::KEY_KP6,
                    EV_KEY::KEY_KPASTERISK,
                    EV_KEY::KEY_BACKSPACE,
                ],
                vec![
                    EV_KEY::KEY_KP1,
                    EV_KEY::KEY_KP2,
                    EV_KEY::KEY_KP3,
                    EV_KEY::KEY_KPMINUS,
                    EV_KEY::KEY_KPENTER,
                ],
                vec![
                    EV_KEY::KEY_KP0,
                    EV_KEY::KEY_KP0,
                    EV_KEY::KEY_KPDOT,
                    EV_KEY::KEY_KPPLUS,
                    EV_KEY::KEY_KPENTER,
                ],
            ],
            top_offset: 0.1,
        }
    }
}
