use std::ffi::CString;
use std::os::raw::c_char;
use std::panic::AssertUnwindSafe;
use std::ptr;

use data_encoding::HEXLOWER;
use futures_util::StreamExt;
use once_cell::sync::Lazy;
use sha1::{Digest, Sha1};

use librespot::core::SessionConfig;
use librespot::discovery::{Credentials, DeviceType, Discovery};
use librespot::protocol::authentication::AuthenticationType;

use crate::error::{clear_error, cspot_error_t, cstring_from_str_lossy, write_error};
use crate::ffi::read_cstr;
use crate::runtime::runtime;

/// Opaque discovery handle for C callers.
#[allow(non_camel_case_types)]
pub struct cspot_discovery_t;

/// Opaque credentials handle for C callers.
#[allow(non_camel_case_types)]
pub struct cspot_credentials_t;

/// Device types exposed to C callers.
#[allow(non_camel_case_types)]
#[repr(C)]
pub enum cspot_device_type_t {
    CSPOT_DEVICE_TYPE_UNKNOWN = 0,
    CSPOT_DEVICE_TYPE_COMPUTER = 1,
    CSPOT_DEVICE_TYPE_TABLET = 2,
    CSPOT_DEVICE_TYPE_SMARTPHONE = 3,
    CSPOT_DEVICE_TYPE_SPEAKER = 4,
    CSPOT_DEVICE_TYPE_TV = 5,
    CSPOT_DEVICE_TYPE_AVR = 6,
    CSPOT_DEVICE_TYPE_STB = 7,
    CSPOT_DEVICE_TYPE_AUDIO_DONGLE = 8,
    CSPOT_DEVICE_TYPE_GAME_CONSOLE = 9,
    CSPOT_DEVICE_TYPE_CAST_AUDIO = 10,
    CSPOT_DEVICE_TYPE_CAST_VIDEO = 11,
    CSPOT_DEVICE_TYPE_AUTOMOBILE = 12,
    CSPOT_DEVICE_TYPE_SMARTWATCH = 13,
    CSPOT_DEVICE_TYPE_CHROMEBOOK = 14,
    CSPOT_DEVICE_TYPE_UNKNOWN_SPOTIFY = 100,
    CSPOT_DEVICE_TYPE_CAR_THING = 101,
    CSPOT_DEVICE_TYPE_OBSERVER = 102,
}

impl From<cspot_device_type_t> for DeviceType {
    fn from(value: cspot_device_type_t) -> Self {
        match value {
            cspot_device_type_t::CSPOT_DEVICE_TYPE_UNKNOWN => DeviceType::Unknown,
            cspot_device_type_t::CSPOT_DEVICE_TYPE_COMPUTER => DeviceType::Computer,
            cspot_device_type_t::CSPOT_DEVICE_TYPE_TABLET => DeviceType::Tablet,
            cspot_device_type_t::CSPOT_DEVICE_TYPE_SMARTPHONE => DeviceType::Smartphone,
            cspot_device_type_t::CSPOT_DEVICE_TYPE_SPEAKER => DeviceType::Speaker,
            cspot_device_type_t::CSPOT_DEVICE_TYPE_TV => DeviceType::Tv,
            cspot_device_type_t::CSPOT_DEVICE_TYPE_AVR => DeviceType::Avr,
            cspot_device_type_t::CSPOT_DEVICE_TYPE_STB => DeviceType::Stb,
            cspot_device_type_t::CSPOT_DEVICE_TYPE_AUDIO_DONGLE => DeviceType::AudioDongle,
            cspot_device_type_t::CSPOT_DEVICE_TYPE_GAME_CONSOLE => DeviceType::GameConsole,
            cspot_device_type_t::CSPOT_DEVICE_TYPE_CAST_AUDIO => DeviceType::CastAudio,
            cspot_device_type_t::CSPOT_DEVICE_TYPE_CAST_VIDEO => DeviceType::CastVideo,
            cspot_device_type_t::CSPOT_DEVICE_TYPE_AUTOMOBILE => DeviceType::Automobile,
            cspot_device_type_t::CSPOT_DEVICE_TYPE_SMARTWATCH => DeviceType::Smartwatch,
            cspot_device_type_t::CSPOT_DEVICE_TYPE_CHROMEBOOK => DeviceType::Chromebook,
            cspot_device_type_t::CSPOT_DEVICE_TYPE_UNKNOWN_SPOTIFY => DeviceType::UnknownSpotify,
            cspot_device_type_t::CSPOT_DEVICE_TYPE_CAR_THING => DeviceType::CarThing,
            cspot_device_type_t::CSPOT_DEVICE_TYPE_OBSERVER => DeviceType::Observer,
        }
    }
}

