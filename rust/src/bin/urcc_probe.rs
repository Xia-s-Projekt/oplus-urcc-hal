// urcc_probe: wire-level test client for the URCC HAL.
//
// Speaks the exact protocol the framework's IUrcc$Stub$Proxy speaks: looks up
// "vendor.oplus.hardware.urcc.IUrcc/default", drives every one of the 14
// transactions plus the reserved getInterfaceVersion / getInterfaceHash pair,
// parses the replies the way the Java/ndk clients do, and registers a local
// IUrccCallback binder to receive the sampler's pushed events.
//
// Build: same crate, `cargo build --release --target aarch64-linux-android
// --bin urcc_probe`. Run as root on the target.

#[path = "../wire.rs"]
mod wire;

use std::ffi::CString;
use std::os::raw::{c_char, c_void};
use std::ptr;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use wire::*;

const INSTANCE: &str = "vendor.oplus.hardware.urcc.IUrcc/default";
const DESCRIPTOR: &str = "vendor.oplus.hardware.urcc.IUrcc";
const CALLBACK_DESCRIPTOR: &str = "vendor.oplus.hardware.urcc.IUrccCallback";

extern "C" {
    pub fn AServiceManager_checkService(instance: *const c_char) -> *mut AIBinder;
}

// Local IUrccCallback implementation.

static CALLBACK_HITS: AtomicI32 = AtomicI32::new(0);

unsafe extern "C" fn cb_on_create(_args: *mut c_void) -> *mut c_void {
    ptr::null_mut()
}
unsafe extern "C" fn cb_on_destroy(_user_data: *mut c_void) {}
unsafe extern "C" fn cb_on_transact(
    _binder: *mut AIBinder,
    code: TransactionCode,
    in_parcel: *const AParcel,
    out_parcel: *mut AParcel,
) -> BinderStatus {
    if code != FIRST_CALL_TRANSACTION {
        return -31; // STATUS_UNKNOWN_TRANSACTION
    }
    // onCallback(in UrccRequestData[]): int32 length + versioned elements.
    let len = match read_i32(in_parcel) {
        Ok(n) => n,
        Err(rc) => return rc,
    };
    let mut rows = Vec::new();
    for _ in 0..len.max(0) {
        let (start, size) = match read_parcelable_header(in_parcel) {
            Ok(t) => t,
            Err(rc) => return rc,
        };
        let res_id = read_i32(in_parcel).unwrap_or(-1);
        let event_id = read_i32(in_parcel).unwrap_or(-1);
        let value = read_string(in_parcel).unwrap_or(None).unwrap_or_default();
        let path = read_string(in_parcel).unwrap_or(None).unwrap_or_default();
        let desc = read_string(in_parcel).unwrap_or(None).unwrap_or_default();
        let _ = finish_parcelable(in_parcel, start, size);
        rows.push(format!("   id=0x{:x} event={} value={} path={} desc={}", res_id, event_id, value, path, desc));
    }
    if !rows.is_empty() {
        let mut hits = String::new();
        for row in &rows {
            hits.push_str(row);
            hits.push('\n');
        }
        *LAST_EVENT.lock().unwrap() = hits;
        CALLBACK_HITS.fetch_add(1, Ordering::SeqCst);
    }
    match write_ok_status(out_parcel) {
        Ok(()) => STATUS_OK,
        Err(rc) => rc,
    }
}

static LAST_EVENT: Mutex<String> = Mutex::new(String::new());

// One synchronous call.

fn read_status_ok(out: *mut AParcel) -> bool {
    let mut status: *mut AStatus = ptr::null_mut();
    // SAFETY: out is a reply parcel from a successful transact.
    let rc = unsafe { AParcel_readStatusHeader(out, &mut status) };
    if rc != STATUS_OK || status.is_null() {
        return false;
    }
    let ok = unsafe { AStatus_isOk(status) };
    unsafe { AStatus_delete(status) };
    ok
}

