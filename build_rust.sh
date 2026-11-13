#!/bin/bash
# Build the Rust port of the OPlus URCC AIDL HAL for aarch64 Android.
#
# Output:
#   rust/target/aarch64-linux-android/release/vendor.oplus.hardware.urcc-service  (the HAL)
#   rust/target/aarch64-linux-android/release/urcc_probe                          (wire-level test client)
#
# Requires: rustc/cargo with the aarch64-linux-android target installed
# (rustup target add aarch64-linux-android) and Android NDK r27+ at
# /opt/android-sdk/ndk/27.0.12077973. The link uses the ROM tree's real
# libbinder_ndk.so (erofs/system/system/lib64) because the platform-only
# AServiceManager_* / ABinderProcess_* entry points are not exported by the
# NDK stub. See .cargo/config.toml.
set -eu

W=$(cd "$(dirname "$0")" && pwd)
cd "$W/rust"

cargo build --release --target aarch64-linux-android --bins
# Host unit tests for the cross-platform core (node map, compat layer).
cargo test

OUT=target/aarch64-linux-android/release
NDK=/opt/android-sdk/ndk/27.0.12077973/toolchains/llvm/prebuilt/linux-x86_64

install -m 0755 "$OUT/urcc_service" "$OUT/vendor.oplus.hardware.urcc-service"

echo
echo "=== built ==="
ls -la "$OUT/vendor.oplus.hardware.urcc-service" "$OUT/urcc_probe"
"$NDK/bin/llvm-readelf" -d "$OUT/vendor.oplus.hardware.urcc-service" | grep NEEDED
"$NDK/bin/llvm-strings" "$OUT/vendor.oplus.hardware.urcc-service" | grep -c 'cae43655eaf0af39cbb6d273c5e2206eebe33a10' >/dev/null && echo "interface hash present"
echo BUILD_OK
