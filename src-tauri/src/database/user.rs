use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::SqlitePool;

use crate::database::models::UserDatabaseHealth;
use crate::database::{db_error, table_count};
use crate::error::{AppError, AppResult};

/// Writable, per-user SQLite database.
///
/// The pool uses WAL mode so reads and writes can overlap, and a busy timeout
/// so short concurrent writes wait instead of failing immediately.
#[derive(Debug)]
pub struct UserDatabase {
    pool: SqlitePool,
    schema_version: i64,
}

impl UserDatabase {
    /// Opens the database, runs migrations, and verifies integrity.
    ///
    /// If the existing file cannot be opened or migrated, it is quarantined and
    /// a fresh database is created once. The original file is retained with a
    /// `.corrupt-<timestamp>` suffix for diagnostics.
    pub async fn initialize(path: &Path) -> AppResult<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        match Self::open(path).await {
            Ok(database) => Ok(database),
            Err(first_error) => {
                tracing::warn!(
                    error = %first_error,
                    "user database failed to initialize; attempting recovery"
                );

                if let Some(backup) = quarantine(path)? {
                    tracing::warn!(
                        backup = %backup.display(),
                        "quarantined unreadable user database"
                    );
                }

                Self::open(path).await.map_err(|second_error| {
                    AppError::Database(format!(
                        "user database recovery failed (first attempt: {first_error}; retry: {second_error})"
                    ))
                })
            }
        }
    }

    async fn open(path: &Path) -> AppResult<Self> {
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .foreign_keys(true)
            .busy_timeout(Duration::from_secs(5));

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(10))
            .connect_with(options)
            .await
            .map_err(|error| db_error("failed to open user database", error))?;

        if let Err(error) = sqlx::migrate!("./migrations").run(&pool).await {
            pool.close().await;
            return Err(db_error("failed to migrate user database", error));
        }

        if let Err(error) = integrity_check(&pool).await {
            pool.close().await;
            return Err(error);
        }

        let schema_version = schema_version(&pool).await?;

        Ok(Self {
            pool,
            schema_version,
        })
    }

    pub fn schema_version(&self) -> i64 {
        self.schema_version
    }

    pub async fn health(&self) -> AppResult<UserDatabaseHealth> {
        let tables = table_count(&self.pool).await?;

        Ok(UserDatabaseHealth {
            status: crate::database::models::DatabaseComponentStatus::Ok,
            schema_version: self.schema_version,
            tables,
            writable: true,
            message: None,
        })
    }

    /// Reads every stored preference row. The table is intentionally small, so
    /// a single query is cheaper than one round trip per preference key.
    pub async fn all_preferences(&self) -> AppResult<Vec<(String, String)>> {
        sqlx::query_as::<_, (String, String)>("SELECT key, value FROM app_preferences")
            .fetch_all(&self.pool)
            .await
            .map_err(|error| db_error("failed to read preferences", error))
    }

    /// Writes a complete preference set atomically.
    pub async fn set_preferences(&self, entries: &[(&str, String)]) -> AppResult<()> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|error| db_error("failed to begin preference update", error))?;

        for (key, value) in entries {
            sqlx::query(
                "INSERT INTO app_preferences (key, value, updated_at)
                 VALUES (?1, ?2, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
                 ON CONFLICT(key) DO UPDATE SET
                     value = excluded.value,
                     updated_at = excluded.updated_at",
            )
            .bind(key)
            .bind(value)
            .execute(&mut *transaction)
            .await
            .map_err(|error| db_error("failed to write preference", error))?;
        }

        transaction
            .commit()
            .await
            .map_err(|error| db_error("failed to commit preference update", error))
    }

    /// Removes the supplied preference keys atomically so a reset is complete
    /// even if one key was never written.
    pub async fn clear_preferences(&self, keys: &[&str]) -> AppResult<()> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|error| db_error("failed to begin preference reset", error))?;

        for key in keys {
            sqlx::query("DELETE FROM app_preferences WHERE key = ?1")
                .bind(key)
                .execute(&mut *transaction)
                .await
                .map_err(|error| db_error("failed to clear preference", error))?;
        }

        transaction
            .commit()
            .await
            .map_err(|error| db_error("failed to commit preference reset", error))
    }

    #[cfg(test)]
    pub(crate) fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

async fn schema_version(pool: &SqlitePool) -> AppResult<i64> {
    sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(MAX(version), 0) FROM _sqlx_migrations WHERE success = 1",
    )
    .fetch_one(pool)
    .await
    .map_err(|error| db_error("failed to read user schema version", error))
}

async fn integrity_check(pool: &SqlitePool) -> AppResult<()> {
    let result = sqlx::query_scalar::<_, String>("PRAGMA quick_check")
        .fetch_one(pool)
        .await
        .map_err(|error| db_error("failed to run user database integrity check", error))?;

    if result.eq_ignore_ascii_case("ok") {
        Ok(())
    } else {
        Err(AppError::Database(format!(
            "user database integrity check failed: {result}"
        )))
    }
}

