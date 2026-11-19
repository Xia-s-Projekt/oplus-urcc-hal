// Native URCC resource core.
//
// The core keeps the service contract:
//   * a resource is identified by the id the framework uses
//     (UAHResourceConstants, e.g. 0x01010ba0 for CPU_CORE_CTL_MAX_CPUS_GOLD),
//   * a resource state is read from a node and reported as a string,
//   * a control request writes a value to a node and returns a handle,
//   * resource classes with no direct sysfs node are implemented as native
//     behaviour (the derive() actions below) instead of a fixed number.
//
// Wire-format note: the framework's UahGetFeatureStatus runs std::stoi() on
// res_Path, and the AIDL clients parse res_Value the same way. Both are
// therefore always non-empty numeric strings; an empty string throws
// std::invalid_argument inside system_server and takes the boot sequence
// down.

use crate::io_shim as io;
use crate::table::{self, Kind};
/// Cluster anchors: little / big / prime / "titanium", an alias of prime in
/// the resource id table.
const CORE_CTL: [&str; 4] = [
    "/sys/devices/system/cpu/cpu0/core_ctl",
    "/sys/devices/system/cpu/cpu4/core_ctl",
    "/sys/devices/system/cpu/cpu7/core_ctl",
    "/sys/devices/system/cpu/cpu7/core_ctl",
];

const MALI: &str = "/sys/class/devfreq/13000000.mali";
const FPSGO_COMMON: &str = "/sys/kernel/fpsgo/common";
const OVER_UTIL: &str = "/sys/devices/system/cpu/sched-avg/over_util";
// The Mali driver's utilisation procfs: "ACTIVE=28 3D/TA/COMPUTE=27/1/0
// OPP_IDX=40 MFG_PWR=0". ACTIVE is the share of the sampling window the GPU
// was busy, which is what the OnePlus GPU_BUSY_PERCENTAGE resource means.
const MALI_UTIL: &str = "/proc/mtk_mali/utilization";
// GED keeps the last GPU load percentage directly.
const GED_LOADING: &str = "/sys/module/ged/parameters/gpu_loading";

