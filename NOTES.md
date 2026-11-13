# URCC — findings for the X6739 (MT6893) port

Status: analysis complete for the configuration format and the device node map.
The vendor half is still staged-but-disabled; nothing has been repacked or flashed.

## 1. What is in the tree today

| Artifact | Path | State |
|---|---|---|
| Rebuilt AIDL service | `erofs/vendor/bin/hw/vendor.oplus.hardware.urcc-service` | staged, 29912 bytes, rebuilt from `src/` |
| AIDL transport sidecar | `erofs/vendor/lib64/vendor.oplus.hardware.urcc-V1-ndk_platform.so` | shipped binary, kept as-is |
| Closed core library | `erofs/vendor/lib64/liburcccore.so` | shipped binary (md5 `37a118117e34eba2e4159b26bf363f21`, identical to the reference ODM) |
| init rc | `erofs/vendor/etc/init/vendor.oplus.hardware.urcc-service.rc` | `disabled`, header documents why |
| VINTF fragment | `ReconstructedHALSource/Oplus/OplusUrcc/scripts/manifest_oplus_urcc_aidl.xml` | not in `erofs/vendor/etc/vintf/manifest/` |

Device state (live): `service list | grep urcc` → empty; the framework logs
`I UAH-UahAdaptHelper: getAidlService uah urcc service =  null`.

The service binary is a thin shim: it `dlopen("liburcccore.so")`, resolves 13
C-linkage entry points, calls `urccModuleInit()` from the constructor and
registers `vendor.oplus.hardware.urcc.IUrcc/default`. All behaviour lives in
the closed core.

## 2. The core's configuration format (recovered from disassembly)

`liburcccore.so` builds its resource map from XML files found on disk.

**Discovery** (`UrccResMap::urccResMapInit`, 0x484e4):
`getConfigFileList("/odm/etc/urcc")` then `getConfigFileList("/vendor/etc/urcc")`.
`getConfigFileList` → `getAllFiles(dir, vector)`; the list is a static global, so
the first directory that yields files wins.

**Per file** (`UrccResMap::urccParseXmlFile`, 0x47ee0):
the file name suffix (last three characters) is compared with `.en`.
- suffix `.en` → the file is AES-decoded (`decode_aes`) into memory and fed to
  `xmlReadMemory(..., "utf-8", 0x100)`.
- any other suffix → parsed directly with `xmlReadFile(path, "UTF-8", 0x100)`.

A plain (non-`.en`) XML file therefore needs no encryption.

**Root element**: must be `ResourceConfigs`, else the file is ignored.
Root attribute `mode_id` is mandatory (`mode_id is null, can not parse this config`).

**Structure**

```
<ResourceConfigs mode_id="...">
  <config module_id="..." module="...">
    <resourcenode name="..." res_id="..." nodepath="..." .../>
    ...
  </config>
  ...
</ResourceConfigs>
```

- `<config>` children carry `module_id` (mandatory) and `module` (the module
  name string, e.g. `cpu`, `gpu`).
- `<resourcenode>` children are parsed by `urccParseResource` (0x463a4).
  Attributes read, in order:
  `name` (mandatory), `res_id` (mandatory, decimal via `stringToInt`),
  `data_type` (`string`|`int`|`long`|`float`, default `string`),
  `opt_type` (`read_only` → 1, otherwise 0),
  `nodepath` (mandatory, the sysfs path),
  `default_value` (default `-1`),
  `setvalue_limit_type` (`no_limit`|`in_list`|`in_range`|`in_list_range`),
  `default_value_update_mode` (via `convertToDVUM`),
  `default_value_update_path`,
  `available_value` (comma separated list),
  plus `available_value_update_mode` / `available_value_update_path`,
  `mutiBiggerIsBetter` and `duration` seen elsewhere in the strings.

**Effective res id.** `urccParseResource` stores
`(module_id << 16) | ((mode_id & 0xf) << 24) | ((res_id & 0x1ff) << 4)`.
The framework's `UAHResourceConstants` ids (`0x1010ba0`, `0x1020080`, …) do
**not** match the library's own built-in table
(`builtin_name_to_resid.tsv`, e.g. `CPU_CORE_CTL_MAX_CPUS_GOLD = 0x01010ba0`,
`GPU_MAX_CORE_SILVER = 0x01020080`), so the exact composition of the id the
framework uses has to be pinned down before a config can be authored. **This is
the one open item in the URCC work.**

