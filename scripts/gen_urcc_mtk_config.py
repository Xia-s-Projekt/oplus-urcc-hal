#!/usr/bin/env python3
"""Generate the MTK URCC resource map for the X6739 (MT6893) port.

The shipped liburcccore.so builds its resource map from two sources:

  * a built-in table of OnePlus resource names (479 of them) with their
    default, Qualcomm-only node paths, and
  * XML files found in /odm/etc/urcc and /vendor/etc/urcc, whose root element
    is <ResourceConfigs>.

This generator writes that XML for the MTK board:

  /vendor/etc/urcc/urcc_x6739.xml

so that every resource id the framework asks for resolves to a real MTK node
instead of a Qualcomm one.

Id composition (recovered from liburcccore.so, UrccResMap::convertToResId and
UrccResMap::urccParseResource):

    id = ((mode_id & 0xf) << 24) | ((module_id & 0xff) << 16) | ((res_id & 0x1ff) << 4)

  * mode_id   comes from the root <ResourceConfigs mode_id="..."> attribute
  * module_id comes from each <config module_id="..."> element
  * res_id    comes from each <resourcenode res_id="..."> attribute and is
    masked to 9 bits, so ids whose res field exceeds 0x1ff cannot be
    expressed in XML (25 built-in MEM_DCVS_* ids have that shape; the framework
    never asks for them and they are skipped here).

The framework's ids (RE/framework/oplus-framework/com/oplus/uah/
UAHResourceConstants.smali) are identical to the built-in table's ids, so the
generated ids match the framework byte for byte.

Node paths come from the verified device sweep in
ReconstructedHALSource/Oplus/OplusUrcc/analysis/x6739-node-map.md.
Classes with no MTK counterpart are pointed at a file under
/data/vendor/urcc/sim/ that the companion helper (sim/urcc_sim.cpp) keeps
up to date; those entries are marked read_only because the value is computed,
not stored.
"""

import argparse
import os
import re
import sys

# --- cluster topology of the X6739 (verified) -------------------------------
# OPlus cluster names -> this board's cpufreq policy and core_ctl anchor CPU.
CLUSTER = {
    "SILVER": ("policy0", "cpu0"),   # 4x A55, little
    "GOLD": ("policy4", "cpu4"),      # 3x A78, big
    "PRIME": ("policy7", "cpu7"),     # 1x A510, prime
    "TITANIUM": ("policy7", "cpu7"),  # no 4th tier on this SoC -> prime
}

MALI = "/sys/class/devfreq/13000000.mali"
DVFSRC = "/sys/class/devfreq/mtk-dvfsrc-devfreq"
FPSGO = "/sys/kernel/fpsgo/common"
THERMAL_CPU = "/sys/class/thermal/thermal_zone5"
SIM_DIR = "/data/vendor/urcc/sim"

# cgroup v1 cpuset group present on the device -> /dev/cpuset/<group>/cpus
CPUSET_GROUPS = {
    "FOREGROUND": "foreground",
    "BACKGROUND": "background",
    "SYSTEM_BACKGROUND": "system-background",
    "TOP_APP": "top-app",
    "DISPLAY": "display",
    "CAMERA_BACKGROUND": "camera-background",
    "CAMERA_DAEMON": "camera-daemon",
    "SURFACEFLINGER": "sf",
}

# cooling_device index per cluster for the thermal boost state
THERMAL_CDEV = {"SILVER": "14", "GOLD": "15", "PRIME": "16"}

MODULE_NAME = {1: "cpu", 2: "gpu", 3: "mem", 5: "thermal", 6: "misc"}


def sim(name):
    """A computed value: the helper service refreshes this file."""
    return "%s/%s" % (SIM_DIR, name)


