use serde::Serialize;
use tauri::AppHandle;

use crate::platform;

/// Safe application metadata used by the frontend to confirm the Rust backend
/// is reachable. No paths or internal configuration are exposed.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub platform: String,
    pub architecture: String,
    pub profile: &'static str,
    pub backend: &'static str,
}

#[tauri::command]
pub fn get_app_info(app: AppHandle) -> AppInfo {
    let package = app.package_info();

    AppInfo {
        name: package.name.clone(),
        version: package.version.to_string(),
        platform: platform::name().to_string(),
        architecture: std::env::consts::ARCH.to_string(),
        profile: if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        },
        backend: "Tauri 2 / Rust",
    }
}
