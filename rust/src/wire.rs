// Raw FFI to libbinder_ndk (the stable C API) and parcel helpers.
//
// The AIDL transport is implemented by hand here instead of through generated
// C++ code: the interface is fixed (vendor.oplus.hardware.urcc.IUrcc, version
// 1, hash cae43655eaf0af39cbb6d273c5e2206eebe33a10, 14 methods numbered 1..14
// plus the reserved pair getInterfaceVersion@0xffffff /
// getInterfaceHash@0xfffffe that this ROM's Java stub uses), so the wire
// format is fully known and the NDK C API is a stable C ABI that Rust can
// bind directly.
//
// Wire format (matches what the generated ndk backend and the framework's
// Java stub produce):
//   * incoming transactions arrive with the interface token already consumed
//     (enforced by libbinder before onTransact is called),
//   * every reply starts with a status header (an int32 exception code, 0 for
//     OK) followed by the out-parameters in declaration order,
//   * parcelables are written as their fields back to back; arrays as an
//     int32 length followed by the elements; strings as int32 length (UTF-8
//     bytes, -1 for null) + bytes + NUL; strong binders inline.

#![allow(dead_code)]

use std::ffi::{c_char, c_void, CString};
use std::os::raw::c_int;
use std::ptr;

#[repr(C)]
pub struct AIBinder {
    _private: [u8; 0],
}
#[repr(C)]
pub struct AIBinder_Class {
    _private: [u8; 0],
}
#[repr(C)]
pub struct AParcel {
    _private: [u8; 0],
}
#[repr(C)]
pub struct AStatus {
    _private: [u8; 0],
}

pub type BinderStatus = c_int;
pub type TransactionCode = u32;

pub const STATUS_OK: BinderStatus = 0;

// The reserved transaction codes this ROM's framework stub uses (the pair
// recovered from IUrcc$Stub.smali: TRANSACTIONS 1..14 for the methods,
// 16777215 = getInterfaceVersion, 16777214 = getInterfaceHash).
pub const FIRST_CALL_TRANSACTION: TransactionCode = 1;
pub const TRANSACTION_GET_INTERFACE_VERSION: TransactionCode = 16777215;
pub const TRANSACTION_GET_INTERFACE_HASH: TransactionCode = 16777214;

pub const LOG_LEVEL_INFO: c_int = 4;
pub const LOG_LEVEL_ERROR: c_int = 6;

pub const PROP_VALUE_MAX: usize = 92;

pub type OnCreate = unsafe extern "C" fn(args: *mut c_void) -> *mut c_void;
pub type OnDestroy = unsafe extern "C" fn(user_data: *mut c_void);
pub type OnTransact = unsafe extern "C" fn(
    binder: *mut AIBinder,
    code: TransactionCode,
    in_parcel: *const AParcel,
    out_parcel: *mut AParcel,
) -> BinderStatus;

