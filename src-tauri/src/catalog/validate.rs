//! Catalog import validation.
//!
//! Validation is deliberately strict about identity, provenance, and official
//! URLs. A malformed or unsupported claim must fail the import instead of
//! producing a catalog row that later looks authoritative.

use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::catalog::metadata::CatalogVersion;
use crate::catalog::model::{
    CatalogDocument, DriverRecord, DriverSourceKind, MatchQuality, WindowsCompatRecord,
};
use crate::catalog::schema::SOURCE_POLICY_VERSION;
use crate::catalog::source_policy::{validate_official_url, validate_source_url};
use crate::catalog::time::{is_iso_date, is_rfc3339_timestamp};
use crate::hardware::id::{HardwareId, HardwareIdKind};

/// Normalized hardware identity derived from a Windows hardware ID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardwareIdentity {
    pub bus_type: &'static str,
    pub hardware_id: String,
    pub vendor_id: Option<String>,
    pub device_id: Option<String>,
    pub subsystem_vendor_id: Option<String>,
    pub subsystem_device_id: Option<String>,
    pub revision: Option<String>,
    pub class_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationIssue {
    pub path: String,
    pub message: String,
}

impl fmt::Display for ValidationIssue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.path, self.message)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ValidationReport {
    pub errors: Vec<ValidationIssue>,
    pub warnings: Vec<ValidationIssue>,
}

impl ValidationReport {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn error(&mut self, path: impl Into<String>, message: impl Into<String>) {
        self.errors.push(ValidationIssue {
            path: path.into(),
            message: message.into(),
        });
    }

    pub fn warning(&mut self, path: impl Into<String>, message: impl Into<String>) {
        self.warnings.push(ValidationIssue {
            path: path.into(),
            message: message.into(),
        });
    }
}

impl fmt::Display for ValidationReport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} error(s), {} warning(s)",
            self.errors.len(),
            self.warnings.len()
        )
    }
}

/// Validates a complete import document and reports every issue it can find.
pub fn validate_document(document: &CatalogDocument) -> ValidationReport {
    let mut report = ValidationReport::default();

    let catalog_version = match CatalogVersion::parse(document.catalog_version.trim()) {
        Ok(version) => Some(version),
        Err(message) => {
            report.error("catalogVersion", message);
            None
        }
    };

    if !is_rfc3339_timestamp(document.generated_at.trim()) {
        report.error(
            "generatedAt",
            "must be an RFC 3339 timestamp with an offset",
        );
    }

    if document.source_policy_version.trim() != SOURCE_POLICY_VERSION {
        report.error(
            "sourcePolicyVersion",
            format!(
                "must be '{SOURCE_POLICY_VERSION}'; this importer rejects other policy versions"
            ),
        );
    }

    if let Some(value) = &document.last_verified_at {
        if !is_rfc3339_timestamp(value.trim()) {
            report.error(
                "lastVerifiedAt",
                "must be an RFC 3339 timestamp with an offset",
            );
        }
    }

    let has_records = !document.manufacturers.is_empty()
        || !document.hardware_devices.is_empty()
        || !document.driver_sources.is_empty()
        || !document.drivers.is_empty();

    if has_records && catalog_version == Some(CatalogVersion::ZERO) {
        report.error(
            "catalogVersion",
            "a catalog containing records must use a version greater than 0.0.0",
        );
    }

    let manufacturer_ids = validate_manufacturers(document, &mut report);
    let hardware_identities = validate_hardware_devices(document, &manufacturer_ids, &mut report);
    let source_ids = validate_driver_sources(document, &manufacturer_ids, &mut report);
    validate_drivers(
        document,
        &manufacturer_ids,
        &hardware_identities,
        &source_ids,
        &mut report,
    );

    report
}

