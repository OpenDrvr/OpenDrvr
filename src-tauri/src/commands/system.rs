use serde::Serialize;
use tauri::State;

use crate::error::{CommandError, CommandResult};
use crate::state::{AppState, ModuleStatus};
use crate::utils;

const MAX_PING_MESSAGE_CHARS: usize = 128;

/// Aggregate backend status used by diagnostics screens.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackendStatus {
    pub version: &'static str,
    pub platform: &'static str,
    pub supported_target: bool,
    pub modules: Vec<ModuleStatus>,
}

#[tauri::command]
pub fn get_backend_status(state: State<'_, AppState>) -> BackendStatus {
    BackendStatus {
        version: env!("CARGO_PKG_VERSION"),
        platform: crate::platform::name(),
        supported_target: crate::platform::is_supported(),
        modules: state.module_status(),
    }
}

/// Harmless round-trip payload that demonstrates the command pattern,
/// including input validation and typed error handling.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PingResponse {
    pub message: String,
    pub backend: &'static str,
}

#[tauri::command]
pub fn ping(message: Option<String>) -> CommandResult<PingResponse> {
    let message = message.unwrap_or_else(|| "pong".to_string());
    utils::validate_user_text(&message, MAX_PING_MESSAGE_CHARS).map_err(CommandError::from)?;

    Ok(PingResponse {
        message,
        backend: "opendrvr",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ping_defaults_to_pong() {
        let response = ping(None).expect("ping");
        assert_eq!(response.message, "pong");
    }

    #[test]
    fn ping_accepts_bounded_input() {
        let response = ping(Some("hello".to_string())).expect("ping");
        assert_eq!(response.message, "hello");
    }

    #[test]
    fn ping_rejects_oversized_input() {
        let response = ping(Some("x".repeat(MAX_PING_MESSAGE_CHARS + 1)));
        assert!(response.is_err());
        assert_eq!(response.unwrap_err().code, "validation");
    }
}
