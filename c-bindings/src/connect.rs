//! C bindings for librespot connect (Spirc).

use std::future::Future;
use std::os::raw::c_char;
use std::panic::AssertUnwindSafe;
use std::pin::Pin;
use std::ptr;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use librespot::connect::{ConnectConfig, LoadRequest, LoadRequestOptions, Spirc};
use librespot::core::{Error as LibrespotError, SpotifyUri};
use librespot::metadata::audio::{AudioItem, UniqueFields};
use librespot::playback::player::{Player, PlayerEvent};
use tokio::task::JoinHandle;

use crate::discovery::{credentials_from_handle, cspot_device_type_t};
use crate::error::{clear_error, cspot_error_t, cstring_from_str_lossy, write_error};
use crate::ffi::read_cstr;
use crate::playback::{cspot_mixer_t, cspot_player_t, mixer_from_handle, player_from_handle};
use crate::runtime::runtime;
use crate::session::{cspot_session_t, session_from_handle};

/// Opaque connect configuration handle for C callers.
#[allow(non_camel_case_types)]
pub struct cspot_connect_config_t;

/// Opaque load request options handle for C callers.
#[allow(non_camel_case_types)]
pub struct cspot_load_request_options_t;

/// Opaque spirc handle for C callers.
#[allow(non_camel_case_types)]
pub struct cspot_spirc_t;

/// Opaque spirc task handle for C callers.
#[allow(non_camel_case_types)]
pub struct cspot_spirc_task_t;

/// Current playback state reported by cspot.
#[allow(non_camel_case_types)]
#[repr(C)]
pub enum cspot_playback_state_t {
    CSPOT_PLAYBACK_STATE_STOPPED = 0,
    CSPOT_PLAYBACK_STATE_LOADING = 1,
    CSPOT_PLAYBACK_STATE_PLAYING = 2,
    CSPOT_PLAYBACK_STATE_PAUSED = 3,
    CSPOT_PLAYBACK_STATE_INVALID = -1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PlaybackState {
    Stopped,
    Loading,
    Playing,
    Paused,
}

impl Default for PlaybackState {
    fn default() -> Self {
        Self::Stopped
    }
}

impl From<PlaybackState> for cspot_playback_state_t {
    fn from(value: PlaybackState) -> Self {
        match value {
            PlaybackState::Stopped => Self::CSPOT_PLAYBACK_STATE_STOPPED,
            PlaybackState::Loading => Self::CSPOT_PLAYBACK_STATE_LOADING,
            PlaybackState::Playing => Self::CSPOT_PLAYBACK_STATE_PLAYING,
            PlaybackState::Paused => Self::CSPOT_PLAYBACK_STATE_PAUSED,
        }
    }
}

#[derive(Clone, Debug, Default)]
struct TrackMetadata {
    spotify_id: Option<String>,
    uri: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    artwork_url: Option<String>,
    title: Option<String>,
    duration_ms: u32,
}

#[derive(Clone, Debug, Default)]
struct SpircStatusSnapshot {
    connected: bool,
    playback_state: PlaybackState,
    position_ms: u32,
    duration_ms: u32,
    volume: u16,
    shuffle_enabled: bool,
    repeat_context_enabled: bool,
    repeat_track_enabled: bool,
    track: TrackMetadata,
}

#[derive(Debug, Default)]
struct SpircRuntimeStatus {
    connected: bool,
    playback_state: PlaybackState,
    position_anchor_ms: u32,
    position_anchor_at: Option<Instant>,
    volume: u16,
    shuffle_enabled: bool,
    repeat_context_enabled: bool,
    repeat_track_enabled: bool,
    track: TrackMetadata,
}

impl SpircRuntimeStatus {
    fn snapshot(&self) -> SpircStatusSnapshot {
        SpircStatusSnapshot {
            connected: self.connected,
            playback_state: self.playback_state,
            position_ms: self.current_position_ms(),
            duration_ms: self.track.duration_ms,
            volume: self.volume,
            shuffle_enabled: self.shuffle_enabled,
            repeat_context_enabled: self.repeat_context_enabled,
            repeat_track_enabled: self.repeat_track_enabled,
            track: self.track.clone(),
        }
    }