extern "C" {
    // android/binder_ibinder.h
    pub fn AIBinder_Class_define(
        interface_descriptor: *const c_char,
        on_create: OnCreate,
        on_destroy: OnDestroy,
        on_transact: OnTransact,
    ) -> *mut AIBinder_Class;
    pub fn AIBinder_new(class: *mut AIBinder_Class, args: *mut c_void) -> *mut AIBinder;
    pub fn AIBinder_associateClass(binder: *mut AIBinder, clazz: *const AIBinder_Class) -> bool;
    pub fn AIBinder_getUserData(binder: *mut AIBinder) -> *mut c_void;
    pub fn AIBinder_prepareTransaction(binder: *mut AIBinder, in_parcel: *mut *mut AParcel) -> BinderStatus;
    pub fn AIBinder_transact(
        binder: *mut AIBinder,
        code: TransactionCode,
        in_parcel: *mut *mut AParcel,
        out_parcel: *mut *mut AParcel,
        flags: u32,
    ) -> BinderStatus;
    pub fn AIBinder_incStrong(binder: *mut AIBinder);
    pub fn AIBinder_decStrong(binder: *mut AIBinder);

    // android/binder_manager.h (platform header)
    pub fn AServiceManager_addService(binder: *mut AIBinder, instance: *const c_char) -> BinderStatus;

    // android/binder_process.h (platform header)
    pub fn ABinderProcess_setThreadPoolMaxThreadCount(num_threads: u32) -> bool;
    pub fn ABinderProcess_startThreadPool();
    pub fn ABinderProcess_joinThreadPool();

    // android/binder_parcel.h
    pub fn AParcel_delete(parcel: *mut AParcel);
    pub fn AParcel_writeInt32(parcel: *mut AParcel, value: i32) -> BinderStatus;
    pub fn AParcel_readInt32(parcel: *const AParcel, value: *mut i32) -> BinderStatus;
    pub fn AParcel_writeBool(parcel: *mut AParcel, value: bool) -> BinderStatus;
    pub fn AParcel_readBool(parcel: *const AParcel, value: *mut bool) -> BinderStatus;
    pub fn AParcel_writeInt64(parcel: *mut AParcel, value: i64) -> BinderStatus;
    pub fn AParcel_readInt64(parcel: *const AParcel, value: *mut i64) -> BinderStatus;
    pub fn AParcel_writeString(parcel: *mut AParcel, string: *const c_char, length: i32) -> BinderStatus;
    pub fn AParcel_readString(
        parcel: *const AParcel,
        string_data: *mut c_void,
        allocator: Option<unsafe extern "C" fn(*mut c_void, i32, *mut *mut c_char) -> bool>,
    ) -> BinderStatus;
    pub fn AParcel_readByteArray(
        parcel: *const AParcel,
        array_data: *mut c_void,
        allocator: Option<unsafe extern "C" fn(*mut c_void, i32, *mut *mut i8) -> bool>,
    ) -> BinderStatus;
    pub fn AParcel_writeByteArray(parcel: *mut AParcel, array_data: *const i8, length: i32) -> BinderStatus;
    pub fn AParcel_readInt32Array(
        parcel: *const AParcel,
        array_data: *mut c_void,
        allocator: Option<unsafe extern "C" fn(*mut c_void, i32, *mut *mut i32) -> bool>,
    ) -> BinderStatus;
    pub fn AParcel_writeStrongBinder(parcel: *mut AParcel, binder: *mut AIBinder) -> BinderStatus;
    pub fn AParcel_readStrongBinder(parcel: *const AParcel, binder: *mut *mut AIBinder) -> BinderStatus;
    pub fn AParcel_writeStatusHeader(parcel: *mut AParcel, status: *const AStatus) -> BinderStatus;
    pub fn AParcel_readStatusHeader(parcel: *const AParcel, status: *mut *mut AStatus) -> BinderStatus;
    pub fn AParcel_getDataPosition(parcel: *const AParcel) -> i32;
    pub fn AParcel_setDataPosition(parcel: *const AParcel, position: i32) -> BinderStatus;
    pub fn AParcel_getDataSize(parcel: *const AParcel) -> i32;

    // android/binder_status.h
    pub fn AStatus_newOk() -> *mut AStatus;
    pub fn AStatus_delete(status: *mut AStatus);
    pub fn AStatus_isOk(status: *const AStatus) -> bool;

    // android/log.h
    pub fn __android_log_print(prio: c_int, tag: *const c_char, fmt: *const c_char, ...) -> c_int;
}

pub fn log(level: c_int, tag: &str, msg: &str) {
    let tag = match CString::new(tag) {
        Ok(t) => t,
        Err(_) => return,
    };
    let fmt = CString::new("%s").unwrap();
    let msg = match CString::new(msg) {
        Ok(m) => m,
        Err(_) => return,
    };
    // SAFETY: all three strings are valid NUL-terminated C strings.
    unsafe {
        __android_log_print(level, tag.as_ptr(), fmt.as_ptr(), msg.as_ptr());
    }
}

