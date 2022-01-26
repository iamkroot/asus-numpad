# Asus Touchpad Numpad Driver

Linux tool to allow using the numpad that is overlayed on various Asus Laptop touchpads.

## Features
This builds upon the work done in [asus-touchpad-numpad-driver](https://github.com/mohamed-badaoui/asus-touchpad-numpad-driver), and adds more goodies that bring it closer to parity with the official Windows driver-
* Hold to toggle numlock/cycle brightness
* Drag to trigger calculator (on supported models)
* Allows using the touchpad when numlock is active
* Ignores touches in margins (outside the numpad)
* Integration with system's NumLock state - toggle with external keyboards

## Installation
### Prerequisites
* Install `libevdev`
    * Debian / Ubuntu / Linux Mint / Pop!\_OS / Zorin OS: `sudo apt install libevdev2`
    * Arch Linux / Manjaro: `sudo pacman -S libevdev`
    * Fedora: `sudo dnf install libevdev`

### Use prebuilt binary
* Download from [`Releases`](https://github.com/iamkroot/asus-numpad/releases) page
* Copy to some directory in PATH. (Further instructions assume it is in `/usr/bin/`)

**OR**

### Compile from source
* Install the Rust 2021 toolchain using [`Rustup`](https://rustup.rs)
* `sudo -E cargo install --root /usr --git="https://github.com/iamkroot/asus-numpad"`

## Run
* `sudo modprobe i2c-dev` and `sudo modprobe uinput`
    * You can have them be loaded automatically at boot. Consult [ArchWiki](https://wiki.archlinux.org/title/Kernel_module#Automatic_module_loading_with_systemd) for details
* Create the config file at `/etc/xdg/asus_numpad.toml` and add `layout = "LAYOUT"`, where `LAYOUT` is one of `UX433FA`, `M433IA`, `UX581`, or `GX701`. See [Configuration](#Configuration) for more options.
* `sudo asus-numpad`

## Running without `sudo`
It is best to run this program through a separate Unix user that is allowed to access input devices.
```bash
# create a group `uinput` and add a `udev` rule for it
# needed to be able to create a dummy virtual keyboard
sudo groupadd uinput
echo 'KERNEL=="uinput", GROUP="uinput", MODE:="0660"' | sudo tee /etc/udev/rules.d/99-input.rules

# create a system user called "asus_numpad" which is a part of the required groups,
# so that the program can access the touchpad events and control its brightness
sudo useradd -Gi2c,input,uinput --no-create-home --system asus_numpad
```

After a reboot, check that the permissions are correct:
* `ls -l /dev/uinput` should show `crw-rw---- 1 root uinput ... /dev/uinput` (The `uinput` after `root` is important)
* Similarly, `ls -l /dev/i2c-*` should be owned by `i2c` group
* Finally, `groups asus_numpad` should include `input`, `i2c` and `uinput`.

## Systemd Service
To enable autoloading at boot, a systemd service has been provided.
* If you have added the new user from previous section, add `User=asus_numpad` the end of `[Service]` section in `tools/asus-numpad.service`.
* Run the following
    ```bash
    # copy the systemd service to a known location
    sudo cp tools/asus-numpad.service /etc/systemd/system/

    # enable and start the service
    sudo systemctl enable --now asus-numpad.service
    ```

## Configuration
The config file is stored in TOML format at `/etc/xdg/asus_numpad.toml`. It supports the following params:
* `layout`: `string`: One of `UX433FA`, `M433IA`, `UX581`, or `GX701`.
* `calc_start_command`: Array of keys from [EV_KEY](https://docs.rs/evdev-rs/latest/evdev_rs/enums/enum.EV_KEY.html), or `{cmd = "some_binary", args = ["arg1", "arg2]}`. Default `["KEY_CALC"]`. Defines what is to be done when calc key is dragged.
* `calc_stop_command`: Array of keys from [EV_KEY](https://docs.rs/evdev-rs/latest/evdev_rs/enums/enum.EV_KEY.html), or `{cmd = "some_binary", args = ["arg1", "arg2]}`. Defines what is to be done when calc key is dragged the second time. If not specified, the `calc_start_command` will be re-ran. 
* `disable_numlock_on_start`: `bool`, default `true`: Specifies whether we should deactivate the numlock when starting up.

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
* [x] Integration with system's NumLock state (toggle with external keyboards)
* [x] `strip` release binaries
* [x] Re-triggering Calc Key should _close_ the previously opened calc
* [x] Run custom command on triggering Calc Key
* [ ] Autodetect laptop model
* [ ] Disable numpad if idle for more than a minute

## Acknowledgements
* This is a rewrite of [asus-touchpad-numpad-driver](https://github.com/mohamed-badaoui/asus-touchpad-numpad-driver)
