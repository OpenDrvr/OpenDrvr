//! Curated vendor identifier tables.
//!
//! Hardware identification must not guess manufacturer names. This module is the
//! single, auditable place where public vendor identifiers are mapped to names.
//! It intentionally contains a useful subset rather than a speculative or
//! exhaustive database; an unknown identifier stays unresolved.
//!
//! Sources for the identifiers:
//!
//! - PCI-SIG member vendor IDs (`VEN_` / `SUBSYS_` vendor fields)
//! - USB-IF vendor IDs (`VID_`)
//! - ACPI/EISA-style vendor prefixes used by Windows PnP IDs
//! - HD Audio codec vendor IDs, which reuse PCI vendor IDs
//!
//! Adding a vendor or a new identifier family only requires another table and a
//! lookup function here. The scanner and identification layer do not change.

/// One known vendor identifier.
#[derive(Debug, Clone, Copy)]
pub struct VendorEntry {
    pub id: &'static str,
    pub name: &'static str,
    aliases: &'static [&'static str],
}

impl VendorEntry {
    /// Whether a Windows-reported manufacturer name is compatible with this
    /// vendor. This is only used to detect conflicts; it never identifies a
    /// vendor from a name.
    pub fn matches_name(&self, reported: &str) -> bool {
        let reported_tokens = normalized_tokens(reported);
        if reported_tokens.is_empty() {
            return false;
        }

        std::iter::once(self.name)
            .chain(self.aliases.iter().copied())
            .any(|candidate| {
                normalized_tokens(candidate)
                    .iter()
                    .any(|token| reported_tokens.contains(token))
            })
    }
}

/// Looks up a PCI vendor or subsystem vendor ID.
pub fn pci(vendor_id: &str) -> Option<&'static VendorEntry> {
    lookup(PCI_VENDORS, vendor_id)
}

/// Looks up a USB vendor ID.
pub fn usb(vendor_id: &str) -> Option<&'static VendorEntry> {
    lookup(USB_VENDORS, vendor_id)
}

/// Looks up an ACPI/EISA-style vendor prefix.
pub fn acpi(vendor_id: &str) -> Option<&'static VendorEntry> {
    lookup(ACPI_VENDORS, vendor_id)
}

/// Looks up an HD Audio codec vendor ID.
///
/// HD Audio `VEN_` values are PCI vendor IDs, so they share the PCI table.
pub fn hdaudio(vendor_id: &str) -> Option<&'static VendorEntry> {
    pci(vendor_id)
}

fn lookup(table: &'static [VendorEntry], vendor_id: &str) -> Option<&'static VendorEntry> {
    let normalized = normalize_id(vendor_id);
    table
        .iter()
        .find(|entry| entry.id.eq_ignore_ascii_case(&normalized))
}

fn normalize_id(vendor_id: &str) -> String {
    vendor_id
        .trim()
        .strip_prefix("0x")
        .or_else(|| vendor_id.trim().strip_prefix("0X"))
        .unwrap_or_else(|| vendor_id.trim())
        .to_ascii_uppercase()
}

fn normalized_tokens(value: &str) -> Vec<String> {
    value
        .split(|character: char| !character.is_ascii_alphanumeric())
        .map(str::to_ascii_lowercase)
        .filter(|token| !token.is_empty() && !GENERIC_TOKENS.contains(&token.as_str()))
        .collect()
}

const GENERIC_TOKENS: &[&str] = &[
    "and",
    "co",
    "company",
    "computer",
    "computers",
    "corp",
    "corporation",
    "electronics",
    "group",
    "inc",
    "incorporated",
    "international",
    "ltd",
    "limited",
    "of",
    "semiconductor",
    "semiconductors",
    "system",
    "systems",
    "technologies",
    "technology",
    "the",
];

const fn vendor(id: &'static str, name: &'static str) -> VendorEntry {
    VendorEntry {
        id,
        name,
        aliases: &[],
    }
}

const fn vendor_with_aliases(
    id: &'static str,
    name: &'static str,
    aliases: &'static [&'static str],
) -> VendorEntry {
    VendorEntry { id, name, aliases }
}

const PCI_VENDORS: &[VendorEntry] = &[
    vendor_with_aliases("1002", "AMD", &["Advanced Micro Devices"]),
    vendor("1013", "Cirrus Logic"),
    vendor_with_aliases("1022", "AMD", &["Advanced Micro Devices"]),
    vendor("1025", "Acer"),
    vendor("1028", "Dell"),
    vendor_with_aliases("103C", "HP", &["Hewlett-Packard", "Hewlett Packard"]),
    vendor_with_aliases("1043", "ASUSTeK", &["ASUS"]),
    vendor("104D", "Sony"),
    vendor_with_aliases("105B", "Foxconn", &["Hon Hai"]),
    vendor("106B", "Apple"),
    vendor("10CF", "Fujitsu"),
    vendor_with_aliases("10DE", "NVIDIA", &["NVIDIA Corporation"]),
    vendor_with_aliases("10EC", "Realtek", &["Realtek Semiconductor"]),
    vendor("1179", "Toshiba"),
    vendor_with_aliases("11AB", "Marvell", &["Marvell Technology"]),
    vendor("1414", "Microsoft"),
    vendor("144D", "Samsung"),
    vendor_with_aliases("1458", "Gigabyte", &["GIGA-BYTE Technology"]),
    vendor_with_aliases(
        "1462",
        "MSI",
        &["Micro-Star International", "Micro Star International"],
    ),
    vendor("14C3", "MediaTek"),
    vendor_with_aliases("14E4", "Broadcom", &["Broadcom Inc."]),
    vendor("1558", "Clevo"),
    vendor("15AD", "VMware"),
    vendor("15B3", "Mellanox"),
    vendor_with_aliases("168C", "Qualcomm Atheros", &["Atheros"]),
    vendor("17AA", "Lenovo"),
    vendor_with_aliases("1814", "Ralink", &["MediaTek"]),
    vendor("1849", "ASRock"),
    vendor("1969", "Qualcomm Atheros"),
    vendor("19DA", "ZOTAC"),
    vendor("1A03", "ASPEED"),
    vendor_with_aliases("1AF4", "Red Hat", &["virtio"]),
    vendor("1B21", "ASMedia"),
    vendor("1B4B", "Marvell"),
    vendor("1D0F", "Amazon"),
    vendor("1D6A", "Aquantia"),
    vendor("3842", "EVGA"),
    vendor("8086", "Intel"),
    vendor("8087", "Intel"),
    vendor("1234", "QEMU"),
];

