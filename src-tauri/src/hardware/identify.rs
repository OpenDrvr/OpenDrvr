//! Structured hardware identification.
//!
//! This module turns parsed Plug and Play identifiers into a small, auditable
//! identity result. It never guesses a manufacturer from a friendly name:
//! names are only used to detect a conflict with an identifier-derived vendor.
//!
//! The distinction between the values is deliberate:
//!
//! - **reported manufacturer** is the string Windows stores in
//!   `DEVPKEY_Device_Manufacturer`
//! - **hardware vendor** is the silicon/device vendor from `VEN_` or `VID_`
//! - **subsystem vendor** is the board/system vendor from `SUBSYS_`, when the
//!   identifier family provides one
//! - **driver provider** remains separate in [`super::model::DriverInfo`]
//!
//! Adding a new identifier family or vendor table does not require changing the
//! Windows scanner; the scanner only fills the portable model.

use serde::Serialize;

use super::{
    id::{DeviceInstanceId, HardwareId, HardwareIdKind},
    vendors::{self, VendorEntry},
};

/// How much of a device's identity is confirmed by identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum IdentificationConfidence {
    /// A known device vendor was resolved from the hardware/compatible IDs and
    /// no reported manufacturer contradicts it.
    Confirmed,
    /// Some identity information is known, but it is incomplete or conflicts
    /// with Windows-reported metadata.
    Partial,
    /// No known vendor identifier matched. Reported values may still be shown.
    Unresolved,
}

/// Identifier family that supplied a vendor match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum IdentifierFamily {
    Pci,
    Usb,
    Acpi,
    Hdaudio,
    Other,
}

/// Role of a vendor ID within a hardware identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum VendorRole {
    /// The device or silicon vendor (`VEN_` or `VID_`).
    Device,
    /// The board/system vendor from a subsystem field.
    Subsystem,
}

/// A vendor resolved from a known identifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentifiedVendor {
    pub name: String,
    /// Canonical identifier value, for example `8086`, `046D`, or `INT`.
    pub vendor_id: String,
    pub family: IdentifierFamily,
    pub role: VendorRole,
}

impl IdentifiedVendor {
    fn matches_reported_name(&self, reported: &str) -> bool {
        let entry = match self.family {
            IdentifierFamily::Pci => vendors::pci(&self.vendor_id),
            IdentifierFamily::Usb => vendors::usb(&self.vendor_id),
            IdentifierFamily::Acpi => vendors::acpi(&self.vendor_id),
            IdentifierFamily::Hdaudio => vendors::hdaudio(&self.vendor_id),
            IdentifierFamily::Other => None,
        };

        entry.is_some_and(|entry| entry.matches_name(reported))
    }
}

/// Portable identification result attached to every device record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceIdentification {
    pub confidence: IdentificationConfidence,
    /// Preferred identified manufacturer: subsystem vendor when available,
    /// otherwise the hardware vendor. Both remain available separately below.
    pub manufacturer: Option<IdentifiedVendor>,
    /// Silicon/device vendor from `VEN_` or `VID_`.
    pub hardware_vendor: Option<IdentifiedVendor>,
    /// Board/system vendor from a subsystem field, when present.
    pub subsystem_vendor: Option<IdentifiedVendor>,
    /// Manufacturer string reported by Windows, preserved verbatim.
    pub reported_manufacturer: Option<String>,
    /// Device name reported by Windows. Descriptive, not an identified model.
    pub reported_name: Option<String>,
    /// Identifier families present on the device, in discovery order.
    pub identifier_families: Vec<IdentifierFamily>,
    /// Explanations for partial, unresolved, or conflicting identification.
    pub notes: Vec<String>,
}

impl Default for DeviceIdentification {
    fn default() -> Self {
        Self {
            confidence: IdentificationConfidence::Unresolved,
            manufacturer: None,
            hardware_vendor: None,
            subsystem_vendor: None,
            reported_manufacturer: None,
            reported_name: None,
            identifier_families: Vec::new(),
            notes: Vec::new(),
        }
    }
}

/// Inputs needed to identify one device without depending on the Windows API.
pub struct IdentificationInput<'a> {
    pub instance_id: &'a DeviceInstanceId,
    pub hardware_ids: &'a [HardwareId],
    pub compatible_ids: &'a [HardwareId],
    pub reported_manufacturer: Option<&'a str>,
    pub reported_name: Option<&'a str>,
}

