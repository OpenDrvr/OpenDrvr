//! Device state and Configuration Manager problem-code mapping.
//!
//! The numeric values mirror the public `CM_PROB_*` and `DN_*` constants from
//! `cfg.h` / `cfgmgr32.h`. Keeping them here makes the hardware domain portable
//! while the Windows provider remains the only code that calls CfgMgr32.

use serde::Serialize;

const DN_DRIVER_LOADED: u32 = 0x0000_0002;
const DN_STARTED: u32 = 0x0000_0008;
const DN_NEED_RESTART: u32 = 0x0000_0100;
const DN_HAS_PROBLEM: u32 = 0x0000_0400;

const CM_PROB_DISABLED: u32 = 22;
const CM_PROB_HARDWARE_DISABLED: u32 = 29;

/// High-level lifecycle state of a device node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeviceState {
    /// The device node is started.
    Started,
    /// A driver is loaded but the device node is not started.
    DriverLoaded,
    /// The device node is present but neither started nor driver-loaded.
    Stopped,
    /// Windows reports the device as disabled.
    Disabled,
    /// Windows reports a non-disabled problem.
    Problem,
    /// The status could not be read.
    Unknown,
}

/// A Configuration Manager problem code with a stable, coarse classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceProblem {
    /// Raw `CM_PROB_*` value. Preserved even when the kind is `Other`.
    pub code: u32,
    pub kind: ProblemKind,
}

impl DeviceProblem {
    pub fn new(code: u32) -> Self {
        Self {
            code,
            kind: ProblemKind::from_code(code),
        }
    }
}

/// Coarse classification of a device problem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProblemKind {
    NotConfigured,
    DriverLoadFailed,
    DriverBlocked,
    DriverUnsigned,
    DeviceDisabled,
    HardwareDisabled,
    DeviceNotPresent,
    FailedStart,
    FailedInstall,
    NeedsRestart,
    NeedsReinstall,
    Phantom,
    ResourceConflict,
    Other,
}

impl ProblemKind {
    pub fn from_code(code: u32) -> Self {
        match code {
            1 => Self::NotConfigured,
            2 | 39 => Self::DriverLoadFailed,
            5 | 6 | 12 | 17 | 30 | 36 => Self::ResourceConflict,
            10 => Self::FailedStart,
            14 => Self::NeedsRestart,
            18 => Self::NeedsReinstall,
            22 => Self::DeviceDisabled,
            24 => Self::DeviceNotPresent,
            28 | 31 => Self::FailedInstall,
            29 => Self::HardwareDisabled,
            45 => Self::Phantom,
            48 => Self::DriverBlocked,
            52 => Self::DriverUnsigned,
            _ => Self::Other,
        }
    }
}

/// Status of a device node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceStatus {
    pub state: DeviceState,
    pub problem: Option<DeviceProblem>,
    /// Raw `CM_DEVNODE_STATUS_FLAGS` bit mask, retained for diagnostics.
    pub flags: u32,
}

impl DeviceStatus {
    /// Derives a portable status from raw Configuration Manager values.
    pub fn from_flags(flags: u32, problem_code: u32) -> Self {
        let problem = (problem_code != 0).then(|| DeviceProblem::new(problem_code));
        let has_problem = flags & DN_HAS_PROBLEM != 0 || problem.is_some();

        let state = if has_problem {
            match problem_code {
                CM_PROB_DISABLED | CM_PROB_HARDWARE_DISABLED => DeviceState::Disabled,
                _ => DeviceState::Problem,
            }
        } else if flags & DN_STARTED != 0 {
            DeviceState::Started
        } else if flags & DN_DRIVER_LOADED != 0 {
            DeviceState::DriverLoaded
        } else {
            DeviceState::Stopped
        };

        Self {
            state,
            problem,
            flags,
        }
    }

    /// Used when the status query itself fails. The device record is still
    /// emitted, but its lifecycle state is explicitly unknown.
    pub fn unknown() -> Self {
        Self {
            state: DeviceState::Unknown,
            problem: None,
            flags: 0,
        }
    }

    /// Whether Windows marked the device as requiring a restart.
    pub fn needs_restart(&self) -> bool {
        self.flags & DN_NEED_RESTART != 0
    }

    /// Whether the device is usable without a reported problem.
    pub fn is_healthy(&self) -> bool {
        matches!(self.state, DeviceState::Started | DeviceState::DriverLoaded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_started_device() {
        let status = DeviceStatus::from_flags(DN_STARTED | DN_DRIVER_LOADED, 0);

        assert_eq!(status.state, DeviceState::Started);
        assert_eq!(status.problem, None);
        assert!(status.is_healthy());
        assert!(!status.needs_restart());
    }

    #[test]
    fn maps_driver_loaded_but_not_started() {
        let status = DeviceStatus::from_flags(DN_DRIVER_LOADED, 0);

        assert_eq!(status.state, DeviceState::DriverLoaded);
        assert!(status.is_healthy());
    }

    #[test]
    fn maps_disabled_problem_codes() {
        let disabled = DeviceStatus::from_flags(DN_HAS_PROBLEM, CM_PROB_DISABLED);
        let hardware_disabled = DeviceStatus::from_flags(DN_HAS_PROBLEM, CM_PROB_HARDWARE_DISABLED);

        assert_eq!(disabled.state, DeviceState::Disabled);
        assert_eq!(
            disabled.problem.map(|problem| problem.kind),
            Some(ProblemKind::DeviceDisabled)
        );
        assert_eq!(hardware_disabled.state, DeviceState::Disabled);
        assert_eq!(
            hardware_disabled.problem.map(|problem| problem.kind),
            Some(ProblemKind::HardwareDisabled)
        );
        assert!(!disabled.is_healthy());
    }

    #[test]
    fn maps_driver_problem_codes() {
        let failed_start = DeviceStatus::from_flags(DN_HAS_PROBLEM, 10);
        let blocked = DeviceStatus::from_flags(DN_HAS_PROBLEM, 48);
        let unsigned = DeviceStatus::from_flags(DN_HAS_PROBLEM, 52);

        assert_eq!(failed_start.state, DeviceState::Problem);
        assert_eq!(
            failed_start.problem.map(|problem| problem.kind),
            Some(ProblemKind::FailedStart)
        );
        assert_eq!(
            blocked.problem.map(|problem| problem.kind),
            Some(ProblemKind::DriverBlocked)
        );
        assert_eq!(
            unsigned.problem.map(|problem| problem.kind),
            Some(ProblemKind::DriverUnsigned)
        );
    }

    #[test]
    fn preserves_unknown_problem_code() {
        let status = DeviceStatus::from_flags(DN_HAS_PROBLEM, 999);

        assert_eq!(status.state, DeviceState::Problem);
        assert_eq!(status.problem.map(|problem| problem.code), Some(999));
        assert_eq!(
            status.problem.map(|problem| problem.kind),
            Some(ProblemKind::Other)
        );
    }

    #[test]
    fn maps_stopped_and_unknown_states() {
        let stopped = DeviceStatus::from_flags(0, 0);
        let unknown = DeviceStatus::unknown();

        assert_eq!(stopped.state, DeviceState::Stopped);
        assert_eq!(unknown.state, DeviceState::Unknown);
        assert!(!unknown.is_healthy());
    }

    #[test]
    fn exposes_restart_flag() {
        let status = DeviceStatus::from_flags(DN_STARTED | DN_NEED_RESTART, 0);

        assert!(status.needs_restart());
    }
}
