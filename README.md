# Asus Touchpad Numpad Driver

Linux tool to allow using the numpad that is overlayed on various Asus Laptop touchpads.

## Features

The following features are planned and implemented for the app:
* [x] Compile time specification of numpad layouts
* [x] Support UX433FA and M433IA
* [x] Hold the numpad button to toggle it
* [x] Fast response to touches
* [ ] Handle Calc Key
* [ ] Ignore touches in margins
* [ ] Support vertical numpad layouts
* [ ] Cycle through multiple brightness options
* [ ] Use [i2cdev](https://crates.io/crates/i2cdev) crate (once [`force_new`](https://github.com/rust-embedded/rust-i2cdev/commit/1c2c672026cd7202ab918879883c8e60aa79c32a) becomes available)
* [ ] Run without `sudo` (is it even possible?)
* [ ] Systemd service to enable autostart
* [ ] Integration with system's NumLock state (toggle with external keyboards)
* [ ] Logging for debugging purposes

## Installation
* Install `libevdev` and `i2c-tools`
    * Debian / Ubuntu / Linux Mint / Pop!_OS / Zorin OS: `sudo apt install libevdev2 i2c-tools`
    * Arch Linux / Manjaro: `sudo pacman -S libevdev i2c-tools`
    * Fedora: `sudo dnf install libevdev i2c-tools`

* Install the Rust 2021 toolchain using [`Rustup`](https://rustup.rs).
* `cargo install --git="https://github.com/iamkroot/asus-numpad"`

## Run
* `sudo modprobe i2c-dev`
    * Loads the I2C module (allows controlling numpad brightness).
    * You can have it be loaded automatically at boot. Consult [ArchWiki](https://wiki.archlinux.org/title/Kernel_module#Automatic_module_loading_with_systemd) for details
* `sudo ~/.cargo/bin/asus-numpad`

## Acknowledgements
* This is a rewrite of [asus-touchpad-numpad-driver](https://github.com/mohamed-badaoui/asus-touchpad-numpad-driver) 
