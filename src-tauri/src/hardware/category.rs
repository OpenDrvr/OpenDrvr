//! Conservative device categorization.
//!
//! Categories are derived from the Windows device class and, when available,
//! PCI class codes. Friendly names are never used. This is deliberately a
//! best-effort grouping for the future UI; the raw device class and hardware
//! IDs remain authoritative.

use serde::Serialize;

use super::id::{BusType, HardwareId, HardwareIdKind};

/// Broad device groups used by the future hardware view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeviceCategory {
    Processor,
    Graphics,
    Motherboard,
    Chipset,
    Pci,
    Usb,
    Network,
    NetworkEthernet,
    NetworkWireless,
    Bluetooth,
    Audio,
    StorageController,
    Unknown,
}

impl DeviceCategory {
    /// Classifies a device without relying on its friendly name.
    ///
    /// `class_name` is `DEVPKEY_Device_Class`. PCI class codes are read from
    /// both hardware and compatible IDs because the full class code is often
    /// only present in the compatible ID list. Devices that cannot be
    /// classified confidently remain `Unknown`.
    pub fn classify(
        class_name: Option<&str>,
        bus_type: &BusType,
        hardware_ids: &[HardwareId],
        compatible_ids: &[HardwareId],
    ) -> Self {
        let class_name = class_name.unwrap_or_default().trim();

        if class_name.eq_ignore_ascii_case("Processor") {
            return Self::Processor;
        }
        if class_name.eq_ignore_ascii_case("Display") {
            return Self::Graphics;
        }
        if class_name.eq_ignore_ascii_case("Bluetooth") {
            return Self::Bluetooth;
        }
        if class_name.eq_ignore_ascii_case("MEDIA") {
            return Self::Audio;
        }
        if class_name.eq_ignore_ascii_case("SCSIAdapter") || class_name.eq_ignore_ascii_case("HDC")
        {
            return Self::StorageController;
        }

        let pci_class = pci_class_code(hardware_ids, compatible_ids);

        if class_name.eq_ignore_ascii_case("Net") {
            return match pci_class {
                Some(class_code) if class_code.starts_with("0200") => Self::NetworkEthernet,
                Some(class_code) if class_code.starts_with("0280") => Self::NetworkWireless,
                _ => Self::Network,
            };
        }

        if let Some(class_code) = pci_class {
            if class_code.starts_with("060") {
                return Self::Chipset;
            }
            if class_code.starts_with("0200") {
                return Self::NetworkEthernet;
            }
            if class_code.starts_with("0280") {
                return Self::NetworkWireless;
            }
            if class_code.starts_with("01") {
                return Self::StorageController;
            }
            if class_code.starts_with("03") {
                return Self::Graphics;
            }
            if class_code.starts_with("0401") {
                return Self::Audio;
            }
        }

        match bus_type {
            BusType::Usb => Self::Usb,
            BusType::Pci => Self::Pci,
            _ => Self::Unknown,
        }
    }
}

fn pci_class_code<'a>(
    hardware_ids: &'a [HardwareId],
    compatible_ids: &'a [HardwareId],
) -> Option<&'a str> {
    hardware_ids
        .iter()
        .chain(compatible_ids)
        .find_map(|identifier| match &identifier.kind {
            HardwareIdKind::Pci(pci) => pci.class_code.as_deref(),
            HardwareIdKind::Usb(_) | HardwareIdKind::Unknown => None,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pci_id(value: &str) -> HardwareId {
        HardwareId::parse(value)
    }

    #[test]
    fn classifies_processor_and_graphics_by_class() {
        assert_eq!(
            DeviceCategory::classify(Some("Processor"), &BusType::Acpi, &[], &[]),
            DeviceCategory::Processor
        );
        assert_eq!(
            DeviceCategory::classify(Some("Display"), &BusType::Pci, &[], &[]),
            DeviceCategory::Graphics
        );
    }

    #[test]
    fn classifies_network_medium_from_pci_class_code() {
        let ethernet = pci_id(r"PCI\VEN_8086&DEV_15B8&CC_020000");
        let wireless = pci_id(r"PCI\VEN_8086&DEV_24F3&CC_028000");

        assert_eq!(
            DeviceCategory::classify(Some("Net"), &BusType::Pci, &[], &[ethernet]),
            DeviceCategory::NetworkEthernet
        );
        assert_eq!(
            DeviceCategory::classify(Some("Net"), &BusType::Pci, &[], &[wireless]),
            DeviceCategory::NetworkWireless
        );
        assert_eq!(
            DeviceCategory::classify(Some("Net"), &BusType::Pci, &[], &[]),
            DeviceCategory::Network
        );
    }

    #[test]
    fn classifies_bluetooth_audio_and_storage() {
        assert_eq!(
            DeviceCategory::classify(Some("Bluetooth"), &BusType::Bluetooth, &[], &[]),
            DeviceCategory::Bluetooth
        );
        assert_eq!(
            DeviceCategory::classify(Some("MEDIA"), &BusType::Pci, &[], &[]),
            DeviceCategory::Audio
        );
        assert_eq!(
            DeviceCategory::classify(Some("SCSIAdapter"), &BusType::Pci, &[], &[]),
            DeviceCategory::StorageController
        );
        assert_eq!(
            DeviceCategory::classify(Some("HDC"), &BusType::Pci, &[], &[]),
            DeviceCategory::StorageController
        );
    }

    #[test]
    fn classifies_chipset_bridges() {
        let bridge = pci_id(r"PCI\VEN_8086&DEV_A70D&CC_060400");

        assert_eq!(
            DeviceCategory::classify(Some("System"), &BusType::Pci, &[], &[bridge]),
            DeviceCategory::Chipset
        );
    }

    #[test]
    fn falls_back_to_bus_family() {
        assert_eq!(
            DeviceCategory::classify(Some("USB"), &BusType::Usb, &[], &[]),
            DeviceCategory::Usb
        );
        assert_eq!(
            DeviceCategory::classify(None, &BusType::Pci, &[], &[]),
            DeviceCategory::Pci
        );
    }

    #[test]
    fn leaves_unclassified_devices_unknown() {
        assert_eq!(
            DeviceCategory::classify(None, &BusType::Acpi, &[], &[]),
            DeviceCategory::Unknown
        );
        assert_eq!(
            DeviceCategory::classify(Some("System"), &BusType::Acpi, &[], &[]),
            DeviceCategory::Unknown
        );
    }
}
