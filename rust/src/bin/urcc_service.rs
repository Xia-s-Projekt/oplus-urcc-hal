// OPlus URCC AIDL HAL service entry point (Rust port).
//
// The startup sequence was recovered from the shipped binary
// (odm/bin/hw/vendor.oplus.hardware.urcc-service):
//
//   ABinderProcess_setThreadPoolMaxThreadCount(0);
//   impl   = new Urcc();                      // ctor calls load_core_lib()
//                                            // and then urccModuleInit()
//   name   = IUrcc::descriptor + "/default";
//   status = AServiceManager_addService(impl->asBinder(), name);
//   set SCHED_FIFO (prio 1) on the main thread, name it "UrccMainThread";
//   mlockall;
//   ABinderProcess_joinThreadPool();
//
// The log tag is "UrccFramework-halservice". In this port the closed
// liburcccore.so is not dlopen'ed at all: the resource core is implemented
// natively (see lib/src/core.rs) because the shipped core drives Qualcomm-only
// nodes and faults on this MTK board.

#[path = "../wire.rs"]
mod wire;
#[path = "../hal.rs"]
mod hal;

use std::ffi::CString;
use std::os::raw::{c_char, c_int};

use hal::{on_create, on_destroy, on_transact, LOG_TAG};
use wire::{
    log, AIBinder_Class_define, AIBinder_new, AServiceManager_addService,
    ABinderProcess_joinThreadPool, ABinderProcess_setThreadPoolMaxThreadCount, LOG_LEVEL_ERROR,
    LOG_LEVEL_INFO,
};

const DESCRIPTOR: &str = "vendor.oplus.hardware.urcc.IUrcc";
const INSTANCE: &str = "vendor.oplus.hardware.urcc.IUrcc/default";
const THREAD_NAME: &str = "UrccMainThread";

// from bionic's sched.h
const SCHED_FIFO: c_int = 1;
#[repr(C)]
struct SchedParam {
    sched_priority: c_int,
}
// from bionic's sys/mman.h
const MCL_CURRENT: c_int = 1;
const MCL_FUTURE: c_int = 2;
// from bionic's sys/prctl.h
const PR_SET_NAME: c_int = 15;

extern "C" {
    fn mlockall(flags: c_int) -> c_int;
    fn prctl(option: c_int, arg2: usize, arg3: usize, arg4: usize, arg5: usize) -> c_int;
    fn sched_setscheduler(pid: c_int, policy: c_int, param: *const SchedParam) -> c_int;
}

fn main() {
    // The shipped service locks its pages in RAM and puts the registering
    // thread on SCHED_FIFO so the Binder worker is scheduled ahead of normal
    // work.
    //
    // SAFETY: plain libc calls with scalar/POD arguments.
    let rc = unsafe { mlockall(MCL_CURRENT | MCL_FUTURE) };
    if rc != 0 {
        log(LOG_LEVEL_ERROR, LOG_TAG, "mlockall failed");
    }

    // SAFETY: configures this process's binder thread pool; no arguments.
    unsafe { ABinderProcess_setThreadPoolMaxThreadCount(0) };

    let descriptor = CString::new(DESCRIPTOR).unwrap();
    let class = unsafe {
        AIBinder_Class_define(
            descriptor.as_ptr() as *const c_char,
            on_create,
            on_destroy,
            on_transact,
        )
    };
    if class.is_null() {
        log(LOG_LEVEL_ERROR, LOG_TAG, "AIBinder_Class_define failed");
        std::process::exit(1);
    }

    // SAFETY: class was just defined; no args are needed (on_create builds
    // the service state itself).
    let binder = unsafe { AIBinder_new(class, std::ptr::null_mut()) };
    if binder.is_null() {
        log(LOG_LEVEL_ERROR, LOG_TAG, "AIBinder_new failed");
        std::process::exit(1);
    }

    let instance = CString::new(INSTANCE).unwrap();
    // SAFETY: binder is a live AIBinder we own; instance is NUL-terminated.
    let status = unsafe { AServiceManager_addService(binder, instance.as_ptr() as *const c_char) };
    if status != 0 {
        log(
            LOG_LEVEL_ERROR,
            LOG_TAG,
            &format!("start urcc hal service failed status:{}", status),
        );
    }

    let param = SchedParam { sched_priority: 1 };
    // SAFETY: param is a plain POD struct of one int.
    let rc = unsafe { sched_setscheduler(0, SCHED_FIFO, &param) };
    if rc != 0 {
        log(LOG_LEVEL_ERROR, LOG_TAG, "set SCHED_FIFO failed");
    } else {
        log(LOG_LEVEL_INFO, LOG_TAG, "scheduler:1");
    }
    let name = CString::new(THREAD_NAME).unwrap();
    // SAFETY: PR_SET_NAME takes a pointer to a 16-byte NUL-terminated name.
    unsafe { prctl(PR_SET_NAME, name.as_ptr() as usize, 0, 0, 0) };

    log(LOG_LEVEL_INFO, LOG_TAG, "Urcc HAL AIDL Service is started.");

    // SAFETY: hands the main thread to the binder thread pool; returns only
    // when the process is shutting down.
    unsafe { ABinderProcess_joinThreadPool() };

    log(LOG_LEVEL_ERROR, LOG_TAG, "Urcc HAL AIDL Service just died.");
    std::process::exit(status);
}
