// A minimal AIDL client that exercises the URCC service over Binder.
// Built against the same generated backend as the service.
#include <android/binder_ibinder.h>
#include <android/binder_manager.h>
#include <android/binder_process.h>
#include <android/log.h>

#include <aidl/vendor/oplus/hardware/urcc/IUrcc.h>
#include <aidl/vendor/oplus/hardware/urcc/UrccRequestData.h>
#include <aidl/vendor/oplus/hardware/urcc/UrccRequestParcel.h>

#include <cstdio>
#include <cstdlib>
#include <string>

using aidl::vendor::oplus::hardware::urcc::IUrcc;
using aidl::vendor::oplus::hardware::urcc::UrccRequestData;
using aidl::vendor::oplus::hardware::urcc::UrccRequestParcel;

int main() {
    setvbuf(stdout, nullptr, _IONBF, 0);
    // The service registers the descriptor plus "/default".
    const std::string instance = std::string(IUrcc::descriptor) + "/default";
    (void) AServiceManager_waitForService(instance.c_str());
    auto binder = AServiceManager_checkService(instance.c_str());
    if (binder == nullptr) {
        printf("CHECK: no service %s\n", IUrcc::descriptor);
        return 1;
    }
    auto service = IUrcc::fromBinder(::ndk::SpAIBinder(binder));
    printf("CHECK: got service %s\n", IUrcc::descriptor);

    int32_t version = -1;
    auto st = service->getInterfaceVersion(&version);
    printf("CHECK: getInterfaceVersion -> %d ok=%d\n", version, st.get() != nullptr);

    std::string hash;
    st = service->getInterfaceHash(&hash);
    printf("CHECK: getInterfaceHash -> %s ok=%d\n", hash.c_str(), st.get() != nullptr);

    st = service->urccInit();
    printf("CHECK: urccInit ok=%d\n", st.get() != nullptr);

    std::string prop;
    st = service->urccPropertyGet("persist.sys.thermal", &prop);
    printf("CHECK: urccPropertyGet -> value=%s ok=%d\n", prop.c_str(),
           st.get() != nullptr);

    UrccRequestParcel request;
    request.type = 1;
    request.pkgName = "com.oplus.phonemanager";
    request.identity = "com.oplus.phonemanager";
    request.callingPID = 1234;
    request.callingUID = 10000;
    request.prio = 1;

    int32_t ctl = -999;
    st = service->urccResCtlRequest(request, &ctl);
    printf("CHECK: urccResCtlRequest -> %d ok=%d\n", ctl, st.get() != nullptr);

    int32_t rel = -999;
    st = service->urccResCtlRelease(ctl, &rel);
    printf("CHECK: urccResCtlRelease -> %d ok=%d\n", rel, st.get() != nullptr);

    std::vector<UrccRequestData> states;
    st = service->urccResStateRequest(request, &states);
    printf("CHECK: urccResStateRequest -> %zu samples ok=%d\n", states.size(),
           st.get() != nullptr);
    for (const auto& s : states) {
        printf("CHECK:   res_Id=%d event_Id=%d value=%s path=%s desc=%s\n", s.res_Id,
               s.event_Id, s.res_Value.c_str(), s.res_Path.c_str(), s.desc.c_str());
    }

    int32_t setrc = -999;
    st = service->urccPropertySet("persist.test.urcc", "1", &setrc);
    printf("CHECK: urccPropertySet -> %d ok=%d\n", setrc, st.get() != nullptr);

    int32_t notify = -999;
    std::vector<int32_t> data{1, 2, 3};
    st = service->uahNotifyExt(1, 1, data, &notify);
    printf("CHECK: uahNotifyExt -> %d ok=%d\n", notify, st.get() != nullptr);

    int32_t related = -999;
    std::vector<uint8_t> info{0};
    st = service->setRelatedSysInfo(1, info, &related);
    printf("CHECK: setRelatedSysInfo -> %d ok=%d\n", related, st.get() != nullptr);

    printf("CHECK: done\n");
    return 0;
}
