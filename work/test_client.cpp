// Test client for the URCC HAL: calls the running service the same way the
// framework does and prints what comes back.
//
//   test_client init
//   test_client ctl <res_id> <value>
//   test_client state <res_id>
//   test_client propset <key> <value>
//   test_client property <key>
#include <android/binder_ibinder.h>
#include <android/binder_manager.h>
#include <android/binder_process.h>
#include <android/binder_status.h>
#include <android/log.h>

#include <cstdio>
#include <cstdlib>
#include <string>

#include <aidl/vendor/oplus/hardware/urcc/IUrcc.h>
#include <aidl/vendor/oplus/hardware/urcc/UrccRequestData.h>
#include <aidl/vendor/oplus/hardware/urcc/UrccRequestParcel.h>

using aidl::vendor::oplus::hardware::urcc::IUrcc;
using aidl::vendor::oplus::hardware::urcc::UrccRequestData;
using aidl::vendor::oplus::hardware::urcc::UrccRequestParcel;

int main(int argc, char** argv) {
    ABinderProcess_setThreadPoolMaxThreadCount(0);

    const std::string instance = std::string(IUrcc::descriptor) + "/default";
    AIBinder* raw = AServiceManager_checkService(instance.c_str());
    if (raw == nullptr) {
        std::printf("no service %s\n", instance.c_str());
        return 1;
    }
    ::ndk::SpAIBinder service(raw);
    auto urcc = IUrcc::fromBinder(service);
    if (urcc == nullptr) {
        std::printf("fromBinder failed\n");
        return 1;
    }
    std::printf("interface hash: %s\n", IUrcc::hash.c_str());

    if (argc < 2) {
        std::printf("usage: init | ctl <id> <value> | state <id> | property <key>\n");
        return 0;
    }
    const std::string cmd = argv[1];

    if (cmd == "init") {
        auto status = urcc->urccInit();
        std::printf("urccInit: %s\n", status.get());
        return 0;
    }
    if (cmd == "ctl" && argc >= 4) {
        UrccRequestParcel request;
        request.type = 1;
        UrccRequestData item;
        item.res_Id = static_cast<int32_t>(strtol(argv[2], nullptr, 0));
        item.res_Value = argv[3];
        request.mUrccRequestData.push_back(item);
        int32_t handle = -1;
        auto status = urcc->urccResCtlRequest(request, &handle);
        std::printf("urccResCtlRequest: %s handle=%d\n",
                    status.get(), handle);
        return 0;
    }
    if (cmd == "state" && argc >= 3) {
        UrccRequestParcel request;
        request.type = 2;
        UrccRequestData item;
        item.res_Id = static_cast<int32_t>(strtol(argv[2], nullptr, 0));
        request.mUrccRequestData.push_back(item);
        std::vector<UrccRequestData> out;
        auto status = urcc->urccResStateRequest(request, &out);
        std::printf("urccResStateRequest: %s count=%zu\n",
                    status.get(), out.size());
        for (const UrccRequestData& r : out) {
            std::printf("  id=0x%x value='%s' path='%s' desc='%s'\n", r.res_Id,
                        r.res_Value.c_str(), r.res_Path.c_str(), r.desc.c_str());
        }
        return 0;
    }
    if (cmd == "property" && argc >= 3) {
        std::string value;
        auto status = urcc->urccPropertyGet(argv[2], &value);
        std::printf("urccPropertyGet('%s'): %s -> '%s'\n", argv[2],
                    status.get(), value.c_str());
        return 0;
    }
    std::printf("unknown command\n");
    return 2;
}