// Versioned-parcelable wrapper.
//
// This AIDL generation writes every parcelable in the versioned format. For a
// non-null parcelable the layout is (see AParcel_writeParcelable /
// AParcel_readParcelable in binder_parcel_utils.h):
//
//   int32 1        non-null marker
//   int32 size     total bytes of the parcelable INCLUDING this size field
//                  (written as 0 and patched once the fields are out, see the
//                  generated UrccRequestData::writeToParcel)
//   ... fields ...
//
// Both directions have to honour it: a missing marker or size on write makes
// the generated client treat the first field as the marker/size and fail, and
// a missing marker on read shifts every following field.

pub fn get_pos(parcel: *const AParcel) -> i32 {
    // SAFETY: read-only query of a valid parcel.
    unsafe { AParcel_getDataPosition(parcel) }
}

pub fn set_pos(parcel: *const AParcel, pos: i32) -> Result<(), BinderStatus> {
    // SAFETY: positions used here come from get_pos on the same parcel.
    let rc = unsafe { AParcel_setDataPosition(parcel, pos) };
    if rc == STATUS_OK { Ok(()) } else { Err(rc) }
}

/// Writes the non-null marker and the size placeholder. Call before the
/// fields and keep the returned start position for end_parcelable.
pub fn begin_parcelable(out: *mut AParcel) -> Result<i32, BinderStatus> {
    write_i32(out, 1)?; // non-null marker
    let start = get_pos(out);
    write_i32(out, 0)?; // size placeholder, patched by end_parcelable
    Ok(start)
}

/// Patches the placeholder written by begin_parcelable with the total size
/// (end - start, including the size field itself) and leaves the position at
/// the end.
pub fn end_parcelable(out: *mut AParcel, start: i32) -> Result<(), BinderStatus> {
    let end = get_pos(out);
    set_pos(out, start)?;
    write_i32(out, end - start)?;
    set_pos(out, end)
}

/// Reads the non-null marker and the versioned size header. Returns
/// (start, size) where start is the position of the size field itself; the
/// caller reads the fields and then jumps to start + size with
/// finish_parcelable (tolerant: a larger parcelable written by a newer client
/// is skipped at the right offset).
pub fn read_parcelable_header(in_parcel: *const AParcel) -> Result<(i32, i32), BinderStatus> {
    let marker = read_i32(in_parcel)?;
    if marker == 0 {
        return Err(-29); // STATUS_UNEXPECTED_NULL, mirroring the generated reader
    }
    let start = get_pos(in_parcel);
    let size = read_i32(in_parcel)?;
    if size < 4 {
        return Err(-22); // STATUS_BAD_VALUE, mirroring the generated reader
    }
    Ok((start, size))
}

pub fn finish_parcelable(in_parcel: *const AParcel, start: i32, size: i32) -> Result<(), BinderStatus> {
    set_pos(in_parcel, start + size)
}

// Write helpers.

pub fn write_ok_status(out: *mut AParcel) -> Result<(), BinderStatus> {
    // SAFETY: out is a valid reply parcel from onTransact.
    let status = unsafe { AStatus_newOk() };
    if status.is_null() {
        return Err(STATUS_OK - 1);
    }
    let rc = unsafe { AParcel_writeStatusHeader(out, status) };
    unsafe { AStatus_delete(status) };
    if rc == STATUS_OK {
        Ok(())
    } else {
        Err(rc)
    }
}

pub fn write_i32(out: *mut AParcel, value: i32) -> Result<(), BinderStatus> {
    let rc = unsafe { AParcel_writeInt32(out, value) };
    if rc == STATUS_OK { Ok(()) } else { Err(rc) }
}

pub fn write_i64(out: *mut AParcel, value: i64) -> Result<(), BinderStatus> {
    let rc = unsafe { AParcel_writeInt64(out, value) };
    if rc == STATUS_OK { Ok(()) } else { Err(rc) }
}

