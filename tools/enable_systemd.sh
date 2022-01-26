#!/usr/bin/env bash

if ! command -v asus-numpad; then
    echo "asus-numpad binary not available in PATH";
    exit 1;
fi

# copy the systemd service to a known location
sudo cp tools/asus-numpad.service /etc/systemd/system/
function get_model() {
    # sorta ugly way to get list of allowed models
    models=$(asus-numpad --help | grep -Po 'l, --layout .*values: \K((\w+)[,\s]*)+(?=\])')
    IFS=", " read -a MODELS <<<"$models"
    declare -i MAX_TRIES="3"
    declare -i t="1"

    echo -n "Specify model [one of: ${MODELS[*]}]: "
    read model

    while [[ $t -lt $MAX_TRIES && ! " ${MODELS[*]} " =~ " $model " ]] ; do 
        echo "Incorrect model"
        t="$((t+1))"
        echo -n "Specify model [one of: ${MODELS[*]}]: "
        read model
    done
    [ $t -eq $MAX_TRIES ] && return 1 || return 0;
}

if ! get_model; then
    echo "Failed to get model"
    exit 1
else
    # edit the file to replace `$LAYOUT`
    sudo sed -i "s/\$LAYOUT/$model/" /etc/systemd/system/asus-numpad.service

    sudo systemctl enable --now asus-numpad.service
    echo "Service started"
fi
