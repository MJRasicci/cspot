//! Logging configuration for cspot's C bindings.

use std::ffi::CStr;
use std::os::raw::{c_char, c_void};
use std::ptr;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Once, RwLock};

use log::{Level, LevelFilter, Log, Metadata, Record};
use once_cell::sync::Lazy;

use crate::error::{clear_error, cspot_error_t, cstring_from_str_lossy, write_error};

const LOGGER_STATE_UNINIT: u8 = 0;
const LOGGER_STATE_READY: u8 = 1;
const LOGGER_STATE_FAILED: u8 = 2;

static LOGGER_STATE: AtomicU8 = AtomicU8::new(LOGGER_STATE_UNINIT);
static LOGGER_INIT: Once = Once::new();
static CSPOT_LOGGER: Lazy<CspotLogger> = Lazy::new(CspotLogger::new);

/// Log level values for cspot logging.
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(C)]
pub enum cspot_log_level_t {
    CSPOT_LOG_LEVEL_OFF = 0,
    CSPOT_LOG_LEVEL_ERROR = 1,
    CSPOT_LOG_LEVEL_WARN = 2,
    CSPOT_LOG_LEVEL_INFO = 3,
    CSPOT_LOG_LEVEL_DEBUG = 4,
    CSPOT_LOG_LEVEL_TRACE = 5,
}

impl From<cspot_log_level_t> for LevelFilter {
    fn from(value: cspot_log_level_t) -> Self {
        match value {
            cspot_log_level_t::CSPOT_LOG_LEVEL_OFF => LevelFilter::Off,
            cspot_log_level_t::CSPOT_LOG_LEVEL_ERROR => LevelFilter::Error,
            cspot_log_level_t::CSPOT_LOG_LEVEL_WARN => LevelFilter::Warn,
            cspot_log_level_t::CSPOT_LOG_LEVEL_INFO => LevelFilter::Info,
            cspot_log_level_t::CSPOT_LOG_LEVEL_DEBUG => LevelFilter::Debug,
            cspot_log_level_t::CSPOT_LOG_LEVEL_TRACE => LevelFilter::Trace,
        }
    }
}

impl From<Level> for cspot_log_level_t {
    fn from(value: Level) -> Self {
        match value {
            Level::Error => cspot_log_level_t::CSPOT_LOG_LEVEL_ERROR,
            Level::Warn => cspot_log_level_t::CSPOT_LOG_LEVEL_WARN,
            Level::Info => cspot_log_level_t::CSPOT_LOG_LEVEL_INFO,
            Level::Debug => cspot_log_level_t::CSPOT_LOG_LEVEL_DEBUG,
            Level::Trace => cspot_log_level_t::CSPOT_LOG_LEVEL_TRACE,
        }
    }
}

/// Structured log record delivered to a C callback.
///
/// String pointers are only valid for the duration of the callback and must not be retained.
/// `module_path` and `file` may be null when unavailable. `line` is 0 when unknown.
#[repr(C)]
pub struct cspot_log_record_t {
    pub level: cspot_log_level_t,
    pub target: *const c_char,
    pub message: *const c_char,
    pub module_path: *const c_char,
    pub file: *const c_char,
    pub line: u32,
}

/// Callback invoked for each log record emitted by cspot.
///
/// The callback may be invoked from any thread that emits a log record.
#[allow(non_camel_case_types)]
pub type cspot_log_callback_t =
    Option<extern "C" fn(record: *const cspot_log_record_t, user_data: *mut c_void)>;

/// Configuration for initializing cspot logging.
///
/// If `filter` is non-null, it is interpreted as an `RUST_LOG`-style filter string and
/// overrides `level`. If `filter` is null and `RUST_LOG` is set in the environment, the
/// environment value is used. Otherwise `level` is applied to librespot/libmdns logs.
/// If `callback` is null, logs are written to stderr. Otherwise they are delivered to the
/// callback with `user_data` forwarded unchanged.
#[repr(C)]
pub struct cspot_log_config_t {
    pub level: cspot_log_level_t,
    pub filter: *const c_char,
    pub callback: cspot_log_callback_t,
    pub user_data: *mut c_void,
}