impl DeviceIdentification {
    /// Resolves identity from the raw identifiers and Windows-reported values.
    pub fn resolve(input: IdentificationInput<'_>) -> Self {
        let mut identifier_families = Vec::new();
        let mut hardware_vendor = None;
        let mut subsystem_vendor = None;

        for identifier in input.hardware_ids.iter().chain(input.compatible_ids.iter()) {
            push_family(&mut identifier_families, identifier);
            collect_vendors(identifier, &mut hardware_vendor, &mut subsystem_vendor);
        }

        // Some root, ACPI, and software devices expose no hardware ID list. The
        // instance ID often still carries the vendor fields, so use it last.
        let mut instance_fallback_used = false;
        if hardware_vendor.is_none() || subsystem_vendor.is_none() {
            if let Some(identifier) = instance_identifier(input.instance_id) {
                push_family(&mut identifier_families, &identifier);
                let had_vendor = hardware_vendor.is_some() || subsystem_vendor.is_some();
                collect_vendors(&identifier, &mut hardware_vendor, &mut subsystem_vendor);
                instance_fallback_used =
                    !had_vendor && (hardware_vendor.is_some() || subsystem_vendor.is_some());
            }
        }

        let manufacturer = subsystem_vendor.clone().or_else(|| hardware_vendor.clone());
        let reported_manufacturer = non_empty(input.reported_manufacturer);
        let reported_name = non_empty(input.reported_name);

        let reported_matches_hardware = reported_manufacturer.as_deref().is_some_and(|reported| {
            hardware_vendor
                .as_ref()
                .is_some_and(|vendor| vendor.matches_reported_name(reported))
        });
        let reported_matches_subsystem = reported_manufacturer.as_deref().is_some_and(|reported| {
            subsystem_vendor
                .as_ref()
                .is_some_and(|vendor| vendor.matches_reported_name(reported))
        });
        let has_conflict = (hardware_vendor.is_some() || subsystem_vendor.is_some())
            && reported_manufacturer.is_some()
            && !reported_matches_hardware
            && !reported_matches_subsystem;

        let mut notes = Vec::new();
        if input.hardware_ids.is_empty() && input.compatible_ids.is_empty() {
            notes.push("No hardware or compatible IDs were reported.".to_string());
        }
        if instance_fallback_used {
            notes.push(
                "Vendor information came from the device instance ID because the hardware ID \
                 list did not provide it."
                    .to_string(),
            );
        }

        if hardware_vendor.is_none() && subsystem_vendor.is_none() {
            match reported_manufacturer.as_deref() {
                Some(reported) => notes.push(format!(
                    "No known vendor identifier matched; Windows reports manufacturer \"{reported}\"."
                )),
                None => notes.push("No known vendor identifier matched.".to_string()),
            }
        } else if hardware_vendor.is_none() {
            notes.push(
                "Only the subsystem vendor is known; the device vendor could not be resolved."
                    .to_string(),
            );
        }

        if has_conflict {
            let identified = [hardware_vendor.as_ref(), subsystem_vendor.as_ref()]
                .into_iter()
                .flatten()
                .map(|vendor| format!("{} ({})", vendor.name, vendor.vendor_id))
                .collect::<Vec<_>>()
                .join(", ");
            notes.push(format!(
                "Windows reports manufacturer \"{}\", which does not match the identified \
                 vendor(s): {identified}.",
                reported_manufacturer.as_deref().unwrap_or_default()
            ));
        }

        let confidence = if hardware_vendor.is_some() && !has_conflict {
            IdentificationConfidence::Confirmed
        } else if hardware_vendor.is_some() || subsystem_vendor.is_some() {
            IdentificationConfidence::Partial
        } else {
            IdentificationConfidence::Unresolved
        };

        Self {
            confidence,
            manufacturer,
            hardware_vendor,
            subsystem_vendor,
            reported_manufacturer,
            reported_name,
            identifier_families,
            notes,
        }
    }
}

fn collect_vendors(
    identifier: &HardwareId,
    hardware_vendor: &mut Option<IdentifiedVendor>,
    subsystem_vendor: &mut Option<IdentifiedVendor>,
) {
    for candidate in vendor_candidates(identifier) {
        match candidate.role {
            VendorRole::Device => {
                if hardware_vendor.is_none() {
                    *hardware_vendor = Some(candidate);
                }
            }
            VendorRole::Subsystem => {
                if subsystem_vendor.is_none() {
                    *subsystem_vendor = Some(candidate);
                }
            }
        }
    }
}

