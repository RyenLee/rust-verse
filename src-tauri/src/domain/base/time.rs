
/// Current Unix timestamp in milliseconds.
///
/// Used across persistence and infrastructure layers for consistent
/// `created_at` values on notifications and other domain records.
pub fn chrono_now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}