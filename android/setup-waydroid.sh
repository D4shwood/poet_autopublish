#!/bin/bash
# One-time setup of Waydroid, an Android container for testing the app on
# this computer. Run it yourself in a terminal: it asks for your sudo password
# and downloads the LineageOS image (about 1 GB).
set -euo pipefail

echo "== 1/3 Waydroid"
sudo pacman -S --needed waydroid

echo "== 2/3 Android image (LineageOS, no Google apps)"
if [ ! -d /var/lib/waydroid/images ] || [ -z "$(ls -A /var/lib/waydroid/images 2>/dev/null)" ]; then
    sudo waydroid init
fi

echo "== 3/3 Container service"
sudo systemctl enable --now waydroid-container

echo
echo "Done. Start Android in a window with:"
echo "  waydroid show-full-ui"
