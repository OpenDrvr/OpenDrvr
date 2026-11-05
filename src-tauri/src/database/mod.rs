//! Local persistence domain.
//!
//! The database file is configured but not opened in this phase. Keeping the
//! connection and migrations behind this module prevents SQL concerns from
//! leaking into commands or other services.

use crate::config::DatabaseConfig;
use crate::state::{ModuleState, ModuleStatus};

#[derive(Debug)]
pub struct Database {
    config: DatabaseConfig,
}

impl Database {
    pub fn new(config: DatabaseConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &DatabaseConfig {
        &self.config
    }

    pub fn status(&self) -> ModuleStatus {
        ModuleStatus {
            name: "database",
            state: ModuleState::Ready,
            detail: format!(
                "Configuration resolved for {}; schema management is planned.",
                self.config.file_name
            ),
        }
    }
}
