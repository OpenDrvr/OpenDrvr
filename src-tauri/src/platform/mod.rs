//! Platform abstraction layer.
//!
//! General application logic must not call Win32 APIs directly. Windows-only
//! integration belongs in [`windows`], which keeps the rest of the backend
//! portable and makes the unsafe surface easy to audit.

use std::path::PathBuf;

use tauri::{AppHandle, Manager};

#[cfg(windows)]
mod windows;

/// Human-readable platform identifier exposed to the frontend.
#[cfg(windows)]
pub fn name() -> &'static str {
    windows::NAME
}

#[cfg(not(windows))]
pub fn name() -> &'static str {
    std::env::consts::OS
}

/// OpenDrvr currently supports Windows 10/11 on x64 only.
pub fn is_supported() -> bool {
    cfg!(windows) && cfg!(target_arch = "x86_64")
}

/// Per-user configuration directory (`%APPDATA%\com.opendrvr.desktop` on
/// Windows, derived from the Tauri bundle identifier).
pub fn config_dir(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_config_dir().ok()
}
