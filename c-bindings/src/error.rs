use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;

/// Opaque error type for C callers.
#[repr(C)]
pub struct cspot_error_t;

struct ErrorHandle {
    message: CString,
}

pub(crate) fn cstring_from_str_lossy(value: &str) -> CString {
    if value.as_bytes().contains(&0) {
        let sanitized: String = value.chars().map(|c| if c == '\0' { ' ' } else { c }).collect();
        CString::new(sanitized).unwrap_or_else(|_| CString::new("invalid utf-8").unwrap())
    } else {
        CString::new(value).unwrap_or_else(|_| CString::new("invalid utf-8").unwrap())
    }
}

pub(crate) fn clear_error(out_error: *mut *mut cspot_error_t) {
    if !out_error.is_null() {
        // Safety: caller provided a valid out_error pointer.
        unsafe {
            *out_error = ptr::null_mut();
        }
    }
}

pub(crate) fn write_error(out_error: *mut *mut cspot_error_t, message: impl Into<String>) {
    if out_error.is_null() {
        return;
    }
    let cstring = cstring_from_str_lossy(&message.into());
    let handle = Box::new(ErrorHandle { message: cstring });
    // Safety: out_error is non-null and points to writable memory.
    unsafe {
        *out_error = Box::into_raw(handle) as *mut cspot_error_t;
    }
}

/// Returns the message for an error allocated by cspot.
///
/// The returned pointer is valid as long as the error handle is alive.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_error_message(error: *const cspot_error_t) -> *const c_char {
    if error.is_null() {
        return ptr::null();
    }
    // Safety: error must be a valid handle allocated by cspot.
    let handle = unsafe { &*(error as *const ErrorHandle) };
    handle.message.as_ptr()
}

/// Frees an error returned by cspot.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_error_free(error: *mut cspot_error_t) {
    if error.is_null() {
        return;
    }
    // Safety: error must be a valid handle allocated by cspot.
    unsafe {
        drop(Box::from_raw(error as *mut ErrorHandle));
    }
}

/// Frees a string allocated by cspot.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_string_free(value: *mut c_char) {
    if value.is_null() {
        return;
    }
    // Safety: value must be a string allocated by cspot_string_free-compatible APIs.
    unsafe {
        drop(CString::from_raw(value));
    }
}
