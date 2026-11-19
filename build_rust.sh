#!/bin/bash
# Build the Rust URCC AIDL HAL for aarch64 Android.
#
# Output, under rust/target/aarch64-linux-android/release/:
#   vendor.oplus.hardware.urcc-service   the HAL binary
#   urcc_probe                           wire-level test client
#
# Requires the pinned Rust toolchain with the aarch64-linux-android target,
# Android NDK r27+ (ANDROID_NDK_HOME) and ROM_TREE pointing at an unpacked
# Android tree that provides libbinder_ndk.so with the platform-only binder
# entry points. Set LIBBINDER_NDK to override the library path.
set -eu

: "${ANDROID_NDK_HOME:?set ANDROID_NDK_HOME to the Android NDK root}"
: "${ROM_TREE:?set ROM_TREE to an unpacked Android tree}"

W=$(cd "$(dirname "$0")" && pwd)
cd "$W/rust"

NDK="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64"
LIBBINDER_NDK=${LIBBINDER_NDK:-"$ROM_TREE/system/system/lib64/libbinder_ndk.so"}

export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$NDK/bin/aarch64-linux-android35-clang"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_RUSTFLAGS="\
-C link-arg=-Wl,--no-undefined \
-C link-arg=-Wl,-z,relro,-z,now \
-C link-arg=-Wl,--build-id=none \
-C link-arg=-llog \
-C link-arg=$LIBBINDER_NDK"

cargo build --release --target aarch64-linux-android --bins
# Host unit tests for the cross-platform core (node map, compat layer).
cargo test

OUT=target/aarch64-linux-android/release

install -m 0755 "$OUT/urcc_service" "$OUT/vendor.oplus.hardware.urcc-service"

echo
echo "=== built ==="
ls -la "$OUT/vendor.oplus.hardware.urcc-service" "$OUT/urcc_probe"
"$NDK/bin/llvm-readelf" -d "$OUT/vendor.oplus.hardware.urcc-service" | grep NEEDED
"$NDK/bin/llvm-strings" "$OUT/vendor.oplus.hardware.urcc-service" | grep -c 'cae43655eaf0af39cbb6d273c5e2206eebe33a10' >/dev/null && echo "interface hash present"
echo BUILD_OK
