#!/bin/sh
# Opens Waydroid in a phone-sized window (450×960) that can be moved like any
# other: Waydroid runs inside a small weston window instead of taking the screen.
# Then connects adb, so `./gradlew installDebug` installs the app there.
set -eu
SOCKET=wayland-poet

waydroid session stop >/dev/null 2>&1 || true
# A socket file can outlive weston, so look for the process itself.
if ! pgrep -f -- "--socket=$SOCKET" >/dev/null; then
    rm -f "$XDG_RUNTIME_DIR/$SOCKET" "$XDG_RUNTIME_DIR/$SOCKET.lock"
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
# Android answers a little after it has an address.
adb="${ANDROID_HOME:-$HOME/Android/Sdk}/platform-tools/adb"
for _ in $(seq 1 30); do
    "$adb" connect "$ip:5555" | grep -q "connected to" && exit 0
    sleep 4
done
echo "adb couldn't reach Android at $ip"
exit 1
