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

The app keeps its own copy of your poems repo and syncs it with GitHub every
15 minutes (a fine-grained token, stored encrypted on the phone), or reads and
writes a folder you pick. The desktop syncs the same repo with git.

Releases: pushing a tag like `android-v0.2.0` builds the signed APK and
publishes it as a release ([workflow](../.github/workflows/android.yml)). The
signing key lives in the repo secrets `POET_KEYSTORE_B64` and
`POET_KEYSTORE_PASSWORD`; every release must use the same key, or phones refuse
the update.

## Building

Once, on Arch / CachyOS: `./setup-toolchain.sh` (JDK 17, rustup with the
Android targets, cargo-ndk, the Android SDK and NDK, and the Gradle wrapper).

For testing on this computer, `./setup-waydroid.sh` once, then
`./waydroid-window.sh` opens Android in a phone-sized window and connects adb.
(With ufw on, allow the Waydroid interface: `sudo ufw allow in on waydroid0`.)

Then:

```sh
./gradlew installDebug     # build and install on a connected phone
./gradlew assembleRelease  # APK in app/build/outputs/apk/release
```

