// The URCC HAL: service state, the onTransact dispatch for all 14 interface
// methods plus the reserved pair, and the sampler threads.
//
// Every handler keeps the wire contract:
//   * the binder status is always OK; a failed request is reported through
//     the out parameter (the framework treats a non-zero int32 return from
//     urccResCtlRequest as a failed request, so 0 means "accepted"),
//   * the log tag is "UrccFramework-halservice",
//   * every string handed back is non-empty and numeric where the client
//     parses it with std::stoi() (res_Value / res_Path), because an empty
//     string throws inside system_server and takes the boot sequence down.

use std::collections::HashMap;
use std::ffi::{c_char, c_void, CString};
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use urcc_service::core::{fill_state, UrccCore};
use urcc_service::io_shim as io;

use crate::wire::*;

pub const LOG_TAG: &str = "UrccFramework-halservice";

const CALLBACK_DESCRIPTOR: &str = "vendor.oplus.hardware.urcc.IUrccCallback";

/// Parsed UrccRequestParcel fields (what the dispatch actually needs).
pub struct RequestParcel {
    pub items: Vec<RequestItem>,
}
pub struct RequestItem {
    pub res_id: i32,
    pub event_id: i32,
    pub res_value: String,
    pub res_path: String,
    pub _desc: String,
}

/// One registered callback: a strong binder reference we own. Touched only
/// under the owning Mutex, and copied only by taking an extra reference.
struct CallbackRef(*mut AIBinder);
unsafe impl Send for CallbackRef {}
unsafe impl Sync for CallbackRef {}

// Noop handlers for the IUrccCallback class on the service side: the class is
// defined so callback binders received from clients can be associated and
// transacted on (AIBinder_prepareTransaction refuses an unassociated remote
// binder). These handlers never run: the callback object lives in the
// client's process.
unsafe extern "C" fn cb_noop_create(_args: *mut c_void) -> *mut c_void {
    ptr::null_mut()
}
unsafe extern "C" fn cb_noop_destroy(_user_data: *mut c_void) {}
unsafe extern "C" fn cb_noop_transact(
    _binder: *mut AIBinder,
    _code: TransactionCode,
    _in_parcel: *const AParcel,
    _out_parcel: *mut AParcel,
) -> BinderStatus {
    -31 // STATUS_UNKNOWN_TRANSACTION
}

fn define_callback_class() -> *mut AIBinder_Class {
    let descriptor = CString::new(CALLBACK_DESCRIPTOR).unwrap();
    // SAFETY: descriptor is NUL-terminated; the handlers match the typedefs.
    unsafe {
        AIBinder_Class_define(
            descriptor.as_ptr() as *const c_char,
            cb_noop_create,
            cb_noop_destroy,
            cb_noop_transact,
        )
    }
}

impl CallbackRef {
    fn copy(&self) -> CallbackRef {
        if !self.0.is_null() {
            // SAFETY: we hold at least one strong reference while &self is
            // live; taking one more keeps the copy valid independent of the
            // original's lifetime.
            unsafe { AIBinder_incStrong(self.0) };
        }
        CallbackRef(self.0)
    }
}

impl Drop for CallbackRef {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: exactly one strong reference per CallbackRef.
            unsafe { AIBinder_decStrong(self.0) };
        }
    }
}

pub struct Service {
    res_callbacks: Mutex<HashMap<i32, CallbackRef>>,
    thermal_callbacks: Mutex<HashMap<i32, CallbackRef>>,
    next_handle: AtomicI32,
    running: AtomicBool,
    /// The IUrccCallback class, as a raw pointer stored as usize (const after
    /// creation, so the raw pointer does not affect Sync).
    callback_class: usize,
}

impl Service {
    pub fn new() -> Self {
        Service {
            res_callbacks: Mutex::new(HashMap::new()),
            thermal_callbacks: Mutex::new(HashMap::new()),
            next_handle: AtomicI32::new(1),
            running: AtomicBool::new(true),
            callback_class: define_callback_class() as usize,
        }
    }