fn invoke(binder: *mut AIBinder, code: TransactionCode, write: impl FnOnce(*mut AParcel)) -> Option<*mut AParcel> {
    let mut in_parcel: *mut AParcel = ptr::null_mut();
    // SAFETY: binder is a live AIBinder the caller owns a reference to.
    if unsafe { AIBinder_prepareTransaction(binder, &mut in_parcel) } != STATUS_OK || in_parcel.is_null() {
        eprintln!("prepareTransaction({}) failed", code);
        return None;
    }
    write(in_parcel);
    let mut out: *mut AParcel = ptr::null_mut();
    // SAFETY: in_parcel came from prepareTransaction; transact takes ownership
    // and hands back out_parcel.
    if unsafe { AIBinder_transact(binder, code, &mut in_parcel, &mut out, 0) } != STATUS_OK || out.is_null() {
        eprintln!("transact({}) failed", code);
        return None;
    }
    Some(out)
}

struct Probe {
    binder: *mut AIBinder,
    failures: usize,
    checks: usize,
}

impl Probe {
    fn run(&mut self, name: &str, code: TransactionCode, write: impl FnOnce(*mut AParcel), check_reply: impl FnOnce(*mut AParcel) -> bool) {
        self.checks += 1;
        let out = match invoke(self.binder, code, write) {
            Some(o) => o,
            None => {
                println!("FAIL {} (transport)", name);
                self.failures += 1;
                return;
            }
        };
        let ok = read_status_ok(out) && check_reply(out);
        // SAFETY: out is a reply parcel we own.
        unsafe { AParcel_delete(out) };
        if ok {
            println!("PASS {}", name);
        } else {
            println!("FAIL {}", name);
            self.failures += 1;
        }
    }
}

// Parcel writers.

// One versioned UrccRequestData: int32 size, int32 res_Id, int32 event_Id,
// String res_Value, String res_Path, String desc.
fn write_request_data(in_parcel: *mut AParcel, res_id: i32, value: &str, path: &str) {
    let start = begin_parcelable(in_parcel).ok();
    write_i32(in_parcel, res_id).ok();
    write_i32(in_parcel, 0).ok();
    write_string(in_parcel, value).ok();
    write_string(in_parcel, path).ok();
    write_string(in_parcel, "probe").ok();
    if let Some(start) = start {
        end_parcelable(in_parcel, start).ok();
    }
}

// UrccRequestParcel field order (fixed by the framework stub):
//   int type; UrccRequestData[] mUrccRequestData; boolean screenoff_support;
//   int prio; int callingPID; int callingUID; long duration; String[]
//   mParams; int handle; IUrccCallback urccCallback; String pkgName; String
//   identity; byte[] otherData;
fn write_request_parcel(in_parcel: *mut AParcel, type_: i32, items: &[(i32, &str, &str)], callback: *mut AIBinder) {
    let p_start = begin_parcelable(in_parcel).ok();
    write_i32(in_parcel, type_).ok();
    write_i32(in_parcel, items.len() as i32).ok();
    for (id, value, path) in items {
        write_request_data(in_parcel, *id, value, path);
    }
    write_bool(in_parcel, false).ok(); // screenoff_support
    write_i32(in_parcel, 0).ok(); // prio
    write_i32(in_parcel, std::process::id() as i32).ok(); // callingPID
    write_i32(in_parcel, 1000).ok(); // callingUID
    write_i64(in_parcel, 0).ok(); // duration
    // mParams: String[]
    write_i32(in_parcel, 1).ok();
    write_string(in_parcel, "probe.param").ok();
    write_i32(in_parcel, 0).ok(); // handle
    // SAFETY: writing a strong binder takes no refcount from the caller.
    let _ = unsafe { AParcel_writeStrongBinder(in_parcel, callback) };
    write_string(in_parcel, "com.oplus.probe").ok(); // pkgName
    write_string(in_parcel, "probe").ok(); // identity
    write_byte_array(in_parcel, &[1, 2, 3]).ok(); // otherData
    if let Some(start) = p_start {
        end_parcelable(in_parcel, start).ok();
    }
}

