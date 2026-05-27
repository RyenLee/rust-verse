//! Mirror management business logic (crm validation).

use crate::domain::error::AppResult;

/// Validate mirror name to prevent command injection.
pub fn validate_mirror_name(name: &str) -> AppResult<()> {
    if name.is_empty() {
        return Err(crate::domain::error::AppError::Command(
            "mirror name cannot be empty".to_string(),
        ));
    }
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(crate::domain::error::AppError::Command(format!(
            "invalid mirror name '{name}': only alphanumeric, hyphen and underscore allowed"
        )));
    }
    Ok(())
}

/// Validate the best mode parameter.
pub fn validate_best_mode(mode: &str) -> AppResult<()> {
    match mode {
        "" | "git" | "sparse" | "git-download" | "sparse-download" => Ok(()),
        other => Err(crate::domain::error::AppError::Command(format!(
            "invalid best mode '{other}': allowed values are '', 'git', 'sparse', 'git-download', 'sparse-download'"
        ))),
    }
}