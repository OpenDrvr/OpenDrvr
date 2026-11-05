//! Hardware identification domain.
//!
//! This module will own device enumeration and normalization of hardware IDs.
//! It is deliberately empty for now: no WMI, SetupAPI, or registry calls are
//! made until the data model is designed. OS access must go through
//! [`crate::platform`], never through commands.

use crate::state::ModuleStatus;

#[derive(Debug, Default)]
pub struct HardwareService;

impl HardwareService {
    pub fn new() -> Self {
        Self
    }

    pub fn status(&self) -> ModuleStatus {
        ModuleStatus::planned("hardware", "Device enumeration is not implemented yet.")
    }
}
