#!/bin/bash
# Stage the built Rust URCC HAL into an unpacked vendor tree.
#
# Destination, under $ROM_TREE:
#   vendor/bin/hw/vendor.oplus.hardware.urcc-service
#
# The init rc at vendor/etc/init/ and the VINTF fragment at
# vendor/etc/vintf/manifest/ already point at this path and are left unchanged.
# A file already at the destination is copied to build/backup/ before it is
# replaced.
#
# This only touches files in the unpacked tree. It does NOT repack, does NOT
# flash, and does NOT touch fs_config or file_contexts.
set -euo pipefail

: "${ROM_TREE:?set ROM_TREE to an unpacked Android tree}"
: "${ANDROID_NDK_HOME:?set ANDROID_NDK_HOME to the Android NDK root}"

W=$(cd "$(dirname "$0")/.." && pwd)
SRC="$W/rust/target/aarch64-linux-android/release/vendor.oplus.hardware.urcc-service"
BACKUP="$W/build/backup"
NDK="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64"

GRN=$'\033[32m'; RED=$'\033[31m'; NC=$'\033[0m'

[[ -f "$SRC" ]] || { echo -e "${RED}missing $SRC: run build_rust.sh first${NC}"; exit 1; }

live="$ROM_TREE/vendor/bin/hw/vendor.oplus.hardware.urcc-service"

# Save whatever is currently at the destination.
if [[ -f "$live" ]]; then
    mkdir -p "$BACKUP"
    keep="$BACKUP/$(basename "$live")"
    [[ -f "$keep" ]] || cp -p "$live" "$keep"
    echo "  preserved previous binary -> build/backup/"
fi

# Stage.
install -D -m 0755 "$SRC" "$live"

# Verify.
echo
echo -e "${GRN}staged${NC}"
rc=0
a=$(sha256sum "$SRC" | cut -c1-16)
b=$(sha256sum "$live" | cut -c1-16)
if [[ "$a" == "$b" ]]; then
    echo "  OK   vendor.oplus.hardware.urcc-service  $b"
else
    echo -e "  ${RED}FAIL${NC} staged=$b src=$a"
    rc=1
fi
m=$(stat -c "%a" "$live")
if [[ "$m" == "755" ]]; then
    echo "  OK   mode $m"
else
    echo -e "  ${RED}FAIL${NC} mode $m"
    rc=1
fi
needed=$("$NDK/bin/llvm-readelf" -d "$live" | grep NEEDED | tr '\n' ' ')
[[ "$needed" == *"libbinder_ndk.so"* ]] \
    && echo "  OK   DT_NEEDED libbinder_ndk.so" \
    || { echo -e "${RED}FAIL${NC} missing libbinder_ndk dependency: $needed"; rc=1; }
hashcount=$("$NDK/bin/llvm-strings" "$live" | grep -c 'cae43655eaf0af39cbb6d273c5e2206eebe33a10' || true)
if [[ "$hashcount" -ge 1 ]]; then
    echo "  OK   interface hash"
else
    echo -e "  ${RED}FAIL${NC} interface hash missing"
    rc=1
fi

echo
echo "  rc:        $ROM_TREE/vendor/etc/init/vendor.oplus.hardware.urcc-service.rc (unchanged, enabled)"
echo "  VINTF:     $ROM_TREE/vendor/etc/vintf/manifest/manifest_oplus_urcc_aidl.xml (unchanged)"
echo "  next:      repack/flash is NOT done. Verify on the target, then repack explicitly if wanted."
exit $rc
