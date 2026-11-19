// Simulator for URCC resource classes that have no direct sysfs node.
//
// Resource values are read from the "nodepath" of the resource map in
// /vendor/etc/urcc/urcc_x6739.xml. Classes without a backing node point at
// /data/vendor/urcc/sim/<RESOURCE_NAME>, which this thread keeps up to date.
//
// The service runs as root (init rc: user root, group root readproc), so it
// can read /proc/stat, the Mali debugfs counters and the thermal zone.

#include "UrccSim.h"

#include <fcntl.h>
#include <sys/stat.h>
#include <unistd.h>

#include <cerrno>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <string>

namespace aidl::vendor::oplus::hardware::urcc::impl::oplus {
namespace {

constexpr const char* kSimDir = "/data/vendor/urcc/sim";
constexpr int kIntervalSeconds = 1;

bool read_file(const char* path, std::string* out) {
    int fd = open(path, O_RDONLY | O_CLOEXEC);
    if (fd < 0) {
        return false;
    }
    char buf[4096];
    ssize_t n = read(fd, buf, sizeof(buf) - 1);
    close(fd);
    if (n <= 0) {
        return false;
    }
    buf[n] = '\0';
    out->assign(buf);
    while (!out->empty() && (out->back() == '\n' || out->back() == '\r' ||
                             out->back() == ' ' || out->back() == '\t')) {
        out->pop_back();
    }
    return true;
}

bool read_int(const char* path, long long* out) {
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

void write_int(const std::string& name, long long value) {
    std::string path = std::string(kSimDir) + "/" + name;
    int fd = open(path.c_str(), O_WRONLY | O_CREAT | O_TRUNC | O_CLOEXEC, 0644);
    if (fd < 0) {
        return;
    }
    std::string text = std::to_string(value);
    ssize_t written = write(fd, text.data(), text.size());
    (void)written;
    close(fd);
}

// Parse /proc/stat into per-cpu busy/total tick counters.
bool read_proc_stat(long long* per_cpu_busy, long long* per_cpu_total) {
    for (int i = 0; i < 8; ++i) {
        per_cpu_busy[i] = 0;
        per_cpu_total[i] = 0;
    }
    std::string stat;
    if (!read_file("/proc/stat", &stat)) {
        return false;
    }
    size_t pos = 0;
    while (pos < stat.size()) {
        size_t eol = stat.find('\n', pos);
        if (eol == std::string::npos) {
            eol = stat.size();
        }
        std::string line = stat.substr(pos, eol - pos);
        pos = eol + 1;
        if (line.size() < 4 || line.compare(0, 3, "cpu") != 0) {
            continue;
        }
        if (line[3] != ' ' && line[3] != '\t') {
            continue;
        }
        int cpu = atoi(line.c_str() + 3);
        if (cpu < 0 || cpu > 7) {
            continue;
        }
        long long values[10] = {0};
        int count = 0;
        const char* p = line.c_str() + 3;
        while (*p && count < 10) {
            while (*p == ' ' || *p == '\t') {
                ++p;
            }
            if (!*p) {
                break;
            }
            char* end = nullptr;
            long long v = strtoll(p, &end, 10);
            if (end == p) {
                break;
            }
            values[count++] = v;
            p = end;
        }
        if (count < 4) {
            continue;
        }
        long long total = 0;
        for (int i = 0; i < count; ++i) {
            total += values[i];
        }
        // user, nice, system, irq, softirq, steal, guest, guest_nice
        long long busy = 0;
        for (int i = 0; i < count && i < 8; ++i) {
            if (i == 3 || i == 7) {  // idle and guest_nice are not busy
                continue;
            }
            busy += values[i];
        }
        per_cpu_busy[cpu] = busy;
        per_cpu_total[cpu] = total;
    }
    return true;
}

int cluster_of_cpu(int cpu) {
    if (cpu <= 3) {
        return 0;
    }
    if (cpu <= 6) {
        return 1;
    }
    return 2;
}

// Mali publishes authoritative busy/idle nanosecond counters here.
bool read_mali_busy(long long* busy_ns, long long* idle_ns) {
    std::string text;
    if (!read_file("/sys/kernel/debug/mali0/dvfs_utilization", &text)) {
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

}  // namespace

void UrccSim::start() {
    mkdir(kSimDir, 0777);

    long long prev_busy[8] = {0};
    long long prev_total[8] = {0};
    read_proc_stat(prev_busy, prev_total);
    long long prev_gpu_busy = 0;
    long long prev_gpu_idle = 0;
    read_mali_busy(&prev_gpu_busy, &prev_gpu_idle);

    while (true) {
        sleep(kIntervalSeconds);

        long long busy[8] = {0};
        long long total[8] = {0};
        if (read_proc_stat(busy, total)) {
            long long cluster_busy[3] = {0};
            long long cluster_total[3] = {0};
            for (int cpu = 0; cpu < 8; ++cpu) {
                int c = cluster_of_cpu(cpu);
                cluster_busy[c] += busy[cpu] - prev_busy[cpu];
                cluster_total[c] += total[cpu] - prev_total[cpu];
            }
            static const char* const kLoadNames[3] = {
                    "CPU_FREQ_TARGET_LOAD_SILVER", "CPU_FREQ_TARGET_LOAD_GOLD",
                    "CPU_FREQ_TARGET_LOAD_PRIME"};
            for (int c = 0; c < 3; ++c) {
                if (cluster_total[c] > 0) {
                    write_int(kLoadNames[c], cluster_busy[c] * 100 / cluster_total[c]);
                }
            }
            for (int cpu = 0; cpu < 8; ++cpu) {
                prev_busy[cpu] = busy[cpu];
                prev_total[cpu] = total[cpu];
            }
        }

        long long gpu_busy = 0;
        long long gpu_idle = 0;
        if (read_mali_busy(&gpu_busy, &gpu_idle)) {
            long long dbusy = gpu_busy - prev_gpu_busy;
            long long didle = gpu_idle - prev_gpu_idle;
            if (dbusy + didle > 0) {
                write_int("GPU_BUSY_PERCENTAGE", dbusy * 100 / (dbusy + didle));
            }
            prev_gpu_busy = gpu_busy;
            prev_gpu_idle = gpu_idle;
        }

        // GPU clock in MHz (the devfreq node is in Hz)
        long long hz = 0;
        if (read_int("/sys/class/devfreq/13000000.mali/min_freq", &hz)) {
            write_int("GPU_MIN_CLOCK_MHZ_SILVER", hz / 1000000);
        }
        if (read_int("/sys/class/devfreq/13000000.mali/max_freq", &hz)) {
            write_int("GPU_MAX_CLOCK_MHZ_SILVER", hz / 1000000);
        }

        // active CPUs per cluster: core_ctl/global_state prints "Active CPUs: N"
        static const char* const kActiveNames[3] = {
                "CPU_CORE_CTL_ACTIVE_CPUS_SILVER", "CPU_CORE_CTL_ACTIVE_CPUS_GOLD",
                "CPU_CORE_CTL_ACTIVE_CPUS_PRIME"};
        static const char* const kCtl[3] = {
                "/sys/devices/system/cpu/cpu0/core_ctl/global_state",
                "/sys/devices/system/cpu/cpu4/core_ctl/global_state",
                "/sys/devices/system/cpu/cpu7/core_ctl/global_state"};
        for (int c = 0; c < 3; ++c) {
            std::string state;
            if (!read_file(kCtl[c], &state)) {
                continue;
            }
            size_t pos = state.find("Active CPUs:");
            if (pos == std::string::npos) {
                continue;
            }
            write_int(kActiveNames[c], atoi(state.c_str() + pos + 12));
        }

        // thermal boost state per cluster
        static const char* const kThermalNames[3] = {
                "THERMAL_BOOST_BREAK_LEVEL_SILVER", "THERMAL_BOOST_BREAK_LEVEL_GOLD",
                "THERMAL_BOOST_BREAK_LEVEL_PRIME"};
        static const char* const kCdev[3] = {
                "/sys/class/thermal/cooling_device14/cur_state",
                "/sys/class/thermal/cooling_device15/cur_state",
                "/sys/class/thermal/cooling_device16/cur_state"};
        for (int c = 0; c < 3; ++c) {
            long long state = 0;
            if (read_int(kCdev[c], &state)) {
                write_int(kThermalNames[c], state);
            }
        }

        // FPSGO thermal governor gate stands in for CPU_FREQ_BOUNCING_ENABLE
        long long thrm = 0;
        if (read_int("/sys/kernel/fpsgo/fbt/thrm_enable", &thrm)) {
            write_int("CPU_FREQ_BOUNCING_ENABLE", thrm);
        }

        // "maximum task_utils" from the EAS capacity table
        std::string over_util;
        if (read_file("/sys/devices/system/cpu/sched-avg/over_util", &over_util)) {
            size_t pos = over_util.find("maximum task_utils");
            if (pos != std::string::npos) {
                write_int("CPU_SCHED_MIN_TASK_UTIL_FOR_COLOCATION",
                          atoi(over_util.c_str() + pos + 18));
            }
            // migration thresholds of the big cluster (capacity 876)
            size_t cpu4 = over_util.find("cpu=4 ");
            if (cpu4 != std::string::npos) {
                long long dn = 0;
                long long up = 0;
                if (sscanf(over_util.c_str() + cpu4,
                           "cpu=4 capacity=%*lld dn_thres=%lld up_thres=%lld", &dn, &up) == 2) {
                    write_int("CPU_SCHED_GROUP_DOWNMIGRATE", dn);
                    write_int("CPU_SCHED_GROUP_UPMIGRATE", up);
                    write_int("CPU_SCHED_DOWNMIGRATE", dn);
                    write_int("CPU_SCHED_UPMIGRATE", up);
                }
            }
        }
    }
}

}  // namespace aidl::vendor::oplus::hardware::urcc::impl::oplus
