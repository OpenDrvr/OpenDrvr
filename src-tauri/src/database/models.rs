use serde::Serialize;

/// Status of one side of the database split.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseComponentStatus {
    Ok,
    Unavailable,
}

/// Combined health of the writable user database and the bundled catalog.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseHealth {
    pub user: UserDatabaseHealth,
    pub catalog: CatalogDatabaseHealth,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserDatabaseHealth {
    pub status: DatabaseComponentStatus,
    pub schema_version: i64,
    pub tables: i64,
    pub writable: bool,
    pub message: Option<String>,
}

impl UserDatabaseHealth {
    pub fn unavailable(message: impl Into<String>) -> Self {
        Self {
            status: DatabaseComponentStatus::Unavailable,
            schema_version: 0,
            tables: 0,
            writable: false,
            message: Some(message.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogDatabaseHealth {
    pub status: DatabaseComponentStatus,
    pub schema_version: Option<i64>,
    pub tables: i64,
    pub manufacturer_count: i64,
    pub read_only: bool,
    pub message: Option<String>,
}

impl CatalogDatabaseHealth {
    pub fn unavailable(message: impl Into<String>) -> Self {
        Self {
            status: DatabaseComponentStatus::Unavailable,
            schema_version: None,
            tables: 0,
            manufacturer_count: 0,
            read_only: true,
            message: Some(message.into()),
        }
    }
}

/// Result of the harmless diagnostic query exposed through Tauri.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseTestResult {
    pub user_database_ready: bool,
    pub catalog_database_ready: bool,
    pub preference: Option<String>,
    pub catalog_schema_version: Option<i64>,
    pub catalog_manufacturer_count: Option<i64>,
}