pub fn write_bool(out: *mut AParcel, value: bool) -> Result<(), BinderStatus> {
    let rc = unsafe { AParcel_writeBool(out, value) };
    if rc == STATUS_OK { Ok(()) } else { Err(rc) }
}

pub fn write_byte_array(out: *mut AParcel, values: &[u8]) -> Result<(), BinderStatus> {
    let rc = unsafe { AParcel_writeByteArray(out, values.as_ptr() as *const i8, values.len() as i32) };
    if rc == STATUS_OK { Ok(()) } else { Err(rc) }
}

pub fn write_string(out: *mut AParcel, value: &str) -> Result<(), BinderStatus> {    let cstr = match CString::new(value) {
        Ok(c) => c,
        Err(_) => return Err(STATUS_OK - 2), // STATUS_BAD_VALUE
    };
    let rc = unsafe { AParcel_writeString(out, cstr.as_ptr(), cstr.as_bytes().len() as i32) };
    if rc == STATUS_OK { Ok(()) } else { Err(rc) }
}

/// Writes a parcelable array as int32 length + elements written by `element`.
pub fn write_parcelable_array<F>(
    out: *mut AParcel,
    items: &[urcc_service::core::ResState],
    mut element: F,
) -> Result<(), BinderStatus>
where
    F: FnMut(&urcc_service::core::ResState) -> Result<(), BinderStatus>,
{
    write_i32(out, items.len() as i32)?;
    for item in items {
        element(item)?;
    }
    Ok(())
}

// Read helpers.

pub fn read_i32(in_parcel: *const AParcel) -> Result<i32, BinderStatus> {
    let mut value: i32 = 0;
    let rc = unsafe { AParcel_readInt32(in_parcel, &mut value) };
    if rc == STATUS_OK { Ok(value) } else { Err(rc) }
}

pub fn read_i64(in_parcel: *const AParcel) -> Result<i64, BinderStatus> {
    let mut value: i64 = 0;
    let rc = unsafe { AParcel_readInt64(in_parcel, &mut value) };
    if rc == STATUS_OK { Ok(value) } else { Err(rc) }
}

pub fn read_bool(in_parcel: *const AParcel) -> Result<bool, BinderStatus> {
    let mut value: bool = false;
    let rc = unsafe { AParcel_readBool(in_parcel, &mut value) };
    if rc == STATUS_OK { Ok(value) } else { Err(rc) }
}

struct StringCtx {
    buf: *mut u8,
    len: i32,
}

impl Drop for StringCtx {
    fn drop(&mut self) {
        if !self.buf.is_null() {
            unsafe {
                let layout = std::alloc::Layout::array::<u8>(self.len.max(0) as usize).unwrap_unchecked();
                std::alloc::dealloc(self.buf, layout);
            }
        }
    }
}

unsafe extern "C" fn string_allocator(ctx: *mut c_void, len: i32, out_buffer: *mut *mut c_char) -> bool {
    let ctx = &mut *(ctx as *mut StringCtx);
    if len < 0 {
        // A 'null' string: nothing to allocate.
        ctx.len = -1;
        ctx.buf = ptr::null_mut();
        return true;
    }
    if len == 0 {
        // Never expected: AParcel_readString passes a length that includes the
        // NUL terminator, so a real string is >= 1 and a null string is -1
        // (see AParcel_stringAllocator in binder_parcel.h). Accepted as
        // success with no buffer; read_string() then reports None.
        ctx.len = 0;
        ctx.buf = ptr::null_mut();
        return true;
    }
    let layout = match std::alloc::Layout::array::<u8>(len as usize) {
        Ok(l) => l,
        Err(_) => return false,
    };
    let p = std::alloc::alloc(layout);
    if p.is_null() {
        return false;
    }
    ctx.buf = p;
    ctx.len = len;
    *out_buffer = p as *mut c_char;
    true
}

