use serde::Serialize;

use crate::{
    config::AppConfig, database::Database, downloads::DownloadManager, drivers::DriverService,
    hardware::HardwareService, platform, sources::SourceRegistry,
};

/// Lifecycle of a backend domain as reported to the frontend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModuleState {
    /// The service is constructed and its configuration is resolved.
    Ready,
    /// The module boundary exists but its feature work is scheduled later.
    Planned,
    /// The module exists but could not be initialized.
    Unavailable,
}

/// Safe, serializable description of a backend domain.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleStatus {
    pub name: &'static str,
    pub state: ModuleState,
    pub detail: String,
}

impl ModuleStatus {
    pub fn planned(name: &'static str, detail: impl Into<String>) -> Self {
        Self {
            name,
            state: ModuleState::Planned,
            detail: detail.into(),
        }
    }
}

/// Central application state managed by Tauri.
///
/// Commands receive this through `tauri::State`; domain services never depend
/// on Tauri themselves. The state is immutable after setup, so no locks are
/// required. Mutable caches will be added per service when they are needed.
#[derive(Debug)]
pub struct AppState {
    pub config: AppConfig,
    pub hardware: HardwareService,
    pub drivers: DriverService,
    pub sources: SourceRegistry,
    pub database: Database,
    pub downloads: DownloadManager,
}

impl AppState {
    pub fn new(config: AppConfig, database: Database) -> Self {
        let hardware = HardwareService::new(platform::device_enumerator());
        let drivers = DriverService::new();
        let sources = SourceRegistry::new();
        let downloads = DownloadManager::new(config.downloads.clone());

        Self {
            config,
            hardware,
            drivers,
            sources,
            database,
            downloads,
        }
    }

    pub fn module_status(&self) -> Vec<ModuleStatus> {
        vec![
            self.hardware.status(),
            self.drivers.status(),
            self.sources.status(),
            self.database.status(),
            self.downloads.status(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_all_domain_modules() {
        let config = AppConfig::default();
        let state = AppState::new(config.clone(), Database::test_fixture(config.database));
        let statuses = state.module_status();
        let names: Vec<_> = statuses.iter().map(|status| status.name).collect();

        assert_eq!(
            names,
            ["hardware", "drivers", "sources", "database", "downloads"]
        );
    }

    #[test]
    fn planned_and_ready_modules_are_distinguished() {
        let config = AppConfig::default();
        let state = AppState::new(config.clone(), Database::test_fixture(config.database));
        let statuses = state.module_status();

        let planned = statuses
            .iter()
            .filter(|status| status.state == ModuleState::Planned)
            .count();
        let ready = statuses
            .iter()
            .filter(|status| status.state == ModuleState::Ready)
            .count();
        let unavailable = statuses
            .iter()
            .filter(|status| status.state == ModuleState::Unavailable)
            .count();

        assert_eq!(planned, 2);
        assert_eq!(ready, 2);
        assert_eq!(unavailable, 1);
    }
}
