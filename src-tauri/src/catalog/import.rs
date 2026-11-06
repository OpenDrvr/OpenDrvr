//! Builds immutable catalog snapshots from validated import documents.
//!
//! The writer never mutates an existing snapshot. It creates a temporary SQLite
//! file in the destination directory, applies the schema, inserts normalized
//! rows in one transaction, verifies integrity and foreign keys, and only then
//! moves the finished file into place. Update delivery should use a new,
//! versioned file name rather than replacing the bundled resource.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{Sqlite, SqlitePool, Transaction};
use thiserror::Error;

use crate::catalog::metadata::{CatalogMetadata, CatalogVersion};
use crate::catalog::model::CatalogDocument;
use crate::catalog::schema::{
    CATALOG_FORMAT, EMPTY_CATALOG_VERSION, EMPTY_GENERATED_AT, SCHEMA_SQL, SCHEMA_VERSION,
    SOURCE_POLICY_VERSION,
};
use crate::catalog::validate::{
    normalize_document, parse_hardware_identity, validate_document, ValidationReport,
};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Error)]
pub enum CatalogImportError {
    #[error("catalog validation failed: {0}")]
    Validation(ValidationReport),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("catalog build failed: {0}")]
    Build(String),
}

/// Counts and versions describing a completed snapshot build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogBuildSummary {
    pub schema_version: i64,
    pub catalog_version: CatalogVersion,
    pub manufacturer_count: usize,
    pub hardware_device_count: usize,
    pub driver_source_count: usize,
    pub driver_count: usize,
    pub warning_count: usize,
}

/// Builds a schema-only snapshot with deterministic metadata.
///
/// This is the snapshot bundled with the application until verified catalog
/// records exist. It intentionally contains no manufacturers or drivers.
pub async fn build_empty_snapshot(
    output: &Path,
    force: bool,
) -> Result<CatalogBuildSummary, CatalogImportError> {
    let document = CatalogDocument {
        catalog_version: EMPTY_CATALOG_VERSION.to_string(),
        generated_at: EMPTY_GENERATED_AT.to_string(),
        source_policy_version: SOURCE_POLICY_VERSION.to_string(),
        last_verified_at: None,
        manufacturers: Vec::new(),
        hardware_devices: Vec::new(),
        driver_sources: Vec::new(),
        drivers: Vec::new(),
    };

    build_snapshot(output, &document, force).await
}

/// Validates an import document and writes a complete catalog snapshot.
///
/// `force` only permits replacing the tool's output path. It never changes the
/// behavior of an installed catalog, which is always opened read-only.
pub async fn build_snapshot(
    output: &Path,
    document: &CatalogDocument,
    force: bool,
) -> Result<CatalogBuildSummary, CatalogImportError> {
    let report = validate_document(document);
    if !report.is_valid() {
        return Err(CatalogImportError::Validation(report));
    }

    let normalized = normalize_document(document);
    let catalog_version =
        CatalogVersion::parse(&normalized.catalog_version).map_err(CatalogImportError::Build)?;

    let metadata = CatalogMetadata {
        schema_version: SCHEMA_VERSION,
        catalog_format: CATALOG_FORMAT.to_string(),
        catalog_version,
        generated_at: normalized.generated_at.clone(),
        source_policy_version: normalized.source_policy_version.clone(),
        last_verified_at: normalized.last_verified_at.clone(),
    };
    metadata
        .validate_runtime_compatibility()
        .map_err(CatalogImportError::Build)?;

    let mut summary = write_snapshot(output, &normalized, &metadata, force).await?;
    summary.warning_count = report.warnings.len();
    Ok(summary)
}