/// Authentication types for credentials.
#[allow(non_camel_case_types)]
#[repr(C)]
pub enum cspot_auth_type_t {
    CSPOT_AUTH_TYPE_USER_PASS = 0,
    CSPOT_AUTH_TYPE_STORED_SPOTIFY_CREDENTIALS = 1,
    CSPOT_AUTH_TYPE_STORED_FACEBOOK_CREDENTIALS = 2,
    CSPOT_AUTH_TYPE_SPOTIFY_TOKEN = 3,
    CSPOT_AUTH_TYPE_FACEBOOK_TOKEN = 4,
    CSPOT_AUTH_TYPE_INVALID = -1,
}

impl From<AuthenticationType> for cspot_auth_type_t {
    fn from(value: AuthenticationType) -> Self {
        match value {
            AuthenticationType::AUTHENTICATION_USER_PASS => Self::CSPOT_AUTH_TYPE_USER_PASS,
            AuthenticationType::AUTHENTICATION_STORED_SPOTIFY_CREDENTIALS => {
                Self::CSPOT_AUTH_TYPE_STORED_SPOTIFY_CREDENTIALS
            }
            AuthenticationType::AUTHENTICATION_STORED_FACEBOOK_CREDENTIALS => {
                Self::CSPOT_AUTH_TYPE_STORED_FACEBOOK_CREDENTIALS
            }
            AuthenticationType::AUTHENTICATION_SPOTIFY_TOKEN => Self::CSPOT_AUTH_TYPE_SPOTIFY_TOKEN,
            AuthenticationType::AUTHENTICATION_FACEBOOK_TOKEN => {
                Self::CSPOT_AUTH_TYPE_FACEBOOK_TOKEN
            }
        }
    }
}

/// Result of polling discovery for the next credential event.
#[allow(non_camel_case_types)]
#[repr(C)]
pub enum cspot_discovery_next_result_t {
    CSPOT_DISCOVERY_NEXT_CREDENTIALS = 0,
    CSPOT_DISCOVERY_NEXT_END = 1,
    CSPOT_DISCOVERY_NEXT_ERROR = 2,
}

struct DiscoveryHandle {
    discovery: Discovery,
}

struct CredentialsHandle {
    credentials: Credentials,
    username: Option<CString>,
}

impl CredentialsHandle {
    fn new(credentials: Credentials) -> Self {
        let username = credentials
            .username
            .as_deref()
            .map(cstring_from_str_lossy);
        Self {
            credentials,
            username,
        }
    }
}

static DEFAULT_CLIENT_ID: Lazy<CString> = Lazy::new(|| {
    let config = SessionConfig::default();
    cstring_from_str_lossy(&config.client_id)
});

/// Returns the default session client id.
///
/// The returned pointer is owned by cspot and must not be freed.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_session_default_client_id() -> *const c_char {
    DEFAULT_CLIENT_ID.as_ptr()
}

/// Computes a Spotify device id by hashing the device name with SHA1.
///
/// The returned string is heap-allocated and must be freed with `cspot_string_free`.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_device_id_from_name(
    name: *const c_char,
    out_error: *mut *mut cspot_error_t,
) -> *mut c_char {
    clear_error(out_error);
    let name = match read_cstr(name, "name", out_error) {
        Some(value) => value,
        None => return ptr::null_mut(),
    };
    let digest = Sha1::digest(name.as_bytes());
    let device_id = HEXLOWER.encode(digest.as_slice());
    match CString::new(device_id) {
        Ok(value) => value.into_raw(),
        Err(_) => {
            write_error(out_error, "device id contained an interior null byte");
            ptr::null_mut()
        }
    }
}

