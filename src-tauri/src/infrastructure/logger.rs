use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

/// Maximum log file size before rotation (5 MB).
const MAX_LOG_SIZE: u64 = 5 * 1024 * 1024;
/// Maximum number of rotated log files to keep.
const MAX_LOG_FILES: usize = 5;

/// Log level enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "trace" => Some(LogLevel::Trace),
            "debug" => Some(LogLevel::Debug),
            "info" => Some(LogLevel::Info),
            "warn" => Some(LogLevel::Warn),
            "error" => Some(LogLevel::Error),
            _ => None,
        }
    }
}

/// Global logger instance.
static LOGGER: OnceLock<FileLogger> = OnceLock::new();

/// Global minimum log level (can be changed at runtime).
static MIN_LOG_LEVEL: OnceLock<Mutex<LogLevel>> = OnceLock::new();

/// Get the global logger instance. Initializes on first call.
pub fn logger() -> &'static FileLogger {
    LOGGER.get_or_init(FileLogger::init)
}

/// Set the minimum log level for filtering.
pub fn set_min_log_level(level: LogLevel) {
    if let Some(lock) = MIN_LOG_LEVEL.get() {
        *lock.lock().unwrap_or_else(|e| e.into_inner()) = level;
    } else {
        MIN_LOG_LEVEL.set(Mutex::new(level)).ok();
    }
}

/// Get the default log level from RUST_LOG environment variable, or ERROR if not set.
fn default_log_level_from_env() -> LogLevel {
    std::env::var("RUST_LOG")
        .ok()
        .and_then(|val| LogLevel::from_str(&val))
        .unwrap_or(LogLevel::Error)
}

/// Get the current minimum log level.
pub fn get_min_log_level() -> LogLevel {
    MIN_LOG_LEVEL
        .get()
        .map(|lock| *lock.lock().unwrap_or_else(|e| e.into_inner()))
        .unwrap_or_else(default_log_level_from_env)
}

/// Get the current minimum log level as a string.
pub fn get_min_log_level_str() -> String {
    get_min_log_level().as_str().to_string()
}

/// Check if a log level should be logged.
fn should_log(level: LogLevel) -> bool {
    level >= get_min_log_level()
}

/// A simple file logger with log rotation.
///
/// Writes structured log lines to a file in the `logs/` directory.
/// When the current log file exceeds `MAX_LOG_SIZE`, it is rotated
/// (renamed to `.1`, `.2`, etc.) and a new file is created.
pub struct FileLogger {
    log_dir: PathBuf,
    file: Mutex<BufWriter<File>>,
}