/// Reads a UTF-8 string; a null string comes back as None.
pub fn read_string(in_parcel: *const AParcel) -> Result<Option<String>, BinderStatus> {
    let mut ctx = StringCtx { buf: ptr::null_mut(), len: 0 };
    let rc = unsafe {
        AParcel_readString(
            in_parcel,
            &mut ctx as *mut StringCtx as *mut c_void,
            Some(string_allocator),
        )
    };
    if rc != STATUS_OK {
        return Err(rc);
    }
    if ctx.len < 0 || ctx.buf.is_null() {
        return Ok(None);
    }
    // The buffer length includes the NUL terminator written by libbinder.
    let bytes = unsafe { std::slice::from_raw_parts(ctx.buf, ctx.len as usize) };
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    let value = String::from_utf8_lossy(&bytes[..end]).into_owned();
    Ok(Some(value))
}

struct BytesCtx {
    buf: *mut u8,
    len: i32,
}

impl Drop for BytesCtx {
    fn drop(&mut self) {
        if !self.buf.is_null() {
            unsafe {
                let layout = std::alloc::Layout::array::<u8>(self.len.max(0) as usize).unwrap_unchecked();
                std::alloc::dealloc(self.buf, layout);
            }
        }
    }
}

unsafe extern "C" fn byte_array_allocator(ctx: *mut c_void, len: i32, out_buffer: *mut *mut i8) -> bool {
    let ctx = &mut *(ctx as *mut BytesCtx);
    if len <= 0 {
        ctx.len = 0;
        ctx.buf = ptr::null_mut();
        return true;
    }
    let layout = match std::alloc::Layout::array::<u8>(len as usize) {
        Ok(l) => l,
        Err(_) => return false,
    };
    let p = std::alloc::alloc(layout);
    if p.is_null() {
        return false;
    }
    ctx.buf = p;
    ctx.len = len;
    *out_buffer = p as *mut i8;
    true
}

/// Reads a byte array (AIDL `byte[]`).
pub fn read_byte_array(in_parcel: *const AParcel) -> Result<Vec<u8>, BinderStatus> {
    let mut ctx = BytesCtx { buf: ptr::null_mut(), len: 0 };
    let rc = unsafe {
        AParcel_readByteArray(
            in_parcel,
            &mut ctx as *mut BytesCtx as *mut c_void,
            Some(byte_array_allocator),
        )
    };
    if rc != STATUS_OK {
        return Err(rc);
    }
    if ctx.buf.is_null() {
        return Ok(Vec::new());
    }
    let bytes = unsafe { std::slice::from_raw_parts(ctx.buf, ctx.len as usize) }.to_vec();
    Ok(bytes)
}

struct Int32ArrayCtx {
    buf: *mut i32,
    len: i32,
}

impl Drop for Int32ArrayCtx {
    fn drop(&mut self) {
        if !self.buf.is_null() {
            unsafe {
                let layout = std::alloc::Layout::array::<i32>(self.len.max(0) as usize).unwrap_unchecked();
                std::alloc::dealloc(self.buf as *mut u8, layout);
            }
        }
    }
}

unsafe extern "C" fn int32_array_allocator(ctx: *mut c_void, len: i32, out_buffer: *mut *mut i32) -> bool {
    let ctx = &mut *(ctx as *mut Int32ArrayCtx);
    if len <= 0 {
        ctx.len = 0;
        ctx.buf = ptr::null_mut();
        return true;
    }
    let layout = match std::alloc::Layout::array::<i32>(len as usize) {
        Ok(l) => l,
        Err(_) => return false,
    };
    let p = std::alloc::alloc(layout) as *mut i32;
    if p.is_null() {
        return false;
    }
    ctx.buf = p;
    ctx.len = len;
    *out_buffer = p;
    true
}