    fn current_position_ms(&self) -> u32 {
        let mut position_ms = self.position_anchor_ms;
        if self.playback_state == PlaybackState::Playing {
            if let Some(anchor) = self.position_anchor_at {
                let elapsed_ms = u32::try_from(anchor.elapsed().as_millis()).unwrap_or(u32::MAX);
                position_ms = position_ms.saturating_add(elapsed_ms);
            }
        }
        if self.track.duration_ms > 0 {
            position_ms.min(self.track.duration_ms)
        } else {
            position_ms
        }
    }

    fn set_playback_state(&mut self, playback_state: PlaybackState) {
        self.playback_state = playback_state;
        if playback_state != PlaybackState::Playing {
            self.position_anchor_at = None;
        }
    }

    fn set_position(&mut self, position_ms: u32, is_playing: bool) {
        let position_ms = if self.track.duration_ms > 0 {
            position_ms.min(self.track.duration_ms)
        } else {
            position_ms
        };
        self.position_anchor_ms = position_ms;
        self.position_anchor_at = if is_playing {
            Some(Instant::now())
        } else {
            None
        };
    }

    fn set_track_identity(&mut self, track_uri: &SpotifyUri) {
        let uri = track_uri.to_uri();
        if self.track.uri.as_deref() != Some(uri.as_str()) {
            self.track.artist = None;
            self.track.album = None;
            self.track.artwork_url = None;
            self.track.title = None;
            self.track.duration_ms = 0;
        }
        self.track.spotify_id = spotify_item_id(track_uri);
        self.track.uri = Some(uri);
    }