    fn next_handle(&self) -> i32 {
        self.next_handle.fetch_add(1, Ordering::SeqCst)
    }

    /// Associates a client-supplied callback binder with the IUrccCallback
    /// class so the sampler threads can transact on it.
    fn associate_callback(&self, callback: *mut AIBinder) -> bool {
        if callback.is_null() {
            return false;
        }
        let class = self.callback_class as *const AIBinder_Class;
        if class.is_null() {
            return false;
        }
        // SAFETY: class was created by AIBinder_Class_define; callback is a
        // live binder we own a reference to.
        let ok = unsafe { AIBinder_associateClass(callback, class) };
        if !ok {
            log(LOG_LEVEL_ERROR, LOG_TAG, "associateClass failed for callback binder");
        }
        ok
    }
}

/// Spawns the sampler threads. `state` must point at a Service that outlives
/// the process (the binder-created instance does).
fn start_samplers(state: *const Service) {
    let state = state as usize;
    thread::Builder::new()
        .name("UrccResSampler".into())
        .spawn(move || res_sampler_loop(state))
        .expect("res sampler thread");
    thread::Builder::new()
        .name("UrccThermalSampler".into())
        .spawn(move || thermal_sampler_loop(state))
        .expect("thermal sampler thread");
}

// The resources the framework polls for the performance gauge.
const RES_GPU_BUSY: u32 = 0x0102_0220;
const RES_CPU_LOAD_SILVER: u32 = 0x0101_08a0;
const RES_CPU_LOAD_GOLD: u32 = 0x0101_08b0;
const RES_CPU_LOAD_PRIME: u32 = 0x0101_08c0;

fn sample_once() -> Vec<urcc_service::core::ResState> {
    vec![
        fill_state(RES_GPU_BUSY),
        fill_state(RES_CPU_LOAD_SILVER),
        fill_state(RES_CPU_LOAD_GOLD),
        fill_state(RES_CPU_LOAD_PRIME),
    ]
}

fn res_sampler_loop(state: usize) {
    let svc = unsafe { &*(state as *const Service) };
    let interval = Duration::from_millis(500);
    while svc.running.load(Ordering::Relaxed) {
        thread::sleep(interval);
        let sample = sample_once();
        // Snapshot with extra strong references so a concurrent unregister
        // cannot release a binder we are about to transact on.
        let snapshot: Vec<(i32, CallbackRef)> = {
            let callbacks = svc.res_callbacks.lock().unwrap();
            callbacks.iter().map(|(k, v)| (*k, v.copy())).collect()
        };
        let mut dead = Vec::new();
        for (handle, callback) in &snapshot {
            if !push_callback_event(callback.0, &sample) {
                dead.push(*handle);
            }
        }
        drop(snapshot);
        if !dead.is_empty() {
            let mut callbacks = svc.res_callbacks.lock().unwrap();
            for handle in dead {
                if let Some(cb) = callbacks.remove(&handle) {
                    drop(cb);
                }
            }
        }
    }
}

fn thermal_sampler_loop(state: usize) {
    let svc = unsafe { &*(state as *const Service) };
    // The thermal resources served from this board's zones/cdevs.
    let ids = [0x0105_1fe0u32, 0x0105_1ff0, 0x0105_1fd0, 0x0105_0030, 0x0105_0050, 0x0105_0040];
    let mut last_temp: Option<i64> = None;
    let interval = Duration::from_secs(1);
    while svc.running.load(Ordering::Relaxed) {
        thread::sleep(interval);
        // mtktscpu (thermal_zone5) in milli-°C: push an event only when the
        // temperature moved by at least 1 °C.
        let temp = io::read_int("/sys/class/thermal/thermal_zone5/temp");
        match (temp, last_temp) {
            (Some(t), Some(prev)) if (t - prev).abs() < 1000 => continue,
            (Some(t), _) => last_temp = Some(t),
            (None, _) => continue,
        }
        let sample: Vec<urcc_service::core::ResState> = ids.iter().map(|id| fill_state(*id)).collect();
        let snapshot: Vec<(i32, CallbackRef)> = {
            let callbacks = svc.thermal_callbacks.lock().unwrap();
            callbacks.iter().map(|(k, v)| (*k, v.copy())).collect()
        };
        let mut dead = Vec::new();
        for (handle, callback) in &snapshot {
            if !push_callback_event(callback.0, &sample) {
                dead.push(*handle);
            }
        }
        drop(snapshot);
        if !dead.is_empty() {
            let mut callbacks = svc.thermal_callbacks.lock().unwrap();
            for handle in dead {
                if let Some(cb) = callbacks.remove(&handle) {
                    drop(cb);
                }
            }
        }
    }
}