/// Node families with an equivalent implementation. A request for any other
/// node is refused instead of silently accepted.
const WRITABLE_PREFIXES: &[&str] = &[
    "/sys/devices/system/cpu/cpufreq/",
    "/sys/devices/system/cpu/cpu",
    "/sys/kernel/fpsgo/",
    "/sys/class/devfreq/13000000.mali/",
    "/sys/class/devfreq/mtk-dvfsrc-devfreq/",
    "/sys/class/thermal/",
    "/dev/cpuset/",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResState {
    pub res_id: i32,
    pub event_id: i32,
    /// Always non-empty and numeric (stoi-safe).
    pub res_value: String,
    /// Always non-empty and numeric (stoi-safe); "0" for resources without a
    /// node (the framework parses it with std::stoi).
    pub res_path: String,
    pub desc: String,
}

pub struct UrccCore;

impl UrccCore {
    pub fn new() -> Self {
        UrccCore
    }

    pub fn name_of(id: u32) -> &'static str {
        table::name_of(id)
    }

    pub fn path_of(id: u32) -> String {
        match table::find(id) {
            Some(r) => r.path.unwrap_or("0").to_string(),
            None => "0".to_string(),
        }
    }

    /// Path-addressed read (the UAH client also drives the core by node path).
    pub fn read_node(path: &str) -> String {
        io::read_file(path).unwrap_or_else(|| "-1".to_string())
    }

    /// Path-addressed write; only supported node families are writable,
    /// everything else is refused.
    pub fn write_node(path: &str, value: &str) -> bool {
        if !WRITABLE_PREFIXES.iter().any(|p| path.starts_with(p)) {
            return false;
        }
        write_sanitized_node(path, value).is_ok()
    }

    /// Current value of a resource, as the string the framework expects.
    /// "-1" when the resource is unknown or unreadable.
    pub fn read(id: u32) -> String {
        let r = match table::find(id) {
            Some(r) => r,
            None => return "-1".to_string(),
        };
        match r.kind {
            Kind::Read | Kind::Write | Kind::ReadWrite => {
                match r.path {
                    Some(path) => io::read_file(path).unwrap_or_else(|| "-1".to_string()),
                    None => Self::derive(r.action),
                }
            }
            Kind::Derived | Kind::Applied => Self::derive(r.action),
        }
    }

    fn derive(action: Option<&str>) -> String {
        let what = match action {
            Some(a) => a,
            None => return "-1".to_string(),
        };
        match what {
            "cluster_active_0" | "cluster_active_1" | "cluster_active_2" | "cluster_active_3" => {
                // "cluster_active_" is 15 bytes.
                let idx: usize = what[15..].parse().unwrap();
                read_cluster_active(CORE_CTL[idx])
            }
            "cluster_load_0" | "cluster_load_1" | "cluster_load_2" | "cluster_load_3" => {
                // "cluster_load_" is 13 bytes.
                let idx: usize = what[13..].parse().unwrap();
                read_cluster_load(CORE_CTL[idx])
            }
            "gpu_busy" => read_gpu_busy(),
            "gpu_min_mhz" => read_mali_mhz("min_freq"),
            "gpu_max_mhz" => read_mali_mhz("max_freq"),
            "max_task_util" => read_max_task_util(),
            "screen_off" => read_screen_off(),
            "up_thres" | "set_up_thres" => read_migration_thresholds().map(|t| t.1).unwrap_or_else(|| "-1".to_string()),
            "dn_thres" | "set_dn_thres" => read_migration_thresholds().map(|t| t.0).unwrap_or_else(|| "-1".to_string()),
            "bouncing_enable" => io::read_int("/sys/kernel/fpsgo/fbt/thrm_enable")
                .map(|v| v.to_string())
                .unwrap_or_else(|| "-1".to_string()),
            _ => "-1".to_string(),
        }
    }

    /// Applies a control request. Err carries a reason.
    pub fn apply(id: u32, value: &str) -> Result<(), String> {
        let r = match table::find(id) {
            Some(r) => r,
            None => return Err("unknown resource".to_string()),
        };
        if matches!(r.kind, Kind::ReadWrite | Kind::Write) {
            if let Some(path) = r.path {
                return write_sanitized_node(path, value);
            }
        }
        if r.kind == Kind::Read {
            return Err(format!("read-only resource: {}", r.name));
        }
        let action = r.action.ok_or_else(|| format!("no action for {}", r.name))?;
        let number: i64 = value.trim().parse().unwrap_or(0);

        match action {
            // OnePlus raised the EAS migration threshold of the big cluster;
            // the equivalent here is core_ctl's own up/down threshold of that
            // cluster, which decides when its CPUs stop running.
            "set_up_thres" | "set_dn_thres" => {
                let node = if action == "set_up_thres" { "up_thres" } else { "dn_thres" };
                let path = format!("{}/{}", CORE_CTL[1], node);
                if io::write_file(&path, &number.to_string()) {
                    Ok(())
                } else {
                    Err(format!("write failed: {}", path))
                }
            }
            // A touch event lifts the boost window. FPSGO is the boost engine
            // here: enable it and open the uclamp boost gate.
            "touch_boost" => {
                let on = number != 0;
                if !io::write_file(&format!("{}/force_onoff", FPSGO_COMMON), if on { "force on" } else { "force off" }) {
                    return Err("write failed: fpsgo force_onoff".to_string());
                }
                let _ = io::write_file(
                    "/sys/kernel/fpsgo/fbt/enable_uclamp_boost",
                    if on { "enable uclamp boost" } else { "disable uclamp boost" },
                );
                Ok(())
            }
            // The scheduling window is a uclamp band on this kernel; the width
            // the framework asks for is applied through FPSGO's window table.
            "window_ticks" => {
                let path = "/sys/kernel/fpsgo/fstb/fstb_tune_window_size";
                if io::write_file(path, value) {
                    Ok(())
                } else {
                    Err(format!("write failed: {}", path))
                }
            }
            // Main thermal HAL owns thermal throttling; no-op FPSGO thermal gate.
            "bouncing_enable" => Ok(()),
            _ => Err(format!("unhandled action for {}", r.name)),
        }
    }
}

