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
pub struct BBox {
    minx: f32,
    maxx: f32,
    miny: f32,
    maxy: f32,
}

impl BBox {
    pub fn new(minx: f32, maxx: f32, miny: f32, maxy: f32) -> Self {
        Self {
            minx,
            maxx,
            miny,
            maxy,
        }
    }

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
    numlock_bbox: BBox,
    calc_bbox: BBox,
}

pub const LAYOUT_NAMES: [&str; 4] = ["ux433fa", "m433ia", "ux581", "gx701"];

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
        debug_assert_eq!(self.rows, self.keys.len());
        self.rows
    }

    pub fn cols(&self) -> usize {
        debug_assert_eq!(self.cols, self.keys[0].len());
        self.cols
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
        self.numlock_bbox.contains(posx, posy)
    }

    pub fn in_calc_bbox(&self, posx: f32, posy: f32) -> bool {
        self.calc_bbox.contains(posx, posy)
    }

    pub fn ux433fa(bbox: BBox) -> Self {
        use EV_KEY::*;
        Self {
            cols: 5,
            rows: 4,
            keys: vec![
                vec![KEY_KP7, KEY_KP8, KEY_KP9, KEY_KPSLASH, KEY_BACKSPACE],
                vec![KEY_KP4, KEY_KP5, KEY_KP6, KEY_KPASTERISK, KEY_BACKSPACE],
                vec![KEY_KP1, KEY_KP2, KEY_KP3, KEY_KPMINUS, KEY_KPENTER],
                vec![KEY_KP0, KEY_KP0, KEY_KPDOT, KEY_KPPLUS, KEY_KPENTER],
            ],
            numpad_bbox: bbox.apply_margins(&Margins {
                top: 0.1,
                bottom: 0.025,
                left: 0.05,
                right: 0.05,
            }),
            numlock_bbox: bbox.apply_margins(&Margins {
                top: 0.0,
                bottom: 0.91,
                left: 0.95,
                right: 0.0,
            }),
            calc_bbox: bbox.apply_margins(&Margins {
                top: 0.0,
                bottom: 0.91,
                left: 0.0,
                right: 0.95,
            }),
            bbox,
        }
    }

    pub fn m433ia(bbox: BBox) -> Self {
        use EV_KEY::*;
        Self {
            cols: 5,
            rows: 4,
            keys: vec![
                vec![KEY_KP7, KEY_KP8, KEY_KP9, KEY_KPSLASH, KEY_BACKSPACE],
                vec![KEY_KP4, KEY_KP5, KEY_KP6, KEY_KPASTERISK, KEY_BACKSPACE],
                vec![KEY_KP1, KEY_KP2, KEY_KP3, KEY_KPMINUS, KEY_5],
                vec![KEY_KP0, KEY_KPDOT, KEY_KPENTER, KEY_KPPLUS, KEY_EQUAL],
            ],
            numpad_bbox: bbox.apply_margins(&Margins {
                top: 0.1,
                bottom: 0.025,
                left: 0.05,
                right: 0.05,
            }),
            numlock_bbox: bbox.apply_margins(&Margins {
                top: 0.0,
                bottom: 0.91,
                left: 0.95,
                right: 0.0,
            }),
            calc_bbox: bbox.apply_margins(&Margins {
                top: 0.0,
                bottom: 0.91,
                left: 0.0,
                right: 0.95,
            }),
            bbox,
        }
    }

    pub fn ux581(bbox: BBox) -> Self {
        use EV_KEY::*;

        Self {
            cols: 4,
            rows: 5,
            keys: vec![
                vec![KEY_KPEQUAL, KEY_5, KEY_BACKSPACE, KEY_BACKSPACE],
                vec![KEY_KP7, KEY_KP8, KEY_KP9, KEY_KPSLASH],
                vec![KEY_KP4, KEY_KP5, KEY_KP6, KEY_KPASTERISK],
                vec![KEY_KP1, KEY_KP2, KEY_KP3, KEY_KPMINUS],
                vec![KEY_KP0, KEY_KPDOT, KEY_KPENTER, KEY_KPPLUS],
            ],
            numpad_bbox: bbox.apply_margins(&Margins {
                top: 0.1,
                bottom: 0.025,
                left: 0.025,
                right: 0.025,
            }),
            numlock_bbox: bbox.apply_margins(&Margins {
                top: 0.0,
                bottom: 0.91,
                left: 0.95,
                right: 0.0,
            }),
            calc_bbox: bbox.apply_margins(&Margins {
                top: 0.0,
                bottom: 0.91,
                left: 0.0,
                right: 0.95,
            }),
            bbox,
        }
    }

    pub fn gx701(bbox: BBox) -> Self {
        use EV_KEY::*;

        Self {
            cols: 4,
            rows: 5,
            keys: vec![
                vec![KEY_KPEQUAL, KEY_5, KEY_BACKSPACE, KEY_BACKSPACE],
                vec![KEY_KP7, KEY_KP8, KEY_KP9, KEY_KPSLASH],
                vec![KEY_KP4, KEY_KP5, KEY_KP6, KEY_KPASTERISK],
                vec![KEY_KP1, KEY_KP2, KEY_KP3, KEY_KPMINUS],
                vec![KEY_KP0, KEY_KPDOT, KEY_KPENTER, KEY_KPPLUS],
            ],
            numpad_bbox: bbox.apply_margins(&Margins {
                top: 0.025,
                bottom: 0.025,
                left: 0.025,
                right: 0.025,
            }),
            // these bboxes aren't present on this model.
            // set to values outside the actual touchpad bbox.
            // this way, they will never be activated.
            numlock_bbox: BBox {
                minx: bbox.maxx + 1.0,
                maxx: bbox.maxx + 2.0,
                miny: bbox.maxy + 1.0,
                maxy: bbox.maxy + 2.0,
            },
            calc_bbox: BBox {
                minx: bbox.maxx + 1.0,
                maxx: bbox.maxx + 2.0,
                miny: bbox.maxy + 1.0,
                maxy: bbox.maxy + 2.0,
            },
            bbox,
        }
    }
}
