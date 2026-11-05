use tauri::State;

use crate::database::{DatabaseHealth, DatabaseTestResult};
use crate::error::{CommandError, CommandResult};
use crate::state::AppState;
use crate::utils;

const MAX_TEST_KEY_CHARS: usize = 128;

/// Returns live health information for both database components.
#[tauri::command]
pub async fn get_database_health(state: State<'_, AppState>) -> CommandResult<DatabaseHealth> {
    Ok(state.database.health().await)
}

/// Harmless diagnostic query: reads a preference by key using a bound
/// parameter and reports catalog availability. No SQL is accepted from the
/// frontend.
#[tauri::command]
pub async fn database_test_query(
    key: Option<String>,
    state: State<'_, AppState>,
) -> CommandResult<DatabaseTestResult> {
    let key = key.unwrap_or_else(|| "opendrvr.test".to_string());
    utils::validate_user_text(&key, MAX_TEST_KEY_CHARS).map_err(CommandError::from)?;

    state
        .database
        .test_query(&key)
        .await
        .map_err(CommandError::from)
}
