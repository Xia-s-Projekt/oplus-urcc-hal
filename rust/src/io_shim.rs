// File and property plumbing for the URCC HAL.
//
// sysfs reads/writes go straight through std::fs; the property API mirrors
// the shipped liburcccore.so behaviour, where urccPropertyGet/urccPropertySet
// are thin wrappers over libcutils property_get/property_set (verified at the
// instruction level in the shipped binary: urccPropertyGet at 0x5b470 calls
// property_get(key, value, NULL), urccPropertySet at 0x63f5c calls
// property_set then special-cases vendor.urcc.zdbdebug_on /
// vendor.urcc.unittest_on). On Android bionic's __system_property_get /
// __system_property_set are called directly, so no DT_NEEDED for libcutils is
// needed. Off-target builds get an in-memory store so the core logic can be
// unit-tested on the host.

#[cfg(target_os = "android")]
use std::ffi::CString;

use std::fs;
use std::path::Path;

pub fn read_file<P: AsRef<Path>>(path: P) -> Option<String> {
    let out = fs::read_to_string(path).ok()?;
    Some(out.trim_end_matches(['\n', '\r']).to_string())
}

pub fn write_file<P: AsRef<Path>>(path: P, value: &str) -> bool {
    fs::write(path, value).is_ok()
}

pub fn read_int<P: AsRef<Path>>(path: P) -> Option<i64> {
    let text = read_file(path)?;
    text.trim().parse::<i64>().ok()
}

#[cfg(target_os = "android")]
pub fn property_get(key: &str) -> Option<String> {
    use std::ffi::CStr;
    use std::os::raw::c_char;
    extern "C" {
        fn __system_property_get(name: *const c_char, value: *mut c_char) -> i32;
    }

    const PROP_VALUE_MAX: usize = 92;

    let name = CString::new(key).ok()?;
    let mut buf = vec![0u8; PROP_VALUE_MAX];
    // SAFETY: name is a valid NUL-terminated C string, buf is PROP_VALUE_MAX
    // bytes as bionic's contract requires.
    let n = unsafe { __system_property_get(name.as_ptr(), buf.as_mut_ptr() as *mut c_char) };
    if n <= 0 {
        return None;
    }
    let value = CStr::from_bytes_with_nul(&buf[..=n as usize]).ok()?;
    Some(value.to_string_lossy().into_owned())
}

#[cfg(target_os = "android")]
pub fn property_set(key: &str, value: &str) -> bool {
    use std::os::raw::c_char;

    extern "C" {
        fn __system_property_set(name: *const c_char, value: *const c_char) -> i32;
    }

    let (name, val) = match (CString::new(key), CString::new(value)) {
        (Ok(n), Ok(v)) => (n, v),
        _ => return false,
    };
    // SAFETY: both are valid NUL-terminated C strings.
    unsafe { __system_property_set(name.as_ptr(), val.as_ptr()) == 0 }
}

// Host stub for unit tests.

#[cfg(not(target_os = "android"))]
pub fn property_get(key: &str) -> Option<String> {
    with_props(|p| p.get(key).cloned())
}

#[cfg(not(target_os = "android"))]
pub fn property_set(key: &str, value: &str) -> bool {
    with_props(|p| {
        p.insert(key.to_string(), value.to_string());
        true
    })
}

#[cfg(not(target_os = "android"))]
fn with_props<R>(f: impl FnOnce(&mut std::collections::HashMap<String, String>) -> R) -> R {
    use std::sync::Mutex;
    static PROPS: Mutex<Option<std::collections::HashMap<String, String>>> = Mutex::new(None);
    let mut store = PROPS.lock().unwrap();
    if store.is_none() {
        *store = Some(std::collections::HashMap::new());
    }
    f(store.as_mut().unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_write_roundtrip() {
        let dir = std::env::temp_dir().join("urcc_io_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("node");
        assert!(write_file(&path, "12345"));
        assert_eq!(read_file(&path).unwrap(), "12345");
        assert_eq!(read_int(&path).unwrap(), 12345);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn missing_file_is_none() {
        assert!(read_file("/this/does/not/exist/urcc").is_none());
        assert!(read_int("/this/does/not/exist/urcc").is_none());
    }

    #[cfg(not(target_os = "android"))]
    #[test]
    fn host_property_roundtrip() {
        assert!(property_get("urcc.test.key").is_none());
        assert!(property_set("urcc.test.key", "value1"));
        assert_eq!(property_get("urcc.test.key").unwrap(), "value1");
    }
}