async fn write_snapshot(
    output: &Path,
    document: &CatalogDocument,
    metadata: &CatalogMetadata,
    force: bool,
) -> Result<CatalogBuildSummary, CatalogImportError> {
    if output.exists() && !force {
        return Err(CatalogImportError::Build(format!(
            "output already exists: {}",
            output.display()
        )));
    }

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }

    let temp = temp_path(output);
    if temp.exists() {
        fs::remove_file(&temp)?;
    }

    match write_snapshot_file(&temp, document, metadata).await {
        Ok(summary) => {
            if output.exists() {
                fs::remove_file(output)?;
            }
            fs::rename(&temp, output)?;
            Ok(summary)
        }
        Err(error) => {
            let _ = fs::remove_file(&temp);
            Err(error)
        }
    }
}

async fn write_snapshot_file(
    path: &Path,
    document: &CatalogDocument,
    metadata: &CatalogMetadata,
) -> Result<CatalogBuildSummary, CatalogImportError> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Delete)
        .synchronous(SqliteSynchronous::Full);

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;

    sqlx::raw_sql(SCHEMA_SQL).execute(&pool).await?;

    let mut transaction = pool.begin().await?;
    insert_metadata(&mut transaction, metadata).await?;
    insert_manufacturers(&mut transaction, document).await?;
    let hardware_ids = insert_hardware_devices(&mut transaction, document).await?;
    insert_driver_sources(&mut transaction, document).await?;
    insert_drivers(&mut transaction, document, &hardware_ids).await?;
    transaction.commit().await?;

    verify_snapshot(&pool, document).await?;
    pool.close().await;

    Ok(CatalogBuildSummary {
        schema_version: SCHEMA_VERSION,
        catalog_version: metadata.catalog_version,
        manufacturer_count: document.manufacturers.len(),
        hardware_device_count: document.hardware_devices.len(),
        driver_source_count: document.driver_sources.len(),
        driver_count: document.drivers.len(),
        warning_count: 0,
    })
}

async fn insert_metadata(
    transaction: &mut Transaction<'_, Sqlite>,
    metadata: &CatalogMetadata,
) -> Result<(), CatalogImportError> {
    let entries = [
        ("schema_version", metadata.schema_version.to_string()),
        ("catalog_format", metadata.catalog_format.clone()),
        ("catalog_version", metadata.catalog_version.to_string()),
        ("generated_at", metadata.generated_at.clone()),
        (
            "source_policy_version",
            metadata.source_policy_version.clone(),
        ),
    ];

    for (key, value) in entries {
        insert_meta(transaction, key, &value).await?;
    }

    if let Some(value) = &metadata.last_verified_at {
        insert_meta(transaction, "last_verified_at", value).await?;
    }

    Ok(())
}

async fn insert_meta(
    transaction: &mut Transaction<'_, Sqlite>,
    key: &str,
    value: &str,
) -> Result<(), CatalogImportError> {
    sqlx::query("INSERT INTO catalog_meta (key, value) VALUES (?1, ?2)")
        .bind(key)
        .bind(value)
        .execute(&mut **transaction)
        .await?;
    Ok(())
}

async fn insert_manufacturers(
    transaction: &mut Transaction<'_, Sqlite>,
    document: &CatalogDocument,
) -> Result<(), CatalogImportError> {
    for manufacturer in &document.manufacturers {
        sqlx::query(
            "INSERT INTO manufacturers (id, name, website_url, support_url)
             VALUES (?1, ?2, ?3, ?4)",
        )
        .bind(&manufacturer.id)
        .bind(&manufacturer.name)
        .bind(&manufacturer.website_url)
        .bind(&manufacturer.support_url)
        .execute(&mut **transaction)
        .await?;
    }

    Ok(())
}

async fn insert_hardware_devices(
    transaction: &mut Transaction<'_, Sqlite>,
    document: &CatalogDocument,
) -> Result<HashMap<String, i64>, CatalogImportError> {
    let mut ids = HashMap::new();

    for device in &document.hardware_devices {
        let identity =
            parse_hardware_identity(&device.hardware_id).map_err(CatalogImportError::Build)?;

        let result = sqlx::query(
            "INSERT INTO hardware_devices (
                 bus_type, hardware_id, vendor_id, device_id,
                 subsystem_vendor_id, subsystem_device_id, revision, class_code,
                 name, manufacturer_id
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        )
        .bind(identity.bus_type)
        .bind(&identity.hardware_id)
        .bind(&identity.vendor_id)
        .bind(&identity.device_id)
        .bind(&identity.subsystem_vendor_id)
        .bind(&identity.subsystem_device_id)
        .bind(&identity.revision)
        .bind(&identity.class_code)
        .bind(&device.name)
        .bind(&device.manufacturer_id)
        .execute(&mut **transaction)
        .await?;

        ids.insert(identity.hardware_id, result.last_insert_rowid());
    }

    Ok(ids)
}

