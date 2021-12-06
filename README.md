# Asus Touchpad Numpad Driver

Linux tool to allow using the numpad that is overlayed on various Asus Laptop touchpads.

## Features
* Supports multiple layouts (`ux433fa`, `m433ia`, `ux581`, or `gx701`). See [asus-touchpad-numpad-driver](https://github.com/mohamed-badaoui/asus-touchpad-numpad-driver) for full list
* Hold to toggle numlock/cycle brightness
* Drag to trigger calculator (similar to official driver)
* Allows using the touchpad when numlock is active
* Ignores touches in margins (outside the numpad)

## Installation
### Prerequisites
* Install `libevdev`
    * Debian / Ubuntu / Linux Mint / Pop!\_OS / Zorin OS: `sudo apt install libevdev2`
    * Arch Linux / Manjaro: `sudo pacman -S libevdev`
    * Fedora: `sudo dnf install libevdev`

### Compile from source
* Install the Rust 2021 toolchain using [`Rustup`](https://rustup.rs)
* `cargo install --git="https://github.com/iamkroot/asus-numpad"`

**OR**

### Use prebuilt binary
* Download from [`Releases`](https://github.com/iamkroot/asus-numpad/releases) page
* Copy to some directory in PATH. (Further instructions assume it is in `~/.cargo/bin/`)

## Run
* `sudo modprobe i2c-dev` and `sudo modprobe uinput`
    * You can have them be loaded automatically at boot. Consult [ArchWiki](https://wiki.archlinux.org/title/Kernel_module#Automatic_module_loading_with_systemd) for details
* `sudo ~/.cargo/bin/asus-numpad -- --layout LAYOUT` where `LAYOUT` is one of `ux433fa`, `m433ia`, `ux581`, or `gx701`.

## Running without `sudo`
1. You need to add your user to the `input` and `i2c` groups so that the program can access the touchpad events and control its brightness.
    * `sudo usermod -a -G input $(whoami)`
    * `sudo usermod -a -G i2c $(whoami)`
2. You'll also have to create a group `uinput`, add yourself to it, and add a `udev` rule to be able to create a virtual keyboard.
    * `sudo groupadd uinput`
    * `sudo usermod -a -G uinput $(whoami)`
    * `echo 'KERNEL=="uinput", GROUP="uinput", MODE:="0660"' | sudo tee /etc/udev/rules.d/99-input.rules`
3. After a reboot, check that the permissions are correct:
    * `ls -l /dev/uinput` should show `crw-rw---- 1 root uinput ... /dev/uinput` (The `uinput` after `root` is important)
    * Similarly, `ls -l /dev/i2c-*` should be owned by `i2c` group
    * Finally, `groups $(whoami)` should include `input`, `i2c` and `uinput`.

## Systemd Service
To enable autoloading at boot, a systemd service has been provided.
1. Copy [`tools/asus-numpad.service`](tools/asus-numpad.service) to a systemd directory.
    * Without `sudo`: `$HOME/.config/systemd/user/`
    * With `sudo`: `/etc/systemd/user/`
2. Edit the file and change `$HOME` to `/home/username` (expand env variable), and specify `$LAYOUT` as detailed in [Run](#Run).
3. Enable and start service:
    * Without sudo: `systemctl --user enable --now asus-numpad.service`
    * With sudo: `systemctl enable --now asus-numpad.service`

## Todo

The following features are planned and implemented for the app:
* [x] Support UX433FA and M433IA
* [x] Hold the numpad button to toggle it
* [x] Use [i2cdev](https://crates.io/crates/i2cdev) crate for setting brightness
* [x] Handle Calc Key
* [x] Cycle through multiple brightness options
* [x] Ignore touches in margins
* [x] Support more numpad layouts (UX581 and GX701)
* [x] Logging for debugging purposes
* [x] Set model via program argument
* [x] Systemd service to enable autostart
* [x] Run without `sudo`
* [x] Start Calc only on drag instead of tap
* [x] Don't panic on errors - exit gracefully
* [ ] Integration with system's NumLock state (toggle with external keyboards)
* [ ] `strip` release binaries (goes from ~5MB to ~1.5MB)
* [ ] Autodetect laptop model
* [ ] Disable numpad if idle for more than a minute

## Acknowledgements
* This is a rewrite of [asus-touchpad-numpad-driver](https://github.com/mohamed-badaoui/asus-touchpad-numpad-driver)
