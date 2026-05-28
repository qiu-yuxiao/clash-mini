#!/bin/bash
chmod +x /usr/bin/clash-verge-service-install
chmod +x /usr/bin/clash-verge-service-uninstall
chmod +x /usr/bin/clash-verge-service

. /etc/os-release

if [ "$ID" = "deepin" ]; then
    PACKAGE_NAME="$DPKG_MAINTSCRIPT_PACKAGE"
    DESKTOP_FILES=$(dpkg -L "$PACKAGE_NAME" 2>/dev/null | grep "\.desktop$")
    echo "$DESKTOP_FILES" | while IFS= read -r f; do
        if [ "$(basename "$f" | tr 'A-Z' 'a-z')" = "clash mini.desktop" ] || [ "$(basename "$f")" = "clash-mini.desktop" ]; then
            echo "Fixing deepin desktop file"
            mv -vf "$f" "/usr/share/applications/clash-mini.desktop"
        fi
    done
fi
