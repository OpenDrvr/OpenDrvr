//! Import document model.
//!
//! This is the offline interchange format consumed by `import_catalog`. It is
//! intentionally nested so a source extract can describe a driver, its hardware
//! matches, Windows compatibility, and provenance together. The importer
//! normalizes it into the relational catalog tables.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogDocument {
    /// Content version of this snapshot, for example `2026.11.6`.
    pub catalog_version: String,
    /// RFC 3339 timestamp describing when the snapshot was generated.
    pub generated_at: String,
    /// Source policy version the extract was validated against.
    pub source_policy_version: String,
    /// Optional RFC 3339 timestamp for the oldest unverified record.
    #[serde(default)]
    pub last_verified_at: Option<String>,
    #[serde(default)]
    pub manufacturers: Vec<ManufacturerRecord>,
    #[serde(default)]
    pub hardware_devices: Vec<HardwareDeviceRecord>,
    #[serde(default)]
    pub driver_sources: Vec<DriverSourceRecord>,
    #[serde(default)]
    pub drivers: Vec<DriverRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManufacturerRecord {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub website_url: Option<String>,
    #[serde(default)]
    pub support_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HardwareDeviceRecord {
    /// A full Windows hardware ID, such as `PCI\VEN_8086&DEV_1234`.
    pub hardware_id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub manufacturer_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DriverSourceRecord {
    pub id: String,
    #[serde(default)]
    pub manufacturer_id: Option<String>,
    pub name: String,
    pub kind: DriverSourceKind,
    pub base_url: String,
    #[serde(default = "default_true")]
    pub official: bool,
    /// Human-readable attribution shown when this source is cited.
    pub attribution: String,
    #[serde(default)]
    pub last_verified_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DriverRecord {
    pub id: String,
    pub manufacturer_id: String,
    /// Driver provider reported by the package, which may differ from the
    /// device or system manufacturer.
    pub provider: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub release_date: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    pub package_type: PackageType,
    #[serde(default)]
    pub file_name: Option<String>,
    /// Direct official package URL. Windows Update entries may omit it.
    #[serde(default)]
    pub download_url: Option<String>,
    #[serde(default)]
    pub size_bytes: Option<i64>,
    #[serde(default)]
    pub checksum_sha256: Option<String>,
    /// RFC 3339 timestamp for the most recent verification of this record.
    pub last_verified_at: String,
    #[serde(default)]
    pub hardware: Vec<DriverHardwareRecord>,
    #[serde(default)]
    pub windows: Vec<WindowsCompatRecord>,
    #[serde(default)]
    pub sources: Vec<DriverSourceRefRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DriverHardwareRecord {
    pub hardware_id: String,
    pub match_quality: MatchQuality,
    #[serde(default)]
    pub evidence: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WindowsCompatRecord {
    pub windows_version: WindowsVersion,
    pub architecture: Architecture,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DriverSourceRefRecord {
    pub source_id: String,
    pub url: String,
    /// RFC 3339 timestamp describing when this reference was retrieved.
    pub retrieved_at: String,
    pub attribution: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DriverSourceKind {
    Manufacturer,
    WindowsUpdate,
    Oem,
    ComponentVendor,
    Other,
}

impl DriverSourceKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Manufacturer => "manufacturer",
            Self::WindowsUpdate => "windows-update",
            Self::Oem => "oem",
            Self::ComponentVendor => "component-vendor",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PackageType {
    Installer,
    Inf,
    Cab,
    Msi,
    Exe,
    Zip,
    WindowsUpdate,
    Other,
}

impl PackageType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Installer => "installer",
            Self::Inf => "inf",
            Self::Cab => "cab",
            Self::Msi => "msi",
            Self::Exe => "exe",
            Self::Zip => "zip",
            Self::WindowsUpdate => "windows-update",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MatchQuality {
    Exact,
    Subsystem,
    Compatible,
}

impl MatchQuality {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Subsystem => "subsystem",
            Self::Compatible => "compatible",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum WindowsVersion {
    #[serde(rename = "windows-10")]
    Windows10,
    #[serde(rename = "windows-11")]
    Windows11,
}

impl WindowsVersion {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Windows10 => "windows-10",
            Self::Windows11 => "windows-11",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Architecture {
    #[serde(rename = "x86")]
    X86,
    #[serde(rename = "x64")]
    X64,
    #[serde(rename = "arm64")]
    Arm64,
}

impl Architecture {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::X86 => "x86",
            Self::X64 => "x64",
            Self::Arm64 => "arm64",
        }
    }
}

fn default_true() -> bool {
    true
}