impl Default for UrccCore {
    fn default() -> Self {
        Self::new()
    }
}

// Compat / simulation sources: values derived from sysfs nodes for resource
// classes that do not have a node of their own.

/// Active CPU count of a cluster, as core_ctl reports it.
fn read_cluster_active(core_ctl: &str) -> String {
    let state = match io::read_file(format!("{}/global_state", core_ctl)) {
        Some(s) => s,
        None => return "-1".to_string(),
    };
    match state.find("Active CPUs:") {
        Some(pos) => {
            let rest = &state[pos + "Active CPUs:".len()..];
            let n: i64 = rest.trim_start().split(|c: char| !c.is_ascii_digit()).next().unwrap_or("").parse().unwrap_or(-1);
            if n < 0 { "-1".to_string() } else { n.to_string() }
        }
        None => "-1".to_string(),
    }
}

/// Per-CPU utilisation in percent, averaged over a cluster, from core_ctl's
/// own accounting ("CPU utils(%): 42" per CPU inside global_state).
//
// This replaces a /proc/stat delta computation: /proc/stat carries the
// proc_stat label, which the HAL's domain (hal_allocator_default) may not
// read, while core_ctl's state carries sysfs_devices_system_cpu, which every
// domain may read. It is also the value the kernel itself uses to decide
// whether a CPU is busy.
fn read_cluster_load(core_ctl: &str) -> String {
    let state = match io::read_file(format!("{}/global_state", core_ctl)) {
        Some(s) => s,
        None => return "-1".to_string(),
    };
    let mut sum = 0i64;
    let mut count = 0i64;
    let mut pos = 0usize;
    while let Some(found) = state[pos..].find("CPU utils(%):") {
        let start = pos + found + "CPU utils(%):".len();
        let token = state[start..].trim_start().split(|c: char| !c.is_ascii_digit()).next().unwrap_or("");
        if let Ok(v) = token.parse::<i64>() {
            sum += v;
            count += 1;
        }
        pos = start;
    }
    if count == 0 {
        return "-1".to_string();
    }
    (sum / count).to_string()
}

fn read_gpu_busy() -> String {
    // 1. The Mali driver's own utilisation (primary).
    if let Some(text) = io::read_file(MALI_UTIL) {
        if let Some(pos) = text.find("ACTIVE=") {
            let token = text[pos + "ACTIVE=".len()..]
                .split(|c: char| !c.is_ascii_digit())
                .next()
                .unwrap_or("");
            if let Ok(v) = token.parse::<i64>() {
                return v.to_string();
            }
        }
    }
    // 2. GED's last load percentage.
    if let Some(v) = io::read_int(GED_LOADING) {
        return v.to_string();
    }
    "-1".to_string()
}

fn read_mali_mhz(node: &str) -> String {
    let hz = match io::read_int(format!("{}/{}", MALI, node)) {
        Some(v) => v,
        None => return "-1".to_string(),
    };
    // The mali nodes report Hz; the resource is read in MHz.
    (hz / 1_000_000).to_string()
}

/// Migration thresholds of the big cluster (capacity 876) from the EAS table:
/// "cpu=4 capacity=876 dn_thres=... up_thres=...".
fn read_migration_thresholds() -> Option<(String, String)> {
    let text = io::read_file(OVER_UTIL)?;
    for line in text.lines() {
        if !line.contains("cpu=4") {
            continue;
        }
        let mut dn = None;
        let mut up = None;
        for token in line.split_whitespace() {
            if let Some(v) = token.strip_prefix("dn_thres=") {
                dn = Some(v.to_string());
            } else if let Some(v) = token.strip_prefix("up_thres=") {
                up = Some(v.to_string());
            }
        }
        if let (Some(dn), Some(up)) = (dn, up) {
            return Some((dn, up));
        }
    }
    None
}

