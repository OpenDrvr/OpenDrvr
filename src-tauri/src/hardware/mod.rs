//! Hardware identification domain.
//!
//! This module owns the portable device model, hardware-ID parsing, status
//! mapping, and the enumeration provider boundary. It never calls Win32
//! directly: the Windows provider lives under
//! [`crate::platform::windows`] and is injected by [`crate::state::AppState`].
//!
//! Phase 6 prepares native enumeration but does not expose a scan command or
//! frontend hardware data. The complete scanner belongs to Phase 7.

pub mod category;
pub mod id;
pub mod model;
pub mod provider;
pub mod status;

use std::sync::Arc;

use crate::state::ModuleStatus;

use self::{
    model::HardwareSnapshot,
    provider::{DeviceEnumerator, HardwareError},
};

/// Hardware domain service.
#[derive(Debug)]
pub struct HardwareService {
    enumerator: Arc<dyn DeviceEnumerator>,
}

impl HardwareService {
    /// Builds the service around a platform-provided enumerator.
    pub fn new(enumerator: Arc<dyn DeviceEnumerator>) -> Self {
        Self { enumerator }
    }

    /// Enumerates the current device tree.
    ///
    /// This is intentionally not registered as an IPC command yet. Phase 7
    /// will add the user-facing scan flow, persistence, and catalog matching.
    pub fn scan(&self) -> Result<HardwareSnapshot, HardwareError> {
        self.enumerator.enumerate()
    }

    pub fn status(&self) -> ModuleStatus {
        ModuleStatus::planned(
            "hardware",
            "Device enumeration layer is prepared; the hardware scan is scheduled for Phase 7.",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hardware::{model::ScanDiagnostics, provider::UnsupportedDeviceEnumerator};

    #[derive(Debug)]
    struct EmptyEnumerator;

    impl DeviceEnumerator for EmptyEnumerator {
        fn enumerate(&self) -> Result<HardwareSnapshot, HardwareError> {
            Ok(HardwareSnapshot {
                devices: Vec::new(),
                diagnostics: ScanDiagnostics {
                    provider: "test".to_string(),
                    present_only: true,
                    ..ScanDiagnostics::default()
                },
            })
        }
    }

    #[test]
    fn service_delegates_to_the_injected_enumerator() {
        let service = HardwareService::new(Arc::new(EmptyEnumerator));
        let snapshot = service.scan().expect("empty snapshot");

        assert!(snapshot.devices.is_empty());
        assert_eq!(snapshot.diagnostics.provider, "test");
        assert!(snapshot.diagnostics.present_only);
    }

    #[test]
    fn service_reports_unsupported_provider_errors() {
        let service = HardwareService::new(Arc::new(UnsupportedDeviceEnumerator));

        assert!(matches!(
            service.scan(),
            Err(HardwareError::UnsupportedPlatform)
        ));
    }

    #[test]
    fn hardware_module_remains_planned_in_phase_six() {
        let service = HardwareService::new(Arc::new(UnsupportedDeviceEnumerator));
        let status = service.status();

        assert_eq!(status.name, "hardware");
        assert_eq!(status.state, crate::state::ModuleState::Planned);
    }
}
