use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;

use crate::catalog::metadata::CatalogMetadata;
use crate::catalog::schema::EXPECTED_TABLES;
use crate::database::models::{CatalogDatabaseHealth, CatalogOrigin, DatabaseComponentStatus};
use crate::database::{db_error, table_count};
use crate::error::{AppError, AppResult};

/// Read-only handle to one immutable catalog snapshot.
///
/// Snapshots are never mutated at runtime. The bundled resource is opened with
/// SQLite's read-only flag, and optional updates are separate versioned files in
/// a per-user directory. The runtime selects the highest valid snapshot so an
/// update can be installed or removed without touching the bundled resource.
#[derive(Debug)]
pub struct CatalogDatabase {
    pool: SqlitePool,
    metadata: CatalogMetadata,
    origin: CatalogOrigin,
}

impl CatalogDatabase {
    /// Opens the bundled catalog resource.
    pub async fn open(path: &Path) -> AppResult<Self> {
        Self::open_snapshot(path, CatalogOrigin::Bundled).await
    }

    /// Opens the highest valid snapshot from `updates_dir`, falling back to the
    /// bundled catalog.
    ///
    /// Invalid, unreadable, or incompatible update files are ignored with a
    /// warning so a bad update cannot disable catalog features. Ties keep the
    /// bundled snapshot because shipped content is the trusted baseline.
    pub async fn open_with_updates(
        bundled_path: &Path,
        updates_dir: Option<&Path>,
    ) -> AppResult<Self> {
        let Some(updates_dir) = updates_dir else {
            return Self::open(bundled_path).await;
        };

        let mut selected = match Self::open_snapshot(bundled_path, CatalogOrigin::Bundled).await {
            Ok(database) => Some(database),
            Err(error) => {
                tracing::warn!(
                    error = %error,
                    path = %bundled_path.display(),
                    "bundled catalog could not be opened"
                );
                None
            }
        };

        for candidate in update_snapshot_paths(updates_dir) {
            match Self::open_snapshot(&candidate, CatalogOrigin::Update).await {
                Ok(database) => {
                    let is_newer = selected
                        .as_ref()
                        .map(|current| {
                            database.metadata.catalog_version > current.metadata.catalog_version
                        })
                        .unwrap_or(true);

                    if is_newer {
                        if let Some(previous) = selected.replace(database) {
                            previous.close().await;
                        }
                    } else {
                        database.close().await;
                    }
                }
                Err(error) => {
                    tracing::warn!(
                        error = %error,
                        path = %candidate.display(),
                        "catalog update snapshot rejected"
                    );
                }
            }
        }

        selected.ok_or_else(|| {
            AppError::Database("no readable catalog snapshot is available".to_string())
        })
    }

    async fn open_snapshot(path: &Path, origin: CatalogOrigin) -> AppResult<Self> {
        if !path.is_file() {
            return Err(AppError::Database(format!(
                "catalog snapshot not found at {}",
                path.display()
            )));
        }

        let options = SqliteConnectOptions::new()
            .filename(path)
            .read_only(true)
            .create_if_missing(false);

        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .acquire_timeout(Duration::from_secs(10))
            .connect_with(options)
            .await
            .map_err(|error| db_error("failed to open catalog snapshot", error))?;

        let metadata = match read_metadata(&pool).await {
            Ok(metadata) => metadata,
            Err(error) => {
                pool.close().await;
                return Err(error);
            }
        };

        if let Err(error) = verify_snapshot(&pool).await {
            pool.close().await;
            return Err(error);
        }

        Ok(Self {
            pool,
            metadata,
            origin,
        })
    }

    pub fn schema_version(&self) -> i64 {
        self.metadata.schema_version
    }

    pub fn catalog_version(&self) -> crate::catalog::metadata::CatalogVersion {
        self.metadata.catalog_version
    }

    pub fn origin(&self) -> CatalogOrigin {
        self.origin
    }

