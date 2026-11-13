#!/bin/bash
# Stage the rebuilt OPlus URCC AIDL HAL into the unpacked vendor tree.
#
# Staged:
#   erofs/vendor/bin/hw/vendor.oplus.hardware.urcc-service   (rebuilt here)
#   erofs/vendor/etc/init/vendor.oplus.hardware.urcc-service.rc
#   erofs/vendor/etc/vintf/manifest/manifest_oplus_urcc_aidl.xml
#
# The rebuilt service owns the AIDL transport itself: it compiles the
# generated IUrcc/UrccRequestData/UrccRequestParcel sources into the binary,
# so the interface descriptor, the interface hash
# (cae43655eaf0af39cbb6d273c5e2206eebe33a10) and the parcel layout all come
# from this build and cannot drift from the shipped sidecar. The shipped
# vendor/lib64/vendor.oplus.hardware.urcc-V1-ndk_platform.so stays in the
# tree untouched for anything else that links it.
#
# The closed liburcccore.so is no longer used: it drives Qualcomm-only nodes
# and faults on this MTK board. It stays in the tree untouched but nothing
# links it any more.
#
# This only touches files in the unpacked tree. It does NOT repack, does NOT
# flash, and does NOT touch fs_config or file_contexts.
set -euo pipefail

W=$(cd "$(dirname "$0")/.." && pwd)
ROM_TREE=${ROM_TREE:-/mnt/project/RomTools/ROM_WORKDIR/oos/erofs}
ORIG="$W/re/original"

GRN=$'\033[32m'; RED=$'\033[31m'; NC=$'\033[0m'

PAIRS=(
  # rebuilt service (self-contained AIDL transport)
  "$W/work/release/vendor.oplus.hardware.urcc-service|bin/hw/vendor.oplus.hardware.urcc-service|0755"
)

# --- preserve the shipped originals ----------------------------------------
mkdir -p "$ORIG"
PRESERVE_ONLY="$W/work/release/vendor.oplus.hardware.urcc-service"
for p in "${PAIRS[@]}"; do
    src="${p%%|*}"; rest="${p#*|}"; dst="${rest%%|*}"
    live="$ROM_TREE/vendor/$dst"
    keep="$ORIG/$(basename "$dst")"
    [[ -f "$live" ]] || continue
    [[ -f "$keep" ]] && continue
    if [[ "$src" != "$PRESERVE_ONLY" ]]; then
        continue
    fi
    a=$(sha256sum "$live" | cut -c1-16)
    b=$(sha256sum "$src"  | cut -c1-16)
    if [[ "$a" == "$b" ]]; then
        echo -e "  ${RED}REFUSING${NC} to preserve $(basename "$dst"): the tree already holds the rebuild"
        echo "         and no shipped original is in $ORIG."
        exit 1
    fi
    cp -p "$live" "$keep"
    echo "  preserved $(basename "$dst") -> re/original/"
done

# --- stage ------------------------------------------------------------------
for p in "${PAIRS[@]}"; do
    src="${p%%|*}"; rest="${p#*|}"; dst="${rest%%|*}"; mode="${rest##*|}"
    [[ -f "$src" ]] || { echo -e "${RED}missing $src${NC}"; exit 1; }
    echo "  staging $(basename "$dst")"
    install -D -m "$mode" "$src" "$ROM_TREE/vendor/$dst"
done

echo "  staging vendor.oplus.hardware.urcc-service.rc"
install -m 0644 "$W/scripts/urcc-service.rc" \
    "$ROM_TREE/vendor/etc/init/vendor.oplus.hardware.urcc-service.rc"

echo "  staging manifest_oplus_urcc_aidl.xml"
install -m 0644 "$W/scripts/manifest_oplus_urcc_aidl.xml" \
    "$ROM_TREE/vendor/etc/vintf/manifest/manifest_oplus_urcc_aidl.xml"

# --- verify -----------------------------------------------------------------
echo
echo -e "${GRN}staged${NC}"
rc=0
for p in "${PAIRS[@]}"; do
    src="${p%%|*}"; rest="${p#*|}"; dst="${rest%%|*}"; mode="${rest##*|}"
    live="$ROM_TREE/vendor/$dst"
    a=$(sha256sum "$src" | cut -c1-16)
    b=$(sha256sum "$live" | cut -c1-16)
    if [[ "$a" == "$b" ]]; then
        echo "  OK   $(basename "$dst")  $b"
    else
        echo -e "  ${RED}FAIL${NC} $(basename "$dst")  staged=$b src=$a"
        rc=1
    fi
    m=$(stat -c "%a" "$live"); want="${mode#0}"; want="${want:-0}"
    if [[ "$m" == "$want" ]]; then
        echo "  OK   mode $m"
    else
        echo -e "  ${RED}FAIL${NC} mode $m, expected $want"
        rc=1
    fi
done

# the rebuilt service must carry the AIDL transport itself
svc="$ROM_TREE/vendor/bin/hw/vendor.oplus.hardware.urcc-service"
if [[ -f "$svc" ]]; then
    syms=$(llvm-nm --defined-only "$svc" 2>/dev/null || true)
    missing=""
    for sym in "BnUrcc12createBinderEv" "BnUrcc19getInterfaceVersionEPi" "IUrcc10descriptorE" \
               "UrccRequestData13readFromParcelEPK6AParcel" \
               "UrccRequestParcel13readFromParcelEPK6AParcel"; do
        grep -q "$sym" <<<"$syms" || missing="$missing $sym"
    done
    if [[ -z "$missing" ]]; then
        echo "  OK   the service carries the AIDL transport"
    else
        echo -e "  ${RED}FAIL${NC} the service lost:$missing"
        rc=1
    fi
    hash=$(llvm-strings -a "$svc" 2>/dev/null | grep -E '^[0-9a-f]{40}$' | head -1)
    if [[ "$hash" == "cae43655eaf0af39cbb6d273c5e2206eebe33a10" ]]; then
        echo "  OK   interface hash $hash"
    else
        echo -e "  ${RED}FAIL${NC} interface hash is '$hash'"
        rc=1
    fi
fi

echo
echo "  next: flash/repack is NOT done. Verify on the device, then repack explicitly if wanted."
exit $rc
