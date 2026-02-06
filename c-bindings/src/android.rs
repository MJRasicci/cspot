//! Android-specific C bindings.

use std::ffi::c_void;
use std::sync::OnceLock;

use crate::error::{clear_error, cspot_error_t, write_error};

#[cfg(target_os = "android")]
static ANDROID_CONTEXT_INITIALIZED: OnceLock<()> = OnceLock::new();

/// Initializes the Android JNI context used by cspot's audio backends.
///
/// Call this exactly once per process before creating a player on Android.
/// `java_vm` must point to the process `JavaVM` and `context` must be a valid
/// global JNI reference to an Android `Context`.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_android_initialize_context(
    java_vm: *mut c_void,
    context: *mut c_void,
    out_error: *mut *mut cspot_error_t,
) -> bool {
    clear_error(out_error);
    if java_vm.is_null() {
        write_error(out_error, "java_vm was null");
        return false;
    }
    if context.is_null() {
        write_error(out_error, "context was null");
        return false;
    }

    #[cfg(target_os = "android")]
    {
        if ANDROID_CONTEXT_INITIALIZED.get().is_some() {
            return true;
        }

        let result = std::panic::catch_unwind(|| {
            // Safety: pointers are provided by JNI and are expected to be valid.
            unsafe {
                ndk_context::initialize_android_context(java_vm, context);
            }
        });

        match result {
            Ok(()) => {
                let _ = ANDROID_CONTEXT_INITIALIZED.set(());
                true
            }
            Err(_) => {
                write_error(out_error, "failed to initialize Android JNI context");
                false
            }
        }
    }

    #[cfg(not(target_os = "android"))]
    {
        write_error(out_error, "cspot_android_initialize_context is only supported on Android");
        false
    }
}
