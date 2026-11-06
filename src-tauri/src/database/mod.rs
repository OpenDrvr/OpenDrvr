//! Embedded SQLite persistence.
//!
//! Two independent databases are owned here: a writable per-user database and
//! an immutable bundled catalog opened read-only. Pools never leave this module;
//! commands receive health summaries or narrow results only.

use std::fmt;
use std::path::Path;

use sqlx::SqlitePool;

use crate::config::DatabaseConfig;
use crate::error::{AppError, AppResult};
use crate::preferences::{AppPreferences, PREFERENCE_KEYS};
use crate::state::{ModuleState, ModuleStatus};

mod catalog;
mod models;
mod user;

#[cfg(test)]
mod test_support;

pub use models::{CatalogDatabaseHealth, DatabaseHealth, UserDatabaseHealth};

use catalog::CatalogDatabase;
use user::UserDatabase;

enum UserState {
    Ready(UserDatabase),
    Failed(String),
}

enum CatalogState {
    Ready(CatalogDatabase),
    Unavailable(String),
}

/// Owns both database components for the lifetime of the application.
///
/// The user database is writable and lives in the Windows app data directory.
/// The catalog is an immutable, read-only resource. Neither pool is exposed
/// outside Rust; commands only receive health summaries or narrow domain data.
pub struct Database {
    config: DatabaseConfig,
    user: UserState,
    catalog: CatalogState,
}

impl fmt::Debug for Database {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let user = match &self.user {
            UserState::Ready(_) => "ready",
            UserState::Failed(_) => "failed",
        };
        let catalog = match &self.catalog {
            CatalogState::Ready(_) => "ready",
            CatalogState::Unavailable(_) => "unavailable",
        };

        formatter
            .debug_struct("Database")
            .field("config", &self.config)
            .field("user", &user)
            .field("catalog", &catalog)
            .finish()
    }
}

impl Database {
    /// Initializes both components. A missing or broken catalog degrades
    /// gracefully; a broken user database is reported through health checks and
    /// does not prevent the application from starting.
    ///
    /// `catalog_updates_dir` is an optional per-user directory of immutable
    /// catalog snapshots. It is separate from the writable user database and
    /// never receives preferences, scan history, or download history.
    pub async fn initialize(
        config: &DatabaseConfig,
        data_dir: Option<&Path>,
        catalog_path: Option<&Path>,
        catalog_updates_dir: Option<&Path>,
    ) -> Self {
        let user = match data_dir {
            Some(data_dir) => {
                let path = data_dir.join(&config.file_name);
                match UserDatabase::initialize(&path).await {
                    Ok(database) => {
                        tracing::info!(
                            path = %path.display(),
                            schema_version = database.schema_version(),
                            "user database ready"
                        );
                        UserState::Ready(database)
                    }
                    Err(error) => {
                        tracing::error!(
                            error = %error,
                            path = %path.display(),
                            "user database unavailable"
                        );
                        UserState::Failed(
                            "User database could not be initialized; see logs.".to_string(),
                        )
                    }
                }
            }
            None => {
                tracing::error!("application data directory is unavailable");
                UserState::Failed(
                    "Application data directory is unavailable; see logs.".to_string(),
                )
            }
        };

        let catalog = match catalog_path {
            Some(path) => match CatalogDatabase::open_with_updates(path, catalog_updates_dir).await
            {
                Ok(database) => {
                    tracing::info!(
                        path = %path.display(),
                        schema_version = database.schema_version(),
                        catalog_version = %database.catalog_version(),
                        origin = ?database.origin(),
                        "catalog snapshot ready"
                    );
                    CatalogState::Ready(database)
                }
                Err(error) => {
                    tracing::warn!(
                        error = %error,
                        path = %path.display(),
                        "catalog unavailable"
                    );
                    CatalogState::Unavailable(
                        "Catalog is unavailable; catalog features are disabled.".to_string(),
                    )
                }
            },
            None => {
                tracing::warn!("bundled catalog resource was not found");
                CatalogState::Unavailable(
                    "Bundled catalog resource was not found; catalog features are disabled."
                        .to_string(),
                )
            }
        };

        Self {
            config: config.clone(),
            user,
            catalog,
        }
    }

    pub fn status(&self) -> ModuleStatus {
        match &self.user {
            UserState::Ready(user) => {
                let detail = match &self.catalog {
                    CatalogState::Ready(catalog) => format!(
                        "User database ready (schema v{}); catalog ready (v{}, {}, schema v{}).",
                        user.schema_version(),
                        catalog.catalog_version(),
                        catalog.origin().as_str(),
                        catalog.schema_version()
                    ),
                    CatalogState::Unavailable(_) => format!(
                        "User database ready (schema v{}); catalog unavailable.",
                        user.schema_version()
                    ),
                };

                ModuleStatus {
                    name: "database",
                    state: ModuleState::Ready,
                    detail,
                }
            }
            UserState::Failed(_) => ModuleStatus {
                name: "database",
                state: ModuleState::Unavailable,
                detail: "User database could not be initialized; see logs.".to_string(),
            },
        }
    }

