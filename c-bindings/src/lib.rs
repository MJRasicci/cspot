//! C FFI entry points for cspot.

/// Temporary placeholder to validate C bindings wiring.
///
/// Returns a fixed marker value that can be checked from C.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_placeholder() -> u32 {
    0xC5_07_0001
}
