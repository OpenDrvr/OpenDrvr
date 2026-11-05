use serde::Serialize;
use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;
pub type CommandResult<T> = Result<T, CommandError>;

/// Errors that can occur inside the Rust backend.
///
/// Commands convert these into [`CommandError`] before crossing the IPC
/// boundary, so paths, OS error strings and other implementation details stay
/// in the structured logs instead of being exposed to the frontend.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("configuration error: {0}")]
    Config(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("validation error: {0}")]
    Validation(String),

    #[error("database error: {0}")]
    Database(String),

    #[error("internal error: {0}")]
    Internal(String),
}

impl AppError {
    /// Stable machine-readable code for frontend error handling.
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Config(_) => "config",
            Self::Io(_) => "io",
            Self::Serialization(_) => "serialization",
            Self::Validation(_) => "validation",
            Self::Database(_) => "database",
            Self::Internal(_) => "internal",
        }
    }

    /// Safe, user-facing description. Detailed errors are logged separately.
    pub const fn public_message(&self) -> &'static str {
        match self {
            Self::Config(_) => "The application configuration is invalid.",
            Self::Io(_) => "A file system operation failed.",
            Self::Serialization(_) => "A data serialization operation failed.",
            Self::Validation(_) => "The request contained invalid input.",
            Self::Database(_) => "A database operation failed.",
            Self::Internal(_) => "An internal backend error occurred.",
        }
    }
}

/// Serializable error payload returned by Tauri commands.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub code: &'static str,
    pub message: &'static str,
}

impl From<AppError> for CommandError {
    fn from(error: AppError) -> Self {
        tracing::error!(
            target: "opendrvr::error",
            code = error.code(),
            error = %error,
            "backend command failed"
        );

        Self {
            code: error.code(),
            message: error.public_message(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_error_does_not_expose_internal_details() {
        let error = AppError::Internal(r"C:\private\driver-cache".to_string());
        let payload = CommandError::from(error);

        assert_eq!(payload.code, "internal");
        assert_eq!(payload.message, "An internal backend error occurred.");
        assert!(!payload.message.contains("driver-cache"));
    }

    #[test]
    fn validation_errors_have_a_stable_code() {
        let error = AppError::Validation("message too long".to_string());
        assert_eq!(error.code(), "validation");
        assert_eq!(CommandError::from(error).code, "validation");
    }
}
