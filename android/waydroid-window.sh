#!/bin/sh
# Opens Waydroid in a phone-sized window (450×960) that can be moved like any
# other: Waydroid runs inside a small weston window instead of taking the screen.
# Then connects adb, so `./gradlew installDebug` installs the app there.
set -eu
SOCKET=wayland-poet

waydroid session stop >/dev/null 2>&1 || true
if [ ! -S "$XDG_RUNTIME_DIR/$SOCKET" ]; then
    setsid weston --backend=wayland --width=450 --height=960 --socket=$SOCKET \
        --shell=kiosk --idle-time=0 >/dev/null 2>&1 &
    sleep 2
fi
WAYLAND_DISPLAY=$SOCKET setsid waydroid show-full-ui >/dev/null 2>&1 &

# Wait for Android's address, then connect adb to it.
for _ in $(seq 1 60); do
    ip=$(waydroid status 2>/dev/null | sed -n 's/^IP address:[[:space:]]*\([0-9.]*\)$/\1/p')
    [ -n "$ip" ] && break
    sleep 2
done
[ -n "${ip:-}" ] || { echo "Android got no IP address (is ufw allowing waydroid0?)"; exit 1; }
"${ANDROID_HOME:-$HOME/Android/Sdk}/platform-tools/adb" connect "$ip:5555"
