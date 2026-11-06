//! Typed application preferences.
//!
//! Preferences are stored as individual keys in the writable user database so
//! they remain inspectable and can evolve one field at a time. This module owns
//! the defaults, the key mapping, validation, and the safe conversion between
//! stored strings and the typed IPC payload.

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::utils;

pub const THEME_KEY: &str = "ui.theme";
pub const UPDATE_CHANNEL_KEY: &str = "updates.channel";
pub const DOWNLOAD_DIRECTORY_KEY: &str = "downloads.directory";
pub const CHECK_CATALOG_UPDATES_KEY: &str = "updates.check_on_launch";
pub const NOTIFY_DOWNLOADS_KEY: &str = "downloads.notify_on_complete";
pub const KEEP_DOWNLOAD_HISTORY_KEY: &str = "downloads.keep_history";

/// Every key this build reads or writes. Reset removes exactly these keys.
pub const PREFERENCE_KEYS: [&str; 6] = [
    THEME_KEY,
    UPDATE_CHANNEL_KEY,
    DOWNLOAD_DIRECTORY_KEY,
    CHECK_CATALOG_UPDATES_KEY,
    NOTIFY_DOWNLOADS_KEY,
    KEEP_DOWNLOAD_HISTORY_KEY,
];

pub const MAX_DOWNLOAD_DIRECTORY_CHARS: usize = 260;

/// Supported interface themes. OpenDrvr is dark-first; additional themes will be
/// added only when they are fully implemented.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThemePreference {
    #[default]
    Dark,
}

impl ThemePreference {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Dark => "dark",
        }
    }
}

/// Update channel. Only the stable channel exists in this build.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum UpdateChannel {
    #[default]
    Stable,
}

impl UpdateChannel {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
        }
    }
}

/// Persisted user preferences exchanged with the frontend.
///
/// Serde defaults apply to missing fields, which keeps older databases readable
/// as the preference set grows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppPreferences {
    pub theme: ThemePreference,
    pub update_channel: UpdateChannel,
    pub download_directory: String,
    pub check_catalog_updates: bool,
    pub notify_downloads: bool,
    pub keep_download_history: bool,
}

impl Default for AppPreferences {
    fn default() -> Self {
        Self {
            theme: ThemePreference::Dark,
            update_channel: UpdateChannel::Stable,
            download_directory: "Downloads".to_string(),
            check_catalog_updates: true,
            notify_downloads: true,
            keep_download_history: true,
        }
    }
}

impl AppPreferences {
    /// Trims user-entered values before validation and persistence.
    pub fn normalized(mut self) -> Self {
        self.download_directory = self.download_directory.trim().to_string();
        self
    }

    pub fn validate(&self) -> AppResult<()> {
        validate_download_directory(&self.download_directory)
    }

    /// Flattens the typed preferences into database rows.
    pub fn to_entries(&self) -> Vec<(&'static str, String)> {
        vec![
            (THEME_KEY, self.theme.as_str().to_string()),
            (UPDATE_CHANNEL_KEY, self.update_channel.as_str().to_string()),
            (DOWNLOAD_DIRECTORY_KEY, self.download_directory.clone()),
            (
                CHECK_CATALOG_UPDATES_KEY,
                self.check_catalog_updates.to_string(),
            ),
            (NOTIFY_DOWNLOADS_KEY, self.notify_downloads.to_string()),
            (
                KEEP_DOWNLOAD_HISTORY_KEY,
                self.keep_download_history.to_string(),
            ),
        ]
    }

