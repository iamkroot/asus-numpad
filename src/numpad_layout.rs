use std::fmt::Debug;
use std::hint::unreachable_unchecked;

use anyhow::Result;
use evdev_rs::enums::EV_KEY;
use serde::{Deserialize, Serialize};

use crate::Point;

#[derive(Debug, Default, Clone, Copy)]
struct Margins {
    top: f32,
    bottom: f32,
    left: f32,
    right: f32,
}

#[derive(Debug)]
pub struct BBox {
    minx: i32,
    maxx: i32,
    miny: i32,
    maxy: i32,
}

impl BBox {
    pub fn new(minx: i32, maxx: i32, miny: i32, maxy: i32) -> Self {
        Self {
            minx,
            maxx,
            miny,
            maxy,
        }
    }

    fn xrange(&self) -> i32 {
        self.maxx - self.minx
    }

    fn yrange(&self) -> i32 {
        self.maxy - self.miny
    }

    fn apply_margins(&self, margins: Margins) -> Self {
        let xrange = self.xrange() as f32;
        let yrange = self.yrange() as f32;
        BBox {
            minx: self.minx + (margins.left * xrange) as i32,
            maxx: self.maxx - (margins.right * xrange) as i32,
            miny: self.miny + (margins.top * yrange) as i32,
            maxy: self.maxy - (margins.bottom * yrange) as i32,
        }
    }

    /// Return a new BBox that is non-intersecting with self.
    /// Used for creating dummy boxes.
    fn disjoint_dummy(&self) -> Self {
        Self {
            minx: self.maxx + 1,
            maxx: self.maxx + 2,
            miny: self.maxy + 1,
            maxy: self.maxy + 2,
        }
    }

    fn contains(&self, pos: Point) -> bool {
        (self.minx <= pos.x && pos.x <= self.maxx) && (self.miny <= pos.y && pos.y <= self.maxy)
    }
}

type Grid = Vec<Vec<EV_KEY>>;

#[derive(Debug)]
pub(crate) struct NumpadLayout {
    /// The matrix of keys
    keys: Grid,
    numpad_bbox: BBox,
    numlock_bbox: BBox,
    calc_bbox: BBox,
    /// The width of one numpad button/key box
    key_width: i32,
    /// The height of one numpad button/key box
    key_height: i32,
}