/// Moves a broken database (and its WAL sidecars) aside before a retry.
///
/// Only regular files are moved. If the configured path is a directory, a
/// symlink, or something else, it is left untouched so recovery can never
/// relocate user data outside the intended file.
fn quarantine(path: &Path) -> AppResult<Option<PathBuf>> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };

    if !metadata.file_type().is_file() {
        tracing::warn!(
            path = %path.display(),
            "refusing to quarantine a user database path that is not a regular file"
        );
        return Ok(None);
    }

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let backup = with_suffix(path, &format!(".corrupt-{timestamp}"));

    fs::rename(path, &backup)?;

    for suffix in ["-wal", "-shm"] {
        let sidecar = with_suffix(path, suffix);
        if sidecar.exists() {
            fs::rename(&sidecar, with_suffix(&backup, suffix))?;
        }
    }

    Ok(Some(backup))
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    PathBuf::from(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_support::TestDir;

    #[tokio::test]
    async fn initializes_reopens_and_persists_preferences() {
        let dir = TestDir::new("user-reopen");
        let path = dir.path().join("opendrvr.sqlite3");

        let database = UserDatabase::initialize(&path).await.expect("initialize");
        assert_eq!(database.schema_version(), 2);
        database
            .set_preferences(&[("theme", "dark".to_string())])
            .await
            .expect("write preference");

        assert!(database
            .all_preferences()
            .await
            .expect("read")
            .contains(&("theme".to_string(), "dark".to_string())));

        drop(database);

        let reopened = UserDatabase::initialize(&path).await.expect("reopen");
        assert_eq!(reopened.schema_version(), 2);
        assert!(reopened
            .all_preferences()
            .await
            .expect("read")
            .contains(&("theme".to_string(), "dark".to_string())));
    }

    #[tokio::test]
    async fn migration_creates_expected_tables_and_indexes() {
        let dir = TestDir::new("user-migration");
        let path = dir.path().join("opendrvr.sqlite3");
        let database = UserDatabase::initialize(&path).await.expect("initialize");

        let tables = sqlx::query_scalar::<_, String>(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
        )
        .fetch_all(database.pool())
        .await
        .expect("list tables");

        for expected in [
            "app_preferences",
            "hardware_scans",
            "hardware_scan_devices",
            "download_history",
            "cached_driver_metadata",
            "user_configuration",
        ] {
            assert!(tables.iter().any(|table| table == expected), "{expected}");
        }

        let indexes = sqlx::query_scalar::<_, String>(
            "SELECT name FROM sqlite_master WHERE type = 'index' AND name LIKE 'idx_%'",
        )
        .fetch_all(database.pool())
        .await
        .expect("list indexes");

        assert!(indexes.contains(&"idx_download_history_started_at".to_string()));
        assert!(indexes.contains(&"idx_hardware_scan_devices_scan".to_string()));
    }

    #[tokio::test]
    async fn recovers_from_a_corrupt_database() {
        let dir = TestDir::new("user-recovery");
        let path = dir.path().join("opendrvr.sqlite3");
        fs::write(&path, b"this is not a sqlite database").expect("write corrupt file");

        let database = UserDatabase::initialize(&path).await.expect("recovery");
        assert_eq!(database.schema_version(), 2);

        let quarantined = fs::read_dir(dir.path())
            .expect("read temp dir")
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_name().to_string_lossy().contains(".corrupt-"))
            .count();

        assert_eq!(quarantined, 1);
    }

    #[tokio::test]
    async fn recovery_never_quarantines_a_directory() {
        let dir = TestDir::new("user-directory-path");
        let path = dir.path().join("opendrvr.sqlite3");
        fs::create_dir(&path).expect("create directory at database path");

        assert!(
            UserDatabase::initialize(&path).await.is_err(),
            "a directory cannot be opened as a database"
        );
        assert!(path.is_dir(), "the directory must be left in place");

        let quarantined = fs::read_dir(dir.path())
            .expect("read temp dir")
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_name().to_string_lossy().contains(".corrupt-"))
            .count();

        assert_eq!(quarantined, 0);
    }

    #[tokio::test]
    async fn preferences_are_written_atomically_and_cleared() {
        let dir = TestDir::new("user-preferences");
        let path = dir.path().join("opendrvr.sqlite3");
        let database = UserDatabase::initialize(&path).await.expect("initialize");

        let entries = [
            ("ui.theme", "dark".to_string()),
            ("downloads.directory", "Downloads".to_string()),
        ];
        database
            .set_preferences(&entries)
            .await
            .expect("write preferences");

        let stored = database.all_preferences().await.expect("read preferences");
        assert!(stored.contains(&("ui.theme".to_string(), "dark".to_string())));
        assert!(stored.contains(&("downloads.directory".to_string(), "Downloads".to_string())));

        database
            .clear_preferences(&["ui.theme"])
            .await
            .expect("clear preference");
        let stored = database.all_preferences().await.expect("read preferences");
        assert!(!stored.iter().any(|(key, _)| key == "ui.theme"));
        assert!(stored.iter().any(|(key, _)| key == "downloads.directory"));
    }

    #[tokio::test]
    async fn health_reports_schema_and_tables() {
        let dir = TestDir::new("user-health");
        let path = dir.path().join("opendrvr.sqlite3");
        let database = UserDatabase::initialize(&path).await.expect("initialize");
        let health = database.health().await.expect("health");

        assert_eq!(
            health.status,
            crate::database::models::DatabaseComponentStatus::Ok
        );
        assert_eq!(health.schema_version, 2);
        assert_eq!(health.tables, 6);
        assert!(health.writable);
    }
}
