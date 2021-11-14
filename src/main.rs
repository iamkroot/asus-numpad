#![feature(iter_advance_by)]
#![feature(with_options)]

mod devices;
mod numpad_layout;

fn main() {
    let (_, _, _) = devices::read_proc_input().expect("Couldn't get proc input devices");
    let layout = numpad_layout::NumpadLayout::ux433fa();
}
