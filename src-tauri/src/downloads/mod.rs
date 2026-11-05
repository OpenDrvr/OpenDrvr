//! Download orchestration domain.
//!
//! Future responsibilities: queue management, concurrency limits, checksum
//! verification, and progress reporting. No network or file transfer code is
//! implemented in this phase.

use crate::config::DownloadConfig;
use crate::state::{ModuleState, ModuleStatus};

#[derive(Debug)]
pub struct DownloadManager {
    config: DownloadConfig,
}

impl DownloadManager {
    pub fn new(config: DownloadConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &DownloadConfig {
        &self.config
    }

    pub fn status(&self) -> ModuleStatus {
        ModuleStatus {
            name: "downloads",
            state: ModuleState::Ready,
            detail: format!(
                "Limits configured ({} concurrent, {}s timeout); transfer support is planned.",
                self.config.max_concurrent, self.config.timeout_seconds
            ),
        }
    }
}