#[derive(Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub(crate) enum SupportedLayout {
    UX433FA,
    M433IA,
    UX581,
    GX701,
    GX531,
    UM5302TA,
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

    /// Get the key at (posx, posy), if it exists
    pub fn get_key(&self, pos: Point) -> Option<EV_KEY> {
        let bbox = &self.numpad_bbox;
        if !bbox.contains(pos) {
            return None;
        }
        let col = ((pos.x - bbox.minx) / self.key_width) as usize;
        let row = ((pos.y - bbox.miny) / self.key_height) as usize;
        // Safety: We have already checked that bbox contains the point
        let key = unsafe { self.keys().get_unchecked(row).get_unchecked(col) };
        Some(*key)
    }

    pub fn _in_margins(&self, pos: Point) -> bool {
        !self.numpad_bbox.contains(pos)
    }

    pub fn in_numlock_bbox(&self, pos: Point) -> bool {
        self.numlock_bbox.contains(pos)
    }

    pub fn in_calc_bbox(&self, pos: Point) -> bool {
        self.calc_bbox.contains(pos)
    }

    fn create(keys: Grid, numpad_bbox: BBox, numlock_bbox: BBox, calc_bbox: BBox) -> Self {
        let key_width = numpad_bbox.xrange() / keys[0].len() as i32;
        let key_height = numpad_bbox.yrange() / keys.len() as i32;
        Self {
            keys,
            numpad_bbox,
            numlock_bbox,
            calc_bbox,
            key_width,
            key_height,
        }
    }

    pub fn ux433fa(bbox: BBox) -> Self {
        use EV_KEY::*;
        Self::create(
            vec![
                vec![KEY_KP7, KEY_KP8, KEY_KP9, KEY_KPSLASH, KEY_BACKSPACE],
                vec![KEY_KP4, KEY_KP5, KEY_KP6, KEY_KPASTERISK, KEY_BACKSPACE],
                vec![KEY_KP1, KEY_KP2, KEY_KP3, KEY_KPMINUS, KEY_KPENTER],
                vec![KEY_KP0, KEY_KP0, KEY_KPDOT, KEY_KPPLUS, KEY_KPENTER],
            ],
            bbox.apply_margins(Margins {
                top: 0.1,
                bottom: 0.025,
                left: 0.05,
                right: 0.05,
            }),
            bbox.apply_margins(Margins {
                top: 0.0,
                bottom: 0.91,
                left: 0.95,
                right: 0.0,
            }),
            bbox.apply_margins(Margins {
                top: 0.0,
                bottom: 0.91,
                left: 0.0,
                right: 0.95,
            }),
        )
    }

    pub fn m433ia(bbox: BBox) -> Self {
        use EV_KEY::*;
        Self::create(
            vec![
                vec![KEY_KP7, KEY_KP8, KEY_KP9, KEY_KPSLASH, KEY_BACKSPACE],
                vec![KEY_KP4, KEY_KP5, KEY_KP6, KEY_KPASTERISK, KEY_BACKSPACE],
                vec![KEY_KP1, KEY_KP2, KEY_KP3, KEY_KPMINUS, KEY_5],
                vec![KEY_KP0, KEY_KPDOT, KEY_KPENTER, KEY_KPPLUS, KEY_EQUAL],
            ],
            bbox.apply_margins(Margins {
                top: 0.1,
                bottom: 0.025,
                left: 0.05,
                right: 0.05,
            }),
            bbox.apply_margins(Margins {
                top: 0.0,
                bottom: 0.91,
                left: 0.95,
                right: 0.0,
            }),
            bbox.apply_margins(Margins {
                top: 0.0,
                bottom: 0.91,
                left: 0.0,
                right: 0.95,
            }),
        )
    }

    pub fn ux581(bbox: BBox) -> Self {
        use EV_KEY::*;
        Self::create(
            vec![
                vec![KEY_KPEQUAL, KEY_5, KEY_BACKSPACE, KEY_BACKSPACE],
                vec![KEY_KP7, KEY_KP8, KEY_KP9, KEY_KPSLASH],
                vec![KEY_KP4, KEY_KP5, KEY_KP6, KEY_KPASTERISK],
                vec![KEY_KP1, KEY_KP2, KEY_KP3, KEY_KPMINUS],
                vec![KEY_KP0, KEY_KPDOT, KEY_KPENTER, KEY_KPPLUS],
            ],
            bbox.apply_margins(Margins {
                top: 0.1,
                bottom: 0.025,
                left: 0.025,
                right: 0.025,
            }),
            bbox.apply_margins(Margins {
                top: 0.0,
                bottom: 0.91,
                left: 0.95,
                right: 0.0,
            }),
            bbox.apply_margins(Margins {
                top: 0.0,
                bottom: 0.91,
                left: 0.0,
                right: 0.95,
            }),
        )
    }

    pub fn gx701(bbox: BBox) -> Self {
        use EV_KEY::*;
        Self::create(
            vec![
                vec![KEY_CALC, KEY_KPSLASH, KEY_KPASTERISK, KEY_KPMINUS],
                vec![KEY_KP7, KEY_KP8, KEY_KP9, KEY_KPPLUS],
                vec![KEY_KP4, KEY_KP5, KEY_KP6, KEY_KPPLUS],
                vec![KEY_KP1, KEY_KP2, KEY_KP3, KEY_KPENTER],
                vec![KEY_KP0, KEY_KP0, KEY_KPDOT, KEY_KPENTER],
            ],
            bbox.apply_margins(Margins {
                top: 0.025,
                bottom: 0.025,
                left: 0.025,
                right: 0.025,
            }),
            // these bboxes aren't present on this model.
            // set to values outside the actual touchpad bbox.
            // this way, they will never be activated.
            bbox.disjoint_dummy(),
            bbox.disjoint_dummy(),
        )
    }

    pub fn gx531(bbox: BBox) -> Self {
        use EV_KEY::*;
        Self::create(
            vec![
                vec![KEY_BACKSLASH, KEY_KPSLASH, KEY_KPASTERISK, KEY_KPMINUS],
                vec![KEY_KP7, KEY_KP8, KEY_KP9, KEY_KPPLUS],
                vec![KEY_KP4, KEY_KP5, KEY_KP6, KEY_KPPLUS],
                vec![KEY_KP1, KEY_KP2, KEY_KP3, KEY_KPENTER],
                vec![KEY_KP0, KEY_KP0, KEY_KPDOT, KEY_KPENTER],
            ],
            bbox.apply_margins(Margins {
                top: 0.005,
                bottom: 0.005,
                left: 0.005,
                right: 0.005,
            }),
            // these bboxes aren't present on this model.
            // set to values outside the actual touchpad bbox.
            // this way, they will never be activated.
            bbox.disjoint_dummy(),
            bbox.disjoint_dummy(),
        )
    }

    pub fn um5302ta(bbox: BBox) -> Self {
        use EV_KEY::*;
        Self::create(
            vec![
                vec![KEY_KP7, KEY_KP8, KEY_KP9, KEY_KPSLASH, KEY_BACKSPACE],
                vec![KEY_KP4, KEY_KP5, KEY_KP6, KEY_KPASTERISK, KEY_BACKSPACE],
                vec![KEY_KP1, KEY_KP2, KEY_KP3, KEY_KPMINUS, KEY_5],
                vec![KEY_KP0, KEY_KPDOT, KEY_KPENTER, KEY_KPPLUS, KEY_EQUAL],
            ],
            bbox.apply_margins(Margins {
                top: 0.1,
                bottom: 0.025,
                left: 0.05,
                right: 0.05,
            }),
            bbox.apply_margins(Margins {
                top: 0.0,
                bottom: 0.91,
                left: 0.95,
                right: 0.0,
            }),
            bbox.apply_margins(Margins {
                top: 0.0,
                bottom: 0.91,
                left: 0.0,
                right: 0.95,
            }),
        )
    }

    pub(crate) fn from_supported_layout(layout: &SupportedLayout, bbox: BBox) -> Result<Self> {
        use SupportedLayout::*;
        let layout = match layout {
            UX433FA => Self::ux433fa(bbox),
            M433IA => Self::m433ia(bbox),
            UX581 => Self::ux581(bbox),
            GX701 => Self::gx701(bbox),
            GX531 => Self::gx531(bbox),
            UM5302TA => Self::um5302ta(bbox),
        };
        Ok(layout)
    }
}
