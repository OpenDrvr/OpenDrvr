mod commands;
mod config;
mod database;
mod downloads;
mod drivers;
mod error;
mod hardware;
mod logging;
mod platform;
mod sources;
mod state;
mod utils;

pub use error::{AppError, AppResult, CommandError, CommandResult};
pub use state::AppState;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // Configuration is loaded before logging so `config.json` controls
            // the subscriber. A malformed file aborts startup instead of being
            // silently replaced with defaults.
            let config_dir = platform::config_dir(app.handle());
            let config = config::AppConfig::load(config_dir.as_deref())?;
            logging::init(&config.logging)?;

            tracing::info!(
                target: "opendrvr::startup",
                version = env!("CARGO_PKG_VERSION"),
                platform = platform::name(),
                supported_target = platform::is_supported(),
                "OpenDrvr backend starting"
            );

            let data_dir = platform::data_dir(app.handle());
            let catalog_path = platform::catalog_path(app.handle());
            let database = tauri::async_runtime::block_on(database::Database::initialize(
                &config.database,
                data_dir.as_deref(),
                catalog_path.as_deref(),
            ));

            app.manage(state::AppState::new(config, database));
            Ok(())
        })
        .invoke_handler(crate::generate_command_handlers!())
        .run(tauri::generate_context!())
        .expect("error while running OpenDrvr");
}
