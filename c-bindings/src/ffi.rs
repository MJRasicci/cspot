//! Shared utilities for cspot's C FFI bindings.

use std::ffi::CStr;
use std::os::raw::c_char;

use crate::error::{cspot_error_t, write_error};

pub(crate) fn read_cstr(
    value: *const c_char,
    field: &'static str,
    out_error: *mut *mut cspot_error_t,
) -> Option<String> {
    if value.is_null() {
        write_error(out_error, format!("{field} was null"));
        return None;
    }
    // Safety: caller guarantees a valid, NUL-terminated C string.
    let cstr = unsafe { CStr::from_ptr(value) };
    Some(cstr.to_string_lossy().into_owned())
}
