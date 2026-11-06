use std::path::Path;
use std::time::Duration;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;

use crate::database::models::{CatalogDatabaseHealth, DatabaseComponentStatus};
use crate::database::{db_error, table_count};
use crate::error::{AppError, AppResult};

/// Read-only handle to the bundled catalog snapshot.
///
/// The file is shipped as a Tauri resource and opened with SQLite's read-only
/// flag, so accidental writes fail at the database layer. Future incremental
/// catalog updates should produce a new immutable snapshot rather than
/// mutating this file in place.
#[derive(Debug)]
pub struct CatalogDatabase {
    pool: SqlitePool,
    schema_version: i64,
}

impl CatalogDatabase {
    pub async fn open(path: &Path) -> AppResult<Self> {
        if !path.is_file() {
            return Err(AppError::Database(format!(
                "bundled catalog not found at {}",
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
            .map_err(|error| db_error("failed to open bundled catalog", error))?;

        let schema_version = match read_schema_version(&pool).await {
            Ok(version) => version,
            Err(error) => {
                pool.close().await;
                return Err(error);
            }
        };

        Ok(Self {
            pool,
            schema_version,
        })
    }

    pub fn schema_version(&self) -> i64 {
        self.schema_version
    }

    pub async fn health(&self) -> AppResult<CatalogDatabaseHealth> {
        let tables = table_count(&self.pool).await?;
        let manufacturer_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM manufacturers")
            .fetch_one(&self.pool)
            .await
            .map_err(|error| db_error("failed to count catalog manufacturers", error))?;

        Ok(CatalogDatabaseHealth {
            status: DatabaseComponentStatus::Ok,
            schema_version: Some(self.schema_version),
            tables,
            manufacturer_count,
            read_only: true,
            message: None,
        })
    }

    #[cfg(test)]
    pub(crate) fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

async fn read_schema_version(pool: &SqlitePool) -> AppResult<i64> {
    let value = sqlx::query_scalar::<_, String>("SELECT value FROM catalog_meta WHERE key = ?1")
        .bind("schema_version")
        .fetch_optional(pool)
        .await
        .map_err(|error| db_error("failed to read catalog schema version", error))?
        .ok_or_else(|| {
            AppError::Database("catalog metadata is missing schema_version".to_string())
        })?;

    value
        .parse::<i64>()
        .map_err(|_| AppError::Database("catalog schema_version is not an integer".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bundled_catalog() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/catalog/catalog.sqlite3")
    }

    #[tokio::test]
    async fn opens_bundled_catalog_read_only() {
        let catalog = CatalogDatabase::open(&bundled_catalog())
            .await
            .expect("open bundled catalog");

        assert_eq!(catalog.schema_version(), 1);

        let health = catalog.health().await.expect("health");
        assert_eq!(health.status, DatabaseComponentStatus::Ok);
        assert_eq!(health.schema_version, Some(1));
        assert_eq!(health.tables, 8);
        assert_eq!(health.manufacturer_count, 0);
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
}
