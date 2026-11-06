use tauri::State;

use crate::error::{CommandError, CommandResult};
use crate::preferences::AppPreferences;
use crate::state::AppState;

/// Returns the persisted application preferences, or application defaults when
/// nothing has been saved yet.
#[tauri::command]
pub async fn get_preferences(state: State<'_, AppState>) -> CommandResult<AppPreferences> {
    state
        .database
        .get_preferences()
        .await
        .map_err(CommandError::from)
}

/// Validates and persists the complete preference set. The returned value is
/// the canonical stored representation, including any normalization applied by
/// the backend.
#[tauri::command]
pub async fn update_preferences(
    preferences: AppPreferences,
    state: State<'_, AppState>,
) -> CommandResult<AppPreferences> {
    state
        .database
        .update_preferences(&preferences)
        .await
        .map_err(CommandError::from)
}

/// Removes saved preferences and returns the application defaults.
#[tauri::command]
pub async fn reset_preferences(state: State<'_, AppState>) -> CommandResult<AppPreferences> {
    state
        .database
        .reset_preferences()
        .await
        .map_err(CommandError::from)
}