fn validate_manufacturers(
    document: &CatalogDocument,
    report: &mut ValidationReport,
) -> HashSet<String> {
    let mut ids = HashSet::new();

    for (index, manufacturer) in document.manufacturers.iter().enumerate() {
        let path = format!("manufacturers[{index}]");
        let id = manufacturer.id.trim();

        if !is_slug(id) {
            report.error(
                format!("{path}.id"),
                "must be a lowercase stable identifier",
            );
        } else if !ids.insert(id.to_string()) {
            report.error(
                format!("{path}.id"),
                format!("duplicate manufacturer id '{id}'"),
            );
        }

        validate_required_text(
            report,
            format!("{path}.name"),
            &manufacturer.name,
            200,
            false,
        );

        for (field, value) in [
            ("websiteUrl", manufacturer.website_url.as_deref()),
            ("supportUrl", manufacturer.support_url.as_deref()),
        ] {
            if let Some(value) = value {
                if let Err(message) = validate_official_url(value) {
                    report.error(format!("{path}.{field}"), message);
                }
            }
        }
    }

    ids
}

fn validate_hardware_devices(
    document: &CatalogDocument,
    manufacturer_ids: &HashSet<String>,
    report: &mut ValidationReport,
) -> HashMap<String, HardwareIdentity> {
    let mut identities = HashMap::new();

    for (index, device) in document.hardware_devices.iter().enumerate() {
        let path = format!("hardwareDevices[{index}]");

        match parse_hardware_identity(&device.hardware_id) {
            Ok(identity) => {
                if identities
                    .insert(identity.hardware_id.clone(), identity.clone())
                    .is_some()
                {
                    report.error(
                        format!("{path}.hardwareId"),
                        format!("duplicate hardware ID '{}'", identity.hardware_id),
                    );
                }
            }
            Err(message) => report.error(format!("{path}.hardwareId"), message),
        }

        if let Some(name) = &device.name {
            validate_required_text(report, format!("{path}.name"), name, 300, false);
        }

        if let Some(manufacturer_id) = &device.manufacturer_id {
            if !manufacturer_ids.contains(manufacturer_id.trim()) {
                report.error(
                    format!("{path}.manufacturerId"),
                    format!("unknown manufacturer '{}'", manufacturer_id.trim()),
                );
            }
        }
    }

    identities
}

fn validate_driver_sources(
    document: &CatalogDocument,
    manufacturer_ids: &HashSet<String>,
    report: &mut ValidationReport,
) -> HashSet<String> {
    let mut ids = HashSet::new();
    let mut keys = HashSet::new();

    for (index, source) in document.driver_sources.iter().enumerate() {
        let path = format!("driverSources[{index}]");
        let id = source.id.trim();

        if !is_slug(id) {
            report.error(
                format!("{path}.id"),
                "must be a lowercase stable identifier",
            );
        } else if !ids.insert(id.to_string()) {
            report.error(format!("{path}.id"), format!("duplicate source id '{id}'"));
        }

        if !source.official {
            report.error(
                format!("{path}.official"),
                "only official sources may be imported",
            );
        }

        validate_required_text(report, format!("{path}.name"), &source.name, 200, false);
        validate_required_text(
            report,
            format!("{path}.attribution"),
            &source.attribution,
            500,
            true,
        );

        if let Err(message) = validate_source_url(source.kind, &source.base_url) {
            report.error(format!("{path}.baseUrl"), message);
        }

        if let Some(manufacturer_id) = &source.manufacturer_id {
            if !manufacturer_ids.contains(manufacturer_id.trim()) {
                report.error(
                    format!("{path}.manufacturerId"),
                    format!("unknown manufacturer '{}'", manufacturer_id.trim()),
                );
            }
        }

        if let Some(value) = &source.last_verified_at {
            if !is_rfc3339_timestamp(value.trim()) {
                report.error(
                    format!("{path}.lastVerifiedAt"),
                    "must be an RFC 3339 timestamp with an offset",
                );
            }
        }

        let key = format!(
            "{}\u{0}{}",
            source.name.trim().to_ascii_lowercase(),
            source.base_url.trim().to_ascii_lowercase()
        );
        if !keys.insert(key) {
            report.error(path.to_string(), "duplicate source name and base URL");
        }

        if source.kind == DriverSourceKind::Other {
            report.warning(
                format!("{path}.kind"),
                "source kind 'other' is allowed but should be reviewed",
            );
        }
    }

    ids
}

