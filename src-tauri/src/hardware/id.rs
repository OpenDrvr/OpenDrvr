//! Device instance IDs and hardware identifiers.
//!
//! Windows identification strings are opaque to applications. The types in this
//! module always retain the exact string reported by the operating system, and
//! only add best-effort PCI/USB structure as a convenience for matching. A
//! malformed or unknown identifier is preserved verbatim instead of being
//! rejected or rewritten.

use std::fmt;

use serde::Serialize;

/// A Plug and Play device instance ID, such as
/// `PCI\VEN_8086&DEV_1234&SUBSYS_12345678&REV_01\3&11583659&0&10`.
///
/// The value is the authoritative identity of one device instance. It is kept
/// exactly as Windows reported it; callers must not derive identity from a
/// friendly name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct DeviceInstanceId(String);

impl DeviceInstanceId {
    /// Creates an instance ID from a raw Windows string.
    ///
    /// Empty strings and strings containing NUL are rejected because they
    /// cannot be valid device instance IDs and would break CfgMgr32 lookups.
    pub fn new(value: impl Into<String>) -> Option<Self> {
        let value = value.into();
        if value.is_empty() || value.contains('\0') {
            return None;
        }

        Some(Self(value))
    }

    /// The raw instance ID exactly as reported by Windows.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The enumerator prefix, for example `PCI`, `USB`, or `ACPI`.
    pub fn enumerator(&self) -> Option<&str> {
        self.0
            .split_once('\\')
            .map(|(enumerator, _)| enumerator)
            .filter(|enumerator| !enumerator.is_empty())
    }

    /// Best-effort bus classification derived from the enumerator prefix.
    pub fn bus_type(&self) -> BusType {
        self.enumerator()
            .map(BusType::from_enumerator)
            .unwrap_or(BusType::Unknown)
    }
}

impl fmt::Display for DeviceInstanceId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Bus or enumerator family associated with a device.
///
/// This is a convenience classification, not a replacement for the raw
/// instance ID or hardware IDs. Unknown enumerators are retained.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BusType {
    Pci,
    Usb,
    Acpi,
    Root,
    SoftwareDevice,
    Bluetooth,
    Audio,
    Storage,
    Other(String),
    Unknown,
}

impl BusType {
    /// Maps a Windows enumerator name to a bus family.
    pub fn from_enumerator(enumerator: &str) -> Self {
        if enumerator.eq_ignore_ascii_case("PCI") {
            Self::Pci
        } else if enumerator.eq_ignore_ascii_case("USB") {
            Self::Usb
        } else if enumerator.eq_ignore_ascii_case("ACPI") {
            Self::Acpi
        } else if enumerator.eq_ignore_ascii_case("ROOT") {
            Self::Root
        } else if enumerator.eq_ignore_ascii_case("SWD") || enumerator.eq_ignore_ascii_case("SW") {
            Self::SoftwareDevice
        } else if enumerator.eq_ignore_ascii_case("BTH")
            || enumerator.eq_ignore_ascii_case("BTHENUM")
            || enumerator.eq_ignore_ascii_case("BTHLE")
            || enumerator.eq_ignore_ascii_case("BTHLEDEVICE")
        {
            Self::Bluetooth
        } else if enumerator.eq_ignore_ascii_case("HDAUDIO")
            || enumerator.eq_ignore_ascii_case("MMDEVAPI")
        {
            Self::Audio
        } else if enumerator.eq_ignore_ascii_case("SCSI")
            || enumerator.eq_ignore_ascii_case("STORAGE")
            || enumerator.eq_ignore_ascii_case("IDE")
            || enumerator.eq_ignore_ascii_case("NVME")
        {
            Self::Storage
        } else {
            Self::Other(enumerator.to_string())
        }
    }
}

/// A hardware or compatible ID reported for a device.
///
/// `raw` is authoritative. `kind` contains best-effort parsed PCI or USB fields
/// and never changes the raw value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HardwareId {
    pub raw: String,
    pub kind: HardwareIdKind,
}

