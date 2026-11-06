use tauri::State;

use crate::error::{AppError, CommandResult};
use crate::hardware::model::HardwareSnapshot;
use crate::state::AppState;

/// Enumerates the local device tree, including non-present devices.
///
/// Enumeration blocks on Configuration Manager, so it runs on a blocking worker
/// instead of the async runtime. Driver matching and catalog lookup are not
/// performed here.
#[tauri::command]
pub async fn scan_hardware(state: State<'_, AppState>) -> CommandResult<HardwareSnapshot> {
    let hardware = state.hardware.clone();

    let snapshot = tauri::async_runtime::spawn_blocking(move || hardware.scan())
        .await
        .map_err(|error| AppError::Hardware(format!("scan task failed: {error}")))?
        .map_err(|error| AppError::Hardware(error.to_string()))?;

    Ok(snapshot)
}

#[cfg(test)]
mod tests {
    use crate::error::CommandError;
    use crate::hardware::provider::HardwareError;

    use super::*;

    #[test]
    fn hardware_failures_are_sanitized_for_the_frontend() {
        let error = AppError::Hardware("CM_Get_Device_ID_ListW failed with code 5".to_string());
        let payload = CommandError::from(error);

        assert_eq!(payload.code, "hardware");
        assert_eq!(payload.message, "Hardware detection failed.");
        assert!(!payload.message.contains("CM_Get_Device_ID_ListW"));
    }

    #[test]
    fn provider_errors_map_to_hardware_errors() {
        let error = HardwareError::ConfigManager {
            operation: "CM_Get_Device_ID_ListW",
            code: 5,
        };
        let app_error = AppError::Hardware(error.to_string());

        assert_eq!(app_error.code(), "hardware");
    }
}
