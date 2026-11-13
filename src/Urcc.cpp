// The URCC HAL method bodies.
//
// The shipped service forwards every call into the closed liburcccore.so.
// That library is Qualcomm-specific (its resource map points at
// /sys/class/kgsl/kgsl-3d0/..., /sys/class/devfreq/*qcom* and
// /sys/devices/platform/soc/*.qcom,cpucc*...), and none of those nodes exist
// on the MT6893 board, so the mapping and the request handling are done here
// instead.  UrccCore owns the mapping and implements the classes that have no
// MediaTek counterpart as real behaviour.
//
// Wire-format constraints kept from the shipped service:
//   * the Binder status is always OK; failures are reported through the out
//     parameter,
//   * the log tag is "UrccFramework-halservice",
//   * every string returned to the client is non-empty and numeric, because
//     liboplus-uah-client.so parses the values with std::stoi() and an empty
//     string throws inside system_server.
#include "Urcc.h"

#include <android/log.h>

#include <cstdarg>
#include <cstdlib>
#include <cstring>

namespace aidl::vendor::oplus::hardware::urcc::impl::oplus {

namespace {

constexpr const char* kLogTag = "UrccFramework-halservice";

// The resources the framework polls for the performance gauge.
constexpr uint32_t kGpuBusy = 0x01020220;  // GPU_BUSY_PERCENTAGE
constexpr uint32_t kCpuLoadSilver = 0x010108a0;
constexpr uint32_t kCpuLoadGold = 0x010108b0;
constexpr uint32_t kCpuLoadPrime = 0x010108c0;

constexpr int kSampleIntervalMs = 500;

void log_msg(const char* fmt, ...) __attribute__((format(printf, 1, 2)));

void log_msg(const char* fmt, ...) {
    va_list args;
    va_start(args, fmt);
    __android_log_vprint(ANDROID_LOG_INFO, kLogTag, fmt, args);
    va_end(args);
}

// std::stoi() accepts a leading '-', so "-1" is a safe "no value" marker.
constexpr const char* kNoValue = "-1";

}  // namespace

Urcc::Urcc() {
    log_msg("Urcc HAL AIDL Service is started.\n");
    mSampler = std::thread([this]() { this->sampler_loop(); });
}

Urcc::~Urcc() {
    mRunning = false;
    if (mSampler.joinable()) {
        mSampler.join();
    }
}

void Urcc::fill_state(uint32_t id, UrccRequestData* out) {
    out->res_Id = static_cast<int32_t>(id);
    out->event_Id = 0;
    out->res_Value = mCore.read(id);
    if (out->res_Value.empty()) {
        out->res_Value = kNoValue;
    }
    // The client parses the returned value with std::stoi().  A node whose
    // content is text rather than a number ("force on", "enable uclamp
    // boost") would throw std::invalid_argument inside system_server and take
    // the boot sequence down, so a non-numeric value is reported as 0 and
    // the raw text goes into desc.
    const char* text = out->res_Value.c_str();
    char* end = nullptr;
    strtol(text, &end, 10);
    const bool numeric = end != text && *end == '\0';
    out->desc = mCore.name_of(id);
    if (!numeric) {
        out->desc += " ";
        out->desc += out->res_Value;
        out->res_Value = "0";
    }
    // res_Path is parsed numerically by the client too.
    out->res_Path = "0";
}

int Urcc::apply_request(const std::vector<UrccRequestData>& data) {
    int applied = 0;
    for (const UrccRequestData& item : data) {
        log_msg("urcc ctl: id=0x%x path=%s value=%s\n", item.res_Id,
                item.res_Path.c_str(), item.res_Value.c_str());
        if (item.res_Value.empty()) {
            continue;
        }
        // The UAH client drives the core by node path as well as by resource
        // id: liboplus-uah-client.so's urcc_resctl_request_by_path() puts the
        // node path in res_Path. A path request is applied to that node
        // directly; an id request goes through the resource table.
        if (!item.res_Path.empty() && item.res_Path[0] == '/') {
            if (mCore.write_node(item.res_Path, item.res_Value)) {
                ++applied;
            } else {
                log_msg("urcc write %s failed\n", item.res_Path.c_str());
            }
            continue;
        }
        std::string error;
        if (mCore.apply(static_cast<uint32_t>(item.res_Id), item.res_Value, &error)) {
            ++applied;
        } else {
            log_msg("urcc apply %s failed: %s\n", mCore.name_of(item.res_Id),
                    error.c_str());
        }
    }
    return applied;
}

void Urcc::sample_once(std::vector<UrccRequestData>* out) {
    out->clear();
    UrccRequestData item;
    fill_state(kGpuBusy, &item);
    out->push_back(item);
    fill_state(kCpuLoadSilver, &item);
    out->push_back(item);
    fill_state(kCpuLoadGold, &item);
    out->push_back(item);
    fill_state(kCpuLoadPrime, &item);
    out->push_back(item);
}

void Urcc::sampler_loop() {
    std::vector<UrccRequestData> sample;
    while (mRunning) {
        sample_once(&sample);

        std::vector<std::shared_ptr<IUrccCallback>> targets;
        {
            std::lock_guard<std::mutex> guard(mLock);
            for (const auto& entry : mCallbacks) {
                targets.push_back(entry.second);
            }
        }
        for (const std::shared_ptr<IUrccCallback>& callback : targets) {
            if (callback == nullptr) {
                continue;
            }
            callback->onCallback(sample);
        }

        std::this_thread::sleep_for(std::chrono::milliseconds(kSampleIntervalMs));
    }
}

::ndk::ScopedAStatus Urcc::urccInit() {
    log_msg("Urcc::init is called \n");
    return ::ndk::ScopedAStatus::ok();
}

::ndk::ScopedAStatus Urcc::urccResCtlRequest(const UrccRequestParcel& request,
                                             int32_t* _aidl_return) {
    // The handle is 0 for an accepted request. A resource this port has no
    // node for is skipped, not an error: the shipped core accepted requests
    // for Qualcomm-only resources and the framework treats a non-zero return
    // as a failed request.
    apply_request(request.mUrccRequestData);
    *_aidl_return = 0;
    return ::ndk::ScopedAStatus::ok();
}

::ndk::ScopedAStatus Urcc::urccResCtlRelease(int32_t handle, int32_t* _aidl_return) {
    std::lock_guard<std::mutex> guard(mLock);
    mCallbacks.erase(handle);
    *_aidl_return = 0;
    return ::ndk::ScopedAStatus::ok();
}

::ndk::ScopedAStatus Urcc::urccResStateRequest(const UrccRequestParcel& request,
                                               std::vector<UrccRequestData>* _aidl_return) {
    _aidl_return->clear();
    for (const UrccRequestData& item : request.mUrccRequestData) {
        log_msg("urcc state: id=0x%x path=%s\n", item.res_Id,
                item.res_Path.c_str());
        UrccRequestData result;
        // A state request may address a node by path instead of by resource
        // id (the UAH client does both).
        if (!item.res_Path.empty() && item.res_Path[0] == '/') {
            result.res_Id = item.res_Id;
            result.event_Id = 0;
            result.res_Value = mCore.read_node(item.res_Path);
            result.res_Path = "0";
            result.desc = item.res_Path;
        } else {
            fill_state(static_cast<uint32_t>(item.res_Id), &result);
        }
        _aidl_return->push_back(result);
    }
    // A request without an explicit resource list is answered with the
    // sampled telemetry, which is what the performance gauge consumes.
    if (_aidl_return->empty()) {
        sample_once(_aidl_return);
    }
    return ::ndk::ScopedAStatus::ok();
}

::ndk::ScopedAStatus Urcc::urccResListeningRegister(
        const UrccRequestParcel& request,
        const std::shared_ptr<IUrccCallback>& callback,
        int32_t* _aidl_return) {
    if (callback == nullptr) {
        *_aidl_return = -1;
        return ::ndk::ScopedAStatus::ok();
    }
    int32_t handle;
    {
        std::lock_guard<std::mutex> guard(mLock);
        handle = mNextHandle++;
        mCallbacks[handle] = callback;
    }
    *_aidl_return = handle;
    return ::ndk::ScopedAStatus::ok();
}

::ndk::ScopedAStatus Urcc::urccResListeningUnRegister(int32_t handle,
                                                      int32_t* _aidl_return) {
    std::lock_guard<std::mutex> guard(mLock);
    mCallbacks.erase(handle);
    *_aidl_return = 0;
    return ::ndk::ScopedAStatus::ok();
}

::ndk::ScopedAStatus Urcc::urccPropertyGet(const std::string& key,
                                           std::string* _aidl_return) {
    std::lock_guard<std::mutex> guard(mLock);
    auto it = mProperties.find(key);
    _aidl_return->assign(it != mProperties.end() ? it->second : "0");
    return ::ndk::ScopedAStatus::ok();
}

::ndk::ScopedAStatus Urcc::urccPropertySet(const std::string& key,
                                           const std::string& value,
                                           int32_t* _aidl_return) {
    {
        std::lock_guard<std::mutex> guard(mLock);
        mProperties[key] = value;
    }
    *_aidl_return = 0;
    return ::ndk::ScopedAStatus::ok();
}

::ndk::ScopedAStatus Urcc::urccThermalListeningRegister(
        const std::vector<int32_t>& types,
        const std::shared_ptr<IUrccCallback>& callback,
        int32_t* _aidl_return) {
    if (callback == nullptr) {
        *_aidl_return = -1;
        return ::ndk::ScopedAStatus::ok();
    }
    int32_t handle;
    {
        std::lock_guard<std::mutex> guard(mLock);
        handle = mNextHandle++;
        mThermalCallbacks[handle] = callback;
    }
    *_aidl_return = handle;
    return ::ndk::ScopedAStatus::ok();
}

::ndk::ScopedAStatus Urcc::urccThermalListeningUnRegister(int32_t handle,
                                                         int32_t* _aidl_return) {
    std::lock_guard<std::mutex> guard(mLock);
    mThermalCallbacks.erase(handle);
    *_aidl_return = 0;
    return ::ndk::ScopedAStatus::ok();
}

::ndk::ScopedAStatus Urcc::uahNotifyExt(int32_t src, int32_t type,
                                        const std::vector<int32_t>& data,
                                        int32_t* _aidl_return) {
    *_aidl_return = 0;
    return ::ndk::ScopedAStatus::ok();
}

::ndk::ScopedAStatus Urcc::setRelatedSysInfo(int32_t type,
                                             const std::vector<uint8_t>& info,
                                             int32_t* _aidl_return) {
    // The shipped service ignores both arguments and writes 0.
    *_aidl_return = 0;
    return ::ndk::ScopedAStatus::ok();
}

::ndk::ScopedAStatus Urcc::urccRuleCtl(int32_t type, int32_t rule,
                                        const std::vector<UrccRequestData>& data,
                                        int32_t* _aidl_return) {
    apply_request(data);
    *_aidl_return = 0;
    return ::ndk::ScopedAStatus::ok();
}

::ndk::ScopedAStatus Urcc::uahResCtlRequestBypass(const UrccRequestParcel& request) {
    apply_request(request.mUrccRequestData);
    return ::ndk::ScopedAStatus::ok();
}

}  // namespace aidl::vendor::oplus::hardware::urcc::impl::oplus