fn validate_drivers(
    document: &CatalogDocument,
    manufacturer_ids: &HashSet<String>,
    hardware_identities: &HashMap<String, HardwareIdentity>,
    source_ids: &HashSet<String>,
    report: &mut ValidationReport,
) {
    let mut ids = HashSet::new();
    let mut keys = HashSet::new();

    for (index, driver) in document.drivers.iter().enumerate() {
        let path = format!("drivers[{index}]");
        let id = driver.id.trim();

        if !is_slug(id) {
            report.error(
                format!("{path}.id"),
                "must be a lowercase stable identifier",
            );
        } else if !ids.insert(id.to_string()) {
            report.error(format!("{path}.id"), format!("duplicate driver id '{id}'"));
        }

        if !manufacturer_ids.contains(driver.manufacturer_id.trim()) {
            report.error(
                format!("{path}.manufacturerId"),
                format!("unknown manufacturer '{}'", driver.manufacturer_id.trim()),
            );
        }

        validate_required_text(
            report,
            format!("{path}.provider"),
            &driver.provider,
            200,
            false,
        );
        validate_required_text(report, format!("{path}.name"), &driver.name, 300, false);
        validate_required_text(
            report,
            format!("{path}.version"),
            &driver.version,
            128,
            false,
        );

        if let Some(value) = &driver.release_date {
            if !is_iso_date(value.trim()) {
                report.error(
                    format!("{path}.releaseDate"),
                    "must be a YYYY-MM-DD calendar date",
                );
            }
        } else {
            report.warning(
                format!("{path}.releaseDate"),
                "release date is unknown; do not infer it from the version",
            );
        }

        if let Some(value) = &driver.description {
            validate_required_text(report, format!("{path}.description"), value, 4000, true);
        }

        if let Some(value) = &driver.file_name {
            if !is_safe_file_name(value.trim()) {
                report.error(
                    format!("{path}.fileName"),
                    "must be a plain file name without path separators or reserved characters",
                );
            }
        }

        if let Some(value) = &driver.download_url {
            if let Err(message) = validate_official_url(value) {
                report.error(format!("{path}.downloadUrl"), message);
            }
        }

        if let Some(value) = driver.size_bytes {
            if value < 0 {
                report.error(format!("{path}.sizeBytes"), "must not be negative");
            }
        }

        if let Some(value) = &driver.checksum_sha256 {
            if !is_sha256(value.trim()) {
                report.error(
                    format!("{path}.checksumSha256"),
                    "must be exactly 64 hexadecimal characters",
                );
            }
        }

        if !is_rfc3339_timestamp(driver.last_verified_at.trim()) {
            report.error(
                format!("{path}.lastVerifiedAt"),
                "must be an RFC 3339 timestamp with an offset",
            );
        }

        let key = format!(
            "{}\u{0}{}\u{0}{}",
            driver.manufacturer_id.trim(),
            driver.name.trim().to_ascii_lowercase(),
            driver.version.trim().to_ascii_lowercase()
        );
        if !keys.insert(key) {
            report.error(
                path.to_string(),
                "duplicate driver manufacturer, name, and version",
            );
        }

        validate_driver_hardware(driver, &path, hardware_identities, report);
        validate_driver_windows(driver, &path, report);
        validate_driver_source_refs(driver, &path, source_ids, report);
    }
}

