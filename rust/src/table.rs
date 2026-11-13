// Resource table for the X6739 (MT6893) port of the OPlus URCC HAL.
//
// Ported 1:1 from the C++ reconstruction (src/UrccResourceTable.inc, generated
// by scripts/gen_urcc_resource_table.py from the OnePlus resource names the
// shipped liburcccore.so carries in its built-in table), so the ids served
// here are the ids the framework and the native UAH client send.
//
// id encoding (recovered from liburcccore.so's urccParseResource):
//   library form   ((mode & 0xf) << 24) | ((module & 0xff) << 16) | ((res & 0x1ff) << 4)
//   framework form the same value with the mode nibble moved to bits 28..31
//   (e.g. 0x10010ba0). normalize_id() folds both onto the library form.

#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Value read from a file.
    Read,
    /// A control request writes the file.
    Write,
    /// Both.
    ReadWrite,
    /// Value computed from MTK sources (compatibility/simulation layer).
    Derived,
    /// No node: a control request performs the equivalent action.
    Applied,
}

#[derive(Debug, Clone, Copy)]
pub struct Resource {
    pub id: u32,
    pub name: &'static str,
    /// 1 cpu, 2 gpu, 3 mem, 5 thermal, 6 misc
    pub module: u8,
    pub kind: Kind,
    pub path: Option<&'static str>,
    pub action: Option<&'static str>,
}

