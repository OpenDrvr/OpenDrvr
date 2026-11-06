//! Driver catalog domain.
//!
//! The catalog is metadata only. It describes hardware identifiers, driver
//! packages, Windows compatibility, and provenance, but never contains driver
//! binaries. Runtime access is read-only; import and update tooling lives in
//! this module and in `examples/import_catalog.rs`.

pub mod import;
pub mod metadata;
pub mod model;
pub mod schema;
pub mod source_policy;
pub mod time;
pub mod validate;

pub use metadata::{CatalogMetadata, CatalogVersion};
pub use schema::{
    CATALOG_FORMAT, EMPTY_CATALOG_VERSION, EMPTY_GENERATED_AT, EXPECTED_TABLES, SCHEMA_SQL,
    SCHEMA_VERSION, SOURCE_POLICY_VERSION,
};
