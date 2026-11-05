use std::sync::OnceLock;

use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::config::{LogFormat, LoggingConfig};
use crate::error::{AppError, AppResult};

static INITIALIZED: OnceLock<()> = OnceLock::new();

/// Installs the global `tracing` subscriber.
///
/// `RUST_LOG` wins when it is set, which keeps debugging convenient without
/// editing the configuration file. Otherwise the level from `config.json` is
/// used. Log output goes to stderr so it never mixes with Tauri IPC traffic.
pub fn init(config: &LoggingConfig) -> AppResult<()> {
    if INITIALIZED.get().is_some() {
        return Ok(());
    }

    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(config.level.trim()))
        .map_err(|error| AppError::Config(format!("invalid logging filter: {error}")))?;

    let registry = tracing_subscriber::registry().with(filter);

    let result = match config.format {
        LogFormat::Json => registry
            .with(fmt::layer().json().with_target(true))
            .try_init(),
        LogFormat::Pretty => registry.with(fmt::layer().with_target(true)).try_init(),
    };

    result.map_err(|error| AppError::Internal(format!("failed to initialize logging: {error}")))?;

    let _ = INITIALIZED.set(());
    Ok(())
}
