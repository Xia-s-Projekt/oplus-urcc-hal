#!/usr/bin/env python3
"""Generate src/UrccResourceTable.inc, the URCC resource table.

The table is derived from the resource-name/id table passed with --builtin so
that the ids served are the ids the framework sends.

Id spaces
---------
A resource id is composed as

    id = ((mode_id & 0xf) << 24) | ((module_id & 0xff) << 16) | ((res_id & 0x1ff) << 4)

The built-in table uses mode 1 (ids like 0x01010ba0), while the framework's
UAHResourceConstants use mode 0 (ids like 0x1010ba0). The ids below are the
built-in ids with the mode nibble cleared.

Node mapping
------------
Resource classes without a direct sysfs node are marked kApplied / kDerived:
the service implements the equivalent behaviour natively instead of reporting
a fixed number.

The generated file is included by src/UrccCore.cpp.
"""

import argparse
import os
import re
import sys

# --- cluster topology ---
CLUSTER = {
    "SILVER": ("policy0", "cpu0"),
    "GOLD": ("policy4", "cpu4"),
    "PRIME": ("policy7", "cpu7"),
    "TITANIUM": ("policy7", "cpu7"),
}
MALI = "/sys/class/devfreq/13000000.mali"
DVFSRC = "/sys/class/devfreq/mtk-dvfsrc-devfreq"
FPSGO = "/sys/kernel/fpsgo/common"
THERMAL = "/sys/class/thermal"

CPUSET = {
    "FOREGROUND": "/dev/cpuset/foreground/cpus",
    "BACKGROUND": "/dev/cpuset/background/cpus",
    "SYSTEM_BACKGROUND": "/dev/cpuset/system-background/cpus",
    "TOP_APP": "/dev/cpuset/top-app/cpus",
    "DISPLAY": "/dev/cpuset/display/cpus",
    "CAMERA_BACKGROUND": "/dev/cpuset/camera-background/cpus",
    "CAMERA_DAEMON": "/dev/cpuset/camera-daemon/cpus",
    "SURFACEFLINGER": "/dev/cpuset/sf/cpus",
}

# cooling_device per cluster for the thermal boost state
THERMAL_CDEV = {"SILVER": 14, "GOLD": 15, "PRIME": 16}

# name -> (kind, path, action)
#   kind: kRead, kWrite, kReadWrite, kDerived, kApplied
MAP = {}


def add(name, kind, path=None, action=None):
    MAP[name] = (kind, path, action)


# CPU frequency limits -> cpufreq policies
for kind_suffix, node in (("MIN", "scaling_min_freq"), ("MAX", "scaling_max_freq")):
    for cluster, (policy, _) in CLUSTER.items():
        add("CPU_FREQ_SCALING_%s_%s" % (kind_suffix, cluster), "kReadWrite",
            "/sys/devices/system/cpu/cpufreq/%s/%s" % (policy, node))
for cluster, (policy, _) in CLUSTER.items():
    add("CPU_FREQ_SCALING_CUR_%s" % cluster, "kRead",
        "/sys/devices/system/cpu/cpufreq/%s/scaling_cur_freq" % policy)
    add("CPU_FREQ_SCALING_AVAILABLE_%s" % cluster, "kRead",
        "/sys/devices/system/cpu/cpufreq/%s/scaling_available_frequencies" % policy)

# core control -> core_ctl
for kind_suffix, node in (("MIN", "min_cpus"), ("MAX", "max_cpus")):
    for cluster, (_, cpu) in CLUSTER.items():
        add("CPU_CORE_CTL_%s_CPUS_%s" % (kind_suffix, cluster), "kReadWrite",
            "/sys/devices/system/cpu/%s/core_ctl/%s" % (cpu, node))
for cluster, (_, cpu) in CLUSTER.items():
    add("CPU_CORE_CTL_ACTIVE_CPUS_%s" % cluster, "kDerived", None,
        "cluster_active_%d" % (list(CLUSTER).index(cluster)))

# cpusets
for group, path in CPUSET.items():
    add("CPU_CPUSET_%s_CPUS" % group, "kReadWrite", path)

# FPSGO
add("CPU_FPSGO_FORCE_ONOFF", "kReadWrite", "%s/force_onoff" % FPSGO)
add("CPU_FPSGO_FBT_SWITCH_IDLEPREFER", "kRead", "/sys/kernel/fpsgo/fbt/switch_idleprefer")

# frequency target load: the load the cluster actually runs at
for i, cluster in enumerate(("SILVER", "GOLD", "PRIME")):
    add("CPU_FREQ_TARGET_LOAD_%s" % cluster, "kDerived", None, "cluster_load_%d" % i)

# scheduling classes with no direct node
add("CPU_SCHED_GROUP_UPMIGRATE", "kReadWrite", None, "set_up_thres")
add("CPU_SCHED_GROUP_DOWNMIGRATE", "kReadWrite", None, "set_dn_thres")
add("CPU_SCHED_UPMIGRATE", "kReadWrite", None, "set_up_thres")
add("CPU_SCHED_DOWNMIGRATE", "kReadWrite", None, "set_dn_thres")
add("CPU_SCHED_MIN_TASK_UTIL_FOR_COLOCATION", "kDerived", None, "max_task_util")
add("CPU_SCHED_TOUCH_BOOST", "kApplied", None, "touch_boost")
add("CPU_SCHED_WINDOW_TICKS_UPDATE", "kApplied", None, "window_ticks")
add("CPU_FREQ_BOUNCING_ENABLE", "kReadWrite", None, "bouncing_enable")

