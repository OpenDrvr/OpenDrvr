//! Hardware enumeration provider boundary.
//!
//! The hardware domain depends on this trait, not on a concrete operating
//! system API. `AppState` injects the platform implementation so the domain
//! remains portable and testable without a running Windows device tree.

use std::fmt::Debug;

use thiserror::Error;

use super::model::HardwareSnapshot;

/// Enumerates devices for one platform.
pub trait DeviceEnumerator: Debug + Send + Sync {
    fn enumerate(&self) -> Result<HardwareSnapshot, HardwareError>;
}

/// Fatal errors from a device enumeration provider.
#[derive(Debug, Error)]
pub enum HardwareError {
    #[error("device enumeration is not supported on this platform")]
    UnsupportedPlatform,

    #[error("Configuration Manager operation '{operation}' failed with code {code}")]
    ConfigManager { operation: &'static str, code: u32 },
}

/// Fallback provider for non-Windows builds and unsupported targets.
#[derive(Debug, Default, Clone, Copy)]
pub struct UnsupportedDeviceEnumerator;

impl DeviceEnumerator for UnsupportedDeviceEnumerator {
    fn enumerate(&self) -> Result<HardwareSnapshot, HardwareError> {
        Err(HardwareError::UnsupportedPlatform)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_provider_returns_a_typed_error() {
        let result = UnsupportedDeviceEnumerator.enumerate();

        assert!(matches!(result, Err(HardwareError::UnsupportedPlatform)));
    }
}
