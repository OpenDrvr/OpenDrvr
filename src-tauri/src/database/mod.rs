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
use crate::state::{ModuleState, ModuleStatus};

mod catalog;
mod models;
mod user;

#[cfg(test)]
mod test_support;

pub use models::{CatalogDatabaseHealth, DatabaseHealth, DatabaseTestResult, UserDatabaseHealth};

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
    pub async fn initialize(
        config: &DatabaseConfig,
        data_dir: Option<&Path>,
        catalog_path: Option<&Path>,
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
            Some(path) => match CatalogDatabase::open(path).await {
                Ok(database) => {
                    tracing::info!(
                        path = %path.display(),
                        schema_version = database.schema_version(),
                        "bundled catalog ready"
                    );
                    CatalogState::Ready(database)
                }
                Err(error) => {
                    tracing::warn!(
                        error = %error,
                        path = %path.display(),
                        "bundled catalog unavailable"
                    );
                    CatalogState::Unavailable(
                        "Bundled catalog is unavailable; catalog features are disabled."
                            .to_string(),
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
                        "User database ready (schema v{}); bundled catalog ready (schema v{}).",
                        user.schema_version(),
                        catalog.schema_version()
                    ),
                    CatalogState::Unavailable(_) => format!(
                        "User database ready (schema v{}); bundled catalog unavailable.",
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

    /// Diagnostic read that exercises a bound parameter and both databases.
    pub async fn test_query(&self, key: &str) -> AppResult<DatabaseTestResult> {
        let user = match &self.user {
            UserState::Ready(database) => database,
            UserState::Failed(_) => {
                return Err(AppError::Database(
                    "user database is unavailable".to_string(),
                ))
            }
        };

        let preference = user.get_preference(key).await?;

        let catalog = match &self.catalog {
            CatalogState::Ready(database) => match database.health().await {
                Ok(health) => Some(health),
                Err(error) => {
                    tracing::warn!(error = %error, "catalog test query failed");
                    None
                }
            },
            CatalogState::Unavailable(_) => None,
        };

        Ok(DatabaseTestResult {
            user_database_ready: true,
            catalog_database_ready: catalog.is_some(),
            preference,
            catalog_schema_version: catalog.as_ref().and_then(|health| health.schema_version),
            catalog_manufacturer_count: catalog.as_ref().map(|health| health.manufacturer_count),
        })
    }

    pub async fn set_preference(&self, key: &str, value: &str) -> AppResult<()> {
        match &self.user {
            UserState::Ready(database) => database.set_preference(key, value).await,
            UserState::Failed(_) => Err(AppError::Database(
                "user database is unavailable".to_string(),
            )),
        }
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
    use crate::database::models::DatabaseComponentStatus;
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
        )
        .await;

        database
            .set_preference("database.probe", "ok")
            .await
            .expect("write user preference");

        let health = database.health().await;
        assert_eq!(health.user.status, DatabaseComponentStatus::Ok);
        assert_eq!(health.catalog.status, DatabaseComponentStatus::Ok);
        assert_eq!(health.catalog.manufacturer_count, 0);
        assert!(health.catalog.read_only);
        assert!(dir.path().join("opendrvr.sqlite3").is_file());
        assert_eq!(
            fs::metadata(&catalog_path).expect("catalog metadata").len(),
            catalog_size
        );
    }

    #[tokio::test]
    async fn missing_catalog_does_not_block_user_database() {
        let dir = TestDir::new("database-no-catalog");
        let database =
            Database::initialize(&DatabaseConfig::default(), Some(dir.path()), None).await;

        let health = database.health().await;
        assert_eq!(health.user.status, DatabaseComponentStatus::Ok);
        assert_eq!(health.catalog.status, DatabaseComponentStatus::Unavailable);
        assert!(health.catalog.message.is_some());
        assert_eq!(database.status().state, ModuleState::Ready);
    }

    #[tokio::test]
    async fn test_query_uses_parameterized_preference_lookup() {
        let dir = TestDir::new("database-test-query");
        let database = Database::initialize(
            &DatabaseConfig::default(),
            Some(dir.path()),
            Some(&bundled_catalog()),
        )
        .await;

        database
            .set_preference("diagnostic.probe", "present")
            .await
            .expect("write preference");

        let result = database
            .test_query("diagnostic.probe")
            .await
            .expect("query");
        assert!(result.user_database_ready);
        assert!(result.catalog_database_ready);
        assert_eq!(result.preference.as_deref(), Some("present"));
        assert_eq!(result.catalog_schema_version, Some(1));
        assert_eq!(result.catalog_manufacturer_count, Some(0));

        let missing = database
            .test_query("diagnostic.missing")
            .await
            .expect("query");
        assert_eq!(missing.preference, None);
    }

    #[tokio::test]
    async fn user_database_failure_is_reported_without_panicking() {
        let dir = TestDir::new("database-user-failure");
        let not_a_directory = dir.path().join("not-a-directory");
        fs::write(&not_a_directory, b"file").expect("create file");

        let database =
            Database::initialize(&DatabaseConfig::default(), Some(&not_a_directory), None).await;

        let health = database.health().await;
        assert_eq!(health.user.status, DatabaseComponentStatus::Unavailable);
        assert_eq!(database.status().state, ModuleState::Unavailable);
        assert!(database.test_query("anything").await.is_err());
    }
}
