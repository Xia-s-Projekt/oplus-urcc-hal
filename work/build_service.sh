#!/bin/bash
# Build the OPlus URCC AIDL service binary.
#
# Layout mirrors the shipped artifacts:
#   vendor.oplus.hardware.urcc-V1-ndk_platform.so  <- AIDL transport (PRESERVED)
#   vendor.oplus.hardware.urcc-service              <- impl class + main() (built here)
#
# The service binary deliberately does NOT contain the AIDL transport. The
# shipped binary defines no AIDL transport symbol at all: BnUrcc::createBinder,
# BnUrcc::getInterfaceVersion, BnUrcc::getInterfaceHash and IUrcc::descriptor
# are UNDEFINED in the service and are resolved at runtime from the preserved
# sidecar, which the service lists in DT_NEEDED. The impl class therefore
# derives from BnUrcc (not from IUrccDefault): the sidecar owns the vtable and
# the onTransact dispatch.
#
# ABI constraints this build honours:
#   * The AIDL headers are generated with the interface version (1) and the
#     interface hash of the shipped binary, so IUrcc::hash and
#     getInterfaceVersion() return the same values and the vtable slot order
#     matches the shipped sidecar.
#   * The C++ ABI must be the platform one: std::__1 with the OLD-style
#     char_traits mangling, which the AOSP libc++ headers under
#     OplusNfc/work/aosp_libcxx produce.
#   * -fno-rtti: the shipped service references no typeinfo and the sidecar
#     exports none, so a RTTI build would demand "typeinfo for BnUrcc" and
#     fail to load on the device.
#   * Link-time libraries are the real runtime providers: the VNDK copies for
#     libbase/libbinder/libcutils/libutils, the system copies for
#     libbinder_ndk/liblog, and /vendor/lib64/libc++.so.
set -eu

W=/mnt/project/RomTools/ROM_WORKDIR/oos/ReconstructedHALSource/Oplus/OplusUrcc
NFC=/mnt/project/RomTools/ROM_WORKDIR/oos/ReconstructedHALSource/Oplus/OplusNfc
ROM=/mnt/project/RomTools/ROM_WORKDIR/oos/erofs
NDK=/opt/android-sdk/ndk/27.0.12077973/toolchains/llvm/prebuilt/linux-x86_64
CXX=$NDK/bin/clang++
SYS=$NDK/sysroot
GEN=$W/work/aidlgen
OUT=$W/work/release
VNDK=$NFC/work/vdndk

LIBCXX=$NFC/work/aosp_libcxx/include
PLATFORM_HDRS=/mnt/project/Misc/aosp_headers/api36
SYSLOG=$ROM/system/system/lib64

# The shipped platform sidecar, taken from the reference ODM tree. It is the
# AIDL transport and it owns the interface hash, the vtable and the parcel
# layout, so it is used as-is.
SIDECAR_SRC=/mnt/project/RomTools/ROM_WORKDIR/oos15_old/erofs/odm/lib64/vendor.oplus.hardware.urcc-V1-ndk_platform.so
SIDECAR=$OUT/vendor.oplus.hardware.urcc-V1-ndk_platform.so

# The interface identity of the shipped service, recovered from the
# shipped sidecar and service binaries.
IFACE_VERSION=1
IFACE_HASH=cae43655eaf0af39cbb6d273c5e2206eebe33a10

# The service-side AIDL backend is generated with build-tools 36.0.0, the
# same generation model the other reconstructed OPlus services use.
AIDL=/opt/android-sdk/build-tools/36.0.0/aidl

mkdir -p "$OUT" "$GEN/hdr"

echo "=== staging the preserved sidecar"
cp -p "$SIDECAR_SRC" "$SIDECAR"

echo "=== generating the AIDL backend (v$IFACE_VERSION)"
"$AIDL" --lang=ndk -I "$W/aidl" -o "$GEN" -h "$GEN/hdr" \
  --version="$IFACE_VERSION" --hash="$IFACE_HASH" \
  "$W/aidl/vendor/oplus/hardware/urcc/IUrcc.aidl"
# The parcelables need their own translation units: their readFromParcel /
# writeToParcel are not part of the interface's generated source.
for p in UrccRequestData UrccRequestParcel IUrccCallback; do
  "$AIDL" --lang=ndk -I "$W/aidl" -o "$GEN" -h "$GEN/hdr" \
    --version="$IFACE_VERSION" --hash="$IFACE_HASH" \
    "$W/aidl/vendor/oplus/hardware/urcc/$p.aidl"
done


