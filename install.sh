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
    supportedModels=("UX433FA" "M433IA" "UX581" "GX701" "GX531")
    echo -e "Select touchpad model:\n0.${supportedModels[0]}\n1.${supportedModels[1]}\n2.${supportedModels[2]}\n3.${supportedModels[3]}\n4.${supportedModels[4]}\nChoose correct number:"
    read -r selectedModelNumber
    echo "Creating config file"
    touch /etc/xdg/asus_numpad.toml
    echo "layout = \"${supportedModels[selectedModelNumber]}\"" > /etc/xdg/asus_numpad.toml
fi

# Copying program file
if [ -f "$PWD/target/debug/asus-numpad" ];
then
    echo "Creating folder /opt/asus-numpad/"
    mkdir /opt/asus-numpad
    echo "Copying program to /opt/asus-numpad/"
    cp -f "$PWD/target/debug/asus-numpad" /opt/asus-numpad/
else
    echo "Program does not exist..."
    exit 1
fi
# Copying systemctl services
cp -f "$PWD/tools/asus-numpad.service" /etc/systemd/system/
# Enabling and starting systemctl service
systemctl enable asus-numpad.service
systemctl start asus-numpad.service