// onTransact plumbing.

pub unsafe extern "C" fn on_create(_args: *mut c_void) -> *mut c_void {
    let svc = Box::new(Service::new());
    let raw = Box::into_raw(svc);
    start_samplers(raw);
    raw as *mut c_void
}

pub unsafe extern "C" fn on_destroy(user_data: *mut c_void) {
    if !user_data.is_null() {
        let svc = Box::from_raw(user_data as *mut Service);
        svc.running.store(false, Ordering::Relaxed);
        drop(svc);
    }
}

pub unsafe extern "C" fn on_transact(
    binder: *mut AIBinder,
    code: TransactionCode,
    in_parcel: *const AParcel,
    out_parcel: *mut AParcel,
) -> BinderStatus {
    log(LOG_LEVEL_INFO, LOG_TAG, &format!("onTransact code={} size={}", code, data_size(in_parcel)));
    let svc: &Service = match AIBinder_getUserData(binder) {
        p if !p.is_null() => &*(p as *const Service),
        _ => return -31, // STATUS_UNKNOWN_TRANSACTION
    };
    let result: Result<(), BinderStatus> = match code {
        1 => dispatch_init(out_parcel),
        2 => dispatch_res_ctl_request(in_parcel, out_parcel),
        3 => dispatch_res_ctl_release(svc, in_parcel, out_parcel),
        4 => dispatch_res_state_request(in_parcel, out_parcel),
        5 => dispatch_res_listening_register(svc, in_parcel, out_parcel),
        6 => dispatch_res_listening_unregister(svc, in_parcel, out_parcel),
        7 => dispatch_property_get(in_parcel, out_parcel),
        8 => dispatch_property_set(in_parcel, out_parcel),
        9 => dispatch_thermal_listening_register(svc, in_parcel, out_parcel),
        10 => dispatch_thermal_listening_unregister(svc, in_parcel, out_parcel),
        11 => dispatch_uah_notify_ext(in_parcel, out_parcel),
        12 => dispatch_set_related_sys_info(in_parcel, out_parcel),
        13 => dispatch_rule_ctl(in_parcel, out_parcel),
        14 => dispatch_res_ctl_request_bypass(in_parcel, out_parcel),
        TRANSACTION_GET_INTERFACE_VERSION => {
            write_ok_status(out_parcel).and_then(|()| write_i32(out_parcel, 1))
        }
        TRANSACTION_GET_INTERFACE_HASH => {
            write_ok_status(out_parcel)
                .and_then(|()| write_string(out_parcel, "cae43655eaf0af39cbb6d273c5e2206eebe33a10"))
        }
        _ => return -31, // STATUS_UNKNOWN_TRANSACTION
    };
    match result {
        Ok(()) => STATUS_OK,
        Err(rc) => rc,
    }
}

type DispatchResult = Result<(), BinderStatus>;

fn dispatch_init(out: *mut AParcel) -> DispatchResult {
    log(LOG_LEVEL_INFO, LOG_TAG, "Urcc::init is called");
    write_ok_status(out)
}