**Missing node.** `getResValue` → `urccReadNode(path, buf, 0x100)`; on failure
the value becomes the string of the negative return code and the request is
still answered with `res_Value` set — a missing node does not crash by itself.
The `SIGSEGV` noted earlier comes from the request/state path when the res map
has no entry for the requested id (`getUrccResNode` returns null), not from the
node read.

## 3. What the shipped core hardcodes (Qualcomm-only)

Built-in node paths in the binary: `/sys/class/kgsl/kgsl-3d0/{min,max,default}_pwrlevel`,
`/sys/class/kgsl/kgsl-3d0/devfreq/{min,max}_freq`,
`/sys/class/devfreq/soc:qcom,cpu0-llcc-ddr-lat/*`,
`/sys/class/devfreq/soc:qcom,cpu-cpu-llcc-bw/*`,
`/sys/class/devfreq/18321000.qcom,devfreq-l3:qcom,cpu0-cpu-l3-lat/*`,
`/sys/devices/system/cpu/cpufreq/policy{0,6,7}/scaling_{min,max}_freq`,
`/sys/devices/system/cpu/cpu{0..7}/online`,
plus `/vendor/lib64/libpowerhal.so` (`libpowerhal_setCgPolicy`) and
`/odm/lib64/libuahcore.so`.
None of the kgsl/qcom paths exist on this MTK board; the cpufreq policy indices
are 0/6/7 here while the device uses 0/4/7.

## 4. MTK node map (all values read on the device)

Full table with commands and raw output: `RE/../analysis/x6739-node-map.md`
(450 lines, `raw_capture.txt`, `collect.sh`).

Cluster mapping on this board: little = `policy0` (cpus 0-3, capacity 227),
big = `policy4` (cpus 4-6, capacity 876), prime = `policy7` (cpu 7, capacity 1024).

| OPlus class | This device |
|---|---|
| `CPU_FREQ_SCALING_MIN/MAX_SILVER` | `policy0/scaling_{min,max}_freq` (500000 / 2000000) |
| `CPU_FREQ_SCALING_MIN/MAX_GOLD` | `policy4/scaling_{min,max}_freq` (437000 / 1740000) |
| `CPU_FREQ_SCALING_MIN/MAX_PRIME` | `policy7/scaling_{min,max}_freq` (659000 / 2284000) |
| `CPU_CORE_CTL_MIN/MAX_CPUS_*` | `cpu{0,4,7}/core_ctl/{min,max}_cpus` (4/4, 2/3, 0/1) — exact 1:1 equivalent |
| `CPU_FPSGO_FORCE_ONOFF` | `/sys/kernel/fpsgo/common/force_onoff` (`force on`) |
| `CPU_CPUSET_FOREGROUND_CPUS` | cgroup v1 `/dev/cpuset/foreground/cpus` = `0-7` |
| `GPU_FREQ_SCALING_MIN/MAX/CUR` | `/sys/class/devfreq/13000000.mali/{min,max,cur}_freq` (350/886/350 MHz, 41 steps) |
| `GPU_MIN/MAX_CORE_SILVER` | same mali node (single clock domain) |
| `GPU_MIN/MAX_CLOCK_MHZ_SILVER` | mali `min_freq`/`max_freq` ÷ 1e6 |
| `GPU_BUSY_PERCENTAGE` | no % node; compute from `/sys/kernel/debug/mali0/dvfs_utilization` busy/idle deltas |
| `MEM_DCVS_DDR_*` | `/sys/class/devfreq/mtk-dvfsrc-devfreq/{cur,min,max}_freq` (800000000) + `/sys/kernel/helio-dvfsrc/dvfsrc_ddr_opp_table` |
| `THERMAL_*` | `thermal_zone5` (mtktscpu) `temp`/`trip_point_*`, `cooling_device{14,15,16}/cur_state` (`cpu_adaptive_0/1/2`), `fbt/thrm_enable`, `fbt/thrm_temp_th` = 65 |
| `MISC_SCREEN_OFF_STATE` | no node; `dumpsys display` → `Display State`, `dumpsys power` → `mWakefulness` |
| `CPU_SCHED_GROUP_UP/DOWNMIGRATE` | no node; closest are `/sys/devices/system/cpu/sched-avg/over_util` (up_thres/dn_thres) and `core_ctl/not_preferred` |
| `CPU_SCHED_WINDOW_TICKS_UPDATE` | no node; use per-task `uclamp.min/max` from `/proc/<pid>/sched` |
| `CPU_FREQ_TARGET_LOAD_*` | no node; derive from `/proc/stat` deltas |
| `CPU_SCHED_TOUCH_BOOST` | no knob; emulate via FPSGO/uclamp boost |
| `CPU_SCHED_MIN_TASK_UTIL_FOR_COLOCATION` | no node; `over_util` `maximum task_utils` + `fbt/table_cap` |

