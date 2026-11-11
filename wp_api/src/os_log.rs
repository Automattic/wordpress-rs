//! Sends `log` records to Apple's unified logging system.
//!
//! `os_log_with_type` is a C macro, so the call goes through `wp_api_os_log_with_type`, defined in
//! `native/swift/Sources/wordpress-api-os-log`. The Swift package compiles it when an app is built,
//! so building the xcframework doesn't need the Apple SDK. Rust-only builds (`cargo test`, the CLI)
//! compile it in `build.rs` instead.

use log::{Level, LevelFilter, Log, Metadata, Record};
use std::collections::HashMap;
use std::ffi::{CString, c_char};
use std::sync::Mutex;

#[repr(C)]
struct OsLogObject {
    _private: [u8; 0],
}

type OsLogRef = *mut OsLogObject;

const OS_LOG_TYPE_DEFAULT: u8 = 0x00;
const OS_LOG_TYPE_INFO: u8 = 0x01;
const OS_LOG_TYPE_DEBUG: u8 = 0x02;
const OS_LOG_TYPE_ERROR: u8 = 0x10;
const OS_LOG_TYPE_FAULT: u8 = 0x11;

unsafe extern "C" {
    fn os_log_create(subsystem: *const c_char, category: *const c_char) -> OsLogRef;
    fn wp_api_os_log_with_type(log: OsLogRef, log_type: u8, message: *const c_char);
}

/// An `os_log_t`. It's never released: one is created per category and kept for the life of the
/// process, as `os_log_create` intends.
struct OsLog(OsLogRef);

// SAFETY: `os_log_t` objects are thread-safe.
unsafe impl Send for OsLog {}

pub(crate) struct OsLogger {
    subsystem: CString,
    // One log per `log` target, which becomes the os_log category.
    logs: Mutex<HashMap<String, OsLog>>,
}

impl OsLogger {
    pub(crate) fn init(subsystem: &str, level: LevelFilter) -> Result<(), log::SetLoggerError> {
        let logger = Self {
            subsystem: to_c_string(subsystem),
            logs: Mutex::new(HashMap::new()),
        };
        // Leaked: the logger lives for the rest of the process, as `log` requires.
        log::set_logger(Box::leak(Box::new(logger)))?;
        log::set_max_level(level);
        Ok(())
    }
}

impl Log for OsLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let message = to_c_string(&record.args().to_string());
        let mut logs = self.logs.lock().unwrap_or_else(|e| e.into_inner());
        let log = logs.entry(record.target().to_string()).or_insert_with(|| {
            let category = to_c_string(record.target());
            // SAFETY: both arguments are valid NUL-terminated strings.
            OsLog(unsafe { os_log_create(self.subsystem.as_ptr(), category.as_ptr()) })
        });
        // SAFETY: `log.0` came from `os_log_create` and `message` is a valid NUL-terminated string.
        unsafe { wp_api_os_log_with_type(log.0, os_log_type(record.level()), message.as_ptr()) };
    }

    fn flush(&self) {}
}

// The same mapping the `oslog` crate used, so log levels in Console are unchanged.
fn os_log_type(level: Level) -> u8 {
    match level {
        Level::Trace => OS_LOG_TYPE_DEBUG,
        Level::Debug => OS_LOG_TYPE_INFO,
        Level::Info => OS_LOG_TYPE_DEFAULT,
        Level::Warn => OS_LOG_TYPE_ERROR,
        Level::Error => OS_LOG_TYPE_FAULT,
    }
}

fn to_c_string(s: &str) -> CString {
    CString::new(s.replace('\0', "(null)")).expect("interior NUL bytes were replaced")
}
