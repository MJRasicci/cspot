//! C bindings for librespot playback components.

use std::panic::AssertUnwindSafe;
use std::ptr;
use std::sync::Arc;

use librespot::playback::{
    audio_backend,
    config::{AudioFormat, PlayerConfig},
    mixer::{self, Mixer, MixerConfig},
    player::Player,
};

use crate::error::{clear_error, cspot_error_t, write_error};
use crate::session::session_from_handle;

/// Opaque mixer handle for C callers.
#[allow(non_camel_case_types)]
pub struct cspot_mixer_t;

/// Opaque player handle for C callers.
#[allow(non_camel_case_types)]
pub struct cspot_player_t;

struct MixerHandle {
    mixer: Arc<dyn Mixer>,
}

struct PlayerHandle {
    player: Arc<Player>,
}

/// Creates a mixer using the default mixer backend and default configuration.
///
/// The returned handle must be released with `cspot_mixer_free`.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_mixer_create_default(
    out_error: *mut *mut cspot_error_t,
) -> *mut cspot_mixer_t {
    clear_error(out_error);
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        let builder = mixer::find(None).ok_or_else(|| "no mixer backend available")?;
        builder(MixerConfig::default()).map_err(|e| e.to_string())
    }));

    match result {
        Ok(Ok(mixer)) => Box::into_raw(Box::new(MixerHandle { mixer })) as *mut cspot_mixer_t,
        Ok(Err(err)) => {
            write_error(out_error, err);
            ptr::null_mut()
        }
        Err(_) => {
            write_error(out_error, "panic while creating mixer");
            ptr::null_mut()
        }
    }
}

/// Creates a player using default configuration and the default audio backend.
///
/// The returned handle must be released with `cspot_player_free`.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_player_create_default(
    session: *const crate::session::cspot_session_t,
    mixer: *const cspot_mixer_t,
    out_error: *mut *mut cspot_error_t,
) -> *mut cspot_player_t {
    clear_error(out_error);
    let session = match session_from_handle(session) {
        Some(value) => value,
        None => {
            write_error(out_error, "session handle was null");
            return ptr::null_mut();
        }
    };
    if mixer.is_null() {
        write_error(out_error, "mixer handle was null");
        return ptr::null_mut();
    }

    // Safety: mixer must be a valid handle allocated by cspot.
    let mixer_handle = unsafe { &*(mixer as *const MixerHandle) };
    let mixer = Arc::clone(&mixer_handle.mixer);

    let result = std::panic::catch_unwind(AssertUnwindSafe(|| -> Result<Arc<Player>, String> {
        let backend = audio_backend::find(None)
            .ok_or_else(|| "no audio backend available".to_string())?;
        let player_config = PlayerConfig::default();
        let audio_format = AudioFormat::default();
        let soft_volume = mixer.get_soft_volume();
        Ok(Player::new(player_config, session, soft_volume, move || {
            backend(None, audio_format)
        }))
    }));

    match result {
        Ok(Ok(player)) => Box::into_raw(Box::new(PlayerHandle { player })) as *mut cspot_player_t,
        Ok(Err(err)) => {
            write_error(out_error, err);
            ptr::null_mut()
        }
        Err(_) => {
            write_error(out_error, "panic while creating player");
            ptr::null_mut()
        }
    }
}

/// Frees a mixer handle.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_mixer_free(mixer: *mut cspot_mixer_t) {
    if mixer.is_null() {
        return;
    }
    // Safety: mixer must be a valid handle allocated by cspot.
    unsafe {
        drop(Box::from_raw(mixer as *mut MixerHandle));
    }
}

/// Frees a player handle.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_player_free(player: *mut cspot_player_t) {
    if player.is_null() {
        return;
    }
    // Safety: player must be a valid handle allocated by cspot.
    unsafe {
        drop(Box::from_raw(player as *mut PlayerHandle));
    }
}

pub(crate) fn player_from_handle(player: *const cspot_player_t) -> Option<Arc<Player>> {
    if player.is_null() {
        return None;
    }
    // Safety: player must be a valid handle allocated by cspot.
    let handle = unsafe { &*(player as *const PlayerHandle) };
    Some(Arc::clone(&handle.player))
}

pub(crate) fn mixer_from_handle(mixer: *const cspot_mixer_t) -> Option<Arc<dyn Mixer>> {
    if mixer.is_null() {
        return None;
    }
    // Safety: mixer must be a valid handle allocated by cspot.
    let handle = unsafe { &*(mixer as *const MixerHandle) };
    Some(Arc::clone(&handle.mixer))
}