impl HardwareId {
    /// Parses an identifier while preserving the original string.
    pub fn parse(raw: impl Into<String>) -> Self {
        let raw = raw.into();
        let kind = if let Some(rest) = strip_prefix_ignore_ascii_case(&raw, "PCI\\") {
            HardwareIdKind::Pci(PciHardwareId::parse(rest))
        } else if let Some(rest) = strip_prefix_ignore_ascii_case(&raw, "USB\\") {
            HardwareIdKind::Usb(UsbHardwareId::parse(rest))
        } else if let Some(rest) = strip_prefix_ignore_ascii_case(&raw, "ACPI\\") {
            HardwareIdKind::Acpi(AcpiHardwareId::parse(rest))
        } else if let Some(rest) = strip_prefix_ignore_ascii_case(&raw, "HDAUDIO\\") {
            HardwareIdKind::Hdaudio(HdaudioHardwareId::parse(rest))
        } else {
            HardwareIdKind::Unknown
        };

        Self { raw, kind }
    }

    /// The raw identifier exactly as reported by Windows.
    pub fn as_str(&self) -> &str {
        &self.raw
    }

    /// Best-effort bus family inferred from the identifier prefix.
    pub fn bus_type(&self) -> BusType {
        match self.kind {
            HardwareIdKind::Pci(_) => BusType::Pci,
            HardwareIdKind::Usb(_) => BusType::Usb,
            HardwareIdKind::Acpi(_) => BusType::Acpi,
            HardwareIdKind::Hdaudio(_) => BusType::Audio,
            HardwareIdKind::Unknown => BusType::Unknown,
        }
    }
}

/// Parsed form of a hardware or compatible ID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "bus", content = "details")]
pub enum HardwareIdKind {
    Pci(PciHardwareId),
    Usb(UsbHardwareId),
    Acpi(AcpiHardwareId),
    Hdaudio(HdaudioHardwareId),
    Unknown,
}

/// Best-effort PCI identifier fields.
///
/// PCI hardware IDs use `PCI\VEN_vvvv&DEV_dddd&SUBSYS_ssssdddd&REV_rr`.
/// Compatible IDs may instead carry `CC_cccccc` and omit the subsystem.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PciHardwareId {
    pub vendor_id: Option<String>,
    pub device_id: Option<String>,
    pub subsystem_vendor_id: Option<String>,
    pub subsystem_device_id: Option<String>,
    pub revision: Option<String>,
    pub class_code: Option<String>,
}

impl PciHardwareId {
    fn parse(value: &str) -> Self {
        let mut parsed = Self::default();

        for token in value.split('&') {
            let Some((key, value)) = token.split_once('_') else {
                continue;
            };

            match key.to_ascii_uppercase().as_str() {
                "VEN" => parsed.vendor_id = hex_field(value, 4),
                "DEV" => parsed.device_id = hex_field(value, 4),
                "SUBSYS" => {
                    // SUBSYS_ssssdddd: the first four hex digits are the
                    // subsystem device ID, the last four the subsystem vendor.
                    if value.len() == 8
                        && value.chars().all(|character| character.is_ascii_hexdigit())
                    {
                        parsed.subsystem_device_id = Some(value[..4].to_ascii_uppercase());
                        parsed.subsystem_vendor_id = Some(value[4..].to_ascii_uppercase());
                    }
                }
                "REV" => parsed.revision = hex_field(value, 2),
                "CC" => parsed.class_code = hex_field(value, 6),
                _ => {}
            }
        }

        parsed
    }
}

/// Best-effort USB identifier fields.
///
/// Hardware IDs use `USB\VID_vvvv&PID_dddd`, optionally followed by `REV_rrrr`
/// or `MI_zz`. Compatible IDs may instead carry `CLASS_cc&SUBCLASS_ss&PROT_pp`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsbHardwareId {
    pub vendor_id: Option<String>,
    pub product_id: Option<String>,
    pub revision: Option<String>,
    pub interface_number: Option<String>,
    pub class_code: Option<String>,
    pub subclass_code: Option<String>,
    pub protocol_code: Option<String>,
}

