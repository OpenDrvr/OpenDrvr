use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

pub const CONFIG_FILE_NAME: &str = "config.json";

/// Application configuration.
///
/// A missing `config.json` is normal and yields defaults. A malformed file is
/// treated as an error so misconfiguration is never silently ignored.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppConfig {
    pub logging: LoggingConfig,
    pub database: DatabaseConfig,
    pub downloads: DownloadConfig,
}

impl AppConfig {
    pub fn path(config_dir: &Path) -> PathBuf {
        config_dir.join(CONFIG_FILE_NAME)
    }

    /// Loads configuration from the platform-specific app config directory.
    ///
    /// Returns defaults when no directory is available or the file does not
    /// exist. Validation is always applied before the config is used.
    pub fn load(config_dir: Option<&Path>) -> AppResult<Self> {
        let Some(config_dir) = config_dir else {
            return Ok(Self::default());
        };

        let path = Self::path(config_dir);
        let config = if path.exists() {
            let contents = std::fs::read_to_string(&path).map_err(|error| {
                AppError::Config(format!("failed to read {}: {error}", path.display()))
            })?;

            serde_json::from_str(&contents).map_err(|error| {
                AppError::Config(format!("failed to parse {}: {error}", path.display()))
            })?
        } else {
            Self::default()
        };

        config.validate()?;
        Ok(config)
    }

    /// Persists the configuration as pretty-printed JSON.
    ///
    /// Validation runs first so an invalid configuration can never be written.
    pub fn save(&self, config_dir: &Path) -> AppResult<()> {
        self.validate()?;
        std::fs::create_dir_all(config_dir)?;

        let path = Self::path(config_dir);
        let contents = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, contents)?;

        Ok(())
    }

    pub fn validate(&self) -> AppResult<()> {
        if self.logging.level.trim().is_empty() {
            return Err(AppError::Config(
                "logging.level must not be empty".to_string(),
            ));
        }

        if self.database.file_name.trim().is_empty() {
            return Err(AppError::Config(
                "database.fileName must not be empty".to_string(),
            ));
        }

        if self.database.file_name.contains('/') || self.database.file_name.contains('\\') {
            return Err(AppError::Config(
                "database.fileName must be a file name, not a path".to_string(),
            ));
        }

        if self.downloads.max_concurrent == 0 {
            return Err(AppError::Config(
                "downloads.maxConcurrent must be greater than zero".to_string(),
            ));
        }

        if self.downloads.timeout_seconds == 0 {
            return Err(AppError::Config(
                "downloads.timeoutSeconds must be greater than zero".to_string(),
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LoggingConfig {
    pub level: String,
    pub format: LogFormat,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: LogFormat::Pretty,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    #[default]
    Pretty,
    Json,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DatabaseConfig {
    pub file_name: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            file_name: "opendrvr.sqlite3".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DownloadConfig {
    pub max_concurrent: usize,
    pub timeout_seconds: u64,
}

impl Default for DownloadConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 3,
            timeout_seconds: 60,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_valid() {
        let config = AppConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn rejects_path_in_database_file_name() {
        let mut config = AppConfig::default();
        config.database.file_name = "nested/db.sqlite3".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn rejects_zero_concurrency() {
        let mut config = AppConfig::default();
        config.downloads.max_concurrent = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn config_round_trips_as_json() {
        let config = AppConfig::default();
        let json = serde_json::to_string(&config).expect("serialize config");
        let parsed: AppConfig = serde_json::from_str(&json).expect("deserialize config");

        assert_eq!(parsed.logging.level, config.logging.level);
        assert_eq!(parsed.database.file_name, config.database.file_name);
        assert_eq!(
            parsed.downloads.max_concurrent,
            config.downloads.max_concurrent
        );
    }

    #[test]
    fn saves_and_loads_config_from_disk() {
        let dir = std::env::temp_dir().join(format!("opendrvr-config-test-{}", std::process::id()));
        let config = AppConfig::default();

        config.save(&dir).expect("save config");
        let loaded = AppConfig::load(Some(&dir)).expect("load config");

        assert_eq!(loaded.logging.level, config.logging.level);
        assert_eq!(loaded.database.file_name, config.database.file_name);
        assert_eq!(
            loaded.downloads.timeout_seconds,
            config.downloads.timeout_seconds
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn missing_config_file_uses_defaults() {
        let dir =
            std::env::temp_dir().join(format!("opendrvr-config-missing-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create temp dir");

        let loaded = AppConfig::load(Some(&dir)).expect("load defaults");
        assert_eq!(loaded.logging.level, "info");

        std::fs::remove_dir_all(&dir).ok();
    }
}
