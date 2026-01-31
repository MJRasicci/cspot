//! C bindings for Spotify URI helpers.

use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;

use librespot::core::{spotify_id::SpotifyId, spotify_uri::SpotifyUri};

use crate::error::{clear_error, cspot_error_t, cstring_from_str_lossy, write_error};
use crate::ffi::read_cstr;

/// Builds a Spotify track URI from either a track URI or base62 track id.
///
/// The returned string is heap-allocated and must be freed with `cspot_string_free`.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_track_uri_from_input(
    input: *const c_char,
    out_error: *mut *mut cspot_error_t,
) -> *mut c_char {
    clear_error(out_error);
    let input = match read_cstr(input, "input", out_error) {
        Some(value) => value,
        None => return ptr::null_mut(),
    };

    if let Ok(uri) = SpotifyUri::from_uri(&input) {
        if matches!(uri, SpotifyUri::Track { .. }) {
            return cstring_from_str_lossy(&uri.to_uri()).into_raw();
        }
        write_error(
            out_error,
            "TRACK must be a Spotify track URI like \"spotify:track:...\" or a base62 track id",
        );
        return ptr::null_mut();
    }

    match SpotifyId::from_base62(&input) {
        Ok(id) => {
            let uri = SpotifyUri::Track { id }.to_uri();
            match CString::new(uri) {
                Ok(value) => value.into_raw(),
                Err(_) => {
                    write_error(out_error, "track URI contained an interior null byte");
                    ptr::null_mut()
                }
            }
        }
        Err(_) => {
            write_error(
                out_error,
                "TRACK must be a Spotify track URI like \"spotify:track:...\" or a base62 track id",
            );
            ptr::null_mut()
        }
    }
}
