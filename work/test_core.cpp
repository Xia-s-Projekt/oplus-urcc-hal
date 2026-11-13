// Test harness for UrccCore: prints what the core reads on the live device.
//
// It exercises the read path of every resource in the table, so the node
// mapping can be verified without flashing the service.
#include <cstdio>
#include <string>

#include "UrccCore.h"

namespace {

struct Entry {
    uint32_t id;
    const char* name;
};

const Entry kProbe[] = {
        {0x01020220, "GPU_BUSY_PERCENTAGE"},
        {0x010101a0, "CPU_FREQ_SCALING_CUR_SILVER"},
        {0x010101b0, "CPU_FREQ_SCALING_CUR_GOLD"},
        {0x010101c0, "CPU_FREQ_SCALING_CUR_PRIME"},
        {0x01010c10, "CPU_CORE_CTL_ACTIVE_CPUS_SILVER"},
        {0x01010c20, "CPU_CORE_CTL_ACTIVE_CPUS_GOLD"},
        {0x01010c30, "CPU_CORE_CTL_ACTIVE_CPUS_PRIME"},
        {0x010108a0, "CPU_FREQ_TARGET_LOAD_SILVER"},
        {0x010108b0, "CPU_FREQ_TARGET_LOAD_GOLD"},
        {0x010108c0, "CPU_FREQ_TARGET_LOAD_PRIME"},
        {0x01010fb0, "CPU_SCHED_GROUP_UPMIGRATE"},
        {0x01010fc0, "CPU_SCHED_GROUP_DOWNMIGRATE"},
        {0x01010f80, "CPU_SCHED_UPMIGRATE"},
        {0x01010f90, "CPU_SCHED_DOWNMIGRATE"},
        {0x01011040, "CPU_SCHED_MIN_TASK_UTIL_FOR_COLOCATION"},
        {0x010112f0, "CPU_SCHED_TOUCH_BOOST"},
        {0x01011070, "CPU_SCHED_WINDOW_TICKS_UPDATE"},
        {0x010103a0, "CPU_FREQ_BOUNCING_ENABLE"},
        {0x01020050, "GPU_FREQ_SCALING_MIN_SILVER"},
        {0x01020060, "GPU_FREQ_SCALING_MAX_SILVER"},
        {0x01020090, "GPU_FREQ_SCALING_CUR_SILVER"},
        {0x010200b0, "GPU_MAX_CLOCK_MHZ_SILVER"},
        {0x010200c0, "GPU_MIN_CLOCK_MHZ_SILVER"},
        {0x01030fc0, "MEM_DCVS_DDR_BOOST_FREQ"},
        {0x01050040, "THERMAL_ZONE_TYPE"},
        {0x01051ff0, "THERMAL_BOOST_BREAK_LEVEL"},
        {0x01051fd0, "THERMAL_BOOST_SERIOUS_LEVEL"},
        {0x01051fe0, "THERMAL_BOOST_ALL_BREAK_LIMIT"},
        {0x01011de0, "CPU_CORE_CTL_MAX_CPUS_SILVER_THERMAL_BOOST"},
        {0x01011df0, "CPU_CORE_CTL_MAX_CPUS_GOLD_THERMAL_BOOST"},
        {0x01011e00, "CPU_CORE_CTL_MAX_CPUS_PRIME_THERMAL_BOOST"},
        {0x01060000, "MISC_SCREEN_OFF_STATE"},
        {0x010115b0, "CPU_CPUSET_FOREGROUND_CPUS"},
        {0x01011910, "CPU_FPSGO_FORCE_ONOFF"},
        {0x01010010, "CPU_FREQ_SCALING_MIN_SILVER"},
        {0x010100a0, "CPU_FREQ_SCALING_MAX_SILVER"},
        {0x01010020, "CPU_FREQ_SCALING_MIN_GOLD"},
        {0x010100b0, "CPU_FREQ_SCALING_MAX_GOLD"},
        {0x01010030, "CPU_FREQ_SCALING_MIN_PRIME"},
        {0x010100c0, "CPU_FREQ_SCALING_MAX_PRIME"},
        {0x01010b10, "CPU_CORE_CTL_MIN_CPUS_SILVER"},
        {0x01010b90, "CPU_CORE_CTL_MAX_CPUS_SILVER"},
};

}  // namespace

int main() {
    aidl::vendor::oplus::hardware::urcc::impl::oplus::UrccCore core;
    for (const Entry& e : kProbe) {
        std::string value = core.read(e.id);
        std::printf("%-46s %-46s = %s\n", e.name, core.name_of(e.id), value.c_str());
    }

    // Write self-test: write each node's current value back. That is a no-op
    // for the hardware, so it proves the node is writable and that the value
    // the core reports is the value the node holds.
    std::printf("\n=== write self-test (writes the current value back)\n");
    int failures = 0;
    for (const Entry& e : kProbe) {
        const std::string path = core.path_of(e.id);
        if (path.empty() || path == "0") {
            continue;  // this class has no node
        }
        const std::string value = core.read(e.id);
        std::string error;
        if (!core.apply(e.id, value, &error)) {
            std::printf("  FAIL %-42s %s\n", e.name, error.c_str());
            ++failures;
            continue;
        }
        const std::string after = core.read(e.id);
        const bool same = after == value;
        std::printf("  %s %-42s %s\n", same ? "OK  " : "DIFF", e.name, path.c_str());
        if (!same) {
            std::printf("       wrote '%s', node now holds '%s'\n", value.c_str(), after.c_str());
            ++failures;
        }
    }
    std::printf("\nwrite failures: %d\n", failures);
    return failures == 0 ? 0 : 1;
}