    pub async fn health(&self) -> AppResult<CatalogDatabaseHealth> {
        let tables = table_count(&self.pool).await?;
        let manufacturer_count = count_rows(&self.pool, "manufacturers").await?;
        let hardware_device_count = count_rows(&self.pool, "hardware_devices").await?;
        let driver_source_count = count_rows(&self.pool, "driver_sources").await?;
        let driver_count = count_rows(&self.pool, "drivers").await?;

        Ok(CatalogDatabaseHealth {
            status: DatabaseComponentStatus::Ok,
            origin: Some(self.origin),
            schema_version: Some(self.metadata.schema_version),
            catalog_version: Some(self.metadata.catalog_version.to_string()),
            generated_at: Some(self.metadata.generated_at.clone()),
            last_verified_at: self.metadata.last_verified_at.clone(),
            tables,
            manufacturer_count,
            hardware_device_count,
            driver_source_count,
            driver_count,
            read_only: true,
            message: None,
        })
    }

    async fn close(self) {
        self.pool.close().await;
    }

    #[cfg(test)]
    pub(crate) fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

async fn read_metadata(pool: &SqlitePool) -> AppResult<CatalogMetadata> {
    let pairs = sqlx::query_as::<_, (String, String)>("SELECT key, value FROM catalog_meta")
        .fetch_all(pool)
        .await
        .map_err(|error| db_error("failed to read catalog metadata", error))?;

    let metadata = CatalogMetadata::from_pairs(pairs)
        .map_err(|message| AppError::Database(format!("invalid catalog metadata: {message}")))?;

    metadata
        .validate_runtime_compatibility()
        .map_err(|message| {
            AppError::Database(format!("incompatible catalog snapshot: {message}"))
        })?;

    Ok(metadata)
}

/// Verifies the structural contract every snapshot must satisfy before it is
/// exposed to the rest of the application.
async fn verify_snapshot(pool: &SqlitePool) -> AppResult<()> {
    for table in EXPECTED_TABLES {
        let exists = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
        )
        .bind(table)
        .fetch_one(pool)
        .await
        .map_err(|error| db_error("failed to inspect catalog tables", error))?;

        if exists != 1 {
            return Err(AppError::Database(format!(
                "catalog snapshot is missing required table '{table}'"
            )));
        }
    }

    let integrity = sqlx::query_scalar::<_, String>("PRAGMA integrity_check")
        .fetch_all(pool)
        .await
        .map_err(|error| db_error("failed to check catalog integrity", error))?;

    if integrity.len() != 1 || !integrity[0].eq_ignore_ascii_case("ok") {
        return Err(AppError::Database(
            "catalog snapshot failed its integrity check".to_string(),
        ));
    }

    let violations = sqlx::query("PRAGMA foreign_key_check")
        .fetch_all(pool)
        .await
        .map_err(|error| db_error("failed to check catalog foreign keys", error))?;

    if !violations.is_empty() {
        return Err(AppError::Database(
            "catalog snapshot contains foreign key violations".to_string(),
        ));
    }

    Ok(())
}

async fn count_rows(pool: &SqlitePool, table: &'static str) -> AppResult<i64> {
    // The table name is a compile-time literal selected from the fixed schema,
    // never user input.
    sqlx::query_scalar::<_, i64>(&format!("SELECT COUNT(*) FROM {table}"))
        .fetch_one(pool)
        .await
        .map_err(|error| db_error("failed to count catalog rows", error))
}