impl UsbHardwareId {
    fn parse(value: &str) -> Self {
        let mut parsed = Self::default();

        for token in value.split('&') {
            let Some((key, value)) = token.split_once('_') else {
                continue;
            };

            match key.to_ascii_uppercase().as_str() {
                "VID" => parsed.vendor_id = hex_field(value, 4),
                "PID" => parsed.product_id = hex_field(value, 4),
                "REV" => parsed.revision = hex_field(value, 4),
                "MI" => parsed.interface_number = hex_field(value, 2),
                "CLASS" => parsed.class_code = hex_field(value, 2),
                "SUBCLASS" => parsed.subclass_code = hex_field(value, 2),
                "PROT" => parsed.protocol_code = hex_field(value, 2),
                _ => {}
            }
        }

        parsed
    }
}

/// Best-effort ACPI identifier fields.
///
/// ACPI hardware IDs use either `ACPI\VEN_vvvv&DEV_dddd` or a compact
/// `ACPI\PNP0A08` / `ACPI\INT3450` / `ACPI\ACPI0003` form. The compact form is
/// an EISA-style vendor prefix followed by a device identifier. The prefix is
/// an ID namespace owner, not necessarily the device's manufacturer.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcpiHardwareId {
    pub vendor_id: Option<String>,
    pub device_id: Option<String>,
}

impl AcpiHardwareId {
    fn parse(value: &str) -> Self {
        let mut parsed = Self::default();

        for token in value.split('&') {
            let Some((key, value)) = token.split_once('_') else {
                continue;
            };

            match key.to_ascii_uppercase().as_str() {
                "VEN" => parsed.vendor_id = alphanumeric_field(value, 4),
                "DEV" => parsed.device_id = alphanumeric_field(value, 4),
                _ => {}
            }
        }

        if parsed.vendor_id.is_none() && parsed.device_id.is_none() {
            let value = value.trim();
            if value.is_ascii()
                && value.len() == 7
                && value[..3]
                    .chars()
                    .all(|character| character.is_ascii_alphabetic())
                && value[3..]
                    .chars()
                    .all(|character| character.is_ascii_hexdigit())
            {
                parsed.vendor_id = Some(value[..3].to_ascii_uppercase());
                parsed.device_id = Some(value.to_ascii_uppercase());
            } else if value.is_ascii()
                && value.len() == 8
                && value[..4]
                    .chars()
                    .all(|character| character.is_ascii_alphabetic())
                && value[4..]
                    .chars()
                    .all(|character| character.is_ascii_hexdigit())
            {
                parsed.vendor_id = Some(value[..4].to_ascii_uppercase());
                parsed.device_id = Some(value.to_ascii_uppercase());
            } else if !value.is_empty() {
                parsed.device_id = Some(value.to_ascii_uppercase());
            }
        }

        parsed
    }
}

/// Best-effort HD Audio identifier fields.
///
/// HD Audio hardware IDs use
/// `HDAUDIO\FUNC_01&VEN_10EC&DEV_0256&SUBSYS_17AA2245&REV_1000`. Unlike PCI,
/// the `SUBSYS` field lists the subsystem vendor first, then the subsystem
/// device.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HdaudioHardwareId {
    pub function_group: Option<String>,
    pub vendor_id: Option<String>,
    pub device_id: Option<String>,
    pub subsystem_vendor_id: Option<String>,
    pub subsystem_device_id: Option<String>,
    pub revision: Option<String>,
}

impl HdaudioHardwareId {
    fn parse(value: &str) -> Self {
        let mut parsed = Self::default();

        for token in value.split('&') {
            let Some((key, value)) = token.split_once('_') else {
                continue;
            };

            match key.to_ascii_uppercase().as_str() {
                "FUNC" => parsed.function_group = hex_field(value, 2),
                "VEN" => parsed.vendor_id = hex_field(value, 4),
                "DEV" => parsed.device_id = hex_field(value, 4),
                "SUBSYS" => {
                    if value.len() == 8
                        && value.chars().all(|character| character.is_ascii_hexdigit())
                    {
                        parsed.subsystem_vendor_id = Some(value[..4].to_ascii_uppercase());
                        parsed.subsystem_device_id = Some(value[4..].to_ascii_uppercase());
                    }
                }
                "REV" => parsed.revision = hex_field(value, 4),
                _ => {}
            }
        }

        parsed
    }
}

