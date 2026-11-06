//! Conservative device categorization.
//!
//! Categories are derived from the Windows device class, PCI class codes, USB
//! interface class codes, and a small set of well-known ACPI device IDs.
//! Friendly names are never used. This is deliberately a best-effort grouping
//! for the UI; the raw device class and hardware IDs remain authoritative.

use serde::Serialize;

use super::id::{BusType, HardwareId, HardwareIdKind};

/// Broad device groups used by the hardware view.
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
    Storage,
    StorageController,
    Input,
    Unknown,
}

impl DeviceCategory {
    /// Classifies a device without relying on its friendly name.
    ///
    /// `class_name` is `DEVPKEY_Device_Class`. PCI and USB class codes are read
    /// from both hardware and compatible IDs because the full class code is
    /// often only present in the compatible ID list. Devices that cannot be
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
        if class_name.eq_ignore_ascii_case("Mouse")
            || class_name.eq_ignore_ascii_case("Keyboard")
            || class_name.eq_ignore_ascii_case("HIDClass")
        {
            return Self::Input;
        }
        if class_name.eq_ignore_ascii_case("DiskDrive")
            || class_name.eq_ignore_ascii_case("CDROM")
            || class_name.eq_ignore_ascii_case("FloppyDisk")
            || class_name.eq_ignore_ascii_case("Volume")
        {
            return Self::Storage;
        }

        if class_name.eq_ignore_ascii_case("Net") {
            return match pci_class_code(hardware_ids, compatible_ids) {
                Some(class_code) if class_code.starts_with("0200") => Self::NetworkEthernet,
                Some(class_code) if class_code.starts_with("0280") => Self::NetworkWireless,
                _ => Self::Network,
            };
        }

        if let Some(class_code) = pci_class_code(hardware_ids, compatible_ids) {
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
            if class_code.starts_with("0C03") {
                return Self::Usb;
            }
        }

        // Well-known ACPI IDs for the PCI root complex, system board, and
        // motherboard resources. These identify platform hardware, not a
        // manufacturer, so only the category is derived here.
        if has_acpi_device_id(hardware_ids, compatible_ids, &["PNP0A03", "PNP0A08"]) {
            return Self::Chipset;
        }
        if has_acpi_device_id(hardware_ids, compatible_ids, &["PNP0C01", "PNP0C02"]) {
            return Self::Motherboard;
        }

        if let Some(class_code) = usb_class_code(hardware_ids, compatible_ids) {
            match class_code {
                "01" => return Self::Audio,
                "02" => return Self::Network,
                "03" => return Self::Input,
                "08" => return Self::Storage,
                "0E" => return Self::Graphics,
                "E0" => return Self::NetworkWireless,
                _ => {}
            }
        }

        match bus_type {
            BusType::Usb => Self::Usb,
            BusType::Pci => Self::Pci,
            BusType::Bluetooth => Self::Bluetooth,
            BusType::Audio => Self::Audio,
            BusType::Storage => Self::Storage,
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
            _ => None,
        })
}

fn usb_class_code<'a>(
    hardware_ids: &'a [HardwareId],
    compatible_ids: &'a [HardwareId],
) -> Option<&'a str> {
    hardware_ids
        .iter()
        .chain(compatible_ids)
        .find_map(|identifier| match &identifier.kind {
            HardwareIdKind::Usb(usb) => usb.class_code.as_deref(),
            _ => None,
        })
}

fn has_acpi_device_id(
    hardware_ids: &[HardwareId],
    compatible_ids: &[HardwareId],
    expected: &[&str],
) -> bool {
    hardware_ids
        .iter()
        .chain(compatible_ids)
        .any(|identifier| match &identifier.kind {
            HardwareIdKind::Acpi(acpi) => acpi.device_id.as_deref().is_some_and(|device_id| {
                expected
                    .iter()
                    .any(|candidate| device_id.eq_ignore_ascii_case(candidate))
            }),
            _ => false,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> HardwareId {
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
        let ethernet = id(r"PCI\VEN_8086&DEV_15B8&CC_020000");
        let wireless = id(r"PCI\VEN_8086&DEV_24F3&CC_028000");

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
    fn classifies_bluetooth_audio_and_storage_controllers() {
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
    fn classifies_input_and_storage_devices_by_class() {
        for class_name in ["Mouse", "Keyboard", "HIDClass"] {
            assert_eq!(
                DeviceCategory::classify(Some(class_name), &BusType::Usb, &[], &[]),
                DeviceCategory::Input
            );
        }
        for class_name in ["DiskDrive", "CDROM", "FloppyDisk", "Volume"] {
            assert_eq!(
                DeviceCategory::classify(Some(class_name), &BusType::Storage, &[], &[]),
                DeviceCategory::Storage
            );
        }
    }

    #[test]
    fn classifies_chipset_bridges_and_usb_controllers() {
        let bridge = id(r"PCI\VEN_8086&DEV_A70D&CC_060400");
        let usb_controller = id(r"PCI\VEN_8086&DEV_A36D&CC_0C0330");

        assert_eq!(
            DeviceCategory::classify(Some("System"), &BusType::Pci, &[], &[bridge]),
            DeviceCategory::Chipset
        );
        assert_eq!(
            DeviceCategory::classify(Some("USB"), &BusType::Pci, &[], &[usb_controller]),
            DeviceCategory::Usb
        );
    }

    #[test]
    fn classifies_known_acpi_platform_ids() {
        let root_complex = id(r"ACPI\PNP0A08");
        let system_board = id(r"ACPI\PNP0C01");

        assert_eq!(
            DeviceCategory::classify(Some("System"), &BusType::Acpi, &[], &[root_complex]),
            DeviceCategory::Chipset
        );
        assert_eq!(
            DeviceCategory::classify(Some("System"), &BusType::Acpi, &[], &[system_board]),
            DeviceCategory::Motherboard
        );
    }

    #[test]
    fn classifies_usb_interface_classes_from_compatible_ids() {
        let hid = id(r"USB\CLASS_03&SUBCLASS_01&PROT_02");
        let mass_storage = id(r"USB\CLASS_08&SUBCLASS_06&PROT_50");
        let audio = id(r"USB\CLASS_01&SUBCLASS_01&PROT_00");
        let wireless = id(r"USB\CLASS_E0&SUBCLASS_01&PROT_01");

        assert_eq!(
            DeviceCategory::classify(None, &BusType::Usb, &[], &[hid]),
            DeviceCategory::Input
        );
        assert_eq!(
            DeviceCategory::classify(None, &BusType::Usb, &[], &[mass_storage]),
            DeviceCategory::Storage
        );
        assert_eq!(
            DeviceCategory::classify(None, &BusType::Usb, &[], &[audio]),
            DeviceCategory::Audio
        );
        assert_eq!(
            DeviceCategory::classify(None, &BusType::Usb, &[], &[wireless]),
            DeviceCategory::NetworkWireless
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
        assert_eq!(
            DeviceCategory::classify(None, &BusType::Bluetooth, &[], &[]),
            DeviceCategory::Bluetooth
        );
        assert_eq!(
            DeviceCategory::classify(None, &BusType::Audio, &[], &[]),
            DeviceCategory::Audio
        );
        assert_eq!(
            DeviceCategory::classify(None, &BusType::Storage, &[], &[]),
            DeviceCategory::Storage
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