Confirmed absent: `/sys/kernel/debug/sched`, `sched/tunables`, `schedtune`,
`/dev/stune`, `/sys/kernel/debug/gpu`, `/sys/kernel/debug/fpsgo`,
`/sys/kernel/debug/thermal`, cgroup-v2 cpuset, `policy*/governor`,
per-CPU `cpu_util`, `/proc/fpsgo`.

## 5. The panel's CPU%/GPU% does not come from URCC

`com.oplus.games` (`/my_stock/priv-app/Games/Games.apk`), classes9.dex:
`com.games.view.toolbox.perf.PerfCpuGpuView` / `PerfDataChartView` /
`GamePerfBar` / `PerformanceModeToolHost` carry **no** usage telemetry — they
render LOW/NORMAL/HIGH performance levels and a radar chart built locally from
COSA perf-mode parameters.

The actual percentages in the float "Data usage" window come from the Oplus
`oplusoiface` binder service (`com.oplus.oiface.IOIfaceService`):
`DataUsageView.K` → `usage/b.a` → `BasicInfoServiceProxy` →
`OifaceManager.getAllLoadInfo(pkg)`, a colon-separated string where field[1] is
CPU load % and field[2] is GPU load %; FPS comes from `getFPS(pkg, 0)`.
No `/proc` or `/sys` read for CPU/GPU load exists in any of the nine dex files,
and osense/orms/UAHAdaptHelper/the vendor performance HAL are not involved.

`oiface` itself (`/system_ext/bin/oiface`, running as `u:r:oiface:s0`, pid
observed) is Qualcomm-oriented for the load path:
`/acct/cpuacct.usage_percpu` (absent here) with `/proc/stat` fallback,
`/sys/class/kgsl/kgsl-3d0/devfreq/gpu_load` (absent) and
`/sys/kernel/gpu/gpu_busy` (absent) — but it does hold an open fd on the MTK
node `/sys/kernel/ged/hal/gpu_utilization`. Its platform probe reads
`/proc/device-tree/chosen/atag,chipid` (root-only readable; the chipid bytes are
present) and `/my_product/etc/extension/license_mtk` (absent from this tree) and
logs `platform is unknown` / `unable to get cpu usage due to unknown platform`
when the probe fails.

**Consequence for the reported symptom:** the 0% CPU and missing GPU usage are
an `oiface` platform-probe / node-permission problem, not a URCC problem.
Finishing URCC will not by itself make those two numbers move.

## 6. Open items

1. Pin down the res-id composition the framework uses, so an XML config can be
   authored (see §2).
2. Decide between shipping an MTK `ResourceConfigs` XML for the closed core
   (cheap, but the core's built-in QCOM table and the id mismatch may still
   bite) and rebuilding the core from source (expensive, but fully under our
   control and consistent with how `OplusPerf` was done — see
   `ReconstructedHALSource/Oplus/OplusPerf/NOTES.md` for the node-remap pattern
   used there).
3. For the telemetry symptom itself: give `oiface` a working platform probe
   and readable MTK load nodes (device-tree chipid access, `license_mtk`, and
   the mali/ged utilisation node permissions).

## 7. Rust rewrite (2026-11-13): DONE, live-tested, staged