/// Reads a UrccRequestParcel without keeping the callback binder.
fn read_request_parcel(in_parcel: *const AParcel) -> Result<RequestParcel, BinderStatus> {
    let (_items, callback) = read_request_parcel_inner(in_parcel)?;
    if !callback.is_null() {
        // SAFETY: read_strong_binder handed us exactly one strong reference.
        unsafe { AIBinder_decStrong(callback) };
    }
    Ok(RequestParcel { items: _items })
}

/// Reads a UrccRequestParcel and hands back the callback binder (one strong
/// reference, owned by the caller).
fn read_request_parcel_with_callback(in_parcel: *const AParcel) -> Result<(Vec<RequestItem>, *mut AIBinder), BinderStatus> {
    read_request_parcel_inner(in_parcel)
}

fn read_request_parcel_inner(in_parcel: *const AParcel) -> Result<(Vec<RequestItem>, *mut AIBinder), BinderStatus> {
    // UrccRequestParcel carries the versioned-parcelable size header.
    let (p_start, p_size) = read_parcelable_header(in_parcel)?;
    // type: int32
    let _type = read_i32(in_parcel)?;
    // mUrccRequestData: UrccRequestData[] (int32 length + elements, each
    // element itself a versioned parcelable)
    let len = read_i32(in_parcel)?.max(0);
    let mut items = Vec::with_capacity(len as usize);
    for _ in 0..len {
        let (i_start, i_size) = read_parcelable_header(in_parcel)?;
        let res_id = read_i32(in_parcel)?;
        let event_id = read_i32(in_parcel)?;
        let res_value = read_string(in_parcel)?.unwrap_or_default();
        let res_path = read_string(in_parcel)?.unwrap_or_default();
        let desc = read_string(in_parcel)?.unwrap_or_default();
        finish_parcelable(in_parcel, i_start, i_size)?;
        items.push(RequestItem { res_id, event_id, res_value, res_path, _desc: desc });
    }
    // screenoff_support: boolean
    let _screenoff_support = read_bool(in_parcel)?;
    // prio, callingPID, callingUID: int32 x3
    let _prio = read_i32(in_parcel)?;
    let _calling_pid = read_i32(in_parcel)?;
    let _calling_uid = read_i32(in_parcel)?;
    // duration: int64
    let _duration = read_i64(in_parcel)?;
    // mParams: String[]
    let _params = read_string_array(in_parcel)?;
    // handle: int32
    let _handle = read_i32(in_parcel)?;
    // urccCallback: strong binder (one strong reference to the caller)
    let callback = read_strong_binder(in_parcel)?;
    // pkgName, identity: String x2
    let _pkg_name = read_string(in_parcel)?;
    let _identity = read_string(in_parcel)?;
    // otherData: byte[]
    let _other_data = read_byte_array(in_parcel)?;
    // Tolerant skip to the end of the parcelable (a newer client may write
    // trailing fields the reader does not know).
    finish_parcelable(in_parcel, p_start, p_size)?;
    Ok((items, callback))
}

/// Applies every entry of a control request; returns the count applied.
fn apply_request(items: &[RequestItem]) -> usize {
    let mut applied = 0usize;
    for item in items {
        log(
            LOG_LEVEL_INFO,
            LOG_TAG,
            &format!("urcc ctl: id=0x{:x} path={} value={}", item.res_id, item.res_path, item.res_value),
        );
        if item.res_value.is_empty() {
            continue;
        }
        // The UAH client drives the core by node path as well as by resource
        // id: liboplus-uah-client.so's urcc_resctl_request_by_path() puts the
        // node path in res_Path.
        if item.res_path.starts_with('/') {
            if UrccCore::write_node(&item.res_path, &item.res_value) {
                applied += 1;
            } else {
                log(LOG_LEVEL_INFO, LOG_TAG, &format!("urcc write {} failed", item.res_path));
            }
            continue;
        }
        match UrccCore::apply(item.res_id as u32, &item.res_value) {
            Ok(()) => applied += 1,
            Err(e) => log(
                LOG_LEVEL_INFO,
                LOG_TAG,
                &format!("urcc apply {} failed: {}", UrccCore::name_of(item.res_id as u32), e),
            ),
        }
    }
    applied
}

