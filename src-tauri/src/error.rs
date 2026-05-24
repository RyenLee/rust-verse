use serde::Serialize;

/// Application-level error type.
///
/// All Tauri commands return `Result<T, AppError>`, and `AppError` must implement
/// `Serialize` so the frontend can receive structured error information.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("command execution failed: {0}")]
    Command(String),

    #[error("command timed out after {0}s")]
    Timeout(u64),

    #[error("binary not found: {0}")]
    BinaryNotFound(String),

    #[error("parse error: {0}")]
    #[allow(dead_code)]
    Parse(String),

    #[error("configuration error: {0}")]
    Config(String),

    #[error("network error: {0}")]
    Network(String),

    #[error("integrity check failed: {0}")]
    Integrity(String),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        struct ErrorResponse {
            kind: String,
            message: String,
        }

        let kind = match self {
            AppError::Command(_) => "command",
            AppError::Timeout(_) => "timeout",
            AppError::BinaryNotFound(_) => "binary_not_found",
            AppError::Parse(_) => "parse",
            AppError::Config(_) => "config",
            AppError::Network(_) => "network",
            AppError::Integrity(_) => "integrity",
        };

        ErrorResponse {
            kind: kind.to_string(),
            message: self.to_string(),
        }
        .serialize(serializer)
    }
}

pub type AppResult<T> = std::result::Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = AppError::Command("test error".to_string());
        assert_eq!(err.to_string(), "command execution failed: test error");

        let err = AppError::Timeout(30);
        assert_eq!(err.to_string(), "command timed out after 30s");

        let err = AppError::BinaryNotFound("rustup".to_string());
        assert_eq!(err.to_string(), "binary not found: rustup");

        let err = AppError::Parse("bad format".to_string());
        assert_eq!(err.to_string(), "parse error: bad format");

        let err = AppError::Config("missing key".to_string());
        assert_eq!(err.to_string(), "configuration error: missing key");
    }

    #[test]
    fn test_error_serialize() {
        let err = AppError::Command("fail".to_string());
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("\"kind\":\"command\""));
        assert!(json.contains("\"message\":\"command execution failed: fail\""));

        let err = AppError::BinaryNotFound("cargo".to_string());
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("\"kind\":\"binary_not_found\""));

        let err = AppError::Timeout(60);
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("\"kind\":\"timeout\""));
    }

    #[test]
    fn test_app_result_ok() {
        let result: AppResult<i32> = Ok(42);
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_app_result_err() {
        let result: AppResult<i32> = Err(AppError::Command("boom".to_string()));
        assert!(result.is_err());
    }
}
