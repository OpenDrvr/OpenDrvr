//! Regenerates the bundled catalog snapshot from the embedded v2 schema.
//!
//! This is a maintenance tool, not part of the application runtime. The
//! generated snapshot contains schema metadata only until verified hardware and
//! driver records exist. Real records must be imported with
//! `examples/import_catalog.rs` so validation and provenance rules apply.
//!
//! Run from `src-tauri`:
//!
//! ```powershell
//! cargo run --example build_catalog
//! ```

use std::path::PathBuf;

use opendrvr_lib::catalog::import::build_empty_snapshot;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tauri::async_runtime::block_on(async {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let catalog_path = root.join("resources/catalog/catalog.sqlite3");

        let summary = build_empty_snapshot(&catalog_path, true).await?;

        println!(
            "wrote {} (schema v{}, catalog v{}, {} drivers)",
            catalog_path.display(),
            summary.schema_version,
            summary.catalog_version,
            summary.driver_count
        );
        Ok::<(), Box<dyn std::error::Error>>(())
    })
}