    fn set_track_metadata(&mut self, audio_item: &AudioItem) {
        self.track.spotify_id = spotify_item_id(&audio_item.track_id);
        self.track.uri = non_empty(audio_item.uri.clone());
        self.track.title = non_empty(audio_item.name.clone());
        self.track.artwork_url = audio_item
            .covers
            .first()
            .map(|cover| cover.url.clone())
            .and_then(non_empty);
        self.track.duration_ms = audio_item.duration_ms;

        let (artist, album) = match &audio_item.unique_fields {
            UniqueFields::Track { artists, album, .. } => {
                let mut names = Vec::new();
                for artist in artists.iter() {
                    if artist.name.is_empty() {
                        continue;
                    }
                    if !names.iter().any(|value: &String| value == &artist.name) {
                        names.push(artist.name.clone());
                    }
                }
                (
                    (!names.is_empty()).then(|| names.join(", ")),
                    non_empty(album.clone()),
                )
            }
            UniqueFields::Local { artists, album, .. } => (
                artists.clone().and_then(non_empty),
                album.clone().and_then(non_empty),
            ),
            UniqueFields::Episode { show_name, .. } => (None, non_empty(show_name.clone())),
        };
        self.track.artist = artist;
        self.track.album = album;

        if self.position_anchor_ms > self.track.duration_ms && self.track.duration_ms > 0 {
            self.position_anchor_ms = self.track.duration_ms;
        }
    }
}

struct ConnectConfigHandle {
    config: ConnectConfig,
}

struct LoadRequestOptionsHandle {
    options: LoadRequestOptions,
}

struct SpircHandle {
    spirc: Spirc,
    status: Arc<Mutex<SpircRuntimeStatus>>,
    status_task: JoinHandle<()>,
}

struct SpircTaskHandle {
    task: Option<Pin<Box<dyn Future<Output = ()> + Send>>>,
}

fn non_empty(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

fn spotify_item_id(uri: &SpotifyUri) -> Option<String> {
    match uri {
        SpotifyUri::Track { id } | SpotifyUri::Episode { id } => Some(id.to_base62()),
        _ => None,
    }
}

fn apply_player_event(status: &mut SpircRuntimeStatus, event: PlayerEvent) {
    match event {
        PlayerEvent::SessionConnected { .. } => status.connected = true,
        PlayerEvent::SessionDisconnected { .. } => status.connected = false,
        PlayerEvent::TrackChanged { audio_item } => status.set_track_metadata(&audio_item),
        PlayerEvent::Loading {
            track_id,
            position_ms,
            ..
        } => {
            status.set_playback_state(PlaybackState::Loading);
            status.set_track_identity(&track_id);
            status.set_position(position_ms, false);
        }
        PlayerEvent::Playing {
            track_id,
            position_ms,
            ..
        }
        | PlayerEvent::PositionChanged {
            track_id,
            position_ms,
            ..
        }
        | PlayerEvent::PositionCorrection {
            track_id,
            position_ms,
            ..
        } => {
            status.set_playback_state(PlaybackState::Playing);
            status.set_track_identity(&track_id);
            status.set_position(position_ms, true);
        }
        PlayerEvent::Paused {
            track_id,
            position_ms,
            ..
        } => {
            status.set_playback_state(PlaybackState::Paused);
            status.set_track_identity(&track_id);
            status.set_position(position_ms, false);
        }
        PlayerEvent::Seeked {
            track_id,
            position_ms,
            ..
        } => {
            status.set_track_identity(&track_id);
            let is_playing = status.playback_state == PlaybackState::Playing;
            status.set_position(position_ms, is_playing);
        }
        PlayerEvent::Stopped { track_id, .. } => {
            status.set_playback_state(PlaybackState::Stopped);
            status.set_track_identity(&track_id);
            status.set_position(0, false);
        }
        PlayerEvent::VolumeChanged { volume } => status.volume = volume,
        PlayerEvent::ShuffleChanged { shuffle } => status.shuffle_enabled = shuffle,
        PlayerEvent::RepeatChanged { context, track } => {
            status.repeat_context_enabled = context;
            status.repeat_track_enabled = track;
        }
        _ => {}
    }
}

fn spawn_status_task(
    player: &Arc<Player>,
    status: Arc<Mutex<SpircRuntimeStatus>>,
) -> JoinHandle<()> {
    let mut event_channel = player.get_player_event_channel();
    runtime().spawn(async move {
        while let Some(event) = event_channel.recv().await {
            let mut guard = status.lock().unwrap_or_else(|err| err.into_inner());
            apply_player_event(&mut guard, event);
        }
    })
}

fn run_spirc_command(
    spirc: *const cspot_spirc_t,
    out_error: *mut *mut cspot_error_t,
    command: impl FnOnce(&Spirc) -> Result<(), LibrespotError>,
) -> bool {
    clear_error(out_error);
    if spirc.is_null() {
        write_error(out_error, "spirc handle was null");
        return false;
    }
    // Safety: spirc must be a valid handle allocated by cspot.
    let handle = unsafe { &*(spirc as *const SpircHandle) };
    match command(&handle.spirc) {
        Ok(()) => true,
        Err(err) => {
            write_error(out_error, err.to_string());
            false
        }
    }
}

fn snapshot_from_spirc(spirc: *const cspot_spirc_t) -> Option<SpircStatusSnapshot> {
    if spirc.is_null() {
        return None;
    }
    // Safety: spirc must be a valid handle allocated by cspot.
    let handle = unsafe { &*(spirc as *const SpircHandle) };
    let guard = handle.status.lock().unwrap_or_else(|err| err.into_inner());
    Some(guard.snapshot())
}

fn string_to_owned_ptr(value: Option<String>) -> *mut c_char {
    match value {
        Some(value) => cstring_from_str_lossy(&value).into_raw(),
        None => ptr::null_mut(),
    }
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
    let spirc_player = Arc::clone(&player);

    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        runtime()
            .block_on(async { Spirc::new(config, session, credentials, spirc_player, mixer).await })
    }));

    match result {
        Ok(Ok((spirc, task))) => {
            let status = Arc::new(Mutex::new(SpircRuntimeStatus::default()));
            let status_task = spawn_status_task(&player, Arc::clone(&status));
            let spirc_handle = Box::new(SpircHandle {
                spirc,
                status,
                status_task,
            });
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
    run_spirc_command(spirc, out_error, Spirc::activate)
}

/// Sends a Connect play command.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_spirc_play(
    spirc: *const cspot_spirc_t,
    out_error: *mut *mut cspot_error_t,
) -> bool {
    run_spirc_command(spirc, out_error, Spirc::play)
}

/// Sends a Connect play command to resume playback.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_spirc_resume(
    spirc: *const cspot_spirc_t,
    out_error: *mut *mut cspot_error_t,
) -> bool {
    run_spirc_command(spirc, out_error, Spirc::play)
}

/// Sends a Connect play/pause toggle command.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_spirc_play_pause(
    spirc: *const cspot_spirc_t,
    out_error: *mut *mut cspot_error_t,
) -> bool {
    run_spirc_command(spirc, out_error, Spirc::play_pause)
}

/// Sends a Connect pause command.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_spirc_pause(
    spirc: *const cspot_spirc_t,
    out_error: *mut *mut cspot_error_t,
) -> bool {
    run_spirc_command(spirc, out_error, Spirc::pause)
}