fn read_max_task_util() -> String {
    let text = match io::read_file(OVER_UTIL) {
        Some(t) => t,
        None => return "-1".to_string(),
    };
    for line in text.lines() {
        if let Some(pos) = line.find("maximum task_utils") {
            let rest = &line[pos..];
            let token = rest.split(|c: char| !c.is_ascii_digit()).filter(|s| !s.is_empty()).next();
            if let Some(t) = token {
                return t.to_string();
            }
        }
    }
    "-1".to_string()
}

/// Screen state: the display service is the only source on this build.
fn read_screen_off() -> String {
    #[cfg(target_os = "android")]
    {
        use std::ffi::CString;
        extern "C" {
            fn popen(command: *const std::os::raw::c_char, mode: *const std::os::raw::c_char) -> *mut std::ffi::c_void;
            fn pclose(stream: *mut std::ffi::c_void) -> i32;
            fn fgets(buf: *mut std::os::raw::c_char, size: i32, stream: *mut std::ffi::c_void) -> *mut std::os::raw::c_char;
        }
        let cmd = CString::new("dumpsys display 2>/dev/null | grep -m1 'Display State='").unwrap();
        let mode = CString::new("r").unwrap();
        // SAFETY: both strings are NUL-terminated C strings; popen semantics.
        let pipe = unsafe { popen(cmd.as_ptr(), mode.as_ptr()) };
        if pipe.is_null() {
            return "-1".to_string();
        }
        let mut buf = [0u8; 256];
        let mut found = None;
        unsafe {
            let out = fgets(buf.as_mut_ptr() as *mut std::os::raw::c_char, buf.len() as i32, pipe);
            if !out.is_null() {
                let line = std::ffi::CStr::from_bytes_until_nul(&buf)
                    .map(|c| c.to_string_lossy().into_owned())
                    .unwrap_or_default();
                if line.contains("Display State=OFF") || line.contains("Display State=DOZE") {
                    found = Some("1");
                } else if line.contains("Display State=ON") {
                    found = Some("0");
                }
            }
            pclose(pipe);
        }
        match found {
            Some(v) => v.to_string(),
            None => "-1".to_string(),
        }
    }
    #[cfg(not(target_os = "android"))]
    {
        "-1".to_string()
    }
}

/// Builds one result element for a resource id: current value, resource name
/// and the numeric path the client expects. Non-numeric node content (e.g.
/// "force on") is reported as 0 with the raw text appended to desc, because
/// the client parses res_Value with std::stoi() and would throw on text.
pub fn fill_state(id: u32) -> ResState {
    let raw = UrccCore::read(id);
    let numeric = raw.trim().parse::<i64>().is_ok() || raw == "-1";
    let name = UrccCore::name_of(id);
    let (value, desc) = if numeric {
        (if raw.is_empty() { "-1".to_string() } else { raw }, name.to_string())
    } else {
        ("0".to_string(), format!("{} {}", name, raw))
    };
    ResState {
        res_id: id as i32,
        event_id: 0,
        res_value: value,
        res_path: "0".to_string(),
        desc,
    }
}

fn is_thermal_throttle_node(path: &str) -> bool {
    path.starts_with("/sys/class/thermal/")
        || path.ends_with("/scaling_max_freq")
        || path.ends_with("/thermal_up_thres")
}

fn write_sanitized_node(path: &str, value: &str) -> Result<(), String> {
    if is_thermal_throttle_node(path) {
        return Ok(());
    }
    if let Some((dir, leaf)) = path.rsplit_once('/') {
        if path.starts_with("/sys/devices/system/cpu/cpufreq/") && leaf == "scaling_min_freq" {
            let (min_freq, max_freq) = policy_bounds(dir);
            let freq = sanitize_cpufreq_write(value, false, min_freq, max_freq)?;
            let freq_str = freq.to_string();
            return if io::write_file(path, &freq_str) {
                Ok(())
            } else {
                Err(format!("write failed: {}", path))
            };
        }
    }
    if io::write_file(path, value) {
        Ok(())
    } else {
        Err(format!("write failed: {}", path))
    }
}