// Reads a versioned UrccRequestData[]: int32 length + elements.
fn read_request_data_array(out: *mut AParcel) -> Result<Vec<String>, ()> {
    let len = read_i32(out).map_err(|_| ())?;
    let mut rows = Vec::new();
    for _ in 0..len.max(0) {
        let (start, size) = read_parcelable_header(out).map_err(|_| ())?;
        let res_id = read_i32(out).unwrap_or(-1);
        let event_id = read_i32(out).unwrap_or(-1);
        let value = read_string(out).unwrap_or(None).unwrap_or_default();
        let path = read_string(out).unwrap_or(None).unwrap_or_default();
        let desc = read_string(out).unwrap_or(None).unwrap_or_default();
        finish_parcelable(out, start, size).map_err(|_| ())?;
        rows.push(format!("   id=0x{:x} event={} value={} path={} desc={}", res_id, event_id, value, path, desc));
    }
    Ok(rows)
}

// int[] as int32 length + values.
fn write_int32_array(in_parcel: *mut AParcel, values: &[i32]) {
    write_i32(in_parcel, values.len() as i32).ok();
    for v in values {
        write_i32(in_parcel, *v).ok();
    }
}

fn make_callback_binder() -> *mut AIBinder {
    let descriptor = CString::new(CALLBACK_DESCRIPTOR).unwrap();
    // SAFETY: descriptor is NUL-terminated; the callbacks are plain functions
    // matching the documented AIBinder_Class typedefs.
    let class = unsafe {
        AIBinder_Class_define(
            descriptor.as_ptr() as *const c_char,
            cb_on_create,
            cb_on_destroy,
            cb_on_transact,
        )
    };
    if class.is_null() {
        return ptr::null_mut();
    }
    // SAFETY: class was just defined; no per-instance args.
    unsafe { AIBinder_new(class, ptr::null_mut()) }
}