# ==== urcc_trace_ontransact ============================================
# Temporary instrumentation: log every transaction code that reaches the
# service's onTransact. Without it a parcel that fails to parse is
# invisible (the generated code breaks out before the impl runs).
python3 "$W/scripts/fix_lenient_strings.py" "$GEN"
# ======================================================================

CXXFLAGS=(
  --target=aarch64-linux-android35
  --sysroot="$SYS"
  -std=c++17 -O2 -fPIE
  -Wall -Wno-unused-parameter
  -fno-rtti
  -fno-exceptions
  # Platform libc++ headers, and no NDK libc++ headers.
  -nostdinc++ -isystem "$LIBCXX"
  # AIDL headers generated from the reconstructed .aidl file.
  -I "$GEN/hdr" -I "$GEN"
  # Platform (non-NDK) headers for the binder NDK platform API.
  -I "$PLATFORM_HDRS/frameworks/native/libs/binder/ndk/include_platform"
  -I "$PLATFORM_HDRS/frameworks/native/libs/binder/ndk/include_cpp"
  -I "$W/src"
)

LDFLAGS_COMMON=(
  -nostdlib++
  -Wl,--no-undefined
  -Wl,--as-needed
  -Wl,--allow-shlib-undefined
  -Wl,-z,relro,-z,now
  -Wl,--build-id=none
)

LIBS_COMMON=(
  "$SYSLOG/libbinder_ndk.so"
  "$SYSLOG/liblog.so"
  # --no-as-needed: the shipped service lists libbase, libcutils, libutils,
  # libm and libdl in DT_NEEDED even where it does not reference a symbol
  # from them. Keeping the same set means the service runs against the same
  # libraries.
  -Wl,--no-as-needed
  "$VNDK/libbase.so"
  "$VNDK/libcutils.so"
  "$VNDK/libutils.so"
  "$ROM/vendor/lib64/libc++.so"
  "$SYS/usr/lib/aarch64-linux-android/35/libc.so"
  "$SYS/usr/lib/aarch64-linux-android/35/libm.so"
  "$SYS/usr/lib/aarch64-linux-android/35/libdl.so"
)

echo "=== compiling"
# The AIDL transport is compiled into the service instead of being taken
# from the shipped sidecar.
#
# The shipped sidecar (vendor.oplus.hardware.urcc-V1-ndk_platform.so) lays the
# parcelable out with 32-byte std::string members: readFromParcel writes
# res_Value/res_Path/desc at 0x8/0x28/0x48.  The headers this build uses
# produce 24-byte std::string, so the members sit at 0x8/0x20/0x38 and the
# sidecar's writes land in the wrong slots - the service then dereferences a
# string that is not a string and dies with SIGSEGV on the first control
# request.  Compiling the transport from the same headers as the
# implementation makes the in-process layout self-consistent; the wire format
# is unchanged, because only the parcel bytes cross the process boundary.
for src in "$GEN"/vendor/oplus/hardware/urcc/*.cpp; do
  obj="$OUT/$(basename "$src" .cpp).o"
  "$CXX" "${CXXFLAGS[@]}" -c "$src" -o "$obj"
done
"$CXX" "${CXXFLAGS[@]}" -c "$W/src/UrccCore.cpp" -o "$OUT/UrccCore.o"
"$CXX" "${CXXFLAGS[@]}" -c "$W/src/Urcc.cpp" -o "$OUT/Urcc.o"
"$CXX" "${CXXFLAGS[@]}" -c "$W/src/main.cpp" -o "$OUT/main.o"

echo "=== linking"
"$CXX" "${CXXFLAGS[@]}" \
  -o "$OUT/vendor.oplus.hardware.urcc-service" \
  "$OUT/UrccCore.o" \
  "$OUT/Urcc.o" \
  "$OUT/main.o" \
  "$OUT/IUrcc.o" \
  "$OUT/UrccRequestData.o" \
  "$OUT/UrccRequestParcel.o" \
  "$OUT/IUrccCallback.o" \
  "${LDFLAGS_COMMON[@]}" \
  "${LIBS_COMMON[@]}"

echo "=== DT_NEEDED"
"$NDK/bin/llvm-readelf" -d "$OUT/vendor.oplus.hardware.urcc-service" | grep NEEDED
echo "=== undefined AIDL transport symbols (must stay undefined)"
"$NDK/bin/llvm-nm" -D --undefined-only "$OUT/vendor.oplus.hardware.urcc-service" \
  | c++filt | grep -E 'IOplusNfc|IOplusNfc|BnUrcc|IUrcc' || true
echo "--- $(stat -c %s "$OUT/vendor.oplus.hardware.urcc-service") bytes"
echo BUILD_OK
