//! Windows-specific integration surface.
//!
//! Win32 calls are confined to this module tree. The hardware domain receives
//! only the portable `DeviceEnumerator` trait; it never links to CfgMgr32
//! directly. Future WMI, SetupAPI driver-package inspection, and registry
//! helpers also belong here.

pub mod device_tree;

/// Canonical platform name reported by the backend.
pub const NAME: &str = "windows";