def node_for(name):
    """Return (nodepath, writable) for a OnePlus resource name.

    writable=False means the value is computed or the node is read-only, and
    the entry is emitted with opt_type="read_only".
    """
    m = re.fullmatch(r"CPU_FREQ_SCALING_(MIN|MAX|CUR)_(SILVER|GOLD|PRIME|TITANIUM)", name)
    if m:
        policy, _ = CLUSTER[m.group(2)]
        return "/sys/devices/system/cpu/cpufreq/%s/scaling_%s_freq" % (policy, m.group(1).lower()), True

    m = re.fullmatch(r"CPU_CORE_CTL_(MIN|MAX)_CPUS_(SILVER|GOLD|PRIME|TITANIUM)", name)
    if m:
        _, cpu = CLUSTER[m.group(3)]
        return "/sys/devices/system/cpu/%s/core_ctl/%s_cpus" % (cpu, m.group(1).lower()), True

    m = re.fullmatch(r"CPU_CORE_CTL_ONLINE_CPU(\d)", name)
    if m:
        return "/sys/devices/system/cpu/cpu%s/online" % m.group(1), True

    m = re.fullmatch(r"CPU_CPUSET_([A-Z_]+)_CPUS", name)
    if m and m.group(1) in CPUSET_GROUPS:
        return "/dev/cpuset/%s/cpus" % CPUSET_GROUPS[m.group(1)], True

    if name == "CPU_FPSGO_FORCE_ONOFF":
        return "%s/force_onoff" % FPSGO, True
    if name == "CPU_FPSGO_FBT_SWITCH_IDLEPREFER":
        return "%s/../fbt/switch_idleprefer" % FPSGO, True

    if name == "GPU_FREQ_SCALING_MIN_SILVER":
        return "%s/min_freq" % MALI, True
    if name == "GPU_FREQ_SCALING_MAX_SILVER":
        return "%s/max_freq" % MALI, True
    if name == "GPU_FREQ_SCALING_CUR_SILVER":
        return "%s/cur_freq" % MALI, False
    if name in ("GPU_MIN_CORE_SILVER", "GPU_HARD_FREQ_SCALING_MIN_SILVER"):
        return "%s/min_freq" % MALI, True
    if name in ("GPU_MAX_CORE_SILVER", "GPU_HARD_FREQ_SCALING_MAX_SILVER"):
        return "%s/max_freq" % MALI, True

    if name == "MEM_DCVS_DDR_BOOST_FREQ":
        return "%s/max_freq" % DVFSRC, False
    if name == "MEM_DCVS_DRAM_OPP_TABLE":
        return "/sys/kernel/helio-dvfsrc/dvfsrc_ddr_opp_table", False

    if name == "THERMAL_ZONE_TYPE":
        return "%s/type" % THERMAL_CPU, False
    if name == "THERMAL_POLICY_POWERHAL":
        return "%s/policy" % THERMAL_CPU, False

    m = re.fullmatch(r"THERMAL_BOOST_BREAK_LEVEL_(SILVER|GOLD|PRIME|TITANIUM)", name)
    if m:
        cdev = THERMAL_CDEV.get(m.group(1))
        if cdev:
            return "/sys/class/thermal/cooling_device%s/cur_state" % cdev, False
        return sim(name), False

    # Everything else has no MTK counterpart: the helper computes it.
    return sim(name), False


def load_builtin(path):
    """name -> id, from the extracted built-in resource table."""
    out = {}
    with open(path) as fh:
        for line in fh:
            parts = line.rstrip("\n").split("\t")
            if len(parts) == 2 and parts[1].startswith("0x"):
                out[parts[0]] = int(parts[1], 16)
    return out


def escape(text):
    return (text.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")
                .replace('"', "&quot;"))


def generate(builtin, only_framework):
    wanted = None
    if only_framework:
        wanted = set(only_framework)

    by_module = {}
    skipped = []
    for name, rid in sorted(builtin.items()):
        mode = (rid >> 24) & 0xf
        module = (rid >> 16) & 0xff
        res = (rid >> 4) & 0x1ff
        # the XML res_id attribute is masked to 9 bits by the shipped core
        # (urccParseResource: (res_id & 0x1ff) << 4), so ids whose res field
        # is wider than that cannot be expressed in the XML at all.
        if ((rid >> 4) & 0xfff) > 0x1ff:
            skipped.append((name, rid))
            continue
        if wanted is not None and name not in wanted:
            continue
        path, writable = node_for(name)
        by_module.setdefault(module, []).append(
            (name, res, path, writable))

    out = ['<?xml version="1.0" encoding="utf-8"?>',
           "<!--",
           "  URCC resource map for the Infinix GT 10 Pro (X6739, MT6893).",
           "",
           "  Generated by ReconstructedHALSource/Oplus/OplusUrcc/scripts/",
           "gen_urcc_mtk_config.py.",
           "  Every nodepath was read on the device; see analysis/x6739-node-map.md.",
           "",
           "  Resources with no MTK node point at /data/vendor/urcc/sim/<NAME>,",
           "  which the companion helper (sim/urcc_sim.cpp) keeps up to date.",
           "  Those entries are read_only.",
           "-->",
           '<ResourceConfigs mode_id="1">']
    for module in sorted(by_module):
        out.append('  <config module_id="%d" module="%s">'
                   % (module, MODULE_NAME.get(module, "module%d" % module)))
        for name, res, path, writable in by_module[module]:
            attrs = ['name="%s"' % escape(name),
                     'res_id="%d"' % res,
                     'nodepath="%s"' % escape(path),
                     'data_type="int"']
            if not writable:
                attrs.append('opt_type="read_only"')
                attrs.append('default_value="0"')
            out.append("    <resourcenode %s/>" % " ".join(attrs))
        out.append("  </config>")
    out.append("</ResourceConfigs>")
    return "\n".join(out) + "\n", skipped, sum(len(v) for v in by_module.values())


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--builtin", required=True,
                    help="name->id table extracted from liburcccore.so")
    ap.add_argument("--framework-constants", action="store_true",
                    help="only emit the resources the framework asks for")
    ap.add_argument("--out", required=True)
    args = ap.parse_args()

    only = None
    if args.framework_constants:
        fw = os.environ.get("URCC_FRAMEWORK_RESOURCES", "")
        only = [x for x in fw.split(",") if x]

    builtin = load_builtin(args.builtin)
    xml, skipped, count = generate(builtin, only)

    os.makedirs(os.path.dirname(os.path.abspath(args.out)), exist_ok=True)
    with open(args.out, "w") as fh:
        fh.write(xml)

    sys.stderr.write("wrote %s: %d resourcenode entries\n" % (args.out, count))
    if skipped:
        sys.stderr.write("skipped %d ids whose res field exceeds 9 bits:\n" % len(skipped))
        for name, rid in skipped:
            sys.stderr.write("  %s %s\n" % (name, hex(rid)))


if __name__ == "__main__":
    main()