/// Starts a discovery service.
///
/// This call blocks while the discovery server is started. On success, the returned
/// handle must be released with `cspot_discovery_free`.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_discovery_create(
    device_id: *const c_char,
    client_id: *const c_char,
    name: *const c_char,
    device_type: cspot_device_type_t,
    out_error: *mut *mut cspot_error_t,
) -> *mut cspot_discovery_t {
    clear_error(out_error);
    let device_id = match read_cstr(device_id, "device_id", out_error) {
        Some(value) => value,
        None => return ptr::null_mut(),
    };
    let client_id = match read_cstr(client_id, "client_id", out_error) {
        Some(value) => value,
        None => return ptr::null_mut(),
    };
    let name = match read_cstr(name, "name", out_error) {
        Some(value) => value,
        None => return ptr::null_mut(),
    };

    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        runtime().block_on(async {
            Discovery::builder(device_id, client_id)
                .name(name)
                .device_type(device_type.into())
                .launch()
        })
    }));

    match result {
        Ok(Ok(discovery)) => Box::into_raw(Box::new(DiscoveryHandle { discovery }))
            as *mut cspot_discovery_t,
        Ok(Err(err)) => {
            write_error(out_error, err.to_string());
            ptr::null_mut()
        }
        Err(_) => {
            write_error(out_error, "panic while starting discovery");
            ptr::null_mut()
        }
    }
}

/// Blocks until the next credential event or until discovery stops.
///
/// Returns `CSPOT_DISCOVERY_NEXT_CREDENTIALS` when credentials are available,
/// `CSPOT_DISCOVERY_NEXT_END` when the discovery stream ends, and
/// `CSPOT_DISCOVERY_NEXT_ERROR` on failure.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_discovery_next(
    discovery: *mut cspot_discovery_t,
    out_credentials: *mut *mut cspot_credentials_t,
    out_error: *mut *mut cspot_error_t,
) -> cspot_discovery_next_result_t {
    clear_error(out_error);
    if out_credentials.is_null() {
        write_error(out_error, "out_credentials was null");
        return cspot_discovery_next_result_t::CSPOT_DISCOVERY_NEXT_ERROR;
    }
    // Safety: out_credentials is non-null and points to writable memory.
    unsafe {
        *out_credentials = ptr::null_mut();
    }
    if discovery.is_null() {
        write_error(out_error, "discovery handle was null");
        return cspot_discovery_next_result_t::CSPOT_DISCOVERY_NEXT_ERROR;
    }

    // Safety: discovery must be a valid handle allocated by cspot.
    let handle = unsafe { &mut *(discovery as *mut DiscoveryHandle) };
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        runtime().block_on(async { handle.discovery.next().await })
    }));

    match result {
        Ok(Some(credentials)) => {
            let handle = CredentialsHandle::new(credentials);
            // Safety: out_credentials is non-null and points to writable memory.
            unsafe {
                *out_credentials = Box::into_raw(Box::new(handle)) as *mut cspot_credentials_t;
            }
            cspot_discovery_next_result_t::CSPOT_DISCOVERY_NEXT_CREDENTIALS
        }
        Ok(None) => cspot_discovery_next_result_t::CSPOT_DISCOVERY_NEXT_END,
        Err(_) => {
            write_error(out_error, "panic while waiting for discovery credentials");
            cspot_discovery_next_result_t::CSPOT_DISCOVERY_NEXT_ERROR
        }
    }
}

/// Shuts down discovery and releases associated resources.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_discovery_free(discovery: *mut cspot_discovery_t) {
    if discovery.is_null() {
        return;
    }
    // Safety: discovery must be a valid handle allocated by cspot.
    let handle = unsafe { Box::from_raw(discovery as *mut DiscoveryHandle) };
    let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
        runtime().block_on(handle.discovery.shutdown())
    }));
}

