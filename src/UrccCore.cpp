// Native URCC resource core.
//
// The core keeps the service contract:
//
//   * a resource is identified by the id the framework uses
//     (UAHResourceConstants, e.g. 0x1010ba0 for CPU_CORE_CTL_MAX_CPUS_GOLD),
//   * a resource state is read from a file path and reported as a string,
//   * a control request writes a value to a file path and returns a handle,
//   * resource classes without a direct sysfs node are implemented here as
//     native behaviour instead of a fixed number.
//
// The resource table is produced by scripts/gen_urcc_resource_table.py from
// the resource-name/id table, so the ids served here are the ids the framework
// sends.
//
// Wire-format note: the framework's UahGetFeatureStatus runs std::stoi() on
// res_Path, and the AIDL clients parse res_Value the same way.  Both are
// therefore always non-empty numeric strings here; an empty string throws
// std::invalid_argument inside system_server and takes the boot sequence
// down with it.

#include "UrccCore.h"

#include <fcntl.h>
#include <sys/stat.h>
#include <unistd.h>

#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <mutex>
#include <string>

namespace aidl::vendor::oplus::hardware::urcc::impl::oplus {
namespace {

// --- supported sysfs nodes --------------------------------------------------

const char* const kMali = "/sys/class/devfreq/13000000.mali";
const char* const kFpsgoCommon = "/sys/kernel/fpsgo/common";
const char* const kOverUtil = "/sys/devices/system/cpu/sched-avg/over_util";
const char* const kMaliDvfs = "/sys/kernel/debug/mali0/dvfs_utilization";
const char* const kGedLoading = "/sys/module/ged/parameters/gpu_loading";
// The Mali driver's own utilisation procfs, the authoritative source on this
// board: "ACTIVE=28 3D/TA/COMPUTE=27/1/0 OPP_IDX=40 MFG_PWR=0".
const char* const kMaliUtil = "/proc/mtk_mali/utilization";

// core_ctl of the little / big / prime cluster
const char* const kCoreCtl[3] = {
        "/sys/devices/system/cpu/cpu0/core_ctl",
        "/sys/devices/system/cpu/cpu4/core_ctl",
        "/sys/devices/system/cpu/cpu7/core_ctl",
};

bool read_file(const std::string& path, std::string* out) {
    int fd = open(path.c_str(), O_RDONLY | O_CLOEXEC);
    if (fd < 0) {
        return false;
    }
    char buf[8192];
    ssize_t n = read(fd, buf, sizeof(buf) - 1);
    close(fd);
    if (n <= 0) {
        return false;
    }
    buf[n] = '\0';
    out->assign(buf);
    while (!out->empty() && (out->back() == '\n' || out->back() == '\r')) {
        out->pop_back();
    }
    return true;
}

bool write_file(const std::string& path, const std::string& value) {
    int fd = open(path.c_str(), O_WRONLY | O_CLOEXEC);
    if (fd < 0) {
        return false;
    }
    ssize_t written = write(fd, value.data(), value.size());
    close(fd);
    return written == static_cast<ssize_t>(value.size());
}

bool read_int(const std::string& path, long long* out) {
    std::string value;
    if (!read_file(path, &value)) {
        return false;
    }
    char* end = nullptr;
    long long parsed = strtoll(value.c_str(), &end, 10);
    if (end == value.c_str()) {
        return false;
    }
    *out = parsed;
    return true;
}

// --- derived value sources -------------------------------------------------

// Mali's own busy/idle nanosecond counters.
bool read_mali_busy(long long* busy_ns, long long* idle_ns) {
    std::string text;
    if (!read_file(kMaliDvfs, &text)) {
        return false;
    }
    long long busy = 0;
    long long idle = 0;
    if (sscanf(text.c_str(), "busy_time: %lld idle_time: %lld", &busy, &idle) != 2) {
        return false;
    }
    *busy_ns = busy;
    *idle_ns = idle;
    return true;
}

// GED keeps the last GPU load percentage directly.
bool read_ged_loading(long long* percent) {
    std::string text;
    if (!read_file(kGedLoading, &text)) {
        return false;
    }
    char* end = nullptr;
    long long value = strtoll(text.c_str(), &end, 10);
    if (end == text.c_str()) {
        return false;
    }
    *percent = value;
    return true;
}

// The Mali driver's utilisation procfs: "ACTIVE=28 3D/TA/COMPUTE=27/1/0
// OPP_IDX=40 MFG_PWR=0".  ACTIVE is the share of the sampling window the GPU
// was busy, which is what the OnePlus GPU_BUSY_PERCENTAGE resource means.
bool read_mali_utilization(long long* percent) {
    std::string text;
    if (!read_file(kMaliUtil, &text)) {
        return false;
    }
    size_t pos = text.find("ACTIVE=");
    if (pos == std::string::npos) {
        return false;
    }
    char* end = nullptr;
    long long value = strtoll(text.c_str() + pos + 7, &end, 10);
    if (end == text.c_str() + pos + 7) {
        return false;
    }
    *percent = value;
    return true;
}

// Per-CPU utilisation in percent, from core_ctl's own accounting:
// "CPU utils(%): 42" per CPU inside global_state.
//
// This replaces a /proc/stat delta computation: /proc/stat carries the
// proc_stat label, which the HAL's domain (hal_allocator_default) may not
// read, while core_ctl's state carries sysfs_devices_system_cpu, which every
// domain may read. It is also the value the kernel itself uses to decide
// whether a CPU is busy.
bool read_cluster_util(int cluster, long long* util_percent) {
    std::string state;
    if (!read_file(std::string(kCoreCtl[cluster]) + "/global_state", &state)) {
        return false;
    }
    long long sum = 0;
    int count = 0;
    size_t pos = 0;
    while ((pos = state.find("CPU utils(%):", pos)) != std::string::npos) {
        sum += atoll(state.c_str() + pos + 12);
        ++count;
        pos += 12;
    }
    if (count == 0) {
        return false;
    }
    *util_percent = sum / count;
    return true;
}

// Active CPU count of a cluster, as core_ctl reports it.
bool read_cluster_active(int cluster, long long* active) {
    std::string state;
    if (!read_file(std::string(kCoreCtl[cluster]) + "/global_state", &state)) {
        return false;
    }
    size_t pos = state.find("Active CPUs:");
    if (pos == std::string::npos) {
        return false;
    }
    *active = atoll(state.c_str() + pos + 12);
    return true;
}

// Migration thresholds of the big cluster (capacity 876) from the EAS table.
bool read_migration_thresholds(long long* dn_thres, long long* up_thres) {
    std::string text;
    if (!read_file(kOverUtil, &text)) {
        return false;
    }
    size_t pos = text.find("cpu=4 ");
    if (pos == std::string::npos) {
        return false;
    }
    return sscanf(text.c_str() + pos,
                  "cpu=4 capacity=%*lld dn_thres=%lld up_thres=%lld", dn_thres, up_thres) == 2;
}

bool read_max_task_util(long long* util) {
    std::string text;
    if (!read_file(kOverUtil, &text)) {
        return false;
    }
    size_t pos = text.find("maximum task_utils");
    if (pos == std::string::npos) {
        return false;
    }
    // "maximum task_utils = 333"
    const char* p = text.c_str() + pos + 18;
    while (*p == ' ' || *p == '=') {
        ++p;
    }
    char* end = nullptr;
    long long value = strtoll(p, &end, 10);
    if (end == p) {
        return false;
    }
    *util = value;
    return true;
}

// Screen state: the display service is the only source on this build. There
// is no sysfs node or property carrying the framework's "Display State", so
// the value has to come from `dumpsys`. That forks a shell on every call, so
// the parsed result is memoized for a short TTL and the shell is only run on a
// cold or expired cache. The mutex is held across the refresh so concurrent
// readers share a single fork instead of each starting their own.
struct ScreenOffCache {
    std::mutex mutex;
    bool valid = false;
    bool found = false;
    bool off = false;
    std::chrono::steady_clock::time_point sampled_at{};
};

bool read_screen_off(bool* off) {
    static ScreenOffCache cache;
    std::lock_guard<std::mutex> lock(cache.mutex);
    const auto now = std::chrono::steady_clock::now();
    if (cache.valid && now - cache.sampled_at < std::chrono::seconds(1)) {
        if (cache.found) {
            *off = cache.off;
        }
        return cache.found;
    }

    FILE* pipe = popen("dumpsys display 2>/dev/null | grep -m1 'Display State='", "r");
    if (pipe == nullptr) {
        return false;
    }
    char line[256] = {0};
    bool found = false;
    if (fgets(line, sizeof(line), pipe) != nullptr) {
        if (strstr(line, "Display State=OFF") != nullptr ||
            strstr(line, "Display State=DOZE") != nullptr) {
            *off = true;
            found = true;
        } else if (strstr(line, "Display State=ON") != nullptr) {
            *off = false;
            found = true;
        }
    }
    pclose(pipe);

    cache.found = found;
    if (found) {
        cache.off = *off;
    }
    cache.valid = true;
    cache.sampled_at = std::chrono::steady_clock::now();
    return found;
}

// The library encodes a resource id as
//   ((mode & 0xf) << 24) | ((module & 0xff) << 16) | ((res & 0x1ff) << 4)
// and the framework sends the same value with the mode nibble in bits 28..31
// (0x01010ba0 vs 0x1010ba0).  Fold both onto the library form.
uint32_t normalize_id(uint32_t id) {
    return (id & 0x0fffffffu) | (((id >> 28) & 0xfu) << 24);
}

}  // namespace

#include "UrccResourceTable.inc"

// --- UrccCore ---------------------------------------------------------------

UrccCore::UrccCore() {
    read_mali_busy(&mPrevGpuBusy, &mPrevGpuIdle);
}

UrccCore::~UrccCore() = default;

const UrccResource* UrccCore::find(uint32_t id) const {
    uint32_t key = normalize_id(id);
    for (size_t i = 0; i < kUrccResourceCount; ++i) {
        if (kUrccResources[i].id == key) {
            return &kUrccResources[i];
        }
    }
    return nullptr;
}

bool UrccCore::write_node(const std::string& path, const std::string& value) {
    // Only the supported node families are writable. A request for any other
    // node is refused rather than silently accepted.
    static const char* const kAllowed[] = {
            "/sys/devices/system/cpu/cpufreq/",
            "/sys/devices/system/cpu/cpu",
            "/sys/kernel/fpsgo/",
            "/sys/class/devfreq/13000000.mali/",
            "/sys/class/devfreq/mtk-dvfsrc-devfreq/",
            "/sys/class/thermal/",
            "/dev/cpuset/",
    };
    bool allowed = false;
    for (const char* prefix : kAllowed) {
        if (path.compare(0, strlen(prefix), prefix) == 0) {
            allowed = true;
            break;
        }
    }
    if (!allowed) {
        return false;
    }
    return write_file(path, value);
}

std::string UrccCore::read_node(const std::string& path) {
    std::string value;
    if (!read_file(path, &value)) {
        return "-1";
    }
    return value;
}

const char* UrccCore::name_of(uint32_t id) {
    const UrccResource* r = find(id);
    return r != nullptr ? r->name : "";
}

std::string UrccCore::path_of(uint32_t id) {
    const UrccResource* r = find(id);
    if (r == nullptr || r->path == nullptr) {
        // res_Path is parsed with std::stoi by the framework, so it must
        // never be empty.
        return "0";
    }
    return r->path;
}

std::string UrccCore::read(uint32_t id) {
    const UrccResource* r = find(id);
    if (r == nullptr) {
        return "-1";
    }
    switch (r->kind) {
        case Kind::kRead:
        case Kind::kReadWrite:
        case Kind::kWrite: {
            // An action-only resource has no node to read; its state comes
            // from the action.
            if (r->path == nullptr) {
                break;
            }
            std::string value;
            if (!read_file(r->path, &value)) {
                return "-1";
            }
            return value;
        }
        case Kind::kDerived:
        case Kind::kApplied:
            break;
    }
    return derive(r->action);
}

std::string UrccCore::derive(const char* action) {
    if (action == nullptr) {
        return "-1";
    }
    std::string what(action);

    if (what == "cluster_active_0" || what == "cluster_active_1" ||
        what == "cluster_active_2") {
        long long active = 0;
        if (read_cluster_active(what.back() - '0', &active)) {
            return std::to_string(active);
        }
        return "-1";
    }
    if (what == "cluster_load_0" || what == "cluster_load_1" || what == "cluster_load_2") {
        return cluster_load(what.back() - '0');
    }
    if (what == "gpu_busy") {
        // The Mali driver's own utilisation is the primary source: ACTIVE is
        // the percentage of the sampling window the GPU was busy. GED's
        // gpu_loading parameter and the Mali busy/idle counters are the
        // fallbacks.
        long long percent = 0;
        if (read_mali_utilization(&percent)) {
            return std::to_string(percent);
        }
        if (read_ged_loading(&percent)) {
            return std::to_string(percent);
        }
        long long busy = 0;
        long long idle = 0;
        if (!read_mali_busy(&busy, &idle)) {
            return "-1";
        }
        long long dbusy = busy - mPrevGpuBusy;
        long long didle = idle - mPrevGpuIdle;
        mPrevGpuBusy = busy;
        mPrevGpuIdle = idle;
        if (dbusy + didle <= 0) {
            return "-1";
        }
        return std::to_string(dbusy * 100 / (dbusy + didle));
    }
    if (what == "gpu_min_mhz" || what == "gpu_max_mhz") {
        long long hz = 0;
        const char* node = what == "gpu_min_mhz" ? "min_freq" : "max_freq";
        if (!read_int(std::string(kMali) + "/" + node, &hz)) {
            return "-1";
        }
        return std::to_string(hz / 1000000);
    }
    if (what == "max_task_util") {
        long long util = 0;
        if (!read_max_task_util(&util)) {
            return "-1";
        }
        return std::to_string(util);
    }
    if (what == "screen_off") {
        bool off = false;
        if (!read_screen_off(&off)) {
            return "-1";
        }
        return off ? "1" : "0";
    }
    if (what == "up_thres" || what == "dn_thres" || what == "set_up_thres" ||
        what == "set_dn_thres") {
        long long dn = 0;
        long long up = 0;
        if (!read_migration_thresholds(&dn, &up)) {
            return "-1";
        }
        return std::to_string(what == "up_thres" || what == "set_up_thres" ? up : dn);
    }
    if (what == "bouncing_enable") {
        long long value = 0;
        if (!read_int("/sys/kernel/fpsgo/fbt/thrm_enable", &value)) {
            return "-1";
        }
        return std::to_string(value);
    }
    return "-1";
}

std::string UrccCore::cluster_load(int cluster) {
    long long util = 0;
    if (!read_cluster_util(cluster, &util)) {
        return "-1";
    }
    return std::to_string(util);
}

// Control requests on resources without a direct node apply the equivalent
// tuning through the available sysfs nodes.
bool UrccCore::apply(uint32_t id, const std::string& value, std::string* error) {
    const UrccResource* r = find(id);
    if (r == nullptr) {
        *error = "unknown resource";
        return false;
    }

    if ((r->kind == Kind::kReadWrite || r->kind == Kind::kWrite) && r->path != nullptr) {
        if (!write_file(r->path, value)) {
            *error = std::string("write failed: ") + r->path;
            return false;
        }
        return true;
    }
    if (r->kind == Kind::kRead) {
        *error = std::string("read-only resource: ") + r->name;
        return false;
    }

    const char* action = r->action;
    if (action == nullptr) {
        *error = std::string("no action for ") + r->name;
        return false;
    }
    std::string what(action);
    long long number = atoll(value.c_str());

    if (what == "set_up_thres" || what == "set_dn_thres") {
        // OnePlus raised the EAS migration threshold of the big cluster; the
        // equivalent here is core_ctl's own up/down threshold of that cluster,
        // which decides when its CPUs stop running.
        const char* node = what == "set_up_thres" ? "up_thres" : "dn_thres";
        std::string path = std::string(kCoreCtl[1]) + "/" + node;
        if (!write_file(path, std::to_string(number))) {
            *error = "write failed: " + path;
            return false;
        }
        return true;
    }
    if (what == "touch_boost") {
        // A touch event lifts the boost window. FPSGO is the boost engine
        // here: enable it and open the uclamp boost gate.
        if (!write_file(std::string(kFpsgoCommon) + "/force_onoff",
                       number != 0 ? "force on" : "force off")) {
            *error = "write failed: fpsgo force_onoff";
            return false;
        }
        write_file("/sys/kernel/fpsgo/fbt/enable_uclamp_boost",
                   number != 0 ? "enable uclamp boost" : "disable uclamp boost");
        return true;
    }
    if (what == "window_ticks") {
        // The scheduling window is a uclamp band on this kernel; the width
        // the framework asks for is applied through FPSGO's window table.
        std::string path = "/sys/kernel/fpsgo/fstb/fstb_tune_window_size";
        if (!write_file(path, value)) {
            *error = "write failed: " + path;
            return false;
        }
        return true;
    }
    if (what == "bouncing_enable") {
        // The frequency bounding gate is FPSGO's thermal governor switch.
        std::string path = "/sys/kernel/fpsgo/fbt/thrm_enable";
        if (!write_file(path, value)) {
            *error = "write failed: " + path;
            return false;
        }
        return true;
    }

    *error = std::string("unhandled action for ") + r->name;
    return false;
}

}  // namespace aidl::vendor::oplus::hardware::urcc::impl::oplus