pub const RESOURCES: &[Resource] = &[
    // --- CPU: core_ctl -----------------------------------------------------
    Resource { id: 0x01010c20, name: "CPU_CORE_CTL_ACTIVE_CPUS_GOLD", module: 1, kind: Kind::Derived, path: None, action: Some("cluster_active_1") },
    Resource { id: 0x01010c30, name: "CPU_CORE_CTL_ACTIVE_CPUS_PRIME", module: 1, kind: Kind::Derived, path: None, action: Some("cluster_active_2") },
    Resource { id: 0x01010c10, name: "CPU_CORE_CTL_ACTIVE_CPUS_SILVER", module: 1, kind: Kind::Derived, path: None, action: Some("cluster_active_0") },
    Resource { id: 0x01010c40, name: "CPU_CORE_CTL_ACTIVE_CPUS_TITANIUM", module: 1, kind: Kind::Derived, path: None, action: Some("cluster_active_3") },
    Resource { id: 0x01010ba0, name: "CPU_CORE_CTL_MAX_CPUS_GOLD", module: 1, kind: Kind::ReadWrite, path: Some("/sys/devices/system/cpu/cpu4/core_ctl/max_cpus"), action: None },
    Resource { id: 0x01011df0, name: "CPU_CORE_CTL_MAX_CPUS_GOLD_THERMAL_BOOST", module: 1, kind: Kind::ReadWrite, path: Some("/sys/devices/system/cpu/cpu4/core_ctl/thermal_up_thres"), action: None },
    Resource { id: 0x01010bb0, name: "CPU_CORE_CTL_MAX_CPUS_PRIME", module: 1, kind: Kind::ReadWrite, path: Some("/sys/devices/system/cpu/cpu7/core_ctl/max_cpus"), action: None },
    Resource { id: 0x01011e00, name: "CPU_CORE_CTL_MAX_CPUS_PRIME_THERMAL_BOOST", module: 1, kind: Kind::ReadWrite, path: Some("/sys/devices/system/cpu/cpu7/core_ctl/thermal_up_thres"), action: None },
    Resource { id: 0x01010b90, name: "CPU_CORE_CTL_MAX_CPUS_SILVER", module: 1, kind: Kind::ReadWrite, path: Some("/sys/devices/system/cpu/cpu0/core_ctl/max_cpus"), action: None },
    Resource { id: 0x01011de0, name: "CPU_CORE_CTL_MAX_CPUS_SILVER_THERMAL_BOOST", module: 1, kind: Kind::ReadWrite, path: Some("/sys/devices/system/cpu/cpu0/core_ctl/thermal_up_thres"), action: None },
    Resource { id: 0x01010bc0, name: "CPU_CORE_CTL_MAX_CPUS_TITANIUM", module: 1, kind: Kind::ReadWrite, path: Some("/sys/devices/system/cpu/cpu7/core_ctl/max_cpus"), action: None },
    Resource { id: 0x01011e10, name: "CPU_CORE_CTL_MAX_CPUS_TITANIUM_THERMAL_BOOST", module: 1, kind: Kind::ReadWrite, path: Some("/sys/devices/system/cpu/cpu7/core_ctl/thermal_up_thres"), action: None },
    Resource { id: 0x01010b20, name: "CPU_CORE_CTL_MIN_CPUS_GOLD", module: 1, kind: Kind::ReadWrite, path: Some("/sys/devices/system/cpu/cpu4/core_ctl/min_cpus"), action: None },
    Resource { id: 0x01010b30, name: "CPU_CORE_CTL_MIN_CPUS_PRIME", module: 1, kind: Kind::ReadWrite, path: Some("/sys/devices/system/cpu/cpu7/core_ctl/min_cpus"), action: None },
    Resource { id: 0x01010b10, name: "CPU_CORE_CTL_MIN_CPUS_SILVER", module: 1, kind: Kind::ReadWrite, path: Some("/sys/devices/system/cpu/cpu0/core_ctl/min_cpus"), action: None },
    Resource { id: 0x01010b40, name: "CPU_CORE_CTL_MIN_CPUS_TITANIUM", module: 1, kind: Kind::ReadWrite, path: Some("/sys/devices/system/cpu/cpu7/core_ctl/min_cpus"), action: None },
    // --- CPU: cpusets ------------------------------------------------------
    Resource { id: 0x01011590, name: "CPU_CPUSET_BACKGROUND_CPUS", module: 1, kind: Kind::ReadWrite, path: Some("/dev/cpuset/background/cpus"), action: None },
    Resource { id: 0x01011690, name: "CPU_CPUSET_CAMERA_BACKGROUND_CPUS", module: 1, kind: Kind::ReadWrite, path: Some("/dev/cpuset/camera-background/cpus"), action: None },
    Resource { id: 0x010115e0, name: "CPU_CPUSET_DISPLAY_CPUS", module: 1, kind: Kind::ReadWrite, path: Some("/dev/cpuset/display/cpus"), action: None },
    Resource { id: 0x010115b0, name: "CPU_CPUSET_FOREGROUND_CPUS", module: 1, kind: Kind::ReadWrite, path: Some("/dev/cpuset/foreground/cpus"), action: None },
    Resource { id: 0x010115d0, name: "CPU_CPUSET_SURFACEFLINGER_CPUS", module: 1, kind: Kind::ReadWrite, path: Some("/dev/cpuset/sf/cpus"), action: None },
    Resource { id: 0x010115c0, name: "CPU_CPUSET_SYSTEM_BACKGROUND_CPUS", module: 1, kind: Kind::ReadWrite, path: Some("/dev/cpuset/system-background/cpus"), action: None },
    Resource { id: 0x010115a0, name: "CPU_CPUSET_TOP_APP_CPUS", module: 1, kind: Kind::ReadWrite, path: Some("/dev/cpuset/top-app/cpus"), action: None },
    // --- CPU: fpsgo --------------------------------------------------------
    Resource { id: 0x01011900, name: "CPU_FPSGO_FBT_SWITCH_IDLEPREFER", module: 1, kind: Kind::Read, path: Some("/sys/kernel/fpsgo/fbt/switch_idleprefer"), action: None },
    Resource { id: 0x01011910, name: "CPU_FPSGO_FORCE_ONOFF", module: 1, kind: Kind::ReadWrite, path: Some("/sys/kernel/fpsgo/common/force_onoff"), action: None },
    // --- CPU: frequency ----------------------------------------------------
    Resource { id: 0x010103a0, name: "CPU_FREQ_BOUNCING_ENABLE", module: 1, kind: Kind::ReadWrite, path: None, action: Some("bouncing_enable") },
    Resource { id: 0x01010130, name: "CPU_FREQ_SCALING_AVAILABLE_GOLD", module: 1, kind: Kind::Read, path: Some("/sys/devices/system/cpu/cpufreq/policy4/scaling_available_frequencies"), action: None },
    Resource { id: 0x01010140, name: "CPU_FREQ_SCALING_AVAILABLE_PRIME", module: 1, kind: Kind::Read, path: Some("/sys/devices/system/cpu/cpufreq/policy7/scaling_available_frequencies"), action: None },
    Resource { id: 0x01010120, name: "CPU_FREQ_SCALING_AVAILABLE_SILVER", module: 1, kind: Kind::Read, path: Some("/sys/devices/system/cpu/cpufreq/policy0/scaling_available_frequencies"), action: None },
    Resource { id: 0x01010150, name: "CPU_FREQ_SCALING_AVAILABLE_TITANIUM", module: 1, kind: Kind::Read, path: Some("/sys/devices/system/cpu/cpufreq/policy7/scaling_available_frequencies"), action: None },
    Resource { id: 0x010101b0, name: "CPU_FREQ_SCALING_CUR_GOLD", module: 1, kind: Kind::Read, path: Some("/sys/devices/system/cpu/cpufreq/policy4/scaling_cur_freq"), action: None },
    Resource { id: 0x010101c0, name: "CPU_FREQ_SCALING_CUR_PRIME", module: 1, kind: Kind::Read, path: Some("/sys/devices/system/cpu/cpufreq/policy7/scaling_cur_freq"), action: None },
    Resource { id: 0x010101a0, name: "CPU_FREQ_SCALING_CUR_SILVER", module: 1, kind: Kind::Read, path: Some("/sys/devices/system/cpu/cpufreq/policy0/scaling_cur_freq"), action: None },
    Resource { id: 0x010101d0, name: "CPU_FREQ_SCALING_CUR_TITANIUM", module: 1, kind: Kind::Read, path: Some("/sys/devices/system/cpu/cpufreq/policy7/scaling_cur_freq"), action: None },
    Resource { id: 0x010100b0, name: "CPU_FREQ_SCALING_MAX_GOLD", module: 1, kind: Kind::ReadWrite, path: Some("/sys/devices/system/cpu/cpufreq/policy4/scaling_max_freq"), action: None },
    Resource { id: 0x01011e70, name: "CPU_FREQ_SCALING_MAX_GOLD_THERMAL_BOOST", module: 1, kind: Kind::ReadWrite, path: Some("/sys/devices/system/cpu/cpufreq/policy4/scaling_max_freq"), action: None },
    Resource { id: 0x010100c0, name: "CPU_FREQ_SCALING_MAX_PRIME", module: 1, kind: Kind::ReadWrite, path: Some("/sys/devices/system/cpu/cpufreq/policy7/scaling_max_freq"), action: None },
    Resource { id: 0x01011e80, name: "CPU_FREQ_SCALING_MAX_PRIME_THERMAL_BOOST", module: 1, kind: Kind::ReadWrite, path: Some("/sys/devices/system/cpu/cpufreq/policy7/scaling_max_freq"), action: None },
    Resource { id: 0x010100a0, name: "CPU_FREQ_SCALING_MAX_SILVER", module: 1, kind: Kind::ReadWrite, path: Some("/sys/devices/system/cpu/cpufreq/policy0/scaling_max_freq"), action: None },
    Resource { id: 0x01011e60, name: "CPU_FREQ_SCALING_MAX_SILVER_THERMAL_BOOST", module: 1, kind: Kind::ReadWrite, path: Some("/sys/devices/system/cpu/cpufreq/policy0/scaling_max_freq"), action: None },
    Resource { id: 0x010100d0, name: "CPU_FREQ_SCALING_MAX_TITANIUM", module: 1, kind: Kind::ReadWrite, path: Some("/sys/devices/system/cpu/cpufreq/policy7/scaling_max_freq"), action: None },
    Resource { id: 0x01011e90, name: "CPU_FREQ_SCALING_MAX_TITANIUM_THERMAL_BOOST", module: 1, kind: Kind::ReadWrite, path: Some("/sys/devices/system/cpu/cpufreq/policy7/scaling_max_freq"), action: None },
    Resource { id: 0x01010020, name: "CPU_FREQ_SCALING_MIN_GOLD", module: 1, kind: Kind::ReadWrite, path: Some("/sys/devices/system/cpu/cpufreq/policy4/scaling_min_freq"), action: None },
    Resource { id: 0x01010030, name: "CPU_FREQ_SCALING_MIN_PRIME", module: 1, kind: Kind::ReadWrite, path: Some("/sys/devices/system/cpu/cpufreq/policy7/scaling_min_freq"), action: None },
    Resource { id: 0x01010010, name: "CPU_FREQ_SCALING_MIN_SILVER", module: 1, kind: Kind::ReadWrite, path: Some("/sys/devices/system/cpu/cpufreq/policy0/scaling_min_freq"), action: None },
    Resource { id: 0x01010040, name: "CPU_FREQ_SCALING_MIN_TITANIUM", module: 1, kind: Kind::ReadWrite, path: Some("/sys/devices/system/cpu/cpufreq/policy7/scaling_min_freq"), action: None },
    // --- CPU: load / scheduler (no MTK node: compat/sim layer) ------------
    Resource { id: 0x010108b0, name: "CPU_FREQ_TARGET_LOAD_GOLD", module: 1, kind: Kind::Derived, path: None, action: Some("cluster_load_1") },
    Resource { id: 0x010108c0, name: "CPU_FREQ_TARGET_LOAD_PRIME", module: 1, kind: Kind::Derived, path: None, action: Some("cluster_load_2") },
    Resource { id: 0x010108a0, name: "CPU_FREQ_TARGET_LOAD_SILVER", module: 1, kind: Kind::Derived, path: None, action: Some("cluster_load_0") },
    Resource { id: 0x01010f90, name: "CPU_SCHED_DOWNMIGRATE", module: 1, kind: Kind::ReadWrite, path: None, action: Some("set_dn_thres") },
    Resource { id: 0x01010fc0, name: "CPU_SCHED_GROUP_DOWNMIGRATE", module: 1, kind: Kind::ReadWrite, path: None, action: Some("set_dn_thres") },
    Resource { id: 0x01010fb0, name: "CPU_SCHED_GROUP_UPMIGRATE", module: 1, kind: Kind::ReadWrite, path: None, action: Some("set_up_thres") },
    Resource { id: 0x01011040, name: "CPU_SCHED_MIN_TASK_UTIL_FOR_COLOCATION", module: 1, kind: Kind::Derived, path: None, action: Some("max_task_util") },
    Resource { id: 0x010112f0, name: "CPU_SCHED_TOUCH_BOOST", module: 1, kind: Kind::Applied, path: None, action: Some("touch_boost") },
    Resource { id: 0x01010f80, name: "CPU_SCHED_UPMIGRATE", module: 1, kind: Kind::ReadWrite, path: None, action: Some("set_up_thres") },
    Resource { id: 0x01011070, name: "CPU_SCHED_WINDOW_TICKS_UPDATE", module: 1, kind: Kind::Applied, path: None, action: Some("window_ticks") },
    // --- GPU ---------------------------------------------------------------
    Resource { id: 0x01020220, name: "GPU_BUSY_PERCENTAGE", module: 2, kind: Kind::Derived, path: None, action: Some("gpu_busy") },
    Resource { id: 0x01020090, name: "GPU_FREQ_SCALING_CUR_SILVER", module: 2, kind: Kind::Read, path: Some("/sys/class/devfreq/13000000.mali/cur_freq"), action: None },
    Resource { id: 0x01020060, name: "GPU_FREQ_SCALING_MAX_SILVER", module: 2, kind: Kind::Read, path: Some("/sys/class/devfreq/13000000.mali/max_freq"), action: None },
    Resource { id: 0x01020050, name: "GPU_FREQ_SCALING_MIN_SILVER", module: 2, kind: Kind::Read, path: Some("/sys/class/devfreq/13000000.mali/min_freq"), action: None },
    Resource { id: 0x01021ff0, name: "GPU_HARD_FREQ_SCALING_MAX_SILVER", module: 2, kind: Kind::Read, path: Some("/sys/class/devfreq/13000000.mali/max_freq"), action: None },
    Resource { id: 0x01021fe0, name: "GPU_HARD_FREQ_SCALING_MIN_SILVER", module: 2, kind: Kind::Read, path: Some("/sys/class/devfreq/13000000.mali/min_freq"), action: None },
    // MHz form: the mali nodes are in Hz, the resource is read in MHz, so the
    // compat layer divides by 1e6 (the unit adjustment the port applies).
    Resource { id: 0x010200b0, name: "GPU_MAX_CLOCK_MHZ_SILVER", module: 2, kind: Kind::Derived, path: None, action: Some("gpu_max_mhz") },
    Resource { id: 0x01020080, name: "GPU_MAX_CORE_SILVER", module: 2, kind: Kind::Read, path: Some("/sys/class/devfreq/13000000.mali/max_freq"), action: None },
    Resource { id: 0x010200c0, name: "GPU_MIN_CLOCK_MHZ_SILVER", module: 2, kind: Kind::Derived, path: None, action: Some("gpu_min_mhz") },
    Resource { id: 0x01020070, name: "GPU_MIN_CORE_SILVER", module: 2, kind: Kind::Read, path: Some("/sys/class/devfreq/13000000.mali/min_freq"), action: None },
    // --- Memory ------------------------------------------------------------
    Resource { id: 0x01030fc0, name: "MEM_DCVS_DDR_BOOST_FREQ", module: 3, kind: Kind::Read, path: Some("/sys/class/devfreq/mtk-dvfsrc-devfreq/max_freq"), action: None },
    Resource { id: 0x010326a0, name: "MEM_DCVS_DRAM_OPP_TABLE", module: 3, kind: Kind::Read, path: Some("/sys/kernel/helio-dvfsrc/dvfsrc_ddr_opp_table"), action: None },
    // --- Misc --------------------------------------------------------------
    Resource { id: 0x01060000, name: "MISC_SCREEN_OFF_STATE", module: 6, kind: Kind::Derived, path: None, action: Some("screen_off") },
    // --- Thermal -----------------------------------------------------------
    Resource { id: 0x01051fe0, name: "THERMAL_BOOST_ALL_BREAK_LIMIT", module: 5, kind: Kind::Read, path: Some("/sys/class/thermal/cooling_device16/cur_state"), action: None },
    Resource { id: 0x01051ff0, name: "THERMAL_BOOST_BREAK_LEVEL", module: 5, kind: Kind::Read, path: Some("/sys/class/thermal/cooling_device14/cur_state"), action: None },
    Resource { id: 0x01051fd0, name: "THERMAL_BOOST_SERIOUS_LEVEL", module: 5, kind: Kind::Read, path: Some("/sys/class/thermal/cooling_device15/cur_state"), action: None },
    Resource { id: 0x01050030, name: "THERMAL_ZONE_CDEV_WEIGHT", module: 5, kind: Kind::Read, path: Some("/sys/class/thermal/thermal_zone5/cdev0_weight"), action: None },
    Resource { id: 0x01050050, name: "THERMAL_ZONE_MODE", module: 5, kind: Kind::Read, path: Some("/sys/class/thermal/thermal_zone5/mode"), action: None },
    Resource { id: 0x01050040, name: "THERMAL_ZONE_TYPE", module: 5, kind: Kind::Read, path: Some("/sys/class/thermal/thermal_zone5/type"), action: None },
];