/// Returns the username from credentials, or null if unavailable.
///
/// The returned pointer is owned by the credentials handle and must not be freed.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_credentials_username(
    credentials: *const cspot_credentials_t,
) -> *const c_char {
    if credentials.is_null() {
        return ptr::null();
    }
    // Safety: credentials must be a valid handle allocated by cspot.
    let handle = unsafe { &*(credentials as *const CredentialsHandle) };
    match handle.username.as_ref() {
        Some(value) => value.as_ptr(),
        None => ptr::null(),
    }
}

/// Returns the authentication type of the credentials.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_credentials_auth_type(
    credentials: *const cspot_credentials_t,
) -> cspot_auth_type_t {
    if credentials.is_null() {
        return cspot_auth_type_t::CSPOT_AUTH_TYPE_INVALID;
    }
    // Safety: credentials must be a valid handle allocated by cspot.
    let handle = unsafe { &*(credentials as *const CredentialsHandle) };
    handle.credentials.auth_type.into()
}

/// Returns a pointer to the authentication data and its length.
///
/// The returned data is owned by the credentials handle and remains valid until it is freed.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_credentials_auth_data(
    credentials: *const cspot_credentials_t,
    out_len: *mut usize,
) -> *const u8 {
    if credentials.is_null() {
        if !out_len.is_null() {
            // Safety: out_len is non-null and points to writable memory.
            unsafe {
                *out_len = 0;
            }
        }
        return ptr::null();
    }
    // Safety: credentials must be a valid handle allocated by cspot.
    let handle = unsafe { &*(credentials as *const CredentialsHandle) };
    if !out_len.is_null() {
        // Safety: out_len is non-null and points to writable memory.
        unsafe {
            *out_len = handle.credentials.auth_data.len();
        }
    }
    handle.credentials.auth_data.as_ptr()
}

/// Frees a credentials handle.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_credentials_free(credentials: *mut cspot_credentials_t) {
    if credentials.is_null() {
        return;
    }
    // Safety: credentials must be a valid handle allocated by cspot.
    unsafe {
        drop(Box::from_raw(credentials as *mut CredentialsHandle));
    }
}

/// Returns a human-readable name for the authentication type.
///
/// The returned pointer is static and must not be freed.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_auth_type_name(value: cspot_auth_type_t) -> *const c_char {
    match value {
        cspot_auth_type_t::CSPOT_AUTH_TYPE_USER_PASS => b"USER_PASS\0".as_ptr() as *const c_char,
        cspot_auth_type_t::CSPOT_AUTH_TYPE_STORED_SPOTIFY_CREDENTIALS => {
            b"STORED_SPOTIFY_CREDENTIALS\0".as_ptr() as *const c_char
        }
        cspot_auth_type_t::CSPOT_AUTH_TYPE_STORED_FACEBOOK_CREDENTIALS => {
            b"STORED_FACEBOOK_CREDENTIALS\0".as_ptr() as *const c_char
        }
        cspot_auth_type_t::CSPOT_AUTH_TYPE_SPOTIFY_TOKEN => {
            b"SPOTIFY_TOKEN\0".as_ptr() as *const c_char
        }
        cspot_auth_type_t::CSPOT_AUTH_TYPE_FACEBOOK_TOKEN => {
            b"FACEBOOK_TOKEN\0".as_ptr() as *const c_char
        }
        cspot_auth_type_t::CSPOT_AUTH_TYPE_INVALID => b"INVALID\0".as_ptr() as *const c_char,
    }
}

pub(crate) fn credentials_from_handle(
    credentials: *const cspot_credentials_t,
) -> Option<Credentials> {
    if credentials.is_null() {
        return None;
    }
    // Safety: credentials must be a valid handle allocated by cspot.
    let handle = unsafe { &*(credentials as *const CredentialsHandle) };
    Some(handle.credentials.clone())
}
