//! C bindings for librespot session setup.

use std::ffi::CString;
use std::os::raw::c_char;
use std::panic::AssertUnwindSafe;
use std::ptr;

use librespot::core::{config::SessionConfig, session::Session};

use crate::error::{clear_error, cspot_error_t, cstring_from_str_lossy, write_error};
use crate::ffi::read_cstr;
use crate::runtime::runtime;

/// Opaque session handle for C callers.
#[repr(C)]
pub struct cspot_session_t;

struct SessionHandle {
    session: Session,
}

/// Creates a new session using the provided device id.
///
/// The returned handle must be released with `cspot_session_free`.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_session_create(
    device_id: *const c_char,
    out_error: *mut *mut cspot_error_t,
) -> *mut cspot_session_t {
    clear_error(out_error);
    let device_id = match read_cstr(device_id, "device_id", out_error) {
        Some(value) => value,
        None => return ptr::null_mut(),
    };

    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        runtime().block_on(async {
            let mut config = SessionConfig::default();
            config.device_id = device_id;
            Session::new(config, None)
        })
    }));

    match result {
        Ok(session) => Box::into_raw(Box::new(SessionHandle { session })) as *mut cspot_session_t,
        Err(_) => {
            write_error(out_error, "panic while creating session");
            ptr::null_mut()
        }
    }
}

/// Returns the session username, or null if unavailable.
///
/// The returned string is heap-allocated and must be freed with `cspot_string_free`.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_session_username(session: *const cspot_session_t) -> *mut c_char {
    if session.is_null() {
        return ptr::null_mut();
    }
    // Safety: session must be a valid handle allocated by cspot.
    let handle = unsafe { &*(session as *const SessionHandle) };
    let username = handle.session.username();
    if username.is_empty() {
        return ptr::null_mut();
    }
    match CString::new(username) {
        Ok(value) => value.into_raw(),
        Err(_) => cstring_from_str_lossy("invalid username").into_raw(),
    }
}

/// Frees a session handle.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_session_free(session: *mut cspot_session_t) {
    if session.is_null() {
        return;
    }
    // Safety: session must be a valid handle allocated by cspot.
    unsafe {
        drop(Box::from_raw(session as *mut SessionHandle));
    }
}

pub(crate) fn session_from_handle(session: *const cspot_session_t) -> Option<Session> {
    if session.is_null() {
        return None;
    }
    // Safety: session must be a valid handle allocated by cspot.
    let handle = unsafe { &*(session as *const SessionHandle) };
    Some(handle.session.clone())
}
