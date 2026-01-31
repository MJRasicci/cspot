//! C bindings for librespot connect (Spirc).

use std::future::Future;
use std::os::raw::c_char;
use std::panic::AssertUnwindSafe;
use std::pin::Pin;
use std::ptr;

use librespot::connect::{ConnectConfig, LoadRequest, LoadRequestOptions, Spirc};

use crate::discovery::{cspot_device_type_t, credentials_from_handle};
use crate::error::{clear_error, cspot_error_t, write_error};
use crate::ffi::read_cstr;
use crate::playback::{mixer_from_handle, player_from_handle, cspot_mixer_t, cspot_player_t};
use crate::runtime::runtime;
use crate::session::{session_from_handle, cspot_session_t};

/// Opaque connect configuration handle for C callers.
#[repr(C)]
pub struct cspot_connect_config_t;

/// Opaque load request options handle for C callers.
#[repr(C)]
pub struct cspot_load_request_options_t;

/// Opaque spirc handle for C callers.
#[repr(C)]
pub struct cspot_spirc_t;

/// Opaque spirc task handle for C callers.
#[repr(C)]
pub struct cspot_spirc_task_t;

struct ConnectConfigHandle {
    config: ConnectConfig,
}

struct LoadRequestOptionsHandle {
    options: LoadRequestOptions,
}

struct SpircHandle {
    spirc: Spirc,
}

struct SpircTaskHandle {
    task: Option<Pin<Box<dyn Future<Output = ()> + Send>>>,
}

/// Creates a connect configuration using default values.
///
/// The returned handle must be released with `cspot_connect_config_free`.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_connect_config_create_default() -> *mut cspot_connect_config_t {
    let handle = ConnectConfigHandle {
        config: ConnectConfig::default(),
    };
    Box::into_raw(Box::new(handle)) as *mut cspot_connect_config_t
}

/// Sets the connect device name.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_connect_config_set_name(
    config: *mut cspot_connect_config_t,
    name: *const c_char,
    out_error: *mut *mut cspot_error_t,
) -> bool {
    clear_error(out_error);
    if config.is_null() {
        write_error(out_error, "config handle was null");
        return false;
    }
    let name = match read_cstr(name, "name", out_error) {
        Some(value) => value,
        None => return false,
    };
    // Safety: config must be a valid handle allocated by cspot.
    let handle = unsafe { &mut *(config as *mut ConnectConfigHandle) };
    handle.config.name = name;
    true
}

/// Sets the connect device type.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_connect_config_set_device_type(
    config: *mut cspot_connect_config_t,
    device_type: cspot_device_type_t,
    out_error: *mut *mut cspot_error_t,
) -> bool {
    clear_error(out_error);
    if config.is_null() {
        write_error(out_error, "config handle was null");
        return false;
    }
    // Safety: config must be a valid handle allocated by cspot.
    let handle = unsafe { &mut *(config as *mut ConnectConfigHandle) };
    handle.config.device_type = device_type.into();
    true
}

/// Frees a connect configuration handle.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_connect_config_free(config: *mut cspot_connect_config_t) {
    if config.is_null() {
        return;
    }
    // Safety: config must be a valid handle allocated by cspot.
    unsafe {
        drop(Box::from_raw(config as *mut ConnectConfigHandle));
    }
}

/// Creates default load request options.
///
/// The returned handle must be released with `cspot_load_request_options_free`.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_load_request_options_create_default() -> *mut cspot_load_request_options_t {
    let handle = LoadRequestOptionsHandle {
        options: LoadRequestOptions::default(),
    };
    Box::into_raw(Box::new(handle)) as *mut cspot_load_request_options_t
}

/// Sets whether the load request should start playing immediately.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_load_request_options_set_start_playing(
    options: *mut cspot_load_request_options_t,
    start_playing: bool,
    out_error: *mut *mut cspot_error_t,
) -> bool {
    clear_error(out_error);
    if options.is_null() {
        write_error(out_error, "options handle was null");
        return false;
    }
    // Safety: options must be a valid handle allocated by cspot.
    let handle = unsafe { &mut *(options as *mut LoadRequestOptionsHandle) };
    handle.options.start_playing = start_playing;
    true
}

