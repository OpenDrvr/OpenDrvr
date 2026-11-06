//! Regenerates the bundled catalog snapshot from `resources/catalog/schema.sql`.
//!
//! This is a maintenance tool, not part of the application runtime. The catalog
//! contains schema metadata only until real hardware and driver data exists.
//!
//! Run from `src-tauri`:
//!
//! ```powershell
//! cargo run --example build_catalog
//! ```

use std::{fs, path::PathBuf};

use sqlx::sqlite::SqliteConnectOptions;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tauri::async_runtime::block_on(async {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let schema_path = root.join("resources/catalog/schema.sql");
        let catalog_path = root.join("resources/catalog/catalog.sqlite3");

        let schema = fs::read_to_string(&schema_path)?;

        if catalog_path.exists() {
            fs::remove_file(&catalog_path)?;
        }

        let options = SqliteConnectOptions::new()
            .filename(&catalog_path)
            .create_if_missing(true);
        let pool = sqlx::SqlitePool::connect_with(options).await?;

        sqlx::raw_sql(&schema).execute(&pool).await?;
        pool.close().await;

        println!("wrote {}", catalog_path.display());
        Ok::<(), Box<dyn std::error::Error>>(())
    })
}
