#!/bin/bash

# Checking if the script is runned as root (via sudo or other)
if [[ $(id -u) != 0 ]];
then
    echo "Please run the uninstallation script as root (using sudo for example)"
    exit 1
fi
systemctl stop asus-numpad.service
if [[ $? != 0 ]]
then
	echo "asus-numpad.service cannot be stopped correctly..."
	exit 1
fi

systemctl disable asus-numpad.service
if [[ $? != 0 ]]
then
	echo "asus-numpad.service cannot be disabled correctly..."
	exit 1
fi

rm -f /etc/systemd/system/asus-numpad.service
if [[ $? != 0 ]]
then
	echo "asus-numpad.service cannot be deleted correctly..."
	exit 1
fi

rm -f /usr/local/bin/asus-numpad
if [[ $? != 0 ]]
then
	echo "asus-numpad driver cannot be deleted correctly..."
	exit 1
fi

rm -f /etc/xdg/asus_numpad.toml
if [[ $? != 0 ]]
then
	echo "asus-numpad configure file cannot be deleted correctly..."
	exit 1
fi