# OPlus URCC HAL

A Rust implementation of the `vendor.oplus.hardware.urcc` AIDL HAL, with the C++ resource core kept alongside it.

## What it does

URCC (Unified Resource Control / Coordination) is the OPlus framework service for reading and tuning platform resources — CPU and GPU frequencies, core control, memory and thermal state. This project implements the `IUrcc` AIDL interface and its parcelables and serves them through a native resource-control core.

The service:

- registers `vendor.oplus.hardware.urcc.IUrcc/default` with the binder service manager and implements all 14 `IUrcc` methods plus the reserved `getInterfaceVersion` / `getInterfaceHash` transactions;
- resolves resource ids to sysfs nodes from a generated resource table, reports state as numeric strings and applies control requests to the backing nodes;
- handles listener registration and pushes sampled resource and thermal state back to clients through `IUrccCallback`;
- implements natively the resource classes that have no direct sysfs node, rather than returning fixed values.

The wire protocol is implemented against the libbinder_ndk C API. The C++ sources in `src/` are the earlier implementation of the same core and are retained for reference.

## Layout

- `src/` — C++ core and HAL implementation (`Urcc*.cpp/.h`, `main.cpp`, and the generated `UrccResourceTable.inc`).
- `rust/` — the Rust crate: `lib.rs`, `core.rs`, `table.rs`, `wire.rs`, `io_shim.rs`, `hal.rs`, the `urcc_service` binary and the `urcc_probe` test client.
- `scripts/` — staging scripts, the VINTF manifest, the init rc, and the generators for the resource table and XML resource map.
- `aidl/` — the AIDL interface and parcelable definitions under `vendor/oplus/hardware/urcc/`.
- `build_rust.sh` — builds the Rust HAL for aarch64 Android and runs the host unit tests.

## Requirements

- The Rust toolchain pinned in `rust-toolchain.toml`, with the `aarch64-linux-android` target installed (`rustup target add aarch64-linux-android`).
- Android NDK r27 or newer, exported as `ANDROID_NDK_HOME`.
- `ROM_TREE` pointing at an unpacked Android tree used to locate `libbinder_ndk.so`. The NDK stub does not export the platform-only `AServiceManager_*` / `ABinderProcess_*` symbols this service uses, so the build links against that library instead. Set `LIBBINDER_NDK` to override the path.
- A C++17 toolchain and the AIDL compiler for the C++ sources and generated headers; the Rust build does not need them.

## Prebuilt

[Get Prebuilt](https://github.com/Xia-s-Projekt/oplus-urcc-hal/releases/latest) — AArch64 ODM archive containing the service under `odm/bin/hw/`, its init rc under `odm/etc/init/`, and the VINTF manifest under `odm/etc/vintf/manifest/`.

## Build

```sh
export ANDROID_NDK_HOME=/path/to/android-ndk
export ROM_TREE=/path/to/unpacked/android/tree
./build_rust.sh
```

`build_rust.sh` builds the aarch64 binaries into `rust/target/`, installs the service under its dotted name `vendor.oplus.hardware.urcc-service`, runs the host unit tests and prints the resulting ELF dependencies.

## Staging

`scripts/stage_rust.sh` installs the built service into an unpacked vendor tree at `bin/hw/vendor.oplus.hardware.urcc-service` and verifies its mode, ELF dependencies and interface hash. `ROM_TREE` selects the destination tree; the script saves any file already at that path to `build/backup/` before replacing it.

`scripts/stage_vendor.sh` additionally installs the init rc and VINTF fragment from `scripts/`. Neither script repacks or flashes an image.