The vendor half is now a from-scratch Rust service; the C++ reconstruction
(`src/`) is kept as the record. The closed `liburcccore.so` is not dlopen'ed
at all (it drives Qualcomm-only nodes and faults on this MTK board), so the
resource handling is native. Location: `rust/` (crate `urcc-service`), built
by `build_rust.sh`, staged by `scripts/stage_rust.sh`.

### Wire format: reference for any HAL rebuilt on this ROM

The binder protocol is implemented by hand against `libbinder_ndk.so`'s C API
(`AIBinder_Class_define` / `AParcel_*`), which is a stable C ABI. Three
non-obvious facts were pinned down by tracing the generated client's parcels:

1. **The interface token IS consumed before `onTransact` sees the parcel.**
   A no-arg transaction arrives with the token already consumed: 84 bytes of
   `[strictmode int32][worksource int32][-1]["SYST" magic][int32 len][UTF-16
   descriptor]` precede the argument data on this build, and the position the
   callback starts at is 84.
2. **Every parcelable is sent as `int32 1` (non-null marker) + `int32 size`
   (bytes INCLUDING the size field itself, written as 0 and patched) +
   fields.** `AParcel_writeParcelable` in `binder_parcel_utils.h` writes the
   marker; the generated `writeToParcel` writes and patches the size. A
   missing marker or size on write makes the generated client treat the first
   field as the marker/size and fail; a missing marker on read shifts every
   following field.
3. **`AIBinder_prepareTransaction` refuses a remote binder that has not been
   associated with a class**: the client AND the service must both
   `AIBinder_associateClass` (this is what `IUrcc::fromBinder` and the
   callback registration do in the generated code). Also note
   `AServiceManager_addService` / `ABinderProcess_*` are platform-only: the
   link must use the ROM tree's real `libbinder_ndk.so`, not the NDK stub.

Reserved codes on this ROM's stub: `getInterfaceVersion` = 16777215,
`getInterfaceHash` = 16777214 (the "original" pair, see
`IUrcc$Stub.smali`). Reply layout: `[int32 status = 0][out-parameters]`.

### Function-by-function coverage vs the shipped original

The 13 `dlsym` entry points the shipped service resolves from
`liburcccore.so`, and the 14 AIDL methods it forwards to them:

| Shipped service / core entry | Rust port (`rust/src/`) | How served |
|---|---|---|
| urccModuleInit / urccModuleExit | `main()` registration + no-op | class define, register, join |
| urccInit (1) | `dispatch_init` | status OK |
| urccResCtlRequest (2) | `dispatch_res_ctl_request` | applies each item, answers handle 0 (framework treats non-zero as failure) |
| urccResCtlRelease (3) | `dispatch_res_ctl_release` | drops the listening callback behind the handle |
| urccResStateRequest (4) | `dispatch_res_state_request` | per-item state, by id or by node path; empty list answers the sampled telemetry (GPU busy + 3 cluster loads) |
| urccResListeningRegister (5) | `dispatch_res_listening_register` | stores the callback binder (class-associated, extra strong ref), returns a handle |
| urccResListeningUnRegister (6) | `dispatch_res_listening_unregister` | drops the handle |
| urccPropertyGet (7) | `dispatch_property_get` | **real system-property read** (`__system_property_get`): the shipped core is literally `property_get(key, buf, NULL)` (verified at 0x5b470); the C++ port wrongly used an in-memory store |
| urccPropertySet (8) | `dispatch_property_set` | `__system_property_set`, plus the `vendor.urcc.zdbdebug_on` / `vendor.urcc.unittest_on` debug-flag special cases (verified at 0x63f5c) |
| urccThermalListeningRegister (9) | `dispatch_thermal_listening_register` | registers the callback; a sampler pushes the THERMAL_* states on a ≥1 °C zone change |
| urccThermalListeningUnRegister (10) | `dispatch_thermal_listening_unregister` | drops the handle (unknown handle still answers 0) |
| uahNotifyExt (11) | `dispatch_uah_notify_ext` | consumes args, answers 0 |
| setRelatedSysInfo (12) | `dispatch_set_related_sys_info` | ignores args, answers 0 (as the shipped service does) |
| urccRuleCtl (13) | `dispatch_rule_ctl` | applies the rule data items |
| uahResCtlRequestBypass (14) | `dispatch_res_ctl_request_bypass` | applies, status-only reply |
| urccSendNotification | sampler threads | the sampler pushes events to registered callbacks (the shipped service exposed it; the framework never calls it directly) |
| getInterfaceVersion / getInterfaceHash (reserved) | dispatch | 1 / `cae43655…` |

