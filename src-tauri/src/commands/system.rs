use serde::Serialize;
use tauri::State;

use crate::state::{AppState, ModuleStatus};

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
