#!/bin/sh
# Builds poet_core for the phone and generates its Kotlin bindings.
# Gradle runs this before every build (the buildRust task); it also works alone.
set -eu
cd "$(dirname "$0")/rust"
export PATH="$HOME/.cargo/bin:$PATH"
export ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-$(ls -d "${ANDROID_HOME:-$HOME/Android/Sdk}"/ndk/* | sort -V | tail -1)}"

profile="${1:-release}"
flag=""
[ "$profile" = release ] && flag="--release"

# Native libraries for real phones (arm64) and the emulator (x86_64).
cargo ndk -t arm64-v8a -t x86_64 -o ../app/src/main/jniLibs build $flag -p poet_core

# The bindings are read from a host build of the same crate.
cargo build $flag -p poet_core
cargo run -q -p uniffi-bindgen -- generate --no-format \
    --library "target/$profile/libpoet_core.so" \
    --language kotlin --out-dir ../app/src/main/java
