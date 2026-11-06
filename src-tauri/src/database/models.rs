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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CatalogOrigin {
    /// The snapshot shipped as a Tauri resource.
    Bundled,
    /// A newer immutable snapshot from the per-user update directory.
    Update,
}

impl CatalogOrigin {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Bundled => "bundled",
            Self::Update => "update",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogDatabaseHealth {
    pub status: DatabaseComponentStatus,
    pub origin: Option<CatalogOrigin>,
    pub schema_version: Option<i64>,
    pub catalog_version: Option<String>,
    pub generated_at: Option<String>,
    pub last_verified_at: Option<String>,
    pub tables: i64,
    pub manufacturer_count: i64,
    pub hardware_device_count: i64,
    pub driver_source_count: i64,
    pub driver_count: i64,
    pub read_only: bool,
    pub message: Option<String>,
}

impl CatalogDatabaseHealth {
    pub fn unavailable(message: impl Into<String>) -> Self {
        Self {
            status: DatabaseComponentStatus::Unavailable,
            origin: None,
            schema_version: None,
            catalog_version: None,
            generated_at: None,
            last_verified_at: None,
            tables: 0,
            manufacturer_count: 0,
            hardware_device_count: 0,
            driver_source_count: 0,
            driver_count: 0,
            read_only: true,
            message: Some(message.into()),
        }
    }
}