fn dispatch_res_ctl_request(in_parcel: *const AParcel, out: *mut AParcel) -> DispatchResult {
    let request = read_request_parcel(in_parcel)?;
    // The handle is 0 for an accepted request. A resource with no backing
    // node is skipped, not an error: the framework treats a non-zero return
    // as a failed request.
    apply_request(&request.items);
    write_ok_status(out)?;
    write_i32(out, 0)
}

fn dispatch_res_ctl_release(svc: &Service, in_parcel: *const AParcel, out: *mut AParcel) -> DispatchResult {
    let handle = read_i32(in_parcel)?;
    let removed = { svc.res_callbacks.lock().unwrap().remove(&handle) };
    drop(removed);
    write_ok_status(out)?;
    write_i32(out, 0)
}

fn dispatch_res_state_request(in_parcel: *const AParcel, out: *mut AParcel) -> DispatchResult {
    let request = read_request_parcel(in_parcel)?;
    let mut results: Vec<urcc_service::core::ResState> = Vec::new();
    for item in &request.items {
        log(LOG_LEVEL_INFO, LOG_TAG, &format!("urcc state: id=0x{:x} path={}", item.res_id, item.res_path));
        // A state request may address a node by path instead of by resource id.
        if item.res_path.starts_with('/') {
            results.push(urcc_service::core::ResState {
                res_id: item.res_id,
                event_id: 0,
                res_value: UrccCore::read_node(&item.res_path),
                res_path: "0".to_string(),
                desc: item.res_path.clone(),
            });
        } else {
            results.push(fill_state(item.res_id as u32));
        }
    }
    // A request without an explicit resource list is answered with the sampled
    // telemetry, which is what the performance gauge consumes.
    if results.is_empty() {
        results = sample_once();
    }
    write_ok_status(out)?;
    write_parcelable_array(out, &results, |item| {
        // Each UrccRequestData is a versioned parcelable.
        let start = begin_parcelable(out)?;
        write_i32(out, item.res_id)?;
        write_i32(out, item.event_id)?;
        write_string(out, &item.res_value)?;
        write_string(out, &item.res_path)?;
        write_string(out, &item.desc)?;
        end_parcelable(out, start)
    })
}

fn dispatch_res_listening_register(svc: &Service, in_parcel: *const AParcel, out: *mut AParcel) -> DispatchResult {
    let (_items, callback) = read_request_parcel_with_callback(in_parcel)?;
    if !svc.associate_callback(callback) {
        // Not a usable callback; release the reference and answer -1.
        if !callback.is_null() {
            // SAFETY: read_strong_binder handed us exactly one strong reference.
            unsafe { AIBinder_decStrong(callback) };
        }
        write_ok_status(out)?;
        return write_i32(out, -1);
    }
    let handle = svc.next_handle();
    svc.res_callbacks.lock().unwrap().insert(handle, CallbackRef(callback));
    write_ok_status(out)?;
    write_i32(out, handle)
}

fn dispatch_res_listening_unregister(svc: &Service, in_parcel: *const AParcel, out: *mut AParcel) -> DispatchResult {
    let handle = read_i32(in_parcel)?;
    let removed = { svc.res_callbacks.lock().unwrap().remove(&handle) };
    drop(removed);
    write_ok_status(out)?;
    write_i32(out, 0)
}

fn dispatch_property_get(in_parcel: *const AParcel, out: *mut AParcel) -> DispatchResult {
    let key = read_string(in_parcel)?.unwrap_or_default();
    // urccPropertyGet is property_get(key, buf, NULL): a real system-property
    // read, not an in-memory store.
    let value = io::property_get(&key).unwrap_or_default();
    log(LOG_LEVEL_INFO, LOG_TAG, &format!("Urcc::urccPropertyGet key={} -> {}", key, value));
    write_ok_status(out)?;
    write_string(out, &value)
}