fn vendor_candidates(identifier: &HardwareId) -> Vec<IdentifiedVendor> {
    match &identifier.kind {
        HardwareIdKind::Pci(pci) => {
            let mut candidates = Vec::new();
            if let Some(entry) = pci.vendor_id.as_deref().and_then(vendors::pci) {
                candidates.push(identified(entry, IdentifierFamily::Pci, VendorRole::Device));
            }
            if let Some(entry) = pci.subsystem_vendor_id.as_deref().and_then(vendors::pci) {
                candidates.push(identified(
                    entry,
                    IdentifierFamily::Pci,
                    VendorRole::Subsystem,
                ));
            }
            candidates
        }
        HardwareIdKind::Usb(usb) => usb
            .vendor_id
            .as_deref()
            .and_then(vendors::usb)
            .map(|entry| vec![identified(entry, IdentifierFamily::Usb, VendorRole::Device)])
            .unwrap_or_default(),
        HardwareIdKind::Acpi(acpi) => acpi
            .vendor_id
            .as_deref()
            .and_then(vendors::acpi)
            .map(|entry| {
                vec![identified(
                    entry,
                    IdentifierFamily::Acpi,
                    VendorRole::Device,
                )]
            })
            .unwrap_or_default(),
        HardwareIdKind::Hdaudio(hdaudio) => {
            let mut candidates = Vec::new();
            if let Some(entry) = hdaudio.vendor_id.as_deref().and_then(vendors::hdaudio) {
                candidates.push(identified(
                    entry,
                    IdentifierFamily::Hdaudio,
                    VendorRole::Device,
                ));
            }
            if let Some(entry) = hdaudio
                .subsystem_vendor_id
                .as_deref()
                .and_then(vendors::pci)
            {
                candidates.push(identified(
                    entry,
                    IdentifierFamily::Hdaudio,
                    VendorRole::Subsystem,
                ));
            }
            candidates
        }
        HardwareIdKind::Unknown => Vec::new(),
    }
}

fn identified(
    entry: &'static VendorEntry,
    family: IdentifierFamily,
    role: VendorRole,
) -> IdentifiedVendor {
    IdentifiedVendor {
        name: entry.name.to_string(),
        vendor_id: entry.id.to_string(),
        family,
        role,
    }
}

fn push_family(families: &mut Vec<IdentifierFamily>, identifier: &HardwareId) {
    let family = match identifier.kind {
        HardwareIdKind::Pci(_) => IdentifierFamily::Pci,
        HardwareIdKind::Usb(_) => IdentifierFamily::Usb,
        HardwareIdKind::Acpi(_) => IdentifierFamily::Acpi,
        HardwareIdKind::Hdaudio(_) => IdentifierFamily::Hdaudio,
        HardwareIdKind::Unknown => IdentifierFamily::Other,
    };

    if !families.contains(&family) {
        families.push(family);
    }
}

/// Rebuilds the device portion of an instance ID without its unique suffix.
///
/// `PCI\VEN_8086&DEV_1234\3&...` becomes `PCI\VEN_8086&DEV_1234`.
fn instance_identifier(instance_id: &DeviceInstanceId) -> Option<HardwareId> {
    let mut parts = instance_id.as_str().split('\\');
    let enumerator = parts.next()?;
    let device = parts.next()?;

    if enumerator.is_empty() || device.is_empty() {
        return None;
    }

    Some(HardwareId::parse(format!("{enumerator}\\{device}")))
}