fn strip_prefix_ignore_ascii_case<'a>(value: &'a str, prefix: &str) -> Option<&'a str> {
    value
        .get(..prefix.len())
        .filter(|head| head.eq_ignore_ascii_case(prefix))
        .map(|_| &value[prefix.len()..])
}

fn hex_field(value: &str, expected_length: usize) -> Option<String> {
    if value.len() == expected_length
        && value.chars().all(|character| character.is_ascii_hexdigit())
    {
        Some(value.to_ascii_uppercase())
    } else {
        None
    }
}

fn alphanumeric_field(value: &str, expected_length: usize) -> Option<String> {
    if value.len() == expected_length
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric())
    {
        Some(value.to_ascii_uppercase())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pci_hardware_id_and_subsystem_order() {
        let id = HardwareId::parse(r"PCI\VEN_8086&DEV_A70D&SUBSYS_12345678&REV_01");

        assert_eq!(id.as_str(), r"PCI\VEN_8086&DEV_A70D&SUBSYS_12345678&REV_01");
        let HardwareIdKind::Pci(pci) = id.kind else {
            panic!("expected PCI identifier");
        };
        assert_eq!(pci.vendor_id.as_deref(), Some("8086"));
        assert_eq!(pci.device_id.as_deref(), Some("A70D"));
        assert_eq!(pci.subsystem_device_id.as_deref(), Some("1234"));
        assert_eq!(pci.subsystem_vendor_id.as_deref(), Some("5678"));
        assert_eq!(pci.revision.as_deref(), Some("01"));
        assert_eq!(pci.class_code, None);
    }

    #[test]
    fn parses_pci_compatible_id_class_code() {
        let id = HardwareId::parse(r"PCI\VEN_8086&DEV_A70D&CC_030000");

        let HardwareIdKind::Pci(pci) = id.kind else {
            panic!("expected PCI identifier");
        };
        assert_eq!(pci.class_code.as_deref(), Some("030000"));
        assert_eq!(pci.subsystem_vendor_id, None);
    }

    #[test]
    fn parses_usb_hardware_id_and_interface() {
        let id = HardwareId::parse(r"USB\VID_046D&PID_C52B&MI_00&REV_2400");

        let HardwareIdKind::Usb(usb) = id.kind else {
            panic!("expected USB identifier");
        };
        assert_eq!(usb.vendor_id.as_deref(), Some("046D"));
        assert_eq!(usb.product_id.as_deref(), Some("C52B"));
        assert_eq!(usb.interface_number.as_deref(), Some("00"));
        assert_eq!(usb.revision.as_deref(), Some("2400"));
    }

    #[test]
    fn parses_usb_compatible_id_class_triplet() {
        let id = HardwareId::parse(r"USB\CLASS_03&SUBCLASS_01&PROT_02");

        let HardwareIdKind::Usb(usb) = id.kind else {
            panic!("expected USB identifier");
        };
        assert_eq!(usb.class_code.as_deref(), Some("03"));
        assert_eq!(usb.subclass_code.as_deref(), Some("01"));
        assert_eq!(usb.protocol_code.as_deref(), Some("02"));
        assert_eq!(usb.vendor_id, None);
    }

    #[test]
    fn parses_acpi_compact_and_ven_dev_identifiers() {
        let compact = HardwareId::parse(r"ACPI\PNP0A08");
        let HardwareIdKind::Acpi(ref acpi) = compact.kind else {
            panic!("expected ACPI identifier");
        };
        assert_eq!(acpi.vendor_id.as_deref(), Some("PNP"));
        assert_eq!(acpi.device_id.as_deref(), Some("PNP0A08"));
        assert_eq!(compact.bus_type(), BusType::Acpi);

        let four_letter = HardwareId::parse(r"ACPI\ACPI0003");
        let HardwareIdKind::Acpi(acpi) = four_letter.kind else {
            panic!("expected ACPI identifier");
        };
        assert_eq!(acpi.vendor_id.as_deref(), Some("ACPI"));
        assert_eq!(acpi.device_id.as_deref(), Some("ACPI0003"));

        let ven_dev = HardwareId::parse(r"ACPI\VEN_INTC&DEV_0A08");
        let HardwareIdKind::Acpi(acpi) = ven_dev.kind else {
            panic!("expected ACPI identifier");
        };
        assert_eq!(acpi.vendor_id.as_deref(), Some("INTC"));
        assert_eq!(acpi.device_id.as_deref(), Some("0A08"));
    }

    #[test]
    fn parses_hdaudio_codec_and_subsystem_fields() {
        let id = HardwareId::parse(r"HDAUDIO\FUNC_01&VEN_10EC&DEV_0256&SUBSYS_17AA2245&REV_1000");

        assert_eq!(id.bus_type(), BusType::Audio);
        let HardwareIdKind::Hdaudio(hdaudio) = id.kind else {
            panic!("expected HD Audio identifier");
        };
        assert_eq!(hdaudio.function_group.as_deref(), Some("01"));
        assert_eq!(hdaudio.vendor_id.as_deref(), Some("10EC"));
        assert_eq!(hdaudio.device_id.as_deref(), Some("0256"));
        assert_eq!(hdaudio.subsystem_vendor_id.as_deref(), Some("17AA"));
        assert_eq!(hdaudio.subsystem_device_id.as_deref(), Some("2245"));
        assert_eq!(hdaudio.revision.as_deref(), Some("1000"));
    }

    #[test]
    fn preserves_malformed_and_unknown_identifiers() {
        let malformed = HardwareId::parse(r"PCI\VEN_ZZZZ&DEV_1234");
        assert_eq!(malformed.as_str(), r"PCI\VEN_ZZZZ&DEV_1234");
        let HardwareIdKind::Pci(pci) = malformed.kind else {
            panic!("expected PCI identifier");
        };
        assert_eq!(pci.vendor_id, None);
        assert_eq!(pci.device_id.as_deref(), Some("1234"));

        let unknown = HardwareId::parse(r"SWD\MMDEVAPI\AudioEndpoint");
        assert_eq!(unknown.as_str(), r"SWD\MMDEVAPI\AudioEndpoint");
        assert_eq!(unknown.kind, HardwareIdKind::Unknown);
        assert_eq!(unknown.bus_type(), BusType::Unknown);
    }

    #[test]
    fn recognizes_prefix_case_insensitively() {
        let id = HardwareId::parse(r"pci\ven_10de&dev_2684");
        assert!(matches!(id.kind, HardwareIdKind::Pci(_)));
        assert_eq!(id.bus_type(), BusType::Pci);
    }

    #[test]
    fn classifies_instance_id_bus() {
        let pci = DeviceInstanceId::new(r"PCI\VEN_8086&DEV_1234\3&11583659&0&10")
            .expect("valid instance id");
        let usb = DeviceInstanceId::new(r"USB\VID_046D&PID_C52B\5&2A1B2C3D&0&1")
            .expect("valid instance id");
        let other = DeviceInstanceId::new(r"ACPI\PNP0A08\0").expect("valid instance id");

        assert_eq!(pci.enumerator(), Some("PCI"));
        assert_eq!(pci.bus_type(), BusType::Pci);
        assert_eq!(usb.bus_type(), BusType::Usb);
        assert_eq!(other.bus_type(), BusType::Acpi);
    }

    #[test]
    fn rejects_empty_or_nul_instance_ids() {
        assert!(DeviceInstanceId::new("").is_none());
        assert!(DeviceInstanceId::new("PCI\0BAD").is_none());
    }
}