pub const RESOURCE_COUNT: usize = RESOURCES.len();

/// Folds the framework's id form (mode nibble in bits 28..31) onto the
/// library form (mode nibble in bits 24..27), so a request from either side
/// finds the same resource.
pub fn normalize_id(id: u32) -> u32 {
    (id & 0x0fff_ffff) | (((id >> 28) & 0xf) << 24)
}

pub fn find(id: u32) -> Option<&'static Resource> {
    let key = normalize_id(id);
    RESOURCES.iter().find(|r| r.id == key)
}

pub fn name_of(id: u32) -> &'static str {
    find(id).map(|r| r.name).unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_is_unique_and_complete() {
        assert_eq!(RESOURCE_COUNT, 75);
        let mut seen = std::collections::HashSet::new();
        for r in RESOURCES {
            assert!(seen.insert(r.id), "duplicate id 0x{:x} ({})", r.id, r.name);
        }
    }

    #[test]
    fn normalize_folds_framework_form() {
        // library form (mode nibble at bits 24..27, mode=1)
        assert_eq!(normalize_id(0x0101_0ba0), 0x0101_0ba0);
        // framework form for the same resource (mode nibble at bits 28..31)
        assert_eq!(normalize_id(0x1001_0ba0), 0x0101_0ba0);
        // a request in the framework form resolves to the resource
        assert_eq!(find(0x1001_0ba0).unwrap().name, "CPU_CORE_CTL_MAX_CPUS_GOLD");
        assert_eq!(find(0x0101_0ba0).unwrap().name, "CPU_CORE_CTL_MAX_CPUS_GOLD");
        // unknown id
        assert!(find(0x0fff_ffff).is_none());
    }

    #[test]
    fn known_ids_resolve() {
        assert_eq!(find(0x0102_0220).unwrap().name, "GPU_BUSY_PERCENTAGE");
        assert_eq!(find(0x0101_08a0).unwrap().name, "CPU_FREQ_TARGET_LOAD_SILVER");
        assert_eq!(find(0x0106_0000).unwrap().name, "MISC_SCREEN_OFF_STATE");
        assert_eq!(find(0x0105_1ff0).unwrap().name, "THERMAL_BOOST_BREAK_LEVEL");
    }
}
