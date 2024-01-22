#!/bin/bash

# Checking if the script is runned as root (via sudo or other)
if [[ $(id -u) != 0 ]];
then
    echo "Please run the installation script as root (using sudo for example)"
    exit 1
fi
# Checking if config file already exists
if [ -f "/etc/xdg/asus_numpad.toml" ];
then
    echo "Config file already exists!"
    cat /etc/xdg/asus_numpad.toml
else
    keyboardModel=$(cat /sys/class/dmi/id/board_name)
    echo "Creating config file"
    touch /etc/xdg/asus_numpad.toml
    echo "layout = \"${keyboardModel}\"" > /etc/xdg/asus_numpad.toml
fi

# Copying program file
if [ -f "$PWD/target/debug/asus-numpad" ];
then
    echo "Copying program to /usr/local/bin/"
    cp -f "$PWD/target/debug/asus-numpad" /usr/local/bin/
else
    echo "Program does not exist..."
    exit 1
fi
# Copying systemctl services
cp -f "$PWD/tools/asus-numpad.service" /etc/systemd/system/
# Enabling and starting systemctl service
systemctl enable asus-numpad.service
systemctl start asus-numpad.service

