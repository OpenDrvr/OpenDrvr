//! Catalog schema constants and the embedded schema source.
//!
//! The shipped SQLite snapshot is generated from `resources/catalog/schema.sql`.
//! Keeping the SQL embedded means the import tool and the runtime agree on the
//! exact table layout without reading files relative to the process working
//! directory.

/// Catalog schema version understood by this build.
pub const SCHEMA_VERSION: i64 = 2;

/// Stable identifier for the catalog file format.
pub const CATALOG_FORMAT: &str = "opendrvr.catalog.v2";

/// Version of the source and validation policy documented in `CATALOG.md`.
pub const SOURCE_POLICY_VERSION: &str = "1";

/// Catalog version used by the schema-only snapshot that ships with the app.
pub const EMPTY_CATALOG_VERSION: &str = "0.0.0";

/// Deterministic timestamp used by the schema-only snapshot.
pub const EMPTY_GENERATED_AT: &str = "1970-01-01T00:00:00Z";

/// Tables every catalog snapshot must contain. Extra tables are tolerated so a
/// future additive format can still be read by this build.
pub const EXPECTED_TABLES: &[&str] = &[
    "catalog_meta",
    "manufacturers",
    "hardware_devices",
    "driver_sources",
    "drivers",
    "driver_hardware",
    "driver_windows_compat",
    "driver_source_refs",
];

/// The complete catalog DDL, embedded at compile time.
pub const SCHEMA_SQL: &str = include_str!("../../resources/catalog/schema.sql");