/// Sends a Connect previous-track command.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_spirc_prev(
    spirc: *const cspot_spirc_t,
    out_error: *mut *mut cspot_error_t,
) -> bool {
    run_spirc_command(spirc, out_error, Spirc::prev)
}

/// Sends a Connect next-track command.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_spirc_next(
    spirc: *const cspot_spirc_t,
    out_error: *mut *mut cspot_error_t,
) -> bool {
    run_spirc_command(spirc, out_error, Spirc::next)
}

/// Increases volume by the configured Connect step.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_spirc_volume_up(
    spirc: *const cspot_spirc_t,
    out_error: *mut *mut cspot_error_t,
) -> bool {
    run_spirc_command(spirc, out_error, Spirc::volume_up)
}

/// Decreases volume by the configured Connect step.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_spirc_volume_down(
    spirc: *const cspot_spirc_t,
    out_error: *mut *mut cspot_error_t,
) -> bool {
    run_spirc_command(spirc, out_error, Spirc::volume_down)
}

/// Sets absolute volume.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_spirc_set_volume(
    spirc: *const cspot_spirc_t,
    volume: u16,
    out_error: *mut *mut cspot_error_t,
) -> bool {
    run_spirc_command(spirc, out_error, move |handle| handle.set_volume(volume))
}

/// Seeks within the current track in milliseconds.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_spirc_seek_to(
    spirc: *const cspot_spirc_t,
    position_ms: u32,
    out_error: *mut *mut cspot_error_t,
) -> bool {
    run_spirc_command(spirc, out_error, move |handle| {
        handle.set_position_ms(position_ms)
    })
}

/// Enables or disables shuffle mode.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_spirc_set_shuffle(
    spirc: *const cspot_spirc_t,
    shuffle: bool,
    out_error: *mut *mut cspot_error_t,
) -> bool {
    run_spirc_command(spirc, out_error, move |handle| handle.shuffle(shuffle))
}

/// Enables or disables repeat-context mode.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_spirc_set_repeat_context(
    spirc: *const cspot_spirc_t,
    repeat: bool,
    out_error: *mut *mut cspot_error_t,
) -> bool {
    run_spirc_command(spirc, out_error, move |handle| handle.repeat(repeat))
}

/// Enables or disables repeat-track mode.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_spirc_set_repeat_track(
    spirc: *const cspot_spirc_t,
    repeat: bool,
    out_error: *mut *mut cspot_error_t,
) -> bool {
    run_spirc_command(spirc, out_error, move |handle| handle.repeat_track(repeat))
}

/// Disconnects the device from Spotify Connect.
///
/// If `pause` is true, playback is paused before disconnecting.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_spirc_disconnect(
    spirc: *const cspot_spirc_t,
    pause: bool,
    out_error: *mut *mut cspot_error_t,
) -> bool {
    run_spirc_command(spirc, out_error, move |handle| handle.disconnect(pause))
}

/// Transfers current playback from another device to this Connect device.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_spirc_transfer(
    spirc: *const cspot_spirc_t,
    out_error: *mut *mut cspot_error_t,
) -> bool {
    run_spirc_command(spirc, out_error, |handle| handle.transfer(None))
}

/// Adds a Spotify URI to the playback queue.
///
/// Accepts track, episode, album, and playlist URIs.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_spirc_add_to_queue(
    spirc: *const cspot_spirc_t,
    uri: *const c_char,
    out_error: *mut *mut cspot_error_t,
) -> bool {
    clear_error(out_error);
    let uri = match read_cstr(uri, "uri", out_error) {
        Some(value) => value,
        None => return false,
    };
    let uri = match SpotifyUri::from_uri(&uri) {
        Ok(value) => value,
        Err(err) => {
            write_error(out_error, err.to_string());
            return false;
        }
    };
    run_spirc_command(spirc, out_error, move |handle| handle.add_to_queue(uri))
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
    run_spirc_command(spirc, out_error, move |handle| handle.load(request))
}

/// Reports whether the connect session is currently active/connected.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_spirc_is_connected(spirc: *const cspot_spirc_t) -> bool {
    match snapshot_from_spirc(spirc) {
        Some(snapshot) => snapshot.connected,
        None => false,
    }
}

/// Returns the current playback state.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_spirc_playback_state(
    spirc: *const cspot_spirc_t,
) -> cspot_playback_state_t {
    match snapshot_from_spirc(spirc) {
        Some(snapshot) => snapshot.playback_state.into(),
        None => cspot_playback_state_t::CSPOT_PLAYBACK_STATE_INVALID,
    }
}