const USB_VENDORS: &[VendorEntry] = &[
    vendor_with_aliases("03F0", "HP", &["Hewlett-Packard", "Hewlett Packard"]),
    vendor("045E", "Microsoft"),
    vendor_with_aliases("046D", "Logitech", &["Logitech International"]),
    vendor_with_aliases("0489", "Foxconn", &["Hon Hai"]),
    vendor("04B3", "IBM"),
    vendor("04CA", "Lite-On"),
    vendor("04E8", "Samsung"),
    vendor("04F2", "Chicony"),
    vendor_with_aliases("04F3", "ELAN", &["Elan Microelectronics"]),
    vendor("054C", "Sony"),
    vendor("056A", "Wacom"),
    vendor("05AC", "Apple"),
    vendor("06A3", "Saitek"),
    vendor("0781", "SanDisk"),
    vendor("0951", "Kingston"),
    vendor_with_aliases("0A5C", "Broadcom", &["Broadcom Inc."]),
    vendor_with_aliases("0B05", "ASUSTeK", &["ASUS"]),
    vendor_with_aliases("0BDA", "Realtek", &["Realtek Semiconductor"]),
    vendor("0BB4", "HTC"),
    vendor("0BC2", "Seagate"),
    vendor_with_aliases("0CF3", "Qualcomm Atheros", &["Atheros"]),
    vendor("0F0D", "Hori"),
    vendor("1038", "SteelSeries"),
    vendor_with_aliases("1058", "Western Digital", &["WD"]),
    vendor("12D1", "Huawei"),
    vendor("13D3", "IMC Networks"),
    vendor("1532", "Razer"),
    vendor("17EF", "Lenovo"),
    vendor("18D1", "Google"),
    vendor("1B1C", "Corsair"),
    vendor("1E7D", "ROCCAT"),
    vendor("22B8", "Motorola"),
    vendor("2717", "Xiaomi"),
    vendor("2A70", "OnePlus"),
    vendor("413C", "Dell"),
    vendor("8087", "Intel"),
];

const ACPI_VENDORS: &[VendorEntry] = &[
    vendor("AMD", "AMD"),
    vendor_with_aliases("APP", "Apple", &["Apple Inc."]),
    vendor("DELL", "Dell"),
    vendor("FUJ", "Fujitsu"),
    vendor_with_aliases("HPQ", "HP", &["Hewlett-Packard", "Hewlett Packard"]),
    vendor("INT", "Intel"),
    vendor("INTC", "Intel"),
    vendor("LEN", "Lenovo"),
    vendor_with_aliases("NVDA", "NVIDIA", &["NVIDIA Corporation"]),
    vendor("QCOM", "Qualcomm"),
    vendor("SONY", "Sony"),
    vendor("TOS", "Toshiba"),
];

// `PNP`, `ACPI`, and `MSFT` are specification namespace prefixes used by ACPI
// IDs such as `PNP0A08` and `MSFT0101`. They are intentionally absent from the
// table: the prefix owner is not necessarily the device manufacturer, so
// identification stays unresolved instead of guessing Microsoft.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looks_up_common_pci_usb_acpi_and_hdaudio_vendors() {
        assert_eq!(pci("8086").map(|entry| entry.name), Some("Intel"));
        assert_eq!(pci("0x10de").map(|entry| entry.name), Some("NVIDIA"));
        assert_eq!(usb("046d").map(|entry| entry.name), Some("Logitech"));
        assert_eq!(usb("8087").map(|entry| entry.name), Some("Intel"));
        assert_eq!(acpi("INT").map(|entry| entry.name), Some("Intel"));
        assert_eq!(acpi("INTC").map(|entry| entry.name), Some("Intel"));
        assert_eq!(hdaudio("10EC").map(|entry| entry.name), Some("Realtek"));
    }

    #[test]
    fn leaves_unknown_vendor_ids_unresolved() {
        assert!(pci("FFFF").is_none());
        assert!(usb("FFFF").is_none());
        assert!(acpi("ZZZ").is_none());
        assert!(hdaudio("FFFF").is_none());
    }

    #[test]
    fn does_not_treat_acpi_namespace_prefixes_as_manufacturers() {
        assert!(acpi("PNP").is_none());
        assert!(acpi("ACPI").is_none());
        assert!(acpi("MSFT").is_none());
    }

    #[test]
    fn matches_reported_manufacturer_names_and_aliases() {
        let intel = pci("8086").expect("Intel");
        assert!(intel.matches_name("Intel Corporation"));
        assert!(intel.matches_name("Genuine Intel(R) CPU"));

        let amd = pci("1002").expect("AMD");
        assert!(amd.matches_name("Advanced Micro Devices, Inc."));

        let hp = pci("103C").expect("HP");
        assert!(hp.matches_name("Hewlett-Packard Company"));

        assert!(!intel.matches_name("NVIDIA"));
        assert!(!intel.matches_name(""));
    }
}
