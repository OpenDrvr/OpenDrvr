//! Driver discovery and compatibility domain.
//!
//! Future responsibilities: match normalized hardware IDs against known driver
//! packages, rank candidates, and describe official sources. This module must
//! not perform downloads; that belongs to [`crate::downloads`].

use crate::state::ModuleStatus;

#[derive(Debug, Default)]
pub struct DriverService;

impl DriverService {
    pub fn new() -> Self {
        Self
    }

    pub fn status(&self) -> ModuleStatus {
        ModuleStatus::planned("drivers", "Driver matching is not implemented yet.")
    }
}
