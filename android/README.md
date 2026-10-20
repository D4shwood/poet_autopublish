# Poet for Android

Write poems on the phone and see your weeks, poems and stats, using the same
rules as the desktop app. The poem logic is the shared Rust core (`../src`),
reached from Kotlin through [UniFFI](https://mozilla.github.io/uniffi-rs/).

```
android/
  rust/          poet_core: the bridge crate (UniFFI) over the shared core
  app/           the Jetpack Compose app
  build-rust.sh  builds the native libraries and Kotlin bindings (Gradle runs it)
```

The app reads and writes the poems folder you pick, through Android's folder
picker. Point it at a synced folder (e.g. Syncthing) to share poems with the
computer.

## Building

Once, on Arch / CachyOS: `./setup-toolchain.sh` (JDK 17, rustup with the
Android targets, cargo-ndk, the Android SDK and NDK, and the Gradle wrapper).

Then:

```sh
./gradlew installDebug     # build and install on a connected phone
./gradlew assembleRelease  # APK in app/build/outputs/apk/release
```

## Pulling desktop changes

The public repo is the `upstream` remote; pushing to it is disabled here.

```sh
git pull upstream main
```