# GPU
add("GPU_FREQ_SCALING_MIN_SILVER", "kRead", "%s/min_freq" % MALI)
add("GPU_FREQ_SCALING_MAX_SILVER", "kRead", "%s/max_freq" % MALI)
add("GPU_FREQ_SCALING_CUR_SILVER", "kRead", "%s/cur_freq" % MALI)
add("GPU_MIN_CORE_SILVER", "kRead", "%s/min_freq" % MALI)
add("GPU_MAX_CORE_SILVER", "kRead", "%s/max_freq" % MALI)
add("GPU_HARD_FREQ_SCALING_MIN_SILVER", "kRead", "%s/min_freq" % MALI)
add("GPU_HARD_FREQ_SCALING_MAX_SILVER", "kRead", "%s/max_freq" % MALI)
add("GPU_MIN_CLOCK_MHZ_SILVER", "kDerived", None, "gpu_min_mhz")
add("GPU_MAX_CLOCK_MHZ_SILVER", "kDerived", None, "gpu_max_mhz")
add("GPU_BUSY_PERCENTAGE", "kDerived", None, "gpu_busy")

# memory
add("MEM_DCVS_DDR_BOOST_FREQ", "kRead", "%s/max_freq" % DVFSRC)
add("MEM_DCVS_DDR_FREQ_SCALE_CEIL_MHZ_GOLD", "kRead", "%s/max_freq" % DVFSRC)
add("MEM_DCVS_DRAM_OPP_TABLE", "kRead", "/sys/kernel/helio-dvfsrc/dvfsrc_ddr_opp_table")

# thermal
add("THERMAL_ZONE_TYPE", "kRead", "%s/thermal_zone5/type" % THERMAL)
add("THERMAL_ZONE_MODE", "kRead", "%s/thermal_zone5/mode" % THERMAL)
add("THERMAL_ZONE_CDEV_WEIGHT", "kRead", "%s/thermal_zone5/cdev0_weight" % THERMAL)
# Thermal-boost classes are per-cluster variants of the core-control and
# frequency-limit resources; the boost gate is core_ctl's thermal_up_thres.
for cluster, (_, cpu) in CLUSTER.items():
    add("CPU_CORE_CTL_MAX_CPUS_%s_THERMAL_BOOST" % cluster, "kReadWrite",
        "/sys/devices/system/cpu/%s/core_ctl/thermal_up_thres" % cpu)
    add("CPU_FREQ_SCALING_MAX_%s_THERMAL_BOOST" % cluster, "kReadWrite",
        "/sys/devices/system/cpu/cpufreq/%s/scaling_max_freq" % CLUSTER[cluster][0])
add("THERMAL_BOOST_BREAK_LEVEL", "kRead", "%s/cooling_device14/cur_state" % THERMAL)
add("THERMAL_BOOST_SERIOUS_LEVEL", "kRead", "%s/cooling_device15/cur_state" % THERMAL)
add("THERMAL_BOOST_ALL_BREAK_LIMIT", "kRead", "%s/cooling_device16/cur_state" % THERMAL)

# misc
add("MISC_SCREEN_OFF_STATE", "kDerived", None, "screen_off")


def load_builtin(path):
    out = {}
    with open(path) as fh:
        for line in fh:
            parts = line.rstrip("\n").split("\t")
            if len(parts) == 2 and parts[1].startswith("0x"):
                out[parts[0]] = int(parts[1], 16)
    return out


def cstr(text):
    return '"%s"' % text.replace("\\", "\\\\").replace('"', '\\"')


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--builtin", required=True)
    ap.add_argument("--out", required=True)
    args = ap.parse_args()

    builtin = load_builtin(args.builtin)

    rows = []
    skipped = []
    for name, rid in sorted(builtin.items()):
        if name not in MAP:
            continue
        # The framework's UAHResourceConstants ids are identical to the
        # built-in table ids (mode 1, module byte, res field), so the id is
        # used unchanged.
        framework_id = rid
        kind, path, action = MAP[name]
        rows.append((framework_id, name, (rid >> 16) & 0xff, kind, path, action))

    lines = [
        "// Auto-generated from scripts/gen_urcc_resource_table.py; do not edit.",
        "//",
        "// Resource table for the OPlus URCC HAL: OnePlus resource name, the id the",
        "// framework uses, and the sysfs node or action backing it.",
        "",
        "const UrccResource kUrccResources[] = {",
    ]
    for rid, name, module, kind, path, action in rows:
        parts = ["0x%08x" % rid, cstr(name), str(module), "Kind::%s" % kind]
        if path is not None:
            parts.append(cstr(path))
        else:
            parts.append("nullptr")
        if action is not None:
            parts.append(cstr(action))
        else:
            parts.append("nullptr")
        lines.append("    {%s}," % ", ".join(parts))
    lines.append("};")
    lines.append("")
    lines.append("const size_t kUrccResourceCount = %d;" % len(rows))
    lines.append("")

    os.makedirs(os.path.dirname(os.path.abspath(args.out)), exist_ok=True)
    with open(args.out, "w") as fh:
        fh.write("\n".join(lines))

    sys.stderr.write("wrote %s: %d resources\n" % (args.out, len(rows)))
    for name, rid in skipped:
        sys.stderr.write("  skipped %s %s (res field exceeds 9 bits)\n" % (name, hex(rid)))


if __name__ == "__main__":
    main()
