#!/bin/bash
# Stage the Rust port of the URCC HAL into the unpacked vendor tree.
#
# Staged:
#   erofs/vendor/bin/hw/vendor.oplus.hardware.urcc-service   (the Rust build)
#
# The init rc (erofs/vendor/etc/init/vendor.oplus.hardware.urcc-service.rc,
# already enabled, class hal, user root) and the VINTF fragment
# (erofs/vendor/etc/vintf/manifest/manifest_oplus_urcc_aidl.xml) point at this
# same path and stay unchanged. The previous C++ reconstruction build is
# preserved under re/cpp_build/ and the genuine shipped binary under
# re/original/.
#
# The binary lands at the same path as the C++ build it replaces, so the
# existing fs_config / file_contexts entries still apply, so nothing is
# regenerated (that only happens at repack time).
#
# This only touches files in the unpacked tree. It does NOT repack, does NOT
# flash, and does NOT touch fs_config or file_contexts.
set -euo pipefail

W=$(cd "$(dirname "$0")/.." && pwd)
ROM_TREE=${ROM_TREE:-/mnt/project/RomTools/ROM_WORKDIR/oos/erofs}
SRC="$W/rust/target/aarch64-linux-android/release/vendor.oplus.hardware.urcc-service"

GRN=$'\033[32m'; RED=$'\033[31m'; NC=$'\033[0m'

[[ -f "$SRC" ]] || { echo -e "${RED}missing $SRC: run build_rust.sh first${NC}"; exit 1; }

live="$ROM_TREE/vendor/bin/hw/vendor.oplus.hardware.urcc-service"

# Preserve the current tree content.
if [[ -f "$live" ]]; then
    mkdir -p "$W/re/cpp_build"
    keep="$W/re/cpp_build/$(basename "$live")"
    [[ -f "$keep" ]] || cp -p "$live" "$keep"
    echo "  preserved the C++ reconstruction build -> re/cpp_build/"
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
NDK=/opt/android-sdk/ndk/27.0.12077973/toolchains/llvm/prebuilt/linux-x86_64
needed=$("$NDK/bin/llvm-readelf" -d "$live" | grep NEEDED | tr '\n' ' ')
[[ "$needed" == *"libbinder_ndk.so"* ]] \
    && echo "  OK   DT_NEEDED libbinder_ndk.so" \
    || { echo -e "  ${RED}FAIL${NC} missing libbinder_ndk dependency: $needed"; rc=1; }
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
echo "  next:      flash/repack is NOT done. The live test was run from /data/local/tmp as root."
exit $rc