/// Reads an int32 array (AIDL `int[]`).
pub fn read_int32_array(in_parcel: *const AParcel) -> Result<Vec<i32>, BinderStatus> {
    let mut ctx = Int32ArrayCtx { buf: ptr::null_mut(), len: 0 };
    let rc = unsafe {
        AParcel_readInt32Array(
            in_parcel,
            &mut ctx as *mut Int32ArrayCtx as *mut c_void,
            Some(int32_array_allocator),
        )
    };
    if rc != STATUS_OK {
        return Err(rc);
    }
    if ctx.buf.is_null() {
        return Ok(Vec::new());
    }
    let values = unsafe { std::slice::from_raw_parts(ctx.buf, ctx.len as usize) }.to_vec();
    Ok(values)
}

/// Reads a string array (AIDL `String[]`): int32 length + strings.
pub fn read_string_array(in_parcel: *const AParcel) -> Result<Vec<String>, BinderStatus> {
    let len = read_i32(in_parcel)?;
    let mut out = Vec::with_capacity(len.max(0) as usize);
    for _ in 0..len.max(0) {
        out.push(read_string(in_parcel)?.unwrap_or_default());
    }
    Ok(out)
}

// Callback invocation.

/// Calls IUrccCallback::onCallback (transaction code 1) with a set of
/// resource states. Returns false when the recipient is dead so the caller
/// can drop the registration.
pub fn push_callback_event(callback: *mut AIBinder, items: &[urcc_service::core::ResState]) -> bool {
    if callback.is_null() {
        return false;
    }
    let mut in_parcel: *mut AParcel = ptr::null_mut();
    // SAFETY: callback is a live AIBinder we hold a strong reference to.
    let rc = unsafe { AIBinder_prepareTransaction(callback, &mut in_parcel) };
    if rc != STATUS_OK || in_parcel.is_null() {
        // A transient failure (the binder is remote but not class-associated,
        // or a parcel allocation failure): keep the registration and let the
        // next sample retry.
        return true;
    }
    // The array: int32 length + one versioned UrccRequestData per element.
    let mut ok = write_i32(in_parcel, items.len() as i32).is_ok();
    if ok {
        for item in items {
            ok = write_one_request_data(in_parcel, item);
            if !ok {
                break;
            }
        }
    }
    if !ok {
        unsafe { AParcel_delete(in_parcel) };
        return false;
    }
    let mut out_parcel: *mut AParcel = ptr::null_mut();
    // SAFETY: in_parcel is the parcel from prepareTransaction; transact takes
    // ownership of it and hands back out_parcel.
    let rc = unsafe { AIBinder_transact(callback, FIRST_CALL_TRANSACTION, &mut in_parcel, &mut out_parcel, 0) };
    if !out_parcel.is_null() {
        unsafe { AParcel_delete(out_parcel) };
    }
    // STATUS_DEAD_OBJECT (-32) or a failed transaction: recipient is gone.
    rc != -32
}

pub fn data_size(parcel: *const AParcel) -> i32 {
    // SAFETY: read-only query of a valid parcel.
    unsafe { AParcel_getDataSize(parcel) }
}

/// Reads a strong binder (one strong reference is passed to the caller).
pub fn read_strong_binder(in_parcel: *const AParcel) -> Result<*mut AIBinder, BinderStatus> {
    let mut binder: *mut AIBinder = ptr::null_mut();
    let rc = unsafe { AParcel_readStrongBinder(in_parcel, &mut binder) };
    if rc == STATUS_OK { Ok(binder) } else { Err(rc) }
}

/// Writes one UrccRequestData in the versioned-parcelable format:
/// int32 size, int32 res_Id, int32 event_Id, String res_Value, String
/// res_Path, String desc.
pub fn write_one_request_data(out: *mut AParcel, item: &urcc_service::core::ResState) -> bool {
    let start = match begin_parcelable(out) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let ok = write_i32(out, item.res_id).is_ok()
        && write_i32(out, item.event_id).is_ok()
        && write_string(out, &item.res_value).is_ok()
        && write_string(out, &item.res_path).is_ok()
        && write_string(out, &item.desc).is_ok();
    if !ok {
        return false;
    }
    end_parcelable(out, start).is_ok()
}

// Unused on host builds.
#[allow(dead_code)]
fn _unused() {}
