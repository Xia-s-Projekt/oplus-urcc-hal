// The URCC HAL implementation.
//
// The shipped service is a thin shim over a closed core library:
//
//   odm/bin/hw/vendor.oplus.hardware.urcc-service
//     dlopen("liburcccore.so", RTLD_NOW)
//     dlsym()s 13 C-linkage entry points into the object
//     calls urccModuleInit() from the constructor
//     registers "vendor.oplus.hardware.urcc.IUrcc/default"
//
//   odm/lib64/liburcccore.so
//     the actual implementation, a closed binary this port ships as-is
//
// liburcccore.so is built for Qualcomm boards: its resource map points at
// /sys/class/kgsl/kgsl-3d0/..., /sys/class/devfreq/*qcom* and
// /sys/devices/platform/soc/*.qcom,cpucc*..., none of which exist on this
// MT6893 board, and its request path faults (SIGSEGV) when a requested
// resource has no entry in that map.  So the resource handling lives here
// instead: UrccCore owns the mapping from OnePlus resource to the MediaTek
// node that carries the same information, and implements the classes that
// have no MediaTek counterpart as real behaviour (see src/UrccCore.cpp).
//
// The AIDL surface is unchanged, because the framework links against it:
//
//   * the parcelable layout comes from the shipped platform sidecar
//     (vendor.oplus.hardware.urcc-V1-ndk_platform.so), which is preserved
//     as-is and owns the interface hash, the vtable and the onTransact
//     dispatch,
//   * the service derives from BnUrcc, not IUrccDefault, so the transport
//     symbols stay undefined in the service and are resolved from the
//     sidecar at load time,
//   * the Binder status is always OK; a failed request is reported through
//     the out parameter, as the shipped service does.
//
// The client parses the returned values numerically: liboplus-uah-client.so
// runs std::stoi() on them.  An empty string throws std::invalid_argument
// inside system_server and takes the boot sequence down with it, so every
// string handed back here is a non-empty numeric string.
#ifndef OPLUS_URCC_URCC_H
#define OPLUS_URCC_URCC_H

#include <android/binder_ibinder.h>
#include <android/binder_process.h>
#include <android/binder_status.h>

#include <aidl/vendor/oplus/hardware/urcc/BnUrcc.h>
#include <aidl/vendor/oplus/hardware/urcc/IUrcc.h>
#include <aidl/vendor/oplus/hardware/urcc/IUrccCallback.h>
#include <aidl/vendor/oplus/hardware/urcc/UrccRequestData.h>
#include <aidl/vendor/oplus/hardware/urcc/UrccRequestParcel.h>

#include <atomic>
#include <cstdint>
#include <map>
#include <mutex>
#include <string>
#include <thread>
#include <vector>

#include "UrccCore.h"

namespace aidl::vendor::oplus::hardware::urcc::impl::oplus {

class Urcc : public BnUrcc {
  public:
    Urcc();
    ~Urcc() override;

    ::ndk::ScopedAStatus urccInit() override;
    ::ndk::ScopedAStatus urccResCtlRequest(
            const UrccRequestParcel& request,
            int32_t* _aidl_return) override;
    ::ndk::ScopedAStatus urccResCtlRelease(int32_t handle,
                                           int32_t* _aidl_return) override;
    ::ndk::ScopedAStatus urccResStateRequest(
            const UrccRequestParcel& request,
            std::vector<UrccRequestData>* _aidl_return) override;
    ::ndk::ScopedAStatus urccResListeningRegister(
            const UrccRequestParcel& request,
            const std::shared_ptr<IUrccCallback>& callback,
            int32_t* _aidl_return) override;
    ::ndk::ScopedAStatus urccResListeningUnRegister(int32_t handle,
                                                    int32_t* _aidl_return) override;
    ::ndk::ScopedAStatus urccPropertyGet(const std::string& key,
                                          std::string* _aidl_return) override;
    ::ndk::ScopedAStatus urccPropertySet(const std::string& key,
                                          const std::string& value,
                                          int32_t* _aidl_return) override;
    ::ndk::ScopedAStatus urccThermalListeningRegister(
            const std::vector<int32_t>& types,
            const std::shared_ptr<IUrccCallback>& callback,
            int32_t* _aidl_return) override;
    ::ndk::ScopedAStatus urccThermalListeningUnRegister(int32_t handle,
                                                         int32_t* _aidl_return) override;
    ::ndk::ScopedAStatus uahNotifyExt(int32_t src, int32_t type,
                                      const std::vector<int32_t>& data,
                                      int32_t* _aidl_return) override;
    ::ndk::ScopedAStatus setRelatedSysInfo(int32_t type,
                                           const std::vector<uint8_t>& info,
                                           int32_t* _aidl_return) override;
    ::ndk::ScopedAStatus urccRuleCtl(int32_t type, int32_t rule,
                                     const std::vector<UrccRequestData>& data,
                                     int32_t* _aidl_return) override;
    ::ndk::ScopedAStatus uahResCtlRequestBypass(
            const UrccRequestParcel& request) override;

  private:
    // Fills one result element for a resource id: the current value, the
    // resource name and the numeric path the client expects.
    void fill_state(uint32_t id, UrccRequestData* out);

    // Applies every entry of a control request. Returns the number of
    // entries that were applied.
    int apply_request(const std::vector<UrccRequestData>& data);

    // Samples the resources the framework watches and pushes them to the
    // registered callbacks.
    void sampler_loop();
    void sample_once(std::vector<UrccRequestData>* out);

    UrccCore mCore;

    std::mutex mLock;
    std::map<int32_t, std::shared_ptr<IUrccCallback>> mCallbacks;
    std::map<int32_t, std::shared_ptr<IUrccCallback>> mThermalCallbacks;
    std::map<std::string, std::string> mProperties;
    int32_t mNextHandle = 1;

    std::atomic<bool> mRunning{true};
    std::thread mSampler;
};

}  // namespace aidl::vendor::oplus::hardware::urcc::impl::oplus

#endif  // OPLUS_URCC_URCC_H