/// Sets the load request seek position in milliseconds.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_load_request_options_set_seek_to(
    options: *mut cspot_load_request_options_t,
    seek_to_ms: u32,
    out_error: *mut *mut cspot_error_t,
) -> bool {
    clear_error(out_error);
    if options.is_null() {
        write_error(out_error, "options handle was null");
        return false;
    }
    // Safety: options must be a valid handle allocated by cspot.
    let handle = unsafe { &mut *(options as *mut LoadRequestOptionsHandle) };
    handle.options.seek_to = seek_to_ms;
    true
}

/// Frees load request options.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_load_request_options_free(options: *mut cspot_load_request_options_t) {
    if options.is_null() {
        return;
    }
    // Safety: options must be a valid handle allocated by cspot.
    unsafe {
        drop(Box::from_raw(options as *mut LoadRequestOptionsHandle));
    }
}

/// Creates a new Spirc instance and returns the associated task handle.
///
/// The returned spirc handle must be released with `cspot_spirc_free`.
/// The task handle must be released with `cspot_spirc_task_free`.
/// The configuration and credentials are cloned; callers may free their handles
/// after this function returns.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_spirc_create(
    config: *const cspot_connect_config_t,
    session: *const cspot_session_t,
    credentials: *const crate::discovery::cspot_credentials_t,
    player: *const cspot_player_t,
    mixer: *const cspot_mixer_t,
    out_task: *mut *mut cspot_spirc_task_t,
    out_error: *mut *mut cspot_error_t,
) -> *mut cspot_spirc_t {
    clear_error(out_error);
    if out_task.is_null() {
        write_error(out_error, "out_task was null");
        return ptr::null_mut();
    }
    // Safety: out_task is non-null and points to writable memory.
    unsafe {
        *out_task = ptr::null_mut();
    }
    if config.is_null() {
        write_error(out_error, "config handle was null");
        return ptr::null_mut();
    }
    let session = match session_from_handle(session) {
        Some(value) => value,
        None => {
            write_error(out_error, "session handle was null");
            return ptr::null_mut();
        }
    };
    if credentials.is_null() {
        write_error(out_error, "credentials handle was null");
        return ptr::null_mut();
    }
    let player = match player_from_handle(player) {
        Some(value) => value,
        None => {
            write_error(out_error, "player handle was null");
            return ptr::null_mut();
        }
    };
    let mixer = match mixer_from_handle(mixer) {
        Some(value) => value,
        None => {
            write_error(out_error, "mixer handle was null");
            return ptr::null_mut();
        }
    };

    // Safety: config must be a valid handle allocated by cspot.
    let config_handle = unsafe { &*(config as *const ConnectConfigHandle) };
    let credentials = match credentials_from_handle(credentials) {
        Some(value) => value,
        None => {
            write_error(out_error, "credentials handle was null");
            return ptr::null_mut();
        }
    };
    let config = config_handle.config.clone();

    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        runtime().block_on(async { Spirc::new(config, session, credentials, player, mixer).await })
    }));

    match result {
        Ok(Ok((spirc, task))) => {
            let spirc_handle = Box::new(SpircHandle { spirc });
            let task_handle = Box::new(SpircTaskHandle {
                task: Some(Box::pin(task)),
            });
            // Safety: out_task is non-null and points to writable memory.
            unsafe {
                *out_task = Box::into_raw(task_handle) as *mut cspot_spirc_task_t;
            }
            Box::into_raw(spirc_handle) as *mut cspot_spirc_t
        }
        Ok(Err(err)) => {
            write_error(out_error, err.to_string());
            ptr::null_mut()
        }
        Err(_) => {
            write_error(out_error, "panic while starting Spirc");
            ptr::null_mut()
        }
    }
}

/// Sends a Connect activate command.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_spirc_activate(
    spirc: *const cspot_spirc_t,
    out_error: *mut *mut cspot_error_t,
) -> bool {
    clear_error(out_error);
    if spirc.is_null() {
        write_error(out_error, "spirc handle was null");
        return false;
    }
    // Safety: spirc must be a valid handle allocated by cspot.
    let handle = unsafe { &*(spirc as *const SpircHandle) };
    match handle.spirc.activate() {
        Ok(()) => true,
        Err(err) => {
            write_error(out_error, err.to_string());
            false
        }
    }
}

