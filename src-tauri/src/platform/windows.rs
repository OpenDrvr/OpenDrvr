//! Windows-specific integration surface.
//!
//! Future work such as WMI queries, SetupAPI/CfgMgr32 device enumeration,
//! registry access, and driver package inspection belongs here (or in sibling
//! modules under `platform::windows`). Keeping those calls isolated means
//! commands and domain services never depend on unsafe OS APIs directly.

/// Canonical platform name reported by the backend.
pub const NAME: &str = "windows";