fn validate_driver_hardware(
    driver: &DriverRecord,
    path: &str,
    hardware_identities: &HashMap<String, HardwareIdentity>,
    report: &mut ValidationReport,
) {
    if driver.hardware.is_empty() {
        report.error(
            format!("{path}.hardware"),
            "must list at least one supported hardware ID",
        );
    }

    let mut seen = HashSet::new();

    for (index, link) in driver.hardware.iter().enumerate() {
        let link_path = format!("{path}.hardware[{index}]");

        match parse_hardware_identity(&link.hardware_id) {
            Ok(identity) => {
                if !seen.insert(identity.hardware_id.clone()) {
                    report.error(
                        format!("{link_path}.hardwareId"),
                        format!("duplicate hardware ID '{}'", identity.hardware_id),
                    );
                }

                if !hardware_identities.contains_key(&identity.hardware_id) {
                    report.error(
                        format!("{link_path}.hardwareId"),
                        format!(
                            "unknown hardware ID '{}'; add it to hardwareDevices first",
                            identity.hardware_id
                        ),
                    );
                }

                if link.match_quality == MatchQuality::Subsystem {
                    if let Some(known) = hardware_identities.get(&identity.hardware_id) {
                        if known.subsystem_vendor_id.is_none()
                            || known.subsystem_device_id.is_none()
                        {
                            report.warning(
                                format!("{link_path}.matchQuality"),
                                "subsystem match but the hardware ID has no subsystem fields",
                            );
                        }
                    }
                }
            }
            Err(message) => report.error(format!("{link_path}.hardwareId"), message),
        }

        if let Some(evidence) = &link.evidence {
            validate_required_text(
                report,
                format!("{link_path}.evidence"),
                evidence,
                1000,
                true,
            );
        }
    }
}

fn validate_driver_windows(driver: &DriverRecord, path: &str, report: &mut ValidationReport) {
    if driver.windows.is_empty() {
        report.error(
            format!("{path}.windows"),
            "must list at least one Windows version and architecture",
        );
    }

    let mut seen = HashSet::new();
    for (index, compatibility) in driver.windows.iter().enumerate() {
        if !seen.insert((compatibility.windows_version, compatibility.architecture)) {
            report.error(
                format!("{path}.windows[{index}]"),
                "duplicate Windows version and architecture",
            );
        }
    }
}

fn validate_driver_source_refs(
    driver: &DriverRecord,
    path: &str,
    source_ids: &HashSet<String>,
    report: &mut ValidationReport,
) {
    if driver.sources.is_empty() {
        report.error(
            format!("{path}.sources"),
            "must cite at least one official source",
        );
    }

    let mut seen = HashSet::new();

    for (index, source_ref) in driver.sources.iter().enumerate() {
        let source_path = format!("{path}.sources[{index}]");

        if !source_ids.contains(source_ref.source_id.trim()) {
            report.error(
                format!("{source_path}.sourceId"),
                format!("unknown source '{}'", source_ref.source_id.trim()),
            );
        }

        if let Err(message) = validate_official_url(&source_ref.url) {
            report.error(format!("{source_path}.url"), message);
        }

        if !is_rfc3339_timestamp(source_ref.retrieved_at.trim()) {
            report.error(
                format!("{source_path}.retrievedAt"),
                "must be an RFC 3339 timestamp with an offset",
            );
        }

        validate_required_text(
            report,
            format!("{source_path}.attribution"),
            &source_ref.attribution,
            500,
            true,
        );

        let key = format!(
            "{}\u{0}{}",
            source_ref.source_id.trim(),
            source_ref.url.trim().to_ascii_lowercase()
        );
        if !seen.insert(key) {
            report.error(source_path.to_string(), "duplicate source reference");
        }
    }
}