    /// Runs live checks against both pools without exposing SQL to the caller.
    pub async fn health(&self) -> DatabaseHealth {
        let user = match &self.user {
            UserState::Ready(database) => match database.health().await {
                Ok(health) => health,
                Err(error) => {
                    tracing::error!(error = %error, "user database health check failed");
                    UserDatabaseHealth::unavailable("User database health check failed.")
                }
            },
            UserState::Failed(message) => UserDatabaseHealth::unavailable(message.clone()),
        };

        let catalog = match &self.catalog {
            CatalogState::Ready(database) => match database.health().await {
                Ok(health) => health,
                Err(error) => {
                    tracing::error!(error = %error, "catalog health check failed");
                    CatalogDatabaseHealth::unavailable("Bundled catalog health check failed.")
                }
            },
            CatalogState::Unavailable(message) => {
                CatalogDatabaseHealth::unavailable(message.clone())
            }
        };

        DatabaseHealth { user, catalog }
    }

    fn user_database(&self) -> AppResult<&UserDatabase> {
        match &self.user {
            UserState::Ready(database) => Ok(database),
            UserState::Failed(_) => Err(AppError::Database(
                "user database is unavailable".to_string(),
            )),
        }
    }

    /// Reads the typed preferences, applying defaults for missing or malformed
    /// individual values.
    pub async fn get_preferences(&self) -> AppResult<AppPreferences> {
        let entries = self.user_database()?.all_preferences().await?;
        Ok(AppPreferences::from_stored(entries))
    }

    /// Validates and writes the complete preference set in one transaction.
    pub async fn update_preferences(
        &self,
        preferences: &AppPreferences,
    ) -> AppResult<AppPreferences> {
        let preferences = preferences.clone().normalized();
        preferences.validate()?;

        let entries = preferences.to_entries();
        self.user_database()?.set_preferences(&entries).await?;

        Ok(preferences)
    }

    /// Removes persisted preferences so the next read returns application
    /// defaults.
    pub async fn reset_preferences(&self) -> AppResult<AppPreferences> {
        self.user_database()?
            .clear_preferences(&PREFERENCE_KEYS)
            .await?;

        Ok(AppPreferences::default())
    }

    #[cfg(test)]
    pub(crate) fn test_fixture(config: DatabaseConfig) -> Self {
        Self {
            config,
            user: UserState::Failed("test fixture".to_string()),
            catalog: CatalogState::Unavailable("test fixture".to_string()),
        }
    }
}

pub(crate) fn db_error(context: &str, error: impl fmt::Display) -> AppError {
    AppError::Database(format!("{context}: {error}"))
}