#[derive(Clone)]
struct TargetFilter {
    target: String,
    level: LevelFilter,
}

#[derive(Clone)]
struct LogFilter {
    default: LevelFilter,
    directives: Vec<TargetFilter>,
}

impl LogFilter {
    fn default_for_level(level: LevelFilter) -> Self {
        Self {
            default: LevelFilter::Off,
            directives: vec![
                TargetFilter {
                    target: "librespot".to_string(),
                    level,
                },
            ],
        }
    }

    fn parse(spec: &str) -> Result<Self, String> {
        let mut default = LevelFilter::Off;
        let mut directives = Vec::new();

        for (index, raw) in spec.split(',').enumerate() {
            let directive = raw.trim();
            if directive.is_empty() {
                continue;
            }
            let mut parts = directive.splitn(2, '=');
            let left = parts.next().unwrap_or_default().trim();
            let right = parts.next().map(str::trim);

            if left.is_empty() {
                return Err(format!("empty log directive at position {index}"));
            }

            if let Some(level_str) = right {
                if level_str.is_empty() {
                    return Err(format!("missing log level for target `{left}`"));
                }
                let level =
                    parse_level(level_str).ok_or_else(|| format!("invalid level `{level_str}`"))?;
                directives.push(TargetFilter {
                    target: left.to_string(),
                    level,
                });
            } else if let Some(level) = parse_level(left) {
                default = level;
            } else {
                directives.push(TargetFilter {
                    target: left.to_string(),
                    level: LevelFilter::Trace,
                });
            }
        }

        Ok(Self { default, directives })
    }

    fn enabled(&self, metadata: &Metadata) -> bool {
        let target = metadata.target();
        let mut best_level = self.default;
        let mut best_len = 0usize;

        for directive in &self.directives {
            if target.starts_with(&directive.target) {
                let len = directive.target.len();
                if len >= best_len {
                    best_len = len;
                    best_level = directive.level;
                }
            }
        }

        let record_level = metadata.level().to_level_filter();
        record_level <= best_level
    }

    fn max_level(&self) -> LevelFilter {
        let mut max_level = self.default;
        for directive in &self.directives {
            if directive.level > max_level {
                max_level = directive.level;
            }
        }
        max_level
    }
}

struct LoggerConfig {
    filter: LogFilter,
    callback: cspot_log_callback_t,
    user_data: usize,
}

impl LoggerConfig {
    fn new(filter: LogFilter, callback: cspot_log_callback_t, user_data: usize) -> Self {
        Self {
            filter,
            callback,
            user_data,
        }
    }
}

struct CspotLogger {
    config: RwLock<LoggerConfig>,
}

impl CspotLogger {
    fn new() -> Self {
        Self {
            config: RwLock::new(LoggerConfig::new(
                LogFilter::default_for_level(LevelFilter::Info),
                None,
                0,
            )),
        }
    }

    fn update(&self, config: LoggerConfig) {
        let mut guard = self
            .config
            .write()
            .unwrap_or_else(|err| err.into_inner());
        *guard = config;
    }

    fn with_config<T>(&self, f: impl FnOnce(&LoggerConfig) -> T) -> T {
        let guard = self.config.read().unwrap_or_else(|err| err.into_inner());
        f(&guard)
    }
}

impl Log for CspotLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.with_config(|config| config.filter.enabled(metadata))
    }

    fn log(&self, record: &Record) {
        let (callback, user_data, enabled) = self.with_config(|config| {
            (
                config.callback,
                config.user_data,
                config.filter.enabled(record.metadata()),
            )
        });

        if !enabled {
            return;
        }

        if let Some(callback) = callback {
            let user_data = user_data as *mut c_void;
            let level = cspot_log_level_t::from(record.level());
            let target = cstring_from_str_lossy(record.target());
            let message = cstring_from_str_lossy(&record.args().to_string());
            let module_path = record.module_path().map(cstring_from_str_lossy);
            let file = record.file().map(cstring_from_str_lossy);
            let record = cspot_log_record_t {
                level,
                target: target.as_ptr(),
                message: message.as_ptr(),
                module_path: module_path.as_ref().map_or(ptr::null(), |value| value.as_ptr()),
                file: file.as_ref().map_or(ptr::null(), |value| value.as_ptr()),
                line: record.line().unwrap_or(0),
            };
            callback(&record, user_data);
        } else {
            eprintln!(
                "{} {}: {}",
                record.level(),
                record.target(),
                record.args()
            );
        }
    }

    fn flush(&self) {}
}