Node mapping (QCOM-only paths in the shipped core → this MT6893 board), with
the compat/simulation classes that have no node:

| OPlus resource class | Served from |
|---|---|
| kgsl `min/max/default_pwrlevel`, `devfreq/{min,max}_freq` | `/sys/class/devfreq/13000000.mali/{min,max,cur}_freq` (kind `gpu_busy` from `/proc/mtk_mali/utilization` ACTIVE=, fallback GED `gpu_loading`) |
| `devfreq/soc:qcom,*` (DDR/LLCC latency) | `/sys/class/devfreq/mtk-dvfsrc-devfreq/*` + `/sys/kernel/helio-dvfsrc/dvfsrc_ddr_opp_table` |
| `cpufreq/policy{0,6,7}` | `policy{0,4,7}` (this board's clusters: little 0-3, big 4-6, prime 7) |
| `cpu{0..7}/online` | `cpu{0,4,7}/core_ctl/{min,max}_cpus` (1:1 equivalent, from the device survey) |
| CPU load / target load (no node) | compat: `core_ctl/global_state` "CPU utils(%):" average per cluster |
| GPU busy % (no node) | compat: mali utilization ACTIVE= → GED `gpu_loading` |
| GPU MHz (unit adjustment) | `mali/{min,max}_freq` ÷ 1e6 (Hz → MHz) |
| sched up/downmigrate (no node) | compat: `core_ctl/cpu4/{up,dn}_thres` (read + write) |
| touch boost (no node) | applied: fpsgo `common/force_onoff` + `fbt/enable_uclamp_boost` |
| window ticks (no node) | applied: `fpsgo/fstb/fstb_tune_window_size` |
| freq bouncing (no node) | applied: `fpsgo/fbt/thrm_enable` |
| max task util for colocation | compat: `sched-avg/over_util` "maximum task_utils" |
| screen-off state (no node) | `dumpsys display` via popen; "-1" when the HAL domain is denied (honest unknown) |
| thermal | `thermal_zone5` (mtktscpu) + `cooling_device{14,15,16}` (cpu_adaptive_0/1/2) |
| `/odm/lib64/libuahcore.so`, `libpowerhal.so` | not loaded: the resource core is native, no helper libs |

Everything the closed core hardcodes is therefore either remapped to the
existing MTK node, or served by a documented compat/simulation layer.

### Test evidence (2026-11-13, all against the running Rust service via root)

- `urcc_probe` (Rust, the framework's exact protocol): **20/20 checks pass**,
  covering all 14 methods plus version and hash, telemetry with live values
  (GPU busy, per-cluster loads, thermal 43 °C, core_ctl), the control request,
  register and unregister, the property round-trip, and sampler events
  delivered to a local `IUrccCallback` binder.
- The generated-ndk C++ test client (a different client implementation,
  linked against the same generated backend the ROM's clients use):
  `state 0x01010010` → `500000` (real `scaling_min_freq`), `state 0x010200c0`
  → `350` (GPU min MHz, unit conversion), `property ro.hardware` → `mt6893`
  (the shipped `property_get` semantics the C++ port got wrong), and the
  write path `ctl 0x01010b10 2` moves `cpu0/core_ctl/min_cpus` 4→2 and back
  to 4 with `handle=0`.
- Host: `cargo test`: 6/6 (table uniqueness/normalization, io round-trip,
  property stub).
- 5-minute soak: single instance, 3 threads, 20 MB RSS, still registered, no
  crash entries.

Nothing has been repacked or flashed; the tree was staged
(`erofs/vendor/bin/hw/vendor.oplus.hardware.urcc-service`, mode 0755, the rc
and VINTF fragment already point at it). The init rc's `user root` already
runs the service in the same HAL context the live test used. The staged binary
is the rebuild after the code-comment audit: comment-only edits shift embedded
panic-location line numbers, so it differs from the live-tested build by 14
bytes of location metadata with an identical .text, and the test evidence above
still describes its behaviour.

