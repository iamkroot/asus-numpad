use std::{fmt::Debug, hint::unreachable_unchecked};

use evdev_rs::enums::EV_KEY;

#[derive(Debug, Default, Clone, Copy)]
struct Margins {
    top: f32,
    bottom: f32,
    left: f32,
    right: f32,
}

// TODO: Configurable percent key (depends on keyboard locale - QWERTY vs AZERTY ..)
pub(crate) trait NumpadLayout: Debug {
    /// The matrix of keys
    ///
    /// Don't need to make it generic over `Rows` and `Cols` for now
    const KEYS: [[EV_KEY; 5]; 4];
    const TOP_OFFSET: f32;
    // const MARGINS: Margins;

    fn new(minx: f32, maxx: f32, miny: f32, maxy: f32) -> Self;

    fn keys(&self) -> [[EV_KEY; 5]; 4] {
        Self::KEYS
    }

    fn needs_multikey(&self, key: EV_KEY) -> bool {
        key == EV_KEY::KEY_5
    }

    fn multikeys(&self, key: EV_KEY) -> [EV_KEY; 2] {
        match key {
            EV_KEY::KEY_5 => [EV_KEY::KEY_LEFTSHIFT, EV_KEY::KEY_5],
            // Safety: We know this method will only be called after
            // needs_multikey returns true
            _ => unsafe { unreachable_unchecked() },
        }
    }

    fn rows(&self) -> usize {
        4
    }

    fn cols(&self) -> usize {
        5
    }

    fn maxx(&self) -> f32;

    fn maxy(&self) -> f32;

    fn top_offset(&self) -> f32 {
        Self::TOP_OFFSET
    }

    /// Get the key at (posx, posy), if it exists
    fn get_key(&self, posx: f32, posy: f32) -> Option<EV_KEY> {
        if self.in_margins(posx, posy) {
            return None;
        }
        let row = ((self.rows() as f32) * posy / self.maxy() - self.top_offset()) as isize;
        if row < 0 {
            return None;
        }
        let col = ((self.cols() as f32) * posx / self.maxx()) as isize;
        Some(self.keys()[row as usize][col as usize])
    }

    fn in_margins(&self, posx: f32, posy: f32) -> bool {
        // TODO: Actually check if we are in margins
        false
    }

    fn in_numpad_bbox(&self, posx: f32, posy: f32) -> bool {
        posx > 0.95 * self.maxx() && posy < 0.09 * self.maxy()
    }
}

#[derive(Debug)]
pub(crate) struct UX433FA {
    maxx: f32,
    maxy: f32,
}

impl NumpadLayout for UX433FA {
    const KEYS: [[EV_KEY; 5]; 4] = [
        [
            EV_KEY::KEY_KP7,
            EV_KEY::KEY_KP8,
            EV_KEY::KEY_KP9,
            EV_KEY::KEY_KPSLASH,
            EV_KEY::KEY_BACKSPACE,
        ],
        [
            EV_KEY::KEY_KP4,
            EV_KEY::KEY_KP5,
            EV_KEY::KEY_KP6,
            EV_KEY::KEY_KPASTERISK,
            EV_KEY::KEY_BACKSPACE,
        ],
        [
            EV_KEY::KEY_KP1,
            EV_KEY::KEY_KP2,
            EV_KEY::KEY_KP3,
            EV_KEY::KEY_KPMINUS,
            EV_KEY::KEY_KPENTER,
        ],
        [
            EV_KEY::KEY_KP0,
            EV_KEY::KEY_KP0,
            EV_KEY::KEY_KPDOT,
            EV_KEY::KEY_KPPLUS,
            EV_KEY::KEY_KPENTER,
        ],
    ];

    const TOP_OFFSET: f32 = 0.1;

    fn new(_minx: f32, maxx: f32, _miny: f32, maxy: f32) -> Self {
        Self { maxx, maxy }
    }

    fn maxx(&self) -> f32 {
        self.maxx
    }

    fn maxy(&self) -> f32 {
        self.maxy
    }
}

#[derive(Debug)]
pub(crate) struct M433IA {
    maxx: f32,
    maxy: f32,
}

impl NumpadLayout for M433IA {
    const KEYS: [[EV_KEY; 5]; 4] = [
        [
            EV_KEY::KEY_KP7,
            EV_KEY::KEY_KP8,
            EV_KEY::KEY_KP9,
            EV_KEY::KEY_KPSLASH,
            EV_KEY::KEY_BACKSPACE,
        ],
        [
            EV_KEY::KEY_KP4,
            EV_KEY::KEY_KP5,
            EV_KEY::KEY_KP6,
            EV_KEY::KEY_KPASTERISK,
            EV_KEY::KEY_BACKSPACE,
        ],
        [
            EV_KEY::KEY_KP1,
            EV_KEY::KEY_KP2,
            EV_KEY::KEY_KP3,
            EV_KEY::KEY_KPMINUS,
            EV_KEY::KEY_5,
        ],
        [
            EV_KEY::KEY_KP0,
            EV_KEY::KEY_KPDOT,
            EV_KEY::KEY_KPENTER,
            EV_KEY::KEY_KPPLUS,
            EV_KEY::KEY_EQUAL,
        ],
    ];

    const TOP_OFFSET: f32 = 0.3;

    fn new(_minx: f32, maxx: f32, _miny: f32, maxy: f32) -> Self {
        Self { maxx, maxy }
    }

    fn maxx(&self) -> f32 {
        self.maxx
    }

    fn maxy(&self) -> f32 {
        self.maxy
    }
}
