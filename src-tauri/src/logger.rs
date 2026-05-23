use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

/// Maximum log file size before rotation (5 MB).
const MAX_LOG_SIZE: u64 = 5 * 1024 * 1024;
/// Maximum number of rotated log files to keep.
const MAX_LOG_FILES: usize = 5;

/// Global logger instance.
static LOGGER: OnceLock<FileLogger> = OnceLock::new();

/// Get the global logger instance. Initializes on first call.
pub fn logger() -> &'static FileLogger {
    LOGGER.get_or_init(FileLogger::init)
}

/// A simple file logger with log rotation.
///
/// Writes structured log lines to a file in the `data/logs/` directory.
/// When the current log file exceeds `MAX_LOG_SIZE`, it is rotated
/// (renamed to `.1`, `.2`, etc.) and a new file is created.
pub struct FileLogger {
    log_dir: PathBuf,
    file: Mutex<File>,
}

impl FileLogger {
    /// Create a new logger that writes to `data/logs/rustverse.log`.
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

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .unwrap_or_else(|e| {
                eprintln!("Failed to open log file {:?}: {e}", log_path);
                // Fallback: try a temp location
                let fallback = std::env::temp_dir().join("rustverse.log");
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&fallback)
                    .expect("cannot open any log file")
            });

        Self {
            log_dir,
            file: Mutex::new(file),
        }
    }

    /// Write a log line with the given level.
    pub fn log(&self, level: &str, module: &str, message: &str) {
        let timestamp = chrono_now();
        let line = format!("[{timestamp}] [{level:>5}] [{module}] {message}\n");

        if let Ok(mut file) = self.file.lock() {
            let _ = file.write_all(line.as_bytes());
            let _ = file.flush();

            // Check if rotation is needed
            if let Ok(metadata) = file.metadata() {
                if metadata.len() > MAX_LOG_SIZE {
                    drop(file); // release lock before rotation
                    rotate_logs(&self.log_dir);
                    // Re-open the file after rotation
                    if let Ok(mut guard) = self.file.lock() {
                        let log_path = self.log_dir.join("rustverse.log");
                        if let Ok(new_file) = OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(&log_path)
                        {
                            *guard = new_file;
                        }
                    }
                }
            }
        }
    }

    /// Convenience methods for log levels.
    #[allow(dead_code)]
    pub fn debug(&self, module: &str, message: &str) {
        self.log("DEBUG", module, message);
    }

    pub fn info(&self, module: &str, message: &str) {
        self.log("INFO", module, message);
    }

    #[allow(dead_code)]
    pub fn warn(&self, module: &str, message: &str) {
        self.log("WARN", module, message);
    }

    pub fn error(&self, module: &str, message: &str) {
        self.log("ERROR", module, message);
    }

    /// Get the log directory path (for frontend to display).
    pub fn log_dir(&self) -> &PathBuf {
        &self.log_dir
    }
}

/// Get the log directory: `<exe_dir>/data/logs/`.
fn get_log_dir() -> PathBuf {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            return parent.join("data").join("logs");
        }
    }
    PathBuf::from("data").join("logs")
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
