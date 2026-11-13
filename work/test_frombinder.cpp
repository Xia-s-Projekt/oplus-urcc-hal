// Calls the SHIPPED client-side transport's IUrcc::fromBinder() against the
// running service, to see whether the shipped client accepts our binder.
//
// fromBinder() in the shipped lib checks the binder's interface descriptor
// against its own IUrcc::descriptor; if our service registered a different
// descriptor it returns nullptr and the framework reports the HAL as
// "not available".
#include <android/binder_ibinder.h>
#include <android/binder_manager.h>
#include <android/binder_process.h>
#include <dlfcn.h>

#include <cstdio>
#include <memory>
#include <cstdlib>
#include <string>

#include <android/binder_auto_utils.h>


// Declared as a static member so the mangled name matches the shipped symbol
// exactly and the compiler emits the correct AArch64 sret ABI (x8).
namespace aidl {
namespace vendor {
namespace oplus {
namespace hardware {
namespace urcc {
class IUrcc {
  public:
    static std::shared_ptr<IUrcc> fromBinder(const ::ndk::SpAIBinder& binder);
};
}  // namespace urcc
}  // namespace hardware
}  // namespace oplus
}  // namespace vendor
}  // namespace aidl

using ShippedIUrcc = aidl::vendor::oplus::hardware::urcc::IUrcc;

int main() {
    setvbuf(stdout, nullptr, _IONBF, 0);
    std::printf("start\n");
    ABinderProcess_setThreadPoolMaxThreadCount(0);

    const char* path = "/system_ext/lib64/vendor.oplus.hardware.urcc-V1-ndk.so";
    void* lib = dlopen(path, RTLD_NOW);
    if (lib == nullptr) {
        std::printf("dlopen failed: %s\n", dlerror());
        return 1;
    }
    // std::shared_ptr<IUrcc> (16 bytes: object ptr + refcount ptr) is
    // returned through the hidden sret pointer.
    using FromBinderFn = std::shared_ptr<ShippedIUrcc> (*)(const ::ndk::SpAIBinder&);
    auto fromBinder = reinterpret_cast<FromBinderFn>(dlsym(
            lib, "_ZN4aidl6vendor5oplus8hardware4urcc5IUrcc10fromBinderERKN3ndk10SpAIBinderE"));
    if (fromBinder == nullptr) {
        std::printf("dlsym failed: %s\n", dlerror());
        return 1;
    }

    const char* instance = "vendor.oplus.hardware.urcc.IUrcc/default";
    std::printf("isDeclared(instance) -> %d\n", AServiceManager_isDeclared(instance));
    std::printf("isDeclared(iface)    -> %d\n",
                AServiceManager_isDeclared("vendor.oplus.hardware.urcc.IUrcc"));
    std::printf("isDeclared(vnd)      -> %d\n",
                AServiceManager_isDeclared("vendor.oplus.hardware.urcc.IUrcc/default"));
    AIBinder* raw = AServiceManager_checkService(instance);
    std::printf("checkService -> %p\n", (void*)raw);
    if (raw == nullptr) {
        std::printf("RESULT: service not registered\n");
        return 1;
    }
    ::ndk::SpAIBinder binder(raw);
    void* proxy = fromBinder(binder).get();
    std::printf("shipped fromBinder -> %p\n", proxy);
    if (proxy == nullptr) {
        std::printf("RESULT: shipped client REJECTED our binder (descriptor mismatch)\n");
        return 1;
    }
    std::printf("RESULT: shipped client accepted our binder\n");
    return 0;
}
