//! Small, dependency-free helpers shared by the backend.

use crate::error::{AppError, AppResult};

/// Validates text received from the frontend before it is used or logged.
///
/// Commands must never trust IPC input. This rejects oversized values and
/// non-printable control characters while allowing normal whitespace.
pub fn validate_user_text(value: &str, max_chars: usize) -> AppResult<()> {
    if value.chars().count() > max_chars {
        return Err(AppError::Validation(format!(
            "value exceeds {max_chars} characters"
        )));
    }

    if value
        .chars()
        .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    {
        return Err(AppError::Validation(
            "value contains unsupported control characters".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_normal_text() {
        assert!(validate_user_text("GPU driver 1.2.3", 64).is_ok());
    }

    #[test]
    fn rejects_oversized_text() {
        assert!(validate_user_text(&"a".repeat(65), 64).is_err());
    }

    #[test]
    fn rejects_control_characters() {
        assert!(validate_user_text("bad\u{0}value", 64).is_err());
    }
}