async fn insert_driver_sources(
    transaction: &mut Transaction<'_, Sqlite>,
    document: &CatalogDocument,
) -> Result<(), CatalogImportError> {
    for source in &document.driver_sources {
        sqlx::query(
            "INSERT INTO driver_sources (
                 id, manufacturer_id, name, kind, base_url, official,
                 attribution, last_verified_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind(&source.id)
        .bind(&source.manufacturer_id)
        .bind(&source.name)
        .bind(source.kind.as_str())
        .bind(&source.base_url)
        .bind(i64::from(source.official))
        .bind(&source.attribution)
        .bind(&source.last_verified_at)
        .execute(&mut **transaction)
        .await?;
    }

    Ok(())
}

async fn insert_drivers(
    transaction: &mut Transaction<'_, Sqlite>,
    document: &CatalogDocument,
    hardware_ids: &HashMap<String, i64>,
) -> Result<(), CatalogImportError> {
    for driver in &document.drivers {
        sqlx::query(
            "INSERT INTO drivers (
                 id, manufacturer_id, provider, name, version, release_date,
                 description, package_type, file_name, download_url, size_bytes,
                 checksum_sha256, last_verified_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        )
        .bind(&driver.id)
        .bind(&driver.manufacturer_id)
        .bind(&driver.provider)
        .bind(&driver.name)
        .bind(&driver.version)
        .bind(&driver.release_date)
        .bind(&driver.description)
        .bind(driver.package_type.as_str())
        .bind(&driver.file_name)
        .bind(&driver.download_url)
        .bind(driver.size_bytes)
        .bind(&driver.checksum_sha256)
        .bind(&driver.last_verified_at)
        .execute(&mut **transaction)
        .await?;

        for link in &driver.hardware {
            let identity =
                parse_hardware_identity(&link.hardware_id).map_err(CatalogImportError::Build)?;
            let hardware_id = hardware_ids.get(&identity.hardware_id).ok_or_else(|| {
                CatalogImportError::Build(format!(
                    "hardware ID '{}' was not inserted",
                    identity.hardware_id
                ))
            })?;

            sqlx::query(
                "INSERT INTO driver_hardware (
                     driver_id, hardware_device_id, match_quality, evidence
                 ) VALUES (?1, ?2, ?3, ?4)",
            )
            .bind(&driver.id)
            .bind(hardware_id)
            .bind(link.match_quality.as_str())
            .bind(&link.evidence)
            .execute(&mut **transaction)
            .await?;
        }

        for compatibility in &driver.windows {
            sqlx::query(
                "INSERT INTO driver_windows_compat (
                     driver_id, windows_version, architecture
                 ) VALUES (?1, ?2, ?3)",
            )
            .bind(&driver.id)
            .bind(compatibility.windows_version.as_str())
            .bind(compatibility.architecture.as_str())
            .execute(&mut **transaction)
            .await?;
        }

        for source_ref in &driver.sources {
            sqlx::query(
                "INSERT INTO driver_source_refs (
                     driver_id, source_id, url, is_official, retrieved_at, attribution
                 ) VALUES (?1, ?2, ?3, 1, ?4, ?5)",
            )
            .bind(&driver.id)
            .bind(&source_ref.source_id)
            .bind(&source_ref.url)
            .bind(&source_ref.retrieved_at)
            .bind(&source_ref.attribution)
            .execute(&mut **transaction)
            .await?;
        }
    }

    Ok(())
}

