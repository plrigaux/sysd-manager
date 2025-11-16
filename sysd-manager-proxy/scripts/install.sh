#!/bin/sh

REL_PATH="."

echo Installing DBus file
sudo install -v -Dm644 "${REL_PATH}/data/org.zbus.MyGreeter.conf" -t "/usr/share/dbus-1/system.d"

echo "END"