//! Official driver source registry.
//!
//! Future responsibilities: describe trusted manufacturer sources and their
//! lookup rules. No manufacturer integration exists in this phase.

use crate::state::ModuleStatus;

#[derive(Debug, Default)]
pub struct SourceRegistry;

impl SourceRegistry {
    pub fn new() -> Self {
        Self
    }

    pub fn status(&self) -> ModuleStatus {
        ModuleStatus::planned(
            "sources",
            "Manufacturer source integration is not implemented yet.",
        )
    }
}
