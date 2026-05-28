#!/bin/bash
/usr/bin/clash-verge-service-uninstall

. /etc/os-release

if [ "$ID" = "deepin" ]; then
    if [ -f "/usr/share/applications/clash-mini.desktop" ]; then
        echo "Removing deepin desktop file"
        rm -vf "/usr/share/applications/clash-mini.desktop"
    fi
fi

