//! Platform abstraction layer.
//!
//! General application logic must not call Win32 APIs directly. Windows-only
//! integration belongs in [`windows`], which keeps the rest of the backend
//! portable and makes the unsafe surface easy to audit.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use tauri::{AppHandle, Manager};

use crate::hardware::provider::{DeviceEnumerator, UnsupportedDeviceEnumerator};

#[cfg(windows)]
mod windows;

/// File name of the bundled catalog snapshot inside the resource directory.
pub const CATALOG_FILE_NAME: &str = "catalog.sqlite3";

/// Per-user directory holding optional immutable catalog update snapshots.
pub const CATALOG_UPDATES_DIR_NAME: &str = "catalog-updates";

/// Returns the platform device enumerator injected into the hardware domain.
///
/// Win32 access stays inside `platform::windows`; the hardware domain only sees
/// the portable [`DeviceEnumerator`] trait. OpenDrvr targets Windows 10/11 on
/// x64, so other Windows architectures fall back to the unsupported provider.
#[cfg(windows)]
pub fn device_enumerator() -> Arc<dyn DeviceEnumerator> {
    if cfg!(target_arch = "x86_64") {
        Arc::new(windows::device_tree::WindowsDeviceEnumerator)
    } else {
        Arc::new(UnsupportedDeviceEnumerator)
    }
}

/// Returns a provider that reports unsupported-platform errors.
#[cfg(not(windows))]
pub fn device_enumerator() -> Arc<dyn DeviceEnumerator> {
    Arc::new(UnsupportedDeviceEnumerator)
}

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

/// Per-user, writable application data directory. This is where the user
/// database and its recovery backups live.
pub fn data_dir(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_data_dir().ok()
}

/// Per-user directory for optional immutable catalog update snapshots.
///
/// The directory is a sibling of the writable user database, not a place to
/// store preferences or history. A missing directory means "use the bundled
/// catalog". Runtime code only reads validated `.sqlite3` snapshots from here.
pub fn catalog_updates_dir(app: &AppHandle) -> Option<PathBuf> {
    resolve_catalog_updates_dir(data_dir(app).as_deref())
}

fn resolve_catalog_updates_dir(data_dir: Option<&Path>) -> Option<PathBuf> {
    data_dir.map(|data_dir| data_dir.join(CATALOG_UPDATES_DIR_NAME))
}

/// Resolves the bundled catalog resource.
///
/// Tauri installs mapped resources under the resource directory. During
/// development the CLI may not copy them, so a manifest-relative fallback is
/// used in debug builds only.
pub fn catalog_path(app: &AppHandle) -> Option<PathBuf> {
    let resource_dir = app.path().resource_dir().ok();
    resolve_catalog_path(resource_dir.as_deref())
}

fn resolve_catalog_path(resource_dir: Option<&Path>) -> Option<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(resource_dir) = resource_dir {
        candidates.push(resource_dir.join("catalog").join(CATALOG_FILE_NAME));
        candidates.push(
            resource_dir
                .join("resources")
                .join("catalog")
                .join(CATALOG_FILE_NAME),
        );
        candidates.push(resource_dir.join(CATALOG_FILE_NAME));
    }

    #[cfg(debug_assertions)]
    candidates.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("catalog")
            .join(CATALOG_FILE_NAME),
    );

    candidates.into_iter().find(|path| path.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_catalog_updates_directory() {
        let data_dir = Path::new("C:\\Users\\example\\AppData\\Roaming\\com.opendrvr.desktop");
        assert_eq!(
            resolve_catalog_updates_dir(Some(data_dir)),
            Some(data_dir.join(CATALOG_UPDATES_DIR_NAME))
        );
        assert_eq!(resolve_catalog_updates_dir(None), None);
    }

    #[test]
    fn resolves_tauri_resource_layout() {
        let root =
            std::env::temp_dir().join(format!("opendrvr-catalog-path-{}", std::process::id()));
        let catalog_dir = root.join("catalog");
        std::fs::create_dir_all(&catalog_dir).expect("create catalog dir");
        let catalog = catalog_dir.join(CATALOG_FILE_NAME);
        std::fs::write(&catalog, b"fixture").expect("write fixture");

        let resolved = resolve_catalog_path(Some(&root)).expect("resolve catalog");
        assert_eq!(resolved, catalog);

        std::fs::remove_dir_all(&root).ok();
    }

    #[cfg(debug_assertions)]
    #[test]
    fn resolves_development_fallback() {
        let resolved = resolve_catalog_path(None).expect("development catalog");
        assert!(resolved.is_file());
        assert!(resolved.ends_with(
            Path::new("resources")
                .join("catalog")
                .join(CATALOG_FILE_NAME)
        ));
    }
}