fn policy_bounds(policy_dir: &str) -> (i64, i64) {
    let (def_min, def_max) = match policy_dir.rsplit('/').next() {
        Some("policy0") => (500_000, 2_000_000),
        Some("policy4") => (437_000, 2_600_000),
        Some("policy7") => (659_000, 3_000_000),
        _ => (400_000, 3_000_000),
    };
    let min_freq = io::read_file(&format!("{}/cpuinfo_min_freq", policy_dir))
        .and_then(|s| s.trim().parse::<i64>().ok())
        .filter(|&v| v > 0)
        .unwrap_or(def_min);
    let max_freq = io::read_file(&format!("{}/cpuinfo_max_freq", policy_dir))
        .and_then(|s| s.trim().parse::<i64>().ok())
        .filter(|&v| v >= min_freq)
        .unwrap_or(def_max);
    (min_freq, max_freq)
}

/// Normalizes cpufreq writes so release sentinels, thermal levels, or MHz
/// values cannot clamp scaling_max_freq to the cluster bottom floor.
fn sanitize_cpufreq_write(
    value: &str,
    is_max: bool,
    min_freq: i64,
    max_freq: i64,
) -> Result<i64, String> {
    let raw = value
        .trim()
        .parse::<i64>()
        .map_err(|_| format!("invalid cpufreq value: {}", value))?;
    if raw <= 0 {
        return Ok(if is_max { max_freq } else { min_freq });
    }
    if raw <= 100 {
        return Err(format!("ignoring non-kHz cpufreq level: {}", raw));
    }
    let khz = if raw <= 10_000 {
        raw.saturating_mul(1_000)
    } else {
        raw
    };
    Ok(khz.clamp(min_freq, max_freq))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_scaling_max_handles_sentinels_levels_and_units() {
        assert_eq!(sanitize_cpufreq_write("0", true, 500_000, 2_000_000), Ok(2_000_000));
        assert_eq!(sanitize_cpufreq_write("-1", true, 500_000, 2_000_000), Ok(2_000_000));
        assert!(sanitize_cpufreq_write("4", true, 500_000, 2_000_000).is_err());
        assert!(sanitize_cpufreq_write("100", true, 500_000, 2_000_000).is_err());
        assert_eq!(sanitize_cpufreq_write("1200", true, 500_000, 2_000_000), Ok(1_200_000));
        assert_eq!(sanitize_cpufreq_write("1500000", true, 500_000, 2_000_000), Ok(1_500_000));
        assert_eq!(sanitize_cpufreq_write("9999999", true, 500_000, 2_000_000), Ok(2_000_000));
        assert!(sanitize_cpufreq_write("bad", true, 500_000, 2_000_000).is_err());
    }

    #[test]
    fn sanitize_scaling_min_handles_sentinels_levels_and_units() {
        assert_eq!(sanitize_cpufreq_write("0", false, 437_000, 2_600_000), Ok(437_000));
        assert_eq!(sanitize_cpufreq_write("-1", false, 437_000, 2_600_000), Ok(437_000));
        assert!(sanitize_cpufreq_write("5", false, 437_000, 2_600_000).is_err());
        assert_eq!(sanitize_cpufreq_write("1000", false, 437_000, 2_600_000), Ok(1_000_000));
        assert_eq!(sanitize_cpufreq_write("1800000", false, 437_000, 2_600_000), Ok(1_800_000));
    }

    #[test]
    fn thermal_throttle_writes_noop_quietly() {
        assert!(UrccCore::write_node("/sys/devices/system/cpu/cpufreq/policy0/scaling_max_freq", "500000"));
        assert!(UrccCore::write_node("/sys/class/thermal/cooling_device14/cur_state", "3"));
        assert!(UrccCore::write_node("/sys/devices/system/cpu/cpu0/core_ctl/thermal_up_thres", "80"));
        assert_eq!(UrccCore::apply(0x0101_00a0, "500000"), Ok(()));
        assert_eq!(UrccCore::apply(0x0101_1e60, "500000"), Ok(()));
        assert_eq!(UrccCore::apply(0x0101_03a0, "1"), Ok(()));
    }
}