fn dispatch_property_set(in_parcel: *const AParcel, out: *mut AParcel) -> DispatchResult {
    let key = read_string(in_parcel)?.unwrap_or_default();
    let value = read_string(in_parcel)?.unwrap_or_default();
    let ok = io::property_set(&key, &value);
    // Debug keys are special-cased after the write
    // (urccUpdateZdbDebugFlag / urccUpdateUnitTestFlag).
    if key == "vendor.urcc.zdbdebug_on" || key == "vendor.urcc.unittest_on" {
        log(LOG_LEVEL_INFO, LOG_TAG, &format!("urcc debug prop {} set to {}", key, value));
    }
    write_ok_status(out)?;
    write_i32(out, if ok { 0 } else { -1 })
}

fn dispatch_thermal_listening_register(svc: &Service, in_parcel: *const AParcel, out: *mut AParcel) -> DispatchResult {
    let _types = read_int32_array(in_parcel)?;
    let callback = read_strong_binder(in_parcel)?;
    if !svc.associate_callback(callback) {
        if !callback.is_null() {
            // SAFETY: read_strong_binder handed us exactly one strong reference.
            unsafe { AIBinder_decStrong(callback) };
        }
        write_ok_status(out)?;
        return write_i32(out, -1);
    }
    let handle = svc.next_handle();
    svc.thermal_callbacks.lock().unwrap().insert(handle, CallbackRef(callback));
    write_ok_status(out)?;
    write_i32(out, handle)
}

fn dispatch_thermal_listening_unregister(svc: &Service, in_parcel: *const AParcel, out: *mut AParcel) -> DispatchResult {
    let handle = read_i32(in_parcel)?;
    let removed = { svc.thermal_callbacks.lock().unwrap().remove(&handle) };
    drop(removed);
    write_ok_status(out)?;
    write_i32(out, 0)
}

fn dispatch_uah_notify_ext(in_parcel: *const AParcel, out: *mut AParcel) -> DispatchResult {
    let _src = read_i32(in_parcel)?;
    let _type = read_i32(in_parcel)?;
    let _data = read_int32_array(in_parcel)?;
    write_ok_status(out)?;
    write_i32(out, 0)
}

fn dispatch_set_related_sys_info(in_parcel: *const AParcel, out: *mut AParcel) -> DispatchResult {
    let _type = read_i32(in_parcel)?;
    let _info = read_byte_array(in_parcel)?;
    // Both arguments are ignored and 0 is returned.
    write_ok_status(out)?;
    write_i32(out, 0)
}

fn dispatch_rule_ctl(in_parcel: *const AParcel, out: *mut AParcel) -> DispatchResult {
    let _type = read_i32(in_parcel)?;
    let _rule = read_i32(in_parcel)?;
    let len = read_i32(in_parcel)?.max(0);
    let mut items = Vec::with_capacity(len as usize);
    for _ in 0..len {
        let (i_start, i_size) = read_parcelable_header(in_parcel)?;
        let res_id = read_i32(in_parcel)?;
        let event_id = read_i32(in_parcel)?;
        let res_value = read_string(in_parcel)?.unwrap_or_default();
        let res_path = read_string(in_parcel)?.unwrap_or_default();
        let desc = read_string(in_parcel)?.unwrap_or_default();
        finish_parcelable(in_parcel, i_start, i_size)?;
        items.push(RequestItem { res_id, event_id, res_value, res_path, _desc: desc });
    }
    apply_request(&items);
    write_ok_status(out)?;
    write_i32(out, 0)
}

fn dispatch_res_ctl_request_bypass(in_parcel: *const AParcel, out: *mut AParcel) -> DispatchResult {
    let request = read_request_parcel(in_parcel)?;
    apply_request(&request.items);
    write_ok_status(out)
}

#[allow(dead_code)]
fn _ptr_unused() {
    let _ = ptr::null::<u8>();
}