impl FileLogger {
    /// Create a new logger that writes to `logs/rustverse.log`.
    fn init() -> Self {
        let log_dir = get_log_dir();
        fs::create_dir_all(&log_dir).ok();

        let log_path = log_dir.join("rustverse.log");

        // Rotate existing log if it's already too large
        if let Ok(metadata) = fs::metadata(&log_path) {
            if metadata.len() > MAX_LOG_SIZE {
                rotate_logs(&log_dir);
            }
        }

        let file = BufWriter::with_capacity(
            64 * 1024,
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_path)
                .unwrap_or_else(|e| {
                    let msg = format!("Failed to open log file {:?}: {e}", log_path);
                    let _ = std::io::stderr().write_all(msg.as_bytes());
                    let fallback = std::env::temp_dir().join("rustverse.log");
                    OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&fallback)
                        .expect("cannot open any log file")
                }),
        );

        Self {
            log_dir,
            file: Mutex::new(file),
        }
    }

    /// Write a log line with the given level.
    pub fn log(&self, level: LogLevel, module: &str, message: &str) {
        if !should_log(level) {
            return;
        }

        let timestamp = chrono_now();
        let line = format!(
            "[{timestamp}] [{:>5}] [{module}] {message}\n",
            level.as_str()
        );

        if let Ok(mut file) = self.file.lock() {
            let _ = file.write_all(line.as_bytes());

            if level >= LogLevel::Error {
                let _ = file.flush();
            }

            if let Ok(metadata) = file.get_ref().metadata() {
                if metadata.len() > MAX_LOG_SIZE {
                    let _ = file.flush();
                    drop(file);
                    rotate_logs(&self.log_dir);
                    if let Ok(mut guard) = self.file.lock() {
                        let log_path = self.log_dir.join("rustverse.log");
                        if let Ok(new_file) =
                            OpenOptions::new().create(true).append(true).open(&log_path)
                        {
                            *guard = BufWriter::with_capacity(64 * 1024, new_file);
                        }
                    }
                }
            }
        }
    }

    /// Convenience methods for log levels.
    pub fn debug(&self, module: &str, message: &str) {
        self.log(LogLevel::Debug, module, message);
    }

    pub fn info(&self, module: &str, message: &str) {
        self.log(LogLevel::Info, module, message);
    }

    pub fn warn(&self, module: &str, message: &str) {
        self.log(LogLevel::Warn, module, message);
    }

    pub fn error(&self, module: &str, message: &str) {
        self.log(LogLevel::Error, module, message);
    }

    /// Log an API request with parameters.
    pub fn log_request(&self, command: &str, params: &str) {
        self.info(
            "api",
            &format!("[REQUEST] {} - params: {}", command, params),
        );
    }

    /// Log from string level (for Tauri frontend_log command).
    pub fn log_from_str(&self, level: &str, module: &str, message: &str) {
        let log_level = LogLevel::from_str(level).unwrap_or(LogLevel::Info);
        self.log(log_level, module, message);
    }

    /// Get the log directory path (for frontend to display).
    pub fn log_dir(&self) -> &PathBuf {
        &self.log_dir
    }
}

/// Get the log directory: `<exe_dir>/logs/`.
fn get_log_dir() -> PathBuf {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            return parent.join("logs");
        }
    }
    PathBuf::from("logs")
}

/// Rotate log files: `rustverse.log` → `rustverse.log.1`, etc.
fn rotate_logs(log_dir: &PathBuf) {
    // Delete the oldest rotated file
    let oldest = log_dir.join(format!("rustverse.log.{MAX_LOG_FILES}"));
    let _ = fs::remove_file(&oldest);

    // Shift rotated files: .4 → .5, .3 → .4, etc.
    for i in (2..=MAX_LOG_FILES).rev() {
        let from = log_dir.join(format!("rustverse.log.{}", i - 1));
        let to = log_dir.join(format!("rustverse.log.{i}"));
        if from.exists() {
            let _ = fs::rename(&from, &to);
        }
    }

    // Rotate current log → .1
    let current = log_dir.join("rustverse.log");
    let first = log_dir.join("rustverse.log.1");
    if current.exists() {
        let _ = fs::rename(&current, &first);
    }
}

/// Simple timestamp without external dependencies.
fn chrono_now() -> String {
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();

    let total_secs = duration.as_secs();
    let days = total_secs / 86400;
    let time_of_day = total_secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;
    let millis = duration.subsec_millis();

    // Days since epoch to date (simplified, no timezone)
    let (year, month, day) = days_to_date(days as i32);

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}",
        year, month, day, hours, minutes, seconds, millis
    )
}

/// Convert days since Unix epoch to (year, month, day).
fn days_to_date(days: i32) -> (i32, u32, u32) {
    // Algorithm from http://howardhinnant.github.io/date_algorithms.html
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m as u32, d as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_days_to_date() {
        // 1970-01-01 = day 0
        let (y, m, d) = days_to_date(0);
        assert_eq!((y, m, d), (1970, 1, 1));

        // 2000-01-01
        let (y, m, d) = days_to_date(10957);
        assert_eq!((y, m, d), (2000, 1, 1));
    }
}