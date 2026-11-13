// Reproduces the framework's client path: links the SHIPPED system_ext AIDL
// transport (vendor.oplus.hardware.urcc-V1-ndk.so) and does exactly what the
// framework does — checkService + IUrcc::fromBinder — then calls the HAL.
#include <android/binder_ibinder.h>
#include <android/binder_manager.h>
#include <android/binder_process.h>
#include <android/binder_status.h>

#include <cstdio>
#include <string>

#include <aidl/vendor/oplus/hardware/urcc/IUrcc.h>
#include <aidl/vendor/oplus/hardware/urcc/UrccRequestData.h>
#include <aidl/vendor/oplus/hardware/urcc/UrccRequestParcel.h>

using aidl::vendor::oplus::hardware::urcc::IUrcc;
using aidl::vendor::oplus::hardware::urcc::UrccRequestData;
using aidl::vendor::oplus::hardware::urcc::UrccRequestParcel;

int main() {
    ABinderProcess_setThreadPoolMaxThreadCount(0);
    std::printf("client descriptor: %s\n", IUrcc::descriptor);
    std::printf("client hash: %s\n", IUrcc::hash.c_str());

    const char* instance = "vendor.oplus.hardware.urcc.IUrcc/default";
    AIBinder* raw = AServiceManager_checkService(instance);
    std::printf("checkService -> %p\n", (void*)raw);
    if (raw == nullptr) {
        std::printf("RESULT: service not found\n");
        return 1;
    }
    ::ndk::SpAIBinder binder(raw);
    auto urcc = IUrcc::fromBinder(binder);
    std::printf("fromBinder -> %p\n", (void*)urcc.get());
    if (urcc == nullptr) {
        std::printf("RESULT: fromBinder failed (descriptor mismatch)\n");
        return 1;
    }

    UrccRequestParcel request;
    request.type = 2;
    UrccRequestData item;
    item.res_Id = 0x1020220;
    request.mUrccRequestData.push_back(item);
    std::vector<UrccRequestData> out;
    auto status = urcc->urccResStateRequest(request, &out);
    std::printf("urccResStateRequest ok=%d count=%zu\n", status.ok() ? 1 : 0, out.size());
    for (const UrccRequestData& r : out) {
        std::printf("  id=0x%x value='%s' path='%s' desc='%s'\n", r.res_Id,
                    r.res_Value.c_str(), r.res_Path.c_str(), r.desc.c_str());
    }
    std::printf("RESULT: ok\n");
    return 0;
}