fn parse_level(value: &str) -> Option<LevelFilter> {
    match value.trim().to_ascii_lowercase().as_str() {
        "off" => Some(LevelFilter::Off),
        "error" => Some(LevelFilter::Error),
        "warn" | "warning" => Some(LevelFilter::Warn),
        "info" => Some(LevelFilter::Info),
        "debug" => Some(LevelFilter::Debug),
        "trace" => Some(LevelFilter::Trace),
        _ => None,
    }
}

fn read_optional_cstr(value: *const c_char) -> Option<String> {
    if value.is_null() {
        return None;
    }
    // Safety: caller guarantees a valid, NUL-terminated C string.
    let cstr = unsafe { CStr::from_ptr(value) };
    Some(cstr.to_string_lossy().into_owned())
}

fn resolve_filter(config: Option<&cspot_log_config_t>) -> Result<LogFilter, String> {
    if let Some(config) = config {
        if let Some(filter) = read_optional_cstr(config.filter) {
            return LogFilter::parse(&filter)
                .map_err(|err| format!("invalid log filter `{filter}`: {err}"));
        }
    }

    if let Ok(filter) = std::env::var("RUST_LOG") {
        return LogFilter::parse(&filter)
            .map_err(|err| format!("invalid RUST_LOG value `{filter}`: {err}"));
    }

    let level = config
        .map(|config| config.level)
        .unwrap_or(cspot_log_level_t::CSPOT_LOG_LEVEL_INFO);
    Ok(LogFilter::default_for_level(level.into()))
}

fn ensure_logger(out_error: *mut *mut cspot_error_t) -> bool {
    LOGGER_INIT.call_once(|| {
        if log::set_logger(&*CSPOT_LOGGER).is_ok() {
            LOGGER_STATE.store(LOGGER_STATE_READY, Ordering::SeqCst);
        } else {
            LOGGER_STATE.store(LOGGER_STATE_FAILED, Ordering::SeqCst);
        }
    });

    match LOGGER_STATE.load(Ordering::SeqCst) {
        LOGGER_STATE_READY => true,
        LOGGER_STATE_FAILED => {
            write_error(out_error, "logging already initialized by another logger");
            false
        }
        LOGGER_STATE_UNINIT => {
            write_error(out_error, "logging failed to initialize");
            false
        }
        _ => {
            write_error(out_error, "logging is in an unknown state");
            false
        }
    }
}

/// Initializes default logging configuration values.
///
/// The defaults select INFO logging for librespot and use no callback.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_log_config_init(config: *mut cspot_log_config_t) {
    if config.is_null() {
        return;
    }
    // Safety: caller provided a writable config pointer.
    unsafe {
        *config = cspot_log_config_t {
            level: cspot_log_level_t::CSPOT_LOG_LEVEL_INFO,
            filter: ptr::null(),
            callback: None,
            user_data: ptr::null_mut(),
        };
    }
}

/// Initializes logging for cspot.
///
/// If `config` is null, defaults are used. This function may be called multiple
/// times to update the logging configuration after initialization.
#[unsafe(no_mangle)]
pub extern "C" fn cspot_log_init(
    config: *const cspot_log_config_t,
    out_error: *mut *mut cspot_error_t,
) -> bool {
    clear_error(out_error);

    let config = unsafe { config.as_ref() };
    let filter = match resolve_filter(config) {
        Ok(filter) => filter,
        Err(message) => {
            write_error(out_error, message);
            return false;
        }
    };

    if !ensure_logger(out_error) {
        return false;
    }

    let callback = config.and_then(|config| config.callback);
    let user_data = config.map(|config| config.user_data as usize).unwrap_or(0);

    let max_level = filter.max_level();
    CSPOT_LOGGER.update(LoggerConfig::new(filter, callback, user_data));
    log::set_max_level(max_level);
    true
}