async fn verify_snapshot(
    pool: &SqlitePool,
    document: &CatalogDocument,
) -> Result<(), CatalogImportError> {
    let integrity = sqlx::query_scalar::<_, String>("PRAGMA integrity_check")
        .fetch_all(pool)
        .await?;

    if integrity.len() != 1 || !integrity[0].eq_ignore_ascii_case("ok") {
        return Err(CatalogImportError::Build(format!(
            "catalog integrity check failed: {}",
            integrity.join("; ")
        )));
    }

    let violations = sqlx::query("PRAGMA foreign_key_check")
        .fetch_all(pool)
        .await?;
    if !violations.is_empty() {
        return Err(CatalogImportError::Build(format!(
            "catalog foreign key check found {} violation(s)",
            violations.len()
        )));
    }

    for (table, expected) in [
        ("manufacturers", document.manufacturers.len()),
        ("hardware_devices", document.hardware_devices.len()),
        ("driver_sources", document.driver_sources.len()),
        ("drivers", document.drivers.len()),
    ] {
        let count = sqlx::query_scalar::<_, i64>(&format!("SELECT COUNT(*) FROM {table}"))
            .fetch_one(pool)
            .await?;

        if count != expected as i64 {
            return Err(CatalogImportError::Build(format!(
                "table {table} contains {count} row(s), expected {expected}"
            )));
        }
    }

    Ok(())
}