/// Parses a Windows hardware ID into normalized catalog fields.
///
/// Only concrete PCI, USB, ACPI, and HD Audio identifiers are accepted. Class
/// compatible IDs without a vendor/device identity are rejected because they
/// are too broad to justify a per-driver compatibility claim.
pub fn parse_hardware_identity(value: &str) -> Result<HardwareIdentity, String> {
    let normalized = value.trim().to_ascii_uppercase();

    if normalized.is_empty() {
        return Err("hardware ID is empty".to_string());
    }

    if normalized.len() > 256 {
        return Err("hardware ID is longer than 256 characters".to_string());
    }

    if normalized.matches('\\').count() != 1 {
        return Err(
            "hardware ID must contain one enumerator prefix and no instance suffix".to_string(),
        );
    }

    let parsed = HardwareId::parse(&normalized);

    match parsed.kind {
        HardwareIdKind::Pci(pci) => {
            let vendor_id = pci
                .vendor_id
                .ok_or_else(|| "PCI hardware ID is missing a valid VEN field".to_string())?;
            let device_id = pci
                .device_id
                .ok_or_else(|| "PCI hardware ID is missing a valid DEV field".to_string())?;

            Ok(HardwareIdentity {
                bus_type: "pci",
                hardware_id: normalized,
                vendor_id: Some(vendor_id),
                device_id: Some(device_id),
                subsystem_vendor_id: pci.subsystem_vendor_id,
                subsystem_device_id: pci.subsystem_device_id,
                revision: pci.revision,
                class_code: pci.class_code,
            })
        }
        HardwareIdKind::Usb(usb) => {
            let vendor_id = usb
                .vendor_id
                .ok_or_else(|| "USB hardware ID is missing a valid VID field".to_string())?;
            let device_id = usb
                .product_id
                .ok_or_else(|| "USB hardware ID is missing a valid PID field".to_string())?;

            Ok(HardwareIdentity {
                bus_type: "usb",
                hardware_id: normalized,
                vendor_id: Some(vendor_id),
                device_id: Some(device_id),
                subsystem_vendor_id: None,
                subsystem_device_id: None,
                revision: usb.revision,
                class_code: usb.class_code,
            })
        }
        HardwareIdKind::Acpi(acpi) => {
            let device_id = acpi
                .device_id
                .ok_or_else(|| "ACPI hardware ID is missing a device identifier".to_string())?;

            Ok(HardwareIdentity {
                bus_type: "acpi",
                hardware_id: normalized,
                vendor_id: acpi.vendor_id,
                device_id: Some(device_id),
                subsystem_vendor_id: None,
                subsystem_device_id: None,
                revision: None,
                class_code: None,
            })
        }
        HardwareIdKind::Hdaudio(hdaudio) => {
            let vendor_id = hdaudio
                .vendor_id
                .ok_or_else(|| "HD Audio hardware ID is missing a valid VEN field".to_string())?;
            let device_id = hdaudio
                .device_id
                .ok_or_else(|| "HD Audio hardware ID is missing a valid DEV field".to_string())?;

            Ok(HardwareIdentity {
                bus_type: "hdaudio",
                hardware_id: normalized,
                vendor_id: Some(vendor_id),
                device_id: Some(device_id),
                subsystem_vendor_id: hdaudio.subsystem_vendor_id,
                subsystem_device_id: hdaudio.subsystem_device_id,
                revision: hdaudio.revision,
                class_code: None,
            })
        }
        HardwareIdKind::Unknown => {
            Err("unsupported hardware ID family; expected PCI, USB, ACPI, or HDAUDIO".to_string())
        }
    }
}

