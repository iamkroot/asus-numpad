use std::{fmt::Debug, hint::unreachable_unchecked};

use evdev_rs::enums::EV_KEY;

#[derive(Debug, Default, Clone, Copy)]
struct Margins {
    top: f32,
    bottom: f32,
    left: f32,
    right: f32,
}

#[derive(Debug)]
struct BBox {
    minx: f32,
    maxx: f32,
    miny: f32,
    maxy: f32,
}

impl BBox {
    fn xrange(&self) -> f32 {
        self.maxx - self.minx
    }

    fn yrange(&self) -> f32 {
        self.maxy - self.miny
    }

    fn xscaled(&self, posx: f32) -> f32 {
        (posx - self.minx) / self.xrange()
    }

    fn yscaled(&self, posy: f32) -> f32 {
        (posy - self.minx) / self.yrange()
    }

    fn apply_margins(&self, margins: &Margins) -> Self {
        let xrange = self.xrange();
        let yrange = self.yrange();
        BBox {
            minx: self.minx + margins.left * xrange,
            maxx: self.maxx - margins.right * xrange,
            miny: self.miny + margins.top * yrange,
            maxy: self.maxy - margins.bottom * yrange,
        }
    }

    fn contains(&self, posx: f32, posy: f32) -> bool {
        (self.minx <= posx && posx <= self.maxx) && (self.miny <= posy && posy <= self.maxy)
    }
}

type Grid = Vec<Vec<EV_KEY>>;

#[derive(Debug)]
pub(crate) struct NumpadLayout {
    cols: usize,
    rows: usize,
    /// The matrix of keys
    keys: Grid,
    bbox: BBox,
    numpad_bbox: BBox,
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
        self.bbox.maxx
    }

    pub fn maxy(&self) -> f32 {
        self.bbox.maxy
    }

    /// Get the key at (posx, posy), if it exists
    pub fn get_key(&self, posx: f32, posy: f32) -> Option<EV_KEY> {
        let x = self.numpad_bbox.xscaled(posx);
        let y = self.numpad_bbox.yscaled(posy);
        if !(0.0..=1.0).contains(&x) || !(0.0..=1.0).contains(&y) {
            // outside numpad bbox
            return None;
        }
        let row = ((self.rows() as f32) * y) as usize;
        let col = ((self.cols() as f32) * x) as usize;
        // Safety: We have constructed the row and col by scaling self.rows and self.cols
        let key = unsafe { self.keys().get_unchecked(row).get_unchecked(col) };
        Some(*key)
    }

    pub fn in_margins(&self, posx: f32, posy: f32) -> bool {
        !self.numpad_bbox.contains(posx, posy)
    }

    pub fn in_numlock_bbox(&self, posx: f32, posy: f32) -> bool {
        posx > 0.95 * self.maxx() && posy < 0.09 * self.maxy()
    }

    pub fn in_calc_bbox(&self, posx: f32, posy: f32) -> bool {
        posx < 0.06 * self.maxx() && posy < 0.09 * self.maxy()
    }

    pub fn ux433fa(minx: f32, maxx: f32, miny: f32, maxy: f32) -> Self {
        let margins = Margins {
            top: 0.1,
            bottom: 0.025,
            left: 0.05,
            right: 0.05,
        };
        let bbox = BBox {
            minx,
            maxx,
            miny,
            maxy,
        };

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
            numpad_bbox: bbox.apply_margins(&margins),
            bbox,
        }
    }

    pub fn m433ia(minx: f32, maxx: f32, miny: f32, maxy: f32) -> Self {
        let margins = Margins {
            top: 0.1,
            bottom: 0.025,
            left: 0.05,
            right: 0.05,
        };
        let bbox = BBox {
            minx,
            maxx,
            miny,
            maxy,
        };

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
            numpad_bbox: bbox.apply_margins(&margins),
            bbox,
        }
    }
}
