//! Hardware identification domain.
//!
//! This module owns the portable device model, hardware-ID parsing, status
//! mapping, and the enumeration provider boundary. It never calls Win32
//! directly: the Windows provider lives under
//! [`crate::platform::windows`] and is injected by [`crate::state::AppState`].
//!
//! Enumeration is exposed through the `scan_hardware` command, but driver
//! matching and catalog lookup remain separate domains.

pub mod category;
pub mod id;
pub mod identify;
pub mod model;
pub mod provider;
pub mod status;
pub mod vendors;

use std::sync::Arc;

use crate::state::{ModuleState, ModuleStatus};

use self::{
    identify::IdentificationConfidence,
    model::HardwareSnapshot,
    provider::{DeviceEnumerator, HardwareError},
};

/// Hardware domain service.
#[derive(Clone, Debug)]
pub struct HardwareService {
    enumerator: Arc<dyn DeviceEnumerator>,
}

impl HardwareService {
    /// Builds the service around a platform-provided enumerator.
    pub fn new(enumerator: Arc<dyn DeviceEnumerator>) -> Self {
        Self { enumerator }
    }

    /// Enumerates the current device tree, including non-present devices.
    ///
    /// Enumeration is synchronous and may block on the operating system, so
    /// callers on an async runtime should run it on a blocking worker.
    pub fn scan(&self) -> Result<HardwareSnapshot, HardwareError> {
        match self.enumerator.enumerate() {
            Ok(snapshot) => {
                let present = snapshot
                    .devices
                    .iter()
                    .filter(|device| device.is_present)
                    .count();
                let mut confirmed = 0;
                let mut partial = 0;
                let mut unresolved = 0;
                for device in &snapshot.devices {
                    match device.identification.confidence {
                        IdentificationConfidence::Confirmed => confirmed += 1,
                        IdentificationConfidence::Partial => partial += 1,
                        IdentificationConfidence::Unresolved => unresolved += 1,
                    }
                }
                tracing::info!(
                    target: "opendrvr::hardware",
                    devices = snapshot.devices.len(),
                    present,
                    non_present = snapshot.devices.len().saturating_sub(present),
                    identified = confirmed,
                    partial,
                    unresolved,
                    property_failures = snapshot.diagnostics.property_failures.len(),
                    device_failures = snapshot.diagnostics.device_failures.len(),
                    duration_ms = snapshot.diagnostics.duration_ms,
                    "hardware scan completed"
                );
                Ok(snapshot)
            }
            Err(error) => {
                tracing::error!(
                    target: "opendrvr::hardware",
                    error = %error,
                    "hardware scan failed"
                );
                Err(error)
            }
        }
    }

    pub fn status(&self) -> ModuleStatus {
        ModuleStatus {
            name: "hardware",
            state: ModuleState::Ready,
            detail: "Windows device enumeration with identifier-based identification is available."
                .to_string(),
        }
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
    fn hardware_module_is_ready() {
        let service = HardwareService::new(Arc::new(UnsupportedDeviceEnumerator));
        let status = service.status();

        assert_eq!(status.name, "hardware");
        assert_eq!(status.state, crate::state::ModuleState::Ready);
    }
}
