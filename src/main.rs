#![feature(iter_advance_by)]
#![feature(with_options)]

mod devices;
mod dummy_keyboard;
mod numpad_layout;

use dummy_keyboard::KeyEvents;
use evdev_rs::enums::EV_KEY;
use numpad_layout::NumpadLayout;

fn main() {
    let (_, _, _) = devices::read_proc_input().expect("Couldn't get proc input devices");
    let layout = NumpadLayout::ux433fa();
    let kb = dummy_keyboard::DummyKeyboard::new(&layout);
    kb.keydown(EV_KEY::KEY_KPASTERISK);
}
