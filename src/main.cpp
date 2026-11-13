/*
 * OPlus URCC AIDL HAL — service entry point.
 *
 * The startup sequence was recovered from the shipped binary
 * (odm/bin/hw/vendor.oplus.hardware.urcc-service):
 *
 *   ABinderProcess_setThreadPoolMaxThreadCount(0);
 *   impl   = new Urcc();                     // ctor calls load_core_lib() and
 *                                            // then urccModuleInit()
 *   name   = IUrcc::descriptor + "/default";
 *   status = AServiceManager_addService(impl->asBinder(), name);
 *   set SCHED_FIFO on the main thread, name it "UrccMainThread";
 *   ABinderProcess_joinThreadPool();
 *
 * The log tag is "UrccFramework-halservice".
 */
#include "Urcc.h"

#include <android/binder_auto_utils.h>
#include <android/binder_ibinder.h>
#include <android/binder_manager.h>
#include <android/binder_process.h>
#include <android/binder_status.h>
#include <android/log.h>
#include <pthread.h>
#include <sched.h>

#include <cstring>
#include <string>

namespace {

constexpr const char* kLogTag = "UrccFramework-halservice";
constexpr const char* kThreadName = "UrccMainThread";

}  // namespace

int main(int argc, char** argv) {
    ABinderProcess_setThreadPoolMaxThreadCount(0);

    auto service = ndk::SharedRefBase::make<
            aidl::vendor::oplus::hardware::urcc::impl::oplus::Urcc>();

    // The shipped binary builds the instance name from the interface
    // descriptor (owned by the sidecar) plus "/default".
    std::string instance = std::string(
            aidl::vendor::oplus::hardware::urcc::IUrcc::descriptor) + "/default";

    const binder_status_t status =
            AServiceManager_addService(service->asBinder().get(), instance.c_str());
    if (status != STATUS_OK) {
        __android_log_print(ANDROID_LOG_ERROR, kLogTag,
                            "start urcc hal service failed status:%d",
                            static_cast<int>(status));
    }

    // The shipped service puts the registering thread on SCHED_FIFO and
    // renames it, so the Binder worker is scheduled ahead of normal work.
    sched_param param{};
    param.sched_priority = 1;
    const int rc = pthread_setschedparam(pthread_self(), SCHED_FIFO, &param);
    if (rc != 0) {
        __android_log_print(ANDROID_LOG_ERROR, kLogTag, "set SCHED_FIFO failed %s",
                            strerror(rc));
    } else {
        __android_log_print(ANDROID_LOG_INFO, kLogTag, "scheduler:%d",
                            param.sched_priority);
    }
    pthread_setname_np(pthread_self(), kThreadName);

    __android_log_print(ANDROID_LOG_INFO, kLogTag, "Urcc HAL AIDL Service is started.");

    ABinderProcess_joinThreadPool();

    __android_log_print(ANDROID_LOG_ERROR, kLogTag, "Urcc HAL AIDL Service just died.");
    return static_cast<int>(status);
}