/// Returns the current playback position in milliseconds.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_spirc_current_position_ms(spirc: *const cspot_spirc_t) -> u32 {
    match snapshot_from_spirc(spirc) {
        Some(snapshot) => snapshot.position_ms,
        None => 0,
    }
}

/// Returns the current track duration in milliseconds.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_spirc_current_track_duration_ms(spirc: *const cspot_spirc_t) -> u32 {
    match snapshot_from_spirc(spirc) {
        Some(snapshot) => snapshot.duration_ms,
        None => 0,
    }
}

/// Returns the current volume.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_spirc_current_volume(spirc: *const cspot_spirc_t) -> u16 {
    match snapshot_from_spirc(spirc) {
        Some(snapshot) => snapshot.volume,
        None => 0,
    }
}

/// Returns whether shuffle mode is currently enabled.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_spirc_is_shuffle_enabled(spirc: *const cspot_spirc_t) -> bool {
    match snapshot_from_spirc(spirc) {
        Some(snapshot) => snapshot.shuffle_enabled,
        None => false,
    }
}

/// Returns whether repeat-context mode is currently enabled.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_spirc_is_repeat_context_enabled(spirc: *const cspot_spirc_t) -> bool {
    match snapshot_from_spirc(spirc) {
        Some(snapshot) => snapshot.repeat_context_enabled,
        None => false,
    }
}

/// Returns whether repeat-track mode is currently enabled.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_spirc_is_repeat_track_enabled(spirc: *const cspot_spirc_t) -> bool {
    match snapshot_from_spirc(spirc) {
        Some(snapshot) => snapshot.repeat_track_enabled,
        None => false,
    }
}

/// Returns the current track Spotify ID, if available.
///
/// The returned string is heap-allocated and must be freed with `cspot_string_free`.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_spirc_current_track_id(spirc: *const cspot_spirc_t) -> *mut c_char {
    let value = snapshot_from_spirc(spirc).and_then(|snapshot| snapshot.track.spotify_id);
    string_to_owned_ptr(value)
}

/// Returns the current track Spotify URI, if available.
///
/// The returned string is heap-allocated and must be freed with `cspot_string_free`.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_spirc_current_track_uri(spirc: *const cspot_spirc_t) -> *mut c_char {
    let value = snapshot_from_spirc(spirc).and_then(|snapshot| snapshot.track.uri);
    string_to_owned_ptr(value)
}

/// Returns the current track artist list, if available.
///
/// The returned string is heap-allocated and must be freed with `cspot_string_free`.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_spirc_current_track_artist(spirc: *const cspot_spirc_t) -> *mut c_char {
    let value = snapshot_from_spirc(spirc).and_then(|snapshot| snapshot.track.artist);
    string_to_owned_ptr(value)
}

/// Returns the current track album or show name, if available.
///
/// The returned string is heap-allocated and must be freed with `cspot_string_free`.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_spirc_current_track_album(spirc: *const cspot_spirc_t) -> *mut c_char {
    let value = snapshot_from_spirc(spirc).and_then(|snapshot| snapshot.track.album);
    string_to_owned_ptr(value)
}

/// Returns the current track artwork URL, if available.
///
/// The returned string is heap-allocated and must be freed with `cspot_string_free`.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_spirc_current_track_artwork_url(
    spirc: *const cspot_spirc_t,
) -> *mut c_char {
    let value = snapshot_from_spirc(spirc).and_then(|snapshot| snapshot.track.artwork_url);
    string_to_owned_ptr(value)
}

/// Returns the current track title, if available.
///
/// The returned string is heap-allocated and must be freed with `cspot_string_free`.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_spirc_current_track_title(spirc: *const cspot_spirc_t) -> *mut c_char {
    let value = snapshot_from_spirc(spirc).and_then(|snapshot| snapshot.track.title);
    string_to_owned_ptr(value)
}

/// Requests a clean Connect shutdown.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_spirc_shutdown(
    spirc: *const cspot_spirc_t,
    out_error: *mut *mut cspot_error_t,
) -> bool {
    run_spirc_command(spirc, out_error, Spirc::shutdown)
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
    let handle = unsafe { Box::from_raw(spirc as *mut SpircHandle) };
    handle.status_task.abort();
}