fn non_empty(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identify(
        instance_id: &str,
        hardware: &[&str],
        compatible: &[&str],
        reported_manufacturer: Option<&str>,
        reported_name: Option<&str>,
    ) -> DeviceIdentification {
        let instance_id = DeviceInstanceId::new(instance_id).expect("valid instance ID");
        let hardware_ids = hardware
            .iter()
            .map(|raw| HardwareId::parse(*raw))
            .collect::<Vec<_>>();
        let compatible_ids = compatible
            .iter()
            .map(|raw| HardwareId::parse(*raw))
            .collect::<Vec<_>>();

        DeviceIdentification::resolve(IdentificationInput {
            instance_id: &instance_id,
            hardware_ids: &hardware_ids,
            compatible_ids: &compatible_ids,
            reported_manufacturer,
            reported_name,
        })
    }

    #[test]
    fn identifies_pci_device_and_subsystem_vendors() {
        let identification = identify(
            r"PCI\VEN_10DE&DEV_2684&SUBSYS_26843842&REV_A1\4&2D8E4F0&0&0010",
            &[r"PCI\VEN_10DE&DEV_2684&SUBSYS_26843842&REV_A1"],
            &[],
            Some("EVGA"),
            Some("NVIDIA GeForce GTX 1080"),
        );

        assert_eq!(
            identification.confidence,
            IdentificationConfidence::Confirmed
        );
        assert_eq!(
            identification
                .hardware_vendor
                .as_ref()
                .map(|vendor| vendor.name.as_str()),
            Some("NVIDIA")
        );
        assert_eq!(
            identification
                .subsystem_vendor
                .as_ref()
                .map(|vendor| vendor.name.as_str()),
            Some("EVGA")
        );
        assert_eq!(
            identification
                .manufacturer
                .as_ref()
                .map(|vendor| vendor.name.as_str()),
            Some("EVGA")
        );
        assert_eq!(
            identification
                .manufacturer
                .as_ref()
                .map(|vendor| vendor.role),
            Some(VendorRole::Subsystem)
        );
        assert_eq!(
            identification.reported_name.as_deref(),
            Some("NVIDIA GeForce GTX 1080")
        );
        assert_eq!(
            identification.identifier_families,
            vec![IdentifierFamily::Pci]
        );
        assert!(identification.notes.is_empty());
    }

    #[test]
    fn identifies_usb_vendor() {
        let identification = identify(
            r"USB\VID_046D&PID_C52B&REV_2400\5&2A1B2C3D&0&1",
            &[r"USB\VID_046D&PID_C52B&REV_2400"],
            &[],
            Some("Logitech"),
            Some("USB Receiver"),
        );

        assert_eq!(
            identification.confidence,
            IdentificationConfidence::Confirmed
        );
        assert_eq!(
            identification
                .hardware_vendor
                .as_ref()
                .map(|vendor| vendor.name.as_str()),
            Some("Logitech")
        );
        assert_eq!(
            identification
                .manufacturer
                .as_ref()
                .map(|vendor| vendor.role),
            Some(VendorRole::Device)
        );
        assert_eq!(
            identification.identifier_families,
            vec![IdentifierFamily::Usb]
        );
    }

    #[test]
    fn identifies_acpi_vendor_prefixes() {
        let intel = identify(r"ACPI\INT3450\1", &[r"ACPI\INT3450"], &[], None, None);
        assert_eq!(intel.confidence, IdentificationConfidence::Confirmed);
        assert_eq!(
            intel
                .hardware_vendor
                .as_ref()
                .map(|vendor| vendor.name.as_str()),
            Some("Intel")
        );
        assert_eq!(intel.identifier_families, vec![IdentifierFamily::Acpi]);

        let pnp = identify(r"ACPI\PNP0A08\0", &[r"ACPI\PNP0A08"], &[], None, None);
        assert_eq!(
            pnp.hardware_vendor
                .as_ref()
                .map(|vendor| vendor.vendor_id.as_str()),
            Some("PNP")
        );
    }

    #[test]
    fn identifies_hdaudio_codec_and_subsystem_vendors() {
        let identification = identify(
            r"HDAUDIO\FUNC_01&VEN_10EC&DEV_0256&SUBSYS_17AA2245&REV_1000\4&2D8E4F0&0&0001",
            &[r"HDAUDIO\FUNC_01&VEN_10EC&DEV_0256&SUBSYS_17AA2245&REV_1000"],
            &[],
            Some("Realtek"),
            Some("Realtek High Definition Audio"),
        );

        assert_eq!(
            identification.confidence,
            IdentificationConfidence::Confirmed
        );
        assert_eq!(
            identification
                .hardware_vendor
                .as_ref()
                .map(|vendor| vendor.name.as_str()),
            Some("Realtek")
        );
        assert_eq!(
            identification
                .subsystem_vendor
                .as_ref()
                .map(|vendor| vendor.name.as_str()),
            Some("Lenovo")
        );
        assert_eq!(
            identification
                .manufacturer
                .as_ref()
                .map(|vendor| vendor.name.as_str()),
            Some("Lenovo")
        );
        assert_eq!(
            identification.identifier_families,
            vec![IdentifierFamily::Hdaudio]
        );
    }

    #[test]
    fn leaves_unknown_vendor_ids_unresolved() {
        let identification = identify(
            r"PCI\VEN_FFFF&DEV_0001\0",
            &[r"PCI\VEN_FFFF&DEV_0001"],
            &[],
            None,
            None,
        );

        assert_eq!(
            identification.confidence,
            IdentificationConfidence::Unresolved
        );
        assert!(identification.manufacturer.is_none());
        assert!(identification.hardware_vendor.is_none());
        assert!(identification
            .notes
            .iter()
            .any(|note| note.contains("No known vendor identifier")));
    }

    #[test]
    fn preserves_reported_manufacturer_without_confirming_it() {
        let identification = identify(
            r"PCI\VEN_FFFF&DEV_0001\0",
            &[r"PCI\VEN_FFFF&DEV_0001"],
            &[],
            Some("Acme Devices"),
            None,
        );

        assert_eq!(
            identification.confidence,
            IdentificationConfidence::Unresolved
        );
        assert_eq!(
            identification.reported_manufacturer.as_deref(),
            Some("Acme Devices")
        );
        assert!(identification.manufacturer.is_none());
        assert!(identification
            .notes
            .iter()
            .any(|note| note.contains("Windows reports manufacturer")));
    }

    #[test]
    fn marks_conflicting_manufacturer_information_as_partial() {
        let identification = identify(
            r"PCI\VEN_8086&DEV_1234\0",
            &[r"PCI\VEN_8086&DEV_1234"],
            &[],
            Some("NVIDIA"),
            None,
        );

        assert_eq!(identification.confidence, IdentificationConfidence::Partial);
        assert_eq!(
            identification
                .hardware_vendor
                .as_ref()
                .map(|vendor| vendor.name.as_str()),
            Some("Intel")
        );
        assert!(identification
            .notes
            .iter()
            .any(|note| note.contains("does not match")));
    }

    #[test]
    fn accepts_common_manufacturer_aliases() {
        let identification = identify(
            r"PCI\VEN_1002&DEV_1234\0",
            &[r"PCI\VEN_1002&DEV_1234"],
            &[],
            Some("Advanced Micro Devices, Inc."),
            None,
        );

        assert_eq!(
            identification.confidence,
            IdentificationConfidence::Confirmed
        );
        assert!(identification.notes.is_empty());
    }

    #[test]
    fn handles_incomplete_identifiers_without_guessing() {
        let missing_vendor = identify(r"PCI\DEV_1234\0", &[r"PCI\DEV_1234"], &[], None, None);
        assert_eq!(
            missing_vendor.confidence,
            IdentificationConfidence::Unresolved
        );

        let class_only = identify(
            r"USB\CLASS_03&SUBCLASS_01&PROT_02\0",
            &[r"USB\CLASS_03&SUBCLASS_01&PROT_02"],
            &[],
            None,
            None,
        );
        assert_eq!(class_only.confidence, IdentificationConfidence::Unresolved);
        assert_eq!(class_only.identifier_families, vec![IdentifierFamily::Usb]);
    }

    #[test]
    fn marks_subsystem_only_identification_as_partial() {
        let identification = identify(
            r"PCI\SUBSYS_26843842\0",
            &[r"PCI\SUBSYS_26843842"],
            &[],
            Some("EVGA"),
            None,
        );

        assert_eq!(identification.confidence, IdentificationConfidence::Partial);
        assert_eq!(
            identification
                .manufacturer
                .as_ref()
                .map(|vendor| vendor.name.as_str()),
            Some("EVGA")
        );
        assert!(identification
            .notes
            .iter()
            .any(|note| note.contains("Only the subsystem vendor")));
    }

    #[test]
    fn falls_back_to_the_instance_id_when_hardware_ids_are_missing() {
        let identification = identify(
            r"PCI\VEN_8086&DEV_1234\3&11583659&0&10",
            &[],
            &[],
            None,
            None,
        );

        assert_eq!(
            identification.confidence,
            IdentificationConfidence::Confirmed
        );
        assert_eq!(
            identification
                .hardware_vendor
                .as_ref()
                .map(|vendor| vendor.name.as_str()),
            Some("Intel")
        );
        assert!(identification
            .notes
            .iter()
            .any(|note| note.contains("instance ID")));
    }

    #[test]
    fn prefers_hardware_ids_over_compatible_ids() {
        let identification = identify(
            r"PCI\VEN_8086&DEV_1234\0",
            &[r"PCI\VEN_8086&DEV_1234"],
            &[r"PCI\VEN_10DE&DEV_2684"],
            None,
            None,
        );

        assert_eq!(
            identification
                .hardware_vendor
                .as_ref()
                .map(|vendor| vendor.name.as_str()),
            Some("Intel")
        );
    }

    #[test]
    fn reports_identifier_families_in_discovery_order() {
        let identification = identify(
            r"PCI\VEN_8086&DEV_1234\0",
            &[r"PCI\VEN_8086&DEV_1234"],
            &[r"ACPI\PNP0A08", r"SWD\MMDEVAPI\AudioEndpoint"],
            None,
            None,
        );

        assert_eq!(
            identification.identifier_families,
            vec![
                IdentifierFamily::Pci,
                IdentifierFamily::Acpi,
                IdentifierFamily::Other
            ]
        );
    }
}