/// Lists candidate update snapshots deterministically.
///
/// Only files with a `.sqlite3` extension are considered. Hidden and temporary
/// files are skipped so an interrupted import cannot be mistaken for a catalog.
fn update_snapshot_paths(directory: &Path) -> Vec<PathBuf> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) => {
            if error.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!(
                    error = %error,
                    path = %directory.display(),
                    "catalog update directory could not be read"
                );
            }
            return Vec::new();
        }
    };

    let mut paths = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            path.extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("sqlite3"))
        })
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_none_or(|name| !name.starts_with('.'))
        })
        .collect::<Vec<_>>();

    paths.sort();
    paths
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::catalog::import::{build_empty_snapshot, build_snapshot};
    use crate::catalog::model::CatalogDocument;
    use crate::catalog::schema::SOURCE_POLICY_VERSION;
    use crate::database::test_support::TestDir;

    fn bundled_catalog() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/catalog/catalog.sqlite3")
    }

    async fn write_empty_version(path: &Path, version: &str) {
        let document = CatalogDocument {
            catalog_version: version.to_string(),
            generated_at: "2026-11-06T00:00:00Z".to_string(),
            source_policy_version: SOURCE_POLICY_VERSION.to_string(),
            last_verified_at: None,
            manufacturers: Vec::new(),
            hardware_devices: Vec::new(),
            driver_sources: Vec::new(),
            drivers: Vec::new(),
        };

        build_snapshot(path, &document, false)
            .await
            .expect("build versioned snapshot");
    }

    #[tokio::test]
    async fn opens_bundled_catalog_read_only() {
        let catalog = CatalogDatabase::open(&bundled_catalog())
            .await
            .expect("open bundled catalog");

        assert_eq!(catalog.schema_version(), 2);
        assert_eq!(
            catalog.catalog_version(),
            crate::catalog::metadata::CatalogVersion::ZERO
        );
        assert_eq!(catalog.origin(), CatalogOrigin::Bundled);

        let health = catalog.health().await.expect("health");
        assert_eq!(health.status, DatabaseComponentStatus::Ok);
        assert_eq!(health.origin, Some(CatalogOrigin::Bundled));
        assert_eq!(health.schema_version, Some(2));
        assert_eq!(health.catalog_version.as_deref(), Some("0.0.0"));
        assert_eq!(health.generated_at.as_deref(), Some("1970-01-01T00:00:00Z"));
        assert_eq!(health.tables, 8);
        assert_eq!(health.manufacturer_count, 0);
        assert_eq!(health.hardware_device_count, 0);
        assert_eq!(health.driver_source_count, 0);
        assert_eq!(health.driver_count, 0);
        assert!(health.read_only);

        // The shipped snapshot contains schema and metadata only.
        for table in [
            "manufacturers",
            "hardware_devices",
            "driver_sources",
            "drivers",
        ] {
            let count = sqlx::query_scalar::<_, i64>(&format!("SELECT COUNT(*) FROM {table}"))
                .fetch_one(catalog.pool())
                .await
                .expect("count catalog rows");
            assert_eq!(count, 0, "{table} must start empty");
        }

        let write = sqlx::query("CREATE TABLE should_not_exist (id INTEGER)")
            .execute(catalog.pool())
            .await;
        assert!(write.is_err(), "catalog connection must reject writes");
    }

    #[tokio::test]
    async fn missing_catalog_is_an_error() {
        let missing = std::env::temp_dir().join("opendrvr-catalog-does-not-exist.sqlite3");
        let result = CatalogDatabase::open(&missing).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn selects_the_highest_valid_update_snapshot() {
        let dir = TestDir::new("catalog-update-select");
        let bundled = dir.path().join("bundled.sqlite3");
        let updates = dir.path().join("updates");
        fs::create_dir_all(&updates).expect("create updates dir");

        build_empty_snapshot(&bundled, false)
            .await
            .expect("build bundled");
        let bundled_size = fs::metadata(&bundled).expect("bundled metadata").len();

        write_empty_version(&updates.join("catalog-2026.1.1.sqlite3"), "2026.1.1").await;
        write_empty_version(&updates.join("catalog-2026.11.6.sqlite3"), "2026.11.6").await;

        let catalog = CatalogDatabase::open_with_updates(&bundled, Some(&updates))
            .await
            .expect("open with updates");

        assert_eq!(catalog.origin(), CatalogOrigin::Update);
        assert_eq!(
            catalog.catalog_version(),
            crate::catalog::metadata::CatalogVersion::new(2026, 11, 6)
        );

        let health = catalog.health().await.expect("health");
        assert_eq!(health.origin, Some(CatalogOrigin::Update));
        assert_eq!(health.catalog_version.as_deref(), Some("2026.11.6"));

        // Selecting an update must never modify the bundled resource.
        assert_eq!(
            fs::metadata(&bundled).expect("bundled metadata").len(),
            bundled_size
        );
    }

    #[tokio::test]
    async fn invalid_updates_fall_back_to_bundled() {
        let dir = TestDir::new("catalog-update-invalid");
        let bundled = dir.path().join("bundled.sqlite3");
        let updates = dir.path().join("updates");
        fs::create_dir_all(&updates).expect("create updates dir");

        build_empty_snapshot(&bundled, false)
            .await
            .expect("build bundled");
        fs::write(updates.join("catalog-broken.sqlite3"), b"not a database")
            .expect("write invalid update");

        let catalog = CatalogDatabase::open_with_updates(&bundled, Some(&updates))
            .await
            .expect("fall back to bundled");

        assert_eq!(catalog.origin(), CatalogOrigin::Bundled);
        assert_eq!(
            catalog.catalog_version(),
            crate::catalog::metadata::CatalogVersion::ZERO
        );
    }

    #[tokio::test]
    async fn older_updates_do_not_override_bundled_content() {
        let dir = TestDir::new("catalog-update-older");
        let bundled = dir.path().join("bundled.sqlite3");
        let updates = dir.path().join("updates");
        fs::create_dir_all(&updates).expect("create updates dir");

        write_empty_version(&bundled, "2026.11.6").await;
        write_empty_version(&updates.join("catalog-2026.1.1.sqlite3"), "2026.1.1").await;

        let catalog = CatalogDatabase::open_with_updates(&bundled, Some(&updates))
            .await
            .expect("open");

        assert_eq!(catalog.origin(), CatalogOrigin::Bundled);
        assert_eq!(
            catalog.catalog_version(),
            crate::catalog::metadata::CatalogVersion::new(2026, 11, 6)
        );
    }

    #[tokio::test]
    async fn rejects_snapshot_with_an_older_schema_version() {
        let dir = TestDir::new("catalog-old-schema");
        let path = dir.path().join("old.sqlite3");

        build_empty_snapshot(&path, false)
            .await
            .expect("build snapshot");

        let options = SqliteConnectOptions::new().filename(&path);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .expect("open writable snapshot");
        sqlx::query("UPDATE catalog_meta SET value = '1' WHERE key = 'schema_version'")
            .execute(&pool)
            .await
            .expect("downgrade metadata");
        pool.close().await;

        assert!(CatalogDatabase::open(&path).await.is_err());
    }

    #[tokio::test]
    async fn rejects_snapshot_missing_required_tables() {
        let dir = TestDir::new("catalog-schema");
        let path = dir.path().join("incomplete.sqlite3");

        let options = SqliteConnectOptions::new()
            .filename(&path)
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .expect("create incomplete snapshot");

        sqlx::raw_sql(
            "CREATE TABLE catalog_meta (key TEXT PRIMARY KEY NOT NULL, value TEXT NOT NULL);
             INSERT INTO catalog_meta (key, value) VALUES
                 ('schema_version', '2'),
                 ('catalog_format', 'opendrvr.catalog.v2'),
                 ('catalog_version', '1.0.0'),
                 ('generated_at', '2026-11-06T00:00:00Z'),
                 ('source_policy_version', '1');",
        )
        .execute(&pool)
        .await
        .expect("write metadata");
        pool.close().await;

        assert!(CatalogDatabase::open(&path).await.is_err());
    }
}
