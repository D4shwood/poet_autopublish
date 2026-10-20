#!/bin/bash
# One-time setup for building the Android app on Arch / CachyOS.
# Run it yourself in a terminal: it asks for your sudo password and for
# accepting the Android SDK licenses.
set -euo pipefail
cd "$(dirname "$0")"
SDK="$HOME/Android/Sdk"

echo "== 1/5 JDK 17, rustup and Gradle (rustup replaces the 'rust' package)"
sudo pacman -S --needed jdk17-openjdk rustup gradle unzip

echo "== 2/5 Rust with the Android targets"
rustup default stable
rustup target add aarch64-linux-android x86_64-linux-android
cargo install cargo-ndk

echo "== 3/5 Android command-line tools from dl.google.com"
if [ ! -x "$SDK/cmdline-tools/latest/bin/sdkmanager" ]; then
    tmp="$(mktemp -d)"
    curl -fL -o "$tmp/tools.zip" https://dl.google.com/android/repository/commandlinetools-linux-11076708_latest.zip
    unzip -q "$tmp/tools.zip" -d "$tmp"
    mkdir -p "$SDK/cmdline-tools"
    mv "$tmp/cmdline-tools" "$SDK/cmdline-tools/latest"
    rm -rf "$tmp"
fi
export JAVA_HOME=/usr/lib/jvm/java-17-openjdk
sdkmanager="$SDK/cmdline-tools/latest/bin/sdkmanager"

echo "== 4/5 SDK licenses (read and accept) and packages"
"$sdkmanager" --licenses
"$sdkmanager" "platform-tools" "platforms;android-35" "build-tools;35.0.0" "ndk;27.2.12479018"

echo "== 5/5 Gradle wrapper and SDK location"
echo "sdk.dir=$SDK" > local.properties
# Made in an empty folder: the system Gradle is newer than this build supports,
# so it must not try to configure the project itself.
if [ ! -f gradlew ]; then
    tmp="$(mktemp -d)"
    (cd "$tmp" && touch settings.gradle.kts && gradle -q wrapper --gradle-version 8.11.1)
    cp -r "$tmp/gradlew" "$tmp/gradlew.bat" "$tmp/gradle" .
    rm -rf "$tmp"
fi

echo
echo "Done. Add these to ~/.config/fish/config.fish:"
echo "  set -gx ANDROID_HOME $SDK"
echo "  set -gx JAVA_HOME /usr/lib/jvm/java-17-openjdk"
echo "  fish_add_path $SDK/platform-tools"
