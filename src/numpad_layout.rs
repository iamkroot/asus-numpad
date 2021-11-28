use std::{fmt::Debug, hint::unreachable_unchecked};

use evdev_rs::enums::EV_KEY;

#[derive(Debug, Default, Clone, Copy)]
struct Margins {
    top: f32,
    bottom: f32,
    left: f32,
    right: f32,
}

type Grid = Vec<Vec<EV_KEY>>;

#[derive(Debug)]
pub(crate) struct NumpadLayout {
    cols: usize,
    rows: usize,
    /// The matrix of keys
    keys: Grid,
    top_offset: f32,
    maxx: f32,
    maxy: f32,
    // margins: Margins,
}

impl NumpadLayout {
    /// Get a reference to the numpad layout's keys.
    pub fn keys(&self) -> &Grid {
        self.keys.as_ref()
    }

    pub fn needs_multikey(&self, key: EV_KEY) -> bool {
        key == EV_KEY::KEY_5
    }

    pub fn multikeys(&self, key: EV_KEY) -> [EV_KEY; 2] {
        match key {
            EV_KEY::KEY_5 => [EV_KEY::KEY_LEFTSHIFT, EV_KEY::KEY_5],
            // Safety: We know this method will only be called after
            // needs_multikey returns true
            _ => unsafe { unreachable_unchecked() },
        }
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn maxx(&self) -> f32 {
        self.maxx
    }

    pub fn maxy(&self) -> f32 {
        self.maxy
    }

    pub fn top_offset(&self) -> f32 {
        self.top_offset
    }

    /// Get the key at (posx, posy), if it exists
    pub fn get_key(&self, posx: f32, posy: f32) -> Option<EV_KEY> {
        if self.in_margins(posx, posy) {
            return None;
        }
        // TODO: Use margins to crop the maxx and maxy
        let row = ((self.rows() as f32) * posy / self.maxy() - self.top_offset()) as isize;
        if row < 0 {
            return None;
        }
        let col = ((self.cols() as f32) * posx / self.maxx()) as isize;
        // Safety: We have constructed the row and col by scaling self.rows and self.cols
        let key = unsafe {self.keys().get_unchecked(row as usize).get_unchecked(col as usize)};
        Some(*key)
    }

    pub fn in_margins(&self, posx: f32, posy: f32) -> bool {
        // TODO: Actually check if we are in margins
        false
    }

    pub fn in_numpad_bbox(&self, posx: f32, posy: f32) -> bool {
        posx > 0.95 * self.maxx() && posy < 0.09 * self.maxy()
    }

    pub fn in_calc_bbox(&self, posx: f32, posy: f32) -> bool {
        posx < 0.06 * self.maxx() && posy < 0.09 * self.maxy()
    }

    pub fn ux433fa(_minx: f32, maxx: f32, _miny: f32, maxy: f32) -> Self {
        Self {
            cols: 5,
            rows: 4,
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
            maxx,
            maxy,
            // margins: todo!(),
        }
    }

    pub fn m433ia(_minx: f32, maxx: f32, _miny: f32, maxy: f32) -> Self {
        Self {
            cols: 5,
            rows: 4,
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
                    EV_KEY::KEY_5,
                ],
                vec![
                    EV_KEY::KEY_KP0,
                    EV_KEY::KEY_KPDOT,
                    EV_KEY::KEY_KPENTER,
                    EV_KEY::KEY_KPPLUS,
                    EV_KEY::KEY_EQUAL,
                ],
            ],
            top_offset: 0.2,
            maxx,
            maxy,
            // margins: todo!(),
        }
    }
}
