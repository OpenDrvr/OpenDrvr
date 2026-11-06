//! Tauri command adapters.
//!
//! Commands are intentionally thin: they validate input, call a domain service
//! through [`crate::state::AppState`], and convert [`crate::error::AppError`]
//! into a serializable [`crate::error::CommandError`]. Business logic belongs
//! in the domain modules, not here.

pub mod app;
pub mod database;
pub mod hardware;
pub mod preferences;
pub mod system;

/// Single registration point for every IPC command.
///
/// Keeping the list here avoids scattering `generate_handler!` entries across
/// the crate and makes missing registrations obvious during review. Only
/// commands used by the frontend are registered; there is no generic query or
/// SQL command.
#[macro_export]
macro_rules! generate_command_handlers {
    () => {
        tauri::generate_handler![
            $crate::commands::app::get_app_info,
            $crate::commands::database::get_database_health,
            $crate::commands::hardware::scan_hardware,
            $crate::commands::preferences::get_preferences,
            $crate::commands::preferences::reset_preferences,
            $crate::commands::preferences::update_preferences,
            $crate::commands::system::get_backend_status,
        ]
    };
}