/// Sends a Connect play command.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_spirc_play(
    spirc: *const cspot_spirc_t,
    out_error: *mut *mut cspot_error_t,
) -> bool {
    clear_error(out_error);
    if spirc.is_null() {
        write_error(out_error, "spirc handle was null");
        return false;
    }
    // Safety: spirc must be a valid handle allocated by cspot.
    let handle = unsafe { &*(spirc as *const SpircHandle) };
    match handle.spirc.play() {
        Ok(()) => true,
        Err(err) => {
            write_error(out_error, err.to_string());
            false
        }
    }
}

/// Loads tracks for playback using the provided URIs.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_spirc_load_tracks(
    spirc: *const cspot_spirc_t,
    uris: *const *const c_char,
    uri_count: usize,
    options: *const cspot_load_request_options_t,
    out_error: *mut *mut cspot_error_t,
) -> bool {
    clear_error(out_error);
    if spirc.is_null() {
        write_error(out_error, "spirc handle was null");
        return false;
    }
    if uri_count > 0 && uris.is_null() {
        write_error(out_error, "uris was null");
        return false;
    }

    let mut tracks = Vec::with_capacity(uri_count);
    for index in 0..uri_count {
        // Safety: uris is valid for uri_count entries.
        let uri_ptr = unsafe { *uris.add(index) };
        let uri = match read_cstr(uri_ptr, "uri", out_error) {
            Some(value) => value,
            None => return false,
        };
        tracks.push(uri);
    }

    let options = if options.is_null() {
        LoadRequestOptions::default()
    } else {
        // Safety: options must be a valid handle allocated by cspot.
        let handle = unsafe { &*(options as *const LoadRequestOptionsHandle) };
        handle.options.clone()
    };

    let request = LoadRequest::from_tracks(tracks, options);
    // Safety: spirc must be a valid handle allocated by cspot.
    let handle = unsafe { &*(spirc as *const SpircHandle) };
    match handle.spirc.load(request) {
        Ok(()) => true,
        Err(err) => {
            write_error(out_error, err.to_string());
            false
        }
    }
}

/// Requests a clean Connect shutdown.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_spirc_shutdown(
    spirc: *const cspot_spirc_t,
    out_error: *mut *mut cspot_error_t,
) -> bool {
    clear_error(out_error);
    if spirc.is_null() {
        write_error(out_error, "spirc handle was null");
        return false;
    }
    // Safety: spirc must be a valid handle allocated by cspot.
    let handle = unsafe { &*(spirc as *const SpircHandle) };
    match handle.spirc.shutdown() {
        Ok(()) => true,
        Err(err) => {
            write_error(out_error, err.to_string());
            false
        }
    }
}

/// Runs the Spirc task until it completes.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_spirc_task_run(
    task: *mut cspot_spirc_task_t,
    out_error: *mut *mut cspot_error_t,
) -> bool {
    clear_error(out_error);
    if task.is_null() {
        write_error(out_error, "spirc task handle was null");
        return false;
    }
    // Safety: task must be a valid handle allocated by cspot.
    let handle = unsafe { &mut *(task as *mut SpircTaskHandle) };
    let task = match handle.task.take() {
        Some(value) => value,
        None => {
            write_error(out_error, "spirc task already completed");
            return false;
        }
    };

    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        runtime().block_on(task);
    }));
    match result {
        Ok(()) => true,
        Err(_) => {
            write_error(out_error, "panic while running Spirc task");
            false
        }
    }
}

/// Frees a spirc task handle.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_spirc_task_free(task: *mut cspot_spirc_task_t) {
    if task.is_null() {
        return;
    }
    // Safety: task must be a valid handle allocated by cspot.
    unsafe {
        drop(Box::from_raw(task as *mut SpircTaskHandle));
    }
}

/// Frees a spirc handle.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_spirc_free(spirc: *mut cspot_spirc_t) {
    if spirc.is_null() {
        return;
    }
    // Safety: spirc must be a valid handle allocated by cspot.
    unsafe {
        drop(Box::from_raw(spirc as *mut SpircHandle));
    }
}