/// Trims and canonicalizes a document for storage. Validation must run first.
pub fn normalize_document(document: &CatalogDocument) -> CatalogDocument {
    let mut normalized = document.clone();

    normalized.catalog_version = document.catalog_version.trim().to_string();
    normalized.generated_at = document.generated_at.trim().to_string();
    normalized.source_policy_version = document.source_policy_version.trim().to_string();
    trim_option(&mut normalized.last_verified_at);

    for manufacturer in &mut normalized.manufacturers {
        manufacturer.id = manufacturer.id.trim().to_string();
        manufacturer.name = manufacturer.name.trim().to_string();
        trim_option(&mut manufacturer.website_url);
        trim_option(&mut manufacturer.support_url);
    }

    for device in &mut normalized.hardware_devices {
        device.hardware_id = normalize_hardware_id(&device.hardware_id);
        trim_option(&mut device.name);
        trim_option(&mut device.manufacturer_id);
    }

    for source in &mut normalized.driver_sources {
        source.id = source.id.trim().to_string();
        trim_option(&mut source.manufacturer_id);
        source.name = source.name.trim().to_string();
        source.base_url = source.base_url.trim().to_string();
        source.attribution = source.attribution.trim().to_string();
        trim_option(&mut source.last_verified_at);
    }

    for driver in &mut normalized.drivers {
        driver.id = driver.id.trim().to_string();
        driver.manufacturer_id = driver.manufacturer_id.trim().to_string();
        driver.provider = driver.provider.trim().to_string();
        driver.name = driver.name.trim().to_string();
        driver.version = driver.version.trim().to_string();
        trim_option(&mut driver.release_date);
        trim_option(&mut driver.description);
        trim_option(&mut driver.file_name);
        trim_option(&mut driver.download_url);
        trim_option(&mut driver.checksum_sha256);
        driver.last_verified_at = driver.last_verified_at.trim().to_string();

        for link in &mut driver.hardware {
            link.hardware_id = normalize_hardware_id(&link.hardware_id);
            trim_option(&mut link.evidence);
        }

        for source_ref in &mut driver.sources {
            source_ref.source_id = source_ref.source_id.trim().to_string();
            source_ref.url = source_ref.url.trim().to_string();
            source_ref.retrieved_at = source_ref.retrieved_at.trim().to_string();
            source_ref.attribution = source_ref.attribution.trim().to_string();
        }
    }

    normalized
        .manufacturers
        .sort_by(|left, right| left.id.cmp(&right.id));
    normalized
        .hardware_devices
        .sort_by(|left, right| left.hardware_id.cmp(&right.hardware_id));
    normalized
        .driver_sources
        .sort_by(|left, right| left.id.cmp(&right.id));
    normalized
        .drivers
        .sort_by(|left, right| left.id.cmp(&right.id));

    for driver in &mut normalized.drivers {
        driver
            .hardware
            .sort_by(|left, right| left.hardware_id.cmp(&right.hardware_id));
        driver.windows.sort_by(compare_windows_compat);
        driver.sources.sort_by(|left, right| {
            (left.source_id.as_str(), left.url.as_str())
                .cmp(&(right.source_id.as_str(), right.url.as_str()))
        });
    }

    normalized
}

fn compare_windows_compat(
    left: &WindowsCompatRecord,
    right: &WindowsCompatRecord,
) -> std::cmp::Ordering {
    (left.windows_version, left.architecture).cmp(&(right.windows_version, right.architecture))
}

fn normalize_hardware_id(value: &str) -> String {
    value.trim().to_ascii_uppercase()
}

fn trim_option(value: &mut Option<String>) {
    if let Some(inner) = value {
        *inner = inner.trim().to_string();
    }
}

fn validate_required_text(
    report: &mut ValidationReport,
    path: String,
    value: &str,
    max_length: usize,
    allow_line_breaks: bool,
) {
    let value = value.trim();

    if value.is_empty() {
        report.error(path, "must not be empty");
        return;
    }

    if value.chars().count() > max_length {
        report.error(path, format!("must be at most {max_length} characters"));
        return;
    }

    let invalid_control = value.chars().any(|character| {
        if allow_line_breaks && matches!(character, '\n' | '\r' | '\t') {
            false
        } else {
            character.is_control()
        }
    });

    if invalid_control {
        report.error(path, "must not contain control characters");
    }
}

/// Returns true for a lowercase, stable, separator-delimited identifier.
pub fn is_slug(value: &str) -> bool {
    let value = value.trim();
    if value.is_empty() || value.len() > 128 {
        return false;
    }

    let bytes = value.as_bytes();
    let is_alphanumeric = |byte: u8| byte.is_ascii_lowercase() || byte.is_ascii_digit();
    let is_separator = |byte: u8| matches!(byte, b'.' | b'_' | b'-');

    if !is_alphanumeric(bytes[0]) || !is_alphanumeric(bytes[bytes.len() - 1]) {
        return false;
    }

    let mut previous_separator = false;
    for byte in bytes {
        if is_separator(*byte) {
            if previous_separator {
                return false;
            }
            previous_separator = true;
        } else if is_alphanumeric(*byte) {
            previous_separator = false;
        } else {
            return false;
        }
    }

    true
}

