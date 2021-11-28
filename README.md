# Asus Touchpad Numpad Driver

Linux tool to allow using the numpad that is overlayed on various Asus Laptop touchpads.

## Features

The following features are planned and implemented for the app:
* [x] Support UX433FA and M433IA
* [x] Hold the numpad button to toggle it
* [x] Fast response to touches
* [x] Use [i2cdev](https://crates.io/crates/i2cdev) crate for setting brightness
* [x] Handle Calc Key
* [x] Cycle through multiple brightness options
* [x] Ignore touches in margins
* [x] Support more numpad layouts (UX581 and GX701)
* [ ] Set model via program argument
* [ ] Autodetect laptop model
* [ ] Run without `sudo` (is it even possible?)
* [ ] Systemd service to enable autostart
* [ ] Integration with system's NumLock state (toggle with external keyboards)
* [ ] Logging for debugging purposes

## Installation
* Install `libevdev`
    * Debian / Ubuntu / Linux Mint / Pop!\_OS / Zorin OS: `sudo apt install libevdev2`
    * Arch Linux / Manjaro: `sudo pacman -S libevdev`
    * Fedora: `sudo dnf install libevdev`

* Install the Rust 2021 toolchain using [`Rustup`](https://rustup.rs).
* `cargo install --git="https://github.com/iamkroot/asus-numpad"`

## Run
* `sudo modprobe i2c-dev`
    * Loads the I2C module (allows controlling numpad brightness).
    * You can have it be loaded automatically at boot. Consult [ArchWiki](https://wiki.archlinux.org/title/Kernel_module#Automatic_module_loading_with_systemd) for details
* `sudo ~/.cargo/bin/asus-numpad`

## Acknowledgements
* This is a rewrite of [asus-touchpad-numpad-driver](https://github.com/mohamed-badaoui/asus-touchpad-numpad-driver) 