fn main() {
    let instance = CString::new(INSTANCE).unwrap();
    // SAFETY: instance is NUL-terminated; the returned binder carries a strong
    // reference we own (never released; the probe is short-lived).
    let binder = unsafe { AServiceManager_checkService(instance.as_ptr() as *const c_char) };
    if binder.is_null() {
        eprintln!("no service {}", INSTANCE);
        std::process::exit(2);
    }
    // A client binder must be associated with the interface class before it
    // can transact (this is what IUrcc::fromBinder does for the ndk
    // generated client). The class needs the interface descriptor; the
    // onTransact handler never runs on a remote proxy.
    let descriptor = CString::new(DESCRIPTOR).unwrap();
    let class = unsafe {
        AIBinder_Class_define(
            descriptor.as_ptr() as *const c_char,
            cb_on_create,
            cb_on_destroy,
            cb_on_transact,
        )
    };
    if class.is_null() || !unsafe { AIBinder_associateClass(binder, class) } {
        eprintln!("associateClass failed");
        std::process::exit(2);
    }
    let mut probe = Probe { binder, failures: 0, checks: 0 };

    // 1: urccInit
    probe.run("urccInit", 1, |_| (), |_| true);

    // 7: urccPropertyGet("ro.hardware") must return the platform's hardware
    // property value; the service forwards to a real system-property read.
    probe.run(
        "urccPropertyGet(ro.hardware)==mt6893",
        7,
        |in_p| { write_string(in_p, "ro.hardware").ok(); },
        |out| read_string(out).unwrap_or(None).unwrap_or_default() == "mt6893",
    );

    // 4: urccResStateRequest with an empty resource list, answered with the
    // sampled telemetry (GPU busy + three cluster loads).
    probe.run(
        "urccResStateRequest(telemetry, 4 rows)",
        4,
        |in_p| write_request_parcel(in_p, 1, &[], ptr::null_mut()),
        |out| match read_request_data_array(out) {
            Ok(rows) => {
                println!("   telemetry ({} rows):", rows.len());
                for row in &rows { println!("{}", row) }
                rows.len() == 4
            }
            Err(_) => false,
        },
    );

    // 4 again, addressed by node path: thermal_zone5/temp.
    probe.run(
        "urccResStateRequest(by path)",
        4,
        |in_p| write_request_parcel(in_p, 1, &[(0, "", "/sys/class/thermal/thermal_zone5/temp")], ptr::null_mut()),
        |out| match read_request_data_array(out) {
            Ok(rows) => {
                for row in &rows { println!("{}", row) }
                rows.len() == 1
            }
            Err(_) => false,
        },
    );

    // 4 with a resource id: CPU_CORE_CTL_MAX_CPUS_GOLD (0x01010ba0).
    probe.run(
        "urccResStateRequest(by id 0x01010ba0)",
        4,
        |in_p| write_request_parcel(in_p, 1, &[(0x0101_0ba0, "", "")], ptr::null_mut()),
        |out| match read_request_data_array(out) {
            Ok(rows) => {
                for row in &rows { println!("{}", row) }
                rows.len() == 1
            }
            Err(_) => false,
        },
    );

    // 2: urccResCtlRequest: set CPU_CORE_CTL_MAX_CPUS_PRIME (0x01010bb0) to
    // its current value (a no-op write that proves the write path resolves).
    probe.run(
        "urccResCtlRequest(max_cpus_prime)",
        2,
        |in_p| {
            let cur = std::fs::read_to_string("/sys/devices/system/cpu/cpu7/core_ctl/max_cpus")
                .unwrap_or_default()
                .trim()
                .to_string();
            write_request_parcel(in_p, 1, &[(0x0101_0bb0, Box::leak(cur.into_boxed_str()), "")], ptr::null_mut());
        },
        |out| read_i32(out).unwrap_or(-1) == 0,
    );

    // 13: urccRuleCtl(1, 1, [UrccRequestData]): the same apply path.
    probe.run(
        "urccRuleCtl",
        13,
        |in_p| {
            write_i32(in_p, 1).ok(); // type
            write_i32(in_p, 1).ok(); // rule
            // UrccRequestData[]
            write_i32(in_p, 1).ok(); // length
            write_request_data(in_p, 0x0101_0b10, "4", ""); // MIN_CPUS_SILVER
        },
        |out| read_i32(out).unwrap_or(-1) == 0,
    );

    // 14: uahResCtlRequestBypass: no return value beyond the status.
    probe.run(
        "uahResCtlRequestBypass",
        14,
        |in_p| write_request_parcel(in_p, 1, &[(0x0101_0b10, "4", "")], ptr::null_mut()),
        |_| true,
    );

    // 5: urccResListeningRegister with a real local callback binder.
    let mut handle: i32 = -1;
    let callback = make_callback_binder();
    probe.run(
        "urccResListeningRegister(handle>0)",
        5,
        |in_p| write_request_parcel(in_p, 1, &[], callback),
        |out| {
            handle = read_i32(out).unwrap_or(-1);
            handle > 0
        },
    );

    // The binder pool must be running for the service to call us back.
    // SAFETY: starts the process's binder pool.
    unsafe { ABinderProcess_startThreadPool() };

    // 9: urccThermalListeningRegister([5], callback)
    let mut thermal_handle: i32 = -1;
    probe.run(
        "urccThermalListeningRegister(handle>0)",
        9,
        |in_p| {
            write_int32_array(in_p, &[5]);
            // SAFETY: writing a strong binder takes no refcount from the caller.
            let _ = unsafe { AParcel_writeStrongBinder(in_p, callback) };
        },
        |out| {
            thermal_handle = read_i32(out).unwrap_or(-1);
            thermal_handle > 0
        },
    );

    // 11: uahNotifyExt(1, 2, [1,2,3])
    probe.run(
        "uahNotifyExt",
        11,
        |in_p| {
            write_i32(in_p, 1).ok();
            write_i32(in_p, 2).ok();
            write_int32_array(in_p, &[1, 2, 3]);
        },
        |out| read_i32(out).unwrap_or(-1) == 0,
    );

    // 12: setRelatedSysInfo(1, [1,2,3])
    probe.run(
        "setRelatedSysInfo",
        12,
        |in_p| {
            write_i32(in_p, 1).ok();
            write_byte_array(in_p, &[1, 2, 3]).ok();
        },
        |out| read_i32(out).unwrap_or(-1) == 0,
    );

    // 8: urccPropertySet + Get round-trip.
    probe.run(
        "urccPropertySet(persist.vendor.urcc.probe)",
        8,
        |in_p| {
            write_string(in_p, "persist.vendor.urcc.probe").ok();
            write_string(in_p, "ok").ok();
        },
        |out| read_i32(out).unwrap_or(-1) == 0,
    );
    probe.run(
        "urccPropertyGet(persist.vendor.urcc.probe)==ok",
        7,
        |in_p| { write_string(in_p, "persist.vendor.urcc.probe").ok(); },
        |out| read_string(out).unwrap_or(None).unwrap_or_default() == "ok",
    );

    // Reserved: 16777215 = getInterfaceVersion, 16777214 = getInterfaceHash.
    probe.run(
        "getInterfaceVersion==1",
        TRANSACTION_GET_INTERFACE_VERSION,
        |_| (),
        |out| read_i32(out).unwrap_or(-1) == 1,
    );
    probe.run(
        "getInterfaceHash==cae43655...",
        TRANSACTION_GET_INTERFACE_HASH,
        |_| (),
        |out| read_string(out).unwrap_or(None).unwrap_or_default() == "cae43655eaf0af39cbb6d273c5e2206eebe33a10",
    );

    // Give the sampler threads a chance to push events to our callback.
    if handle > 0 {
        println!("waiting up to 4s for sampler events on handle {}...", handle);
        for _ in 0..40 {
            if CALLBACK_HITS.load(Ordering::Relaxed) > 0 {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
        let hits = CALLBACK_HITS.load(Ordering::Relaxed);
        println!("callback events received: {}", hits);
        let last = LAST_EVENT.lock().unwrap().clone();
        if !last.is_empty() {
            println!("last event:"); 
            print!("{}", last);
        }
        probe.checks += 1;
        if hits > 0 {
            println!("PASS callback events delivered");
        } else {
            println!("FAIL callback events delivered");
            probe.failures += 1;
        }

        // 3: urccResCtlRelease(handle)
        probe.run(
            "urccResCtlRelease",
            3,
            |in_p| { write_i32(in_p, handle).ok(); },
            |out| read_i32(out).unwrap_or(-1) == 0,
        );

        // 6: urccResListeningUnRegister(handle)
        probe.run(
            "urccResListeningUnRegister",
            6,
            |in_p| { write_i32(in_p, handle).ok(); },
            |out| read_i32(out).unwrap_or(-1) == 0,
        );
    }

    // 10: urccThermalListeningUnRegister: use handle 999 to exercise the
    // unknown-handle path (must still answer 0, not fail).
    probe.run(
        "urccThermalListeningUnRegister(unknown handle)",
        10,
        |in_p| { write_i32(in_p, 999).ok(); },
        |out| read_i32(out).unwrap_or(-1) == 0,
    );

    println!();
    println!("{} checks, {} failures", probe.checks, probe.failures);
    if probe.failures == 0 {
        println!("ALL CHECKS PASSED");
        std::process::exit(0);
    }
    std::process::exit(1);
}