pub(crate) async fn table_count(pool: &SqlitePool) -> AppResult<i64> {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM sqlite_master
         WHERE type = 'table'
           AND name NOT LIKE 'sqlite_%'
           AND name NOT LIKE '_sqlx_%'",
    )
    .fetch_one(pool)
    .await
    .map_err(|error| db_error("failed to count database tables", error))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::catalog::import::build_snapshot;
    use crate::catalog::model::CatalogDocument;
    use crate::catalog::schema::SOURCE_POLICY_VERSION;
    use crate::database::models::{CatalogOrigin, DatabaseComponentStatus};
    use crate::database::test_support::TestDir;

    fn bundled_catalog() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/catalog/catalog.sqlite3")
    }

    #[tokio::test]
    async fn user_and_catalog_are_separate_and_catalog_is_unchanged() {
        let dir = TestDir::new("database-separation");
        let catalog_path = bundled_catalog();
        let catalog_size = fs::metadata(&catalog_path).expect("catalog metadata").len();

        let database = Database::initialize(
            &DatabaseConfig::default(),
            Some(dir.path()),
            Some(&catalog_path),
            None,
        )
        .await;

        database
            .update_preferences(&AppPreferences {
                notify_downloads: false,
                ..AppPreferences::default()
            })
            .await
            .expect("write user preference");

        let health = database.health().await;
        assert_eq!(health.user.status, DatabaseComponentStatus::Ok);
        assert_eq!(health.catalog.status, DatabaseComponentStatus::Ok);
        assert_eq!(health.catalog.origin, Some(CatalogOrigin::Bundled));
        assert_eq!(health.catalog.catalog_version.as_deref(), Some("0.0.0"));
        assert_eq!(health.catalog.manufacturer_count, 0);
        assert!(health.catalog.read_only);
        assert!(dir.path().join("opendrvr.sqlite3").is_file());
        assert_eq!(
            fs::metadata(&catalog_path).expect("catalog metadata").len(),
            catalog_size
        );
    }

    #[tokio::test]
    async fn catalog_update_does_not_touch_user_data() {
        let dir = TestDir::new("database-update-separation");
        let bundled = bundled_catalog();
        let bundled_size = fs::metadata(&bundled).expect("catalog metadata").len();
        let updates = dir.path().join("catalog-updates");
        fs::create_dir_all(&updates).expect("create updates directory");

        let database = Database::initialize(
            &DatabaseConfig::default(),
            Some(dir.path()),
            Some(&bundled),
            Some(&updates),
        )
        .await;

        let preferences = AppPreferences {
            notify_downloads: false,
            ..AppPreferences::default()
        };
        database
            .update_preferences(&preferences)
            .await
            .expect("save preferences");
        drop(database);

        let document = CatalogDocument {
            catalog_version: "2026.11.6".to_string(),
            generated_at: "2026-11-06T00:00:00Z".to_string(),
            source_policy_version: SOURCE_POLICY_VERSION.to_string(),
            last_verified_at: None,
            manufacturers: Vec::new(),
            hardware_devices: Vec::new(),
            driver_sources: Vec::new(),
            drivers: Vec::new(),
        };
        build_snapshot(&updates.join("catalog-2026.11.6.sqlite3"), &document, false)
            .await
            .expect("build update snapshot");

        let reopened = Database::initialize(
            &DatabaseConfig::default(),
            Some(dir.path()),
            Some(&bundled),
            Some(&updates),
        )
        .await;

        let health = reopened.health().await;
        assert_eq!(health.catalog.origin, Some(CatalogOrigin::Update));
        assert_eq!(health.catalog.catalog_version.as_deref(), Some("2026.11.6"));
        assert_eq!(
            reopened.get_preferences().await.expect("preferences"),
            preferences
        );
        assert_eq!(
            fs::metadata(&bundled).expect("catalog metadata").len(),
            bundled_size
        );
    }

    #[tokio::test]
    async fn missing_catalog_does_not_block_user_database() {
        let dir = TestDir::new("database-no-catalog");
        let database =
            Database::initialize(&DatabaseConfig::default(), Some(dir.path()), None, None).await;

        let health = database.health().await;
        assert_eq!(health.user.status, DatabaseComponentStatus::Ok);
        assert_eq!(health.catalog.status, DatabaseComponentStatus::Unavailable);
        assert!(health.catalog.message.is_some());
        assert_eq!(database.status().state, ModuleState::Ready);
    }

    #[tokio::test]
    async fn preferences_persist_across_reopen_and_reset() {
        let dir = TestDir::new("database-preferences");
        let database =
            Database::initialize(&DatabaseConfig::default(), Some(dir.path()), None, None).await;

        assert_eq!(
            database.get_preferences().await.expect("defaults"),
            AppPreferences::default()
        );

        let preferences = AppPreferences {
            download_directory: "D:\\OpenDrvr".to_string(),
            check_catalog_updates: false,
            notify_downloads: false,
            keep_download_history: false,
            ..AppPreferences::default()
        };
        let saved = database
            .update_preferences(&preferences)
            .await
            .expect("save preferences");
        assert_eq!(saved, preferences);

        drop(database);

        let reopened =
            Database::initialize(&DatabaseConfig::default(), Some(dir.path()), None, None).await;
        assert_eq!(
            reopened.get_preferences().await.expect("reload"),
            preferences
        );

        assert_eq!(
            reopened.reset_preferences().await.expect("reset"),
            AppPreferences::default()
        );
        assert_eq!(
            reopened.get_preferences().await.expect("after reset"),
            AppPreferences::default()
        );
    }

    #[tokio::test]
    async fn invalid_preferences_are_rejected_before_writing() {
        let dir = TestDir::new("database-invalid-preferences");
        let database =
            Database::initialize(&DatabaseConfig::default(), Some(dir.path()), None, None).await;

        let preferences = AppPreferences {
            download_directory: "  ".to_string(),
            ..AppPreferences::default()
        };
        assert!(database.update_preferences(&preferences).await.is_err());
        assert_eq!(
            database.get_preferences().await.expect("defaults"),
            AppPreferences::default()
        );
    }

    #[tokio::test]
    async fn user_database_failure_is_reported_without_panicking() {
        let dir = TestDir::new("database-user-failure");
        let not_a_directory = dir.path().join("not-a-directory");
        fs::write(&not_a_directory, b"file").expect("create file");

        let database = Database::initialize(
            &DatabaseConfig::default(),
            Some(&not_a_directory),
            None,
            None,
        )
        .await;

        let health = database.health().await;
        assert_eq!(health.user.status, DatabaseComponentStatus::Unavailable);
        assert_eq!(database.status().state, ModuleState::Unavailable);
        assert!(database.get_preferences().await.is_err());
    }
}