    /// Builds preferences from stored rows.
    ///
    /// Missing rows use defaults. Unknown or malformed values are ignored
    /// individually, so one bad value cannot discard every other preference.
    pub fn from_stored(entries: impl IntoIterator<Item = (String, String)>) -> Self {
        let mut preferences = Self::default();

        for (key, value) in entries {
            match key.as_str() {
                THEME_KEY => match value.as_str() {
                    "dark" => preferences.theme = ThemePreference::Dark,
                    _ => warn_unsupported(&key),
                },
                UPDATE_CHANNEL_KEY => match value.as_str() {
                    "stable" => preferences.update_channel = UpdateChannel::Stable,
                    _ => warn_unsupported(&key),
                },
                DOWNLOAD_DIRECTORY_KEY => {
                    if validate_download_directory(&value).is_ok() {
                        preferences.download_directory = value;
                    } else {
                        warn_unsupported(&key);
                    }
                }
                CHECK_CATALOG_UPDATES_KEY => {
                    if let Some(parsed) = parse_bool(&value) {
                        preferences.check_catalog_updates = parsed;
                    } else {
                        warn_unsupported(&key);
                    }
                }
                NOTIFY_DOWNLOADS_KEY => {
                    if let Some(parsed) = parse_bool(&value) {
                        preferences.notify_downloads = parsed;
                    } else {
                        warn_unsupported(&key);
                    }
                }
                KEEP_DOWNLOAD_HISTORY_KEY => {
                    if let Some(parsed) = parse_bool(&value) {
                        preferences.keep_download_history = parsed;
                    } else {
                        warn_unsupported(&key);
                    }
                }
                _ => {}
            }
        }

        preferences
    }
}

fn validate_download_directory(value: &str) -> AppResult<()> {
    let value = value.trim();

    if value.is_empty() {
        return Err(AppError::Validation(
            "download directory must not be empty".to_string(),
        ));
    }

    utils::validate_user_text(value, MAX_DOWNLOAD_DIRECTORY_CHARS)?;

    if value.chars().any(|character| character.is_control()) {
        return Err(AppError::Validation(
            "download directory must be a single line".to_string(),
        ));
    }

    Ok(())
}

fn parse_bool(value: &str) -> Option<bool> {
    match value {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn warn_unsupported(key: &str) {
    tracing::warn!(key = %key, "ignoring unsupported stored preference value");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_valid_and_round_trip() {
        let preferences = AppPreferences::default();
        assert!(preferences.validate().is_ok());

        let entries = preferences
            .to_entries()
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect::<Vec<_>>();
        assert_eq!(AppPreferences::from_stored(entries), preferences);
    }

    #[test]
    fn missing_values_use_defaults() {
        assert_eq!(
            AppPreferences::from_stored(Vec::new()),
            AppPreferences::default()
        );
    }

    #[test]
    fn invalid_values_fall_back_per_field() {
        let preferences = AppPreferences::from_stored(vec![
            (THEME_KEY.to_string(), "neon".to_string()),
            (UPDATE_CHANNEL_KEY.to_string(), "nightly".to_string()),
            (DOWNLOAD_DIRECTORY_KEY.to_string(), "   ".to_string()),
            (CHECK_CATALOG_UPDATES_KEY.to_string(), "yes".to_string()),
            (NOTIFY_DOWNLOADS_KEY.to_string(), "false".to_string()),
            (KEEP_DOWNLOAD_HISTORY_KEY.to_string(), "false".to_string()),
        ]);

        assert_eq!(preferences.theme, ThemePreference::Dark);
        assert_eq!(preferences.update_channel, UpdateChannel::Stable);
        assert_eq!(preferences.download_directory, "Downloads");
        assert!(preferences.check_catalog_updates);
        assert!(!preferences.notify_downloads);
        assert!(!preferences.keep_download_history);
    }

    #[test]
    fn rejects_empty_or_oversized_download_directory() {
        let empty = AppPreferences {
            download_directory: "  ".to_string(),
            ..AppPreferences::default()
        };
        assert!(empty.validate().is_err());

        let oversized = AppPreferences {
            download_directory: "a".repeat(MAX_DOWNLOAD_DIRECTORY_CHARS + 1),
            ..AppPreferences::default()
        };
        assert!(oversized.validate().is_err());
    }

    #[test]
    fn rejects_control_characters_in_download_directory() {
        let preferences = AppPreferences {
            download_directory: "Downloads\nOther".to_string(),
            ..AppPreferences::default()
        };
        assert!(preferences.validate().is_err());
    }

    #[test]
    fn normalization_trims_the_download_directory() {
        let preferences = AppPreferences {
            download_directory: "  D:\\OpenDrvr  ".to_string(),
            ..AppPreferences::default()
        }
        .normalized();

        assert_eq!(preferences.download_directory, "D:\\OpenDrvr");
    }

    #[test]
    fn serde_uses_camel_case_fields() {
        let value = serde_json::to_value(AppPreferences::default()).expect("serialize");
        assert_eq!(value["updateChannel"], "stable");
        assert_eq!(value["downloadDirectory"], "Downloads");
        assert_eq!(value["checkCatalogUpdates"], true);
    }
}
