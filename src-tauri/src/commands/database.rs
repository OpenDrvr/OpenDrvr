use tauri::State;

use crate::database::DatabaseHealth;
use crate::error::CommandResult;
use crate::state::AppState;

/// Returns live health information for both database components.
#[tauri::command]
pub async fn get_database_health(state: State<'_, AppState>) -> CommandResult<DatabaseHealth> {
    Ok(state.database.health().await)
}
