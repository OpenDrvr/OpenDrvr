//! Portable hardware snapshot model.
//!
//! These types describe what a provider found without depending on Win32 or
//! Tauri. The Windows provider fills them from CfgMgr32; future providers can
//! fill the same shape from another source.

use serde::Serialize;

use super::{
    category::DeviceCategory,
    id::{BusType, DeviceInstanceId, HardwareId},
    identify::DeviceIdentification,
    status::{DeviceCondition, DeviceStatus},
};

/// One Plug and Play device instance.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceRecord {
    /// Authoritative device instance ID.
    pub instance_id: DeviceInstanceId,
    /// Hardware IDs in the order Windows reported them.
    pub hardware_ids: Vec<HardwareId>,
    /// Compatible IDs in the order Windows reported them.
    pub compatible_ids: Vec<HardwareId>,
    pub bus_type: BusType,
    pub category: DeviceCategory,
    /// Structured identity derived from the raw identifiers.
    pub identification: DeviceIdentification,
    /// Windows device class, for example `Net` or `Display`.
    pub class_name: Option<String>,
    /// Windows device class GUID, formatted as a canonical UUID string.
    pub class_guid: Option<String>,
    pub manufacturer: Option<String>,
    /// User-visible friendly name. May be absent or generic.
    pub friendly_name: Option<String>,
    /// Driver-provided device description. May differ from the friendly name.
    pub device_description: Option<String>,
    pub enumerator: Option<String>,
    pub parent_instance_id: Option<DeviceInstanceId>,
    pub location: DeviceLocation,
    pub driver: Option<DriverInfo>,
    pub status: DeviceStatus,
    pub is_present: bool,
    /// Coarse classification combining presence, status, and driver metadata.
    pub condition: DeviceCondition,
}

impl DeviceRecord {
    /// The best available human-readable name, without treating it as identity.
    pub fn display_name(&self) -> Option<&str> {
        self.friendly_name
            .as_deref()
            .or(self.device_description.as_deref())
    }

    /// The first hardware ID, which is usually the most specific match key.
    pub fn primary_hardware_id(&self) -> Option<&HardwareId> {
        self.hardware_ids.first()
    }

    pub fn has_driver(&self) -> bool {
        self.driver.is_some()
    }

    pub fn has_problem(&self) -> bool {
        self.status.problem.is_some()
    }
}

/// Bus location facts reported by the parent bus driver.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceLocation {
    pub location_info: Option<String>,
    pub location_paths: Vec<String>,
    pub bus_number: Option<u32>,
    pub address: Option<u32>,
}

/// Driver package facts available from device properties.
///
/// This is descriptive metadata, not proof that an update exists.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriverInfo {
    pub provider: Option<String>,
    pub version: Option<String>,
    /// UTC timestamp formatted from `DEVPKEY_Device_DriverDate`.
    pub date: Option<String>,
    pub description: Option<String>,
    /// INF file name reported by `DEVPKEY_Device_DriverInfPath`.
    pub inf_name: Option<String>,
    pub service: Option<String>,
}

/// A complete scan result from one provider.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HardwareSnapshot {
    pub devices: Vec<DeviceRecord>,
    pub diagnostics: ScanDiagnostics,
}

impl HardwareSnapshot {
    pub fn device(&self, instance_id: &DeviceInstanceId) -> Option<&DeviceRecord> {
        self.devices
            .iter()
            .find(|device| &device.instance_id == instance_id)
    }
}

/// Non-fatal problems encountered while building a snapshot.
///
/// Missing optional properties are normal and are not recorded. Real property
/// read failures and devices that could not be located are recorded here while
/// the rest of the scan continues.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanDiagnostics {
    /// Provider name, for example `CfgMgr32`.
    pub provider: String,
    /// Whether the provider enumerated only present devices.
    pub present_only: bool,
    pub device_count: usize,
    pub property_failures: Vec<PropertyFailure>,
    pub device_failures: Vec<DeviceFailure>,
    pub duration_ms: u64,
}

/// A property that existed or failed unexpectedly for one device.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PropertyFailure {
    pub instance_id: DeviceInstanceId,
    pub property: String,
    pub error: String,
}

/// A device that could not be located or whose status could not be read.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceFailure {
    pub instance_id: String,
    pub operation: String,
    pub error: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hardware::status::{DeviceState, DeviceStatus};

    fn record(friendly_name: Option<&str>, description: Option<&str>) -> DeviceRecord {
        DeviceRecord {
            instance_id: DeviceInstanceId::new(r"PCI\VEN_8086&DEV_1234\0").expect("instance id"),
            hardware_ids: vec![HardwareId::parse(r"PCI\VEN_8086&DEV_1234")],
            compatible_ids: Vec::new(),
            bus_type: BusType::Pci,
            category: DeviceCategory::Pci,
            identification: DeviceIdentification::default(),
            class_name: Some("System".to_string()),
            class_guid: None,
            manufacturer: None,
            friendly_name: friendly_name.map(ToOwned::to_owned),
            device_description: description.map(ToOwned::to_owned),
            enumerator: Some("PCI".to_string()),
            parent_instance_id: None,
            location: DeviceLocation::default(),
            driver: None,
            status: DeviceStatus::from_flags(0, 0),
            is_present: true,
            condition: DeviceCondition::MissingDriver,
        }
    }

    #[test]
    fn prefers_friendly_name_then_description() {
        let named = record(Some("Friendly"), Some("Description"));
        let described = record(None, Some("Description"));

        assert_eq!(named.display_name(), Some("Friendly"));
        assert_eq!(described.display_name(), Some("Description"));
    }

    #[test]
    fn exposes_primary_hardware_id_and_status_helpers() {
        let device = record(Some("Friendly"), None);

        assert_eq!(
            device.primary_hardware_id().map(HardwareId::as_str),
            Some(r"PCI\VEN_8086&DEV_1234")
        );
        assert!(!device.has_driver());
        assert!(!device.has_problem());
        assert_eq!(device.status.state, DeviceState::Stopped);
    }

    #[test]
    fn finds_device_by_instance_id() {
        let device = record(Some("Friendly"), None);
        let snapshot = HardwareSnapshot {
            diagnostics: ScanDiagnostics {
                device_count: 1,
                ..ScanDiagnostics::default()
            },
            devices: vec![device.clone()],
        };

        assert_eq!(
            snapshot
                .device(&device.instance_id)
                .map(DeviceRecord::display_name),
            Some(Some("Friendly"))
        );
        assert!(snapshot
            .device(&DeviceInstanceId::new("USB\\VID_0000").expect("instance id"))
            .is_none());
    }
}