/// Returns true for a plain file name that cannot escape a download directory.
pub fn is_safe_file_name(value: &str) -> bool {
    if value.is_empty() || value.len() > 260 || value == "." || value == ".." {
        return false;
    }

    if value.ends_with(' ') || value.ends_with('.') {
        return false;
    }

    !value.chars().any(|character| {
        character.is_control()
            || matches!(
                character,
                '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
            )
    })
}

pub fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::model::{
        Architecture, DriverHardwareRecord, DriverRecord, DriverSourceKind, DriverSourceRecord,
        DriverSourceRefRecord, HardwareDeviceRecord, ManufacturerRecord, PackageType,
        WindowsVersion,
    };

    fn manufacturer() -> ManufacturerRecord {
        ManufacturerRecord {
            id: "test-manufacturer".to_string(),
            name: "Test Manufacturer".to_string(),
            website_url: Some("https://www.intel.com".to_string()),
            support_url: None,
        }
    }

    fn hardware() -> HardwareDeviceRecord {
        HardwareDeviceRecord {
            hardware_id: r"pci\ven_8086&dev_1234&subsys_56781234&rev_01".to_string(),
            name: Some("Test Device".to_string()),
            manufacturer_id: Some("test-manufacturer".to_string()),
        }
    }

    fn source() -> DriverSourceRecord {
        DriverSourceRecord {
            id: "test-source".to_string(),
            manufacturer_id: Some("test-manufacturer".to_string()),
            name: "Test Source".to_string(),
            kind: DriverSourceKind::Manufacturer,
            base_url: "https://www.intel.com/content/www/us/en/download-center/home.html"
                .to_string(),
            official: true,
            attribution: "Test Manufacturer official download center".to_string(),
            last_verified_at: Some("2026-11-05T12:00:00Z".to_string()),
        }
    }

    fn driver() -> DriverRecord {
        DriverRecord {
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
        }
    }

    fn document() -> CatalogDocument {
        CatalogDocument {
            catalog_version: "2026.11.6".to_string(),
            generated_at: "2026-11-06T00:00:00Z".to_string(),
            source_policy_version: SOURCE_POLICY_VERSION.to_string(),
            last_verified_at: Some("2026-11-05T12:00:00Z".to_string()),
            manufacturers: vec![manufacturer()],
            hardware_devices: vec![hardware()],
            driver_sources: vec![source()],
            drivers: vec![driver()],
        }
    }

    #[test]
    fn accepts_a_complete_document() {
        let report = validate_document(&document());
        assert!(report.is_valid(), "{report:?}");
        assert!(report.warnings.is_empty(), "{report:?}");
    }

    #[test]
    fn normalizes_and_sorts_hardware_ids() {
        let normalized = normalize_document(&document());
        assert_eq!(
            normalized.hardware_devices[0].hardware_id,
            r"PCI\VEN_8086&DEV_1234&SUBSYS_56781234&REV_01"
        );
        assert_eq!(
            normalized.drivers[0].hardware[0].hardware_id,
            r"PCI\VEN_8086&DEV_1234&SUBSYS_56781234&REV_01"
        );
    }

    #[test]
    fn rejects_duplicate_hardware_ids_case_insensitively() {
        let mut document = document();
        let mut duplicate = hardware();
        duplicate.hardware_id = duplicate.hardware_id.to_ascii_lowercase();
        document.hardware_devices.push(duplicate);

        let report = validate_document(&document);
        assert!(report
            .errors
            .iter()
            .any(|issue| issue.message.contains("duplicate hardware ID")));
    }

    #[test]
    fn rejects_malformed_and_unknown_hardware_ids() {
        for hardware_id in [
            r"PCI\VEN_ZZZZ&DEV_1234",
            r"PCI\VEN_8086&DEV_1234\3&11583659&0&10",
            r"SWD\MMDEVAPI\AudioEndpoint",
            r"USB\CLASS_03&SUBCLASS_01&PROT_02",
            "",
        ] {
            assert!(
                parse_hardware_identity(hardware_id).is_err(),
                "{hardware_id}"
            );
        }
    }

    #[test]
    fn rejects_unknown_references_and_missing_relationships() {
        let mut document = document();
        document.drivers[0].manufacturer_id = "missing".to_string();
        document.drivers[0].hardware[0].hardware_id = r"PCI\VEN_FFFF&DEV_FFFF".to_string();
        document.drivers[0].sources[0].source_id = "missing".to_string();
        document.drivers[0].windows.clear();
        document.drivers[0].sources.clear();

        let report = validate_document(&document);
        let messages = report
            .errors
            .iter()
            .map(|issue| issue.message.as_str())
            .collect::<Vec<_>>();

        assert!(messages
            .iter()
            .any(|message| message.contains("unknown manufacturer")));
        assert!(messages
            .iter()
            .any(|message| message.contains("unknown hardware ID")));
        assert!(messages
            .iter()
            .any(|message| message.contains("at least one Windows")));
        assert!(messages
            .iter()
            .any(|message| message.contains("at least one official source")));
    }

    #[test]
    fn rejects_invalid_urls_and_non_official_sources() {
        let mut document = document();
        document.driver_sources[0].base_url = "https://example.com/drivers".to_string();
        document.drivers[0].download_url = Some("http://www.intel.com/driver.exe".to_string());
        document.drivers[0].sources[0].url = "https://example.com/driver".to_string();

        let report = validate_document(&document);
        assert!(report.errors.iter().any(|issue| issue
            .message
            .contains("not in the official source registry")));
        assert!(report
            .errors
            .iter()
            .any(|issue| issue.message.contains("must use HTTPS")));
    }

    #[test]
    fn rejects_missing_metadata_and_invalid_policy_version() {
        let mut document = document();
        document.source_policy_version = "0".to_string();
        document.drivers[0].name = "   ".to_string();
        document.drivers[0].version = "".to_string();
        document.drivers[0].last_verified_at = "not-a-timestamp".to_string();
        document.drivers[0].checksum_sha256 = Some("abc".to_string());

        let report = validate_document(&document);
        let messages = report
            .errors
            .iter()
            .map(|issue| issue.message.as_str())
            .collect::<Vec<_>>();

        assert!(report
            .errors
            .iter()
            .any(|issue| issue.path == "sourcePolicyVersion"));
        assert!(messages
            .iter()
            .any(|message| message.contains("must not be empty")));
        assert!(messages.iter().any(|message| message.contains("RFC 3339")));
        assert!(messages
            .iter()
            .any(|message| message.contains("64 hexadecimal")));
    }

    #[test]
    fn rejects_records_with_the_empty_catalog_version() {
        let mut document = document();
        document.catalog_version = "0.0.0".to_string();

        let report = validate_document(&document);
        assert!(report
            .errors
            .iter()
            .any(|issue| issue.message.contains("greater than 0.0.0")));
    }

    #[test]
    fn rejects_unsafe_file_names() {
        for value in [
            "..\\driver.exe",
            "folder/driver.exe",
            "C:driver.exe",
            "driver.exe.",
        ] {
            assert!(!is_safe_file_name(value), "{value}");
        }
        assert!(is_safe_file_name("driver.exe"));
    }

    #[test]
    fn validates_slugs() {
        assert!(is_slug("intel-wifi-ax200"));
        assert!(is_slug("windows.update-2026_11"));
        assert!(!is_slug("Intel-WiFi"));
        assert!(!is_slug("-intel"));
        assert!(!is_slug("intel--wifi"));
        assert!(!is_slug("intel wifi"));
    }
}