fn temp_path(output: &Path) -> PathBuf {
    let file_name = output
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("catalog.sqlite3");
    let unique = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);

    output.with_file_name(format!(".{file_name}.tmp-{}-{unique}", std::process::id()))
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::catalog::model::{
        Architecture, DriverHardwareRecord, DriverRecord, DriverSourceKind, DriverSourceRecord,
        DriverSourceRefRecord, HardwareDeviceRecord, ManufacturerRecord, MatchQuality, PackageType,
        WindowsCompatRecord, WindowsVersion,
    };
    use crate::catalog::validate::validate_document;

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "opendrvr-catalog-test-{}-{name}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).expect("create test directory");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn document() -> CatalogDocument {
        CatalogDocument {
            catalog_version: "2026.11.6".to_string(),
            generated_at: "2026-11-06T00:00:00Z".to_string(),
            source_policy_version: SOURCE_POLICY_VERSION.to_string(),
            last_verified_at: Some("2026-11-05T12:00:00Z".to_string()),
            manufacturers: vec![ManufacturerRecord {
                id: "test-manufacturer".to_string(),
                name: "Test Manufacturer".to_string(),
                website_url: Some("https://www.intel.com".to_string()),
                support_url: None,
            }],
            hardware_devices: vec![HardwareDeviceRecord {
                hardware_id: r"PCI\VEN_8086&DEV_1234&SUBSYS_56781234&REV_01".to_string(),
                name: Some("Test Device".to_string()),
                manufacturer_id: Some("test-manufacturer".to_string()),
            }],
            driver_sources: vec![DriverSourceRecord {
                id: "test-source".to_string(),
                manufacturer_id: Some("test-manufacturer".to_string()),
                name: "Test Source".to_string(),
                kind: DriverSourceKind::Manufacturer,
                base_url: "https://www.intel.com/content/www/us/en/download-center/home.html"
                    .to_string(),
                official: true,
                attribution: "Test Manufacturer official download center".to_string(),
                last_verified_at: Some("2026-11-05T12:00:00Z".to_string()),
            }],
            drivers: vec![DriverRecord {
                id: "test-driver".to_string(),
                manufacturer_id: "test-manufacturer".to_string(),
                provider: "Test Provider".to_string(),
                name: "Test Driver".to_string(),
                version: "1.2.3".to_string(),
                release_date: Some("2026-10-01".to_string()),
                description: None,
                package_type: PackageType::Installer,
                file_name: Some("test-driver.exe".to_string()),
                download_url: Some("https://www.intel.com/content/dam/test-driver.exe".to_string()),
                size_bytes: Some(1024),
                checksum_sha256: Some("a".repeat(64)),
                last_verified_at: "2026-11-05T12:00:00Z".to_string(),
                hardware: vec![DriverHardwareRecord {
                    hardware_id: r"PCI\VEN_8086&DEV_1234&SUBSYS_56781234&REV_01".to_string(),
                    match_quality: MatchQuality::Subsystem,
                    evidence: Some("Vendor support page lists this subsystem ID".to_string()),
                }],
                windows: vec![WindowsCompatRecord {
                    windows_version: WindowsVersion::Windows11,
                    architecture: Architecture::X64,
                }],
                sources: vec![DriverSourceRefRecord {
                    source_id: "test-source".to_string(),
                    url: "https://www.intel.com/content/www/us/en/download-center/home.html"
                        .to_string(),
                    retrieved_at: "2026-11-05T12:00:00Z".to_string(),
                    attribution: "Test Manufacturer official download center".to_string(),
                }],
            }],
        }
    }

    #[tokio::test]
    async fn builds_and_reopens_a_catalog_snapshot() {
        let dir = TestDir::new("build");
        let output = dir.path().join("catalog.sqlite3");

        let summary = build_snapshot(&output, &document(), false)
            .await
            .expect("build snapshot");
        assert_eq!(summary.schema_version, SCHEMA_VERSION);
        assert_eq!(summary.catalog_version, CatalogVersion::new(2026, 11, 6));
        assert_eq!(summary.driver_count, 1);
        assert_eq!(summary.warning_count, 0);

        let options = SqliteConnectOptions::new()
            .filename(&output)
            .read_only(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .expect("open snapshot");

        let drivers = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM drivers")
            .fetch_one(&pool)
            .await
            .expect("count drivers");
        assert_eq!(drivers, 1);

        let metadata = sqlx::query_as::<_, (String, String)>(
            "SELECT key, value FROM catalog_meta WHERE key = 'catalog_version'",
        )
        .fetch_one(&pool)
        .await
        .expect("metadata");
        assert_eq!(metadata.1, "2026.11.6");

        let write = sqlx::query("CREATE TABLE should_not_exist (id INTEGER)")
            .execute(&pool)
            .await;
        assert!(
            write.is_err(),
            "snapshot must be read-only when opened that way"
        );

        pool.close().await;
    }

    #[tokio::test]
    async fn builds_a_schema_only_snapshot() {
        let dir = TestDir::new("empty");
        let output = dir.path().join("catalog.sqlite3");

        let summary = build_empty_snapshot(&output, false)
            .await
            .expect("build empty snapshot");
        assert_eq!(summary.catalog_version, CatalogVersion::ZERO);
        assert_eq!(summary.driver_count, 0);
        assert_eq!(summary.warning_count, 0);

        let options = SqliteConnectOptions::new()
            .filename(&output)
            .read_only(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .expect("open snapshot");

        let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM drivers")
            .fetch_one(&pool)
            .await
            .expect("count drivers");
        assert_eq!(count, 0);
        pool.close().await;
    }

    #[tokio::test]
    async fn refuses_to_overwrite_without_force() {
        let dir = TestDir::new("force");
        let output = dir.path().join("catalog.sqlite3");

        build_empty_snapshot(&output, false)
            .await
            .expect("first build");
        assert!(build_empty_snapshot(&output, false).await.is_err());
        build_empty_snapshot(&output, true)
            .await
            .expect("forced rebuild");
    }

    #[tokio::test]
    async fn validation_failure_does_not_create_output() {
        let dir = TestDir::new("invalid");
        let output = dir.path().join("catalog.sqlite3");
        let mut document = document();
        document.drivers[0].sources.clear();

        let error = build_snapshot(&output, &document, false)
            .await
            .expect_err("invalid document");
        assert!(matches!(error, CatalogImportError::Validation(_)));
        assert!(!output.exists());
    }

    #[tokio::test]
    async fn normalized_document_still_validates() {
        let normalized = normalize_document(&document());
        assert!(validate_document(&normalized).is_valid());
    }
}
