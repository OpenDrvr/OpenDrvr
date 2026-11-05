//! Tauri command adapters.
//!
//! Commands are intentionally thin: they validate input, call a domain service
//! through [`crate::state::AppState`], and convert [`crate::error::AppError`]
//! into a serializable [`crate::error::CommandError`]. Business logic belongs
//! in the domain modules, not here.

pub mod app;
pub mod database;
pub mod system;

/// Single registration point for every IPC command.
///
/// Keeping the list here avoids scattering `generate_handler!` entries across
/// the crate and makes missing registrations obvious during review.
#[macro_export]
macro_rules! generate_command_handlers {
    () => {
        tauri::generate_handler![
            $crate::commands::app::get_app_info,
            $crate::commands::database::database_test_query,
            $crate::commands::database::get_database_health,
            $crate::commands::system::get_backend_status,
            $crate::commands::system::ping,
        ]
    };
}
