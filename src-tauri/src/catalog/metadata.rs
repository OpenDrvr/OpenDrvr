//! Catalog metadata and content versions.
//!
//! `schema_version` describes the SQLite layout. `catalog_version` describes the
//! content snapshot and is used to decide whether an update snapshot is newer
//! than the bundled one. Both are independent so a schema change never implies
//! a content change, and a content refresh never mutates an installed snapshot.

use std::fmt;
use std::str::FromStr;

use crate::catalog::schema::{CATALOG_FORMAT, SCHEMA_VERSION};
use crate::catalog::time::is_rfc3339_timestamp;

/// A `MAJOR.MINOR.PATCH` catalog content version.
///
/// The catalog deliberately uses a simple, ordered version instead of a date so
/// two snapshots can be compared without timezone ambiguity. Build tooling
/// should use a date-derived value such as `2026.11.6` when convenient.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CatalogVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl CatalogVersion {
    pub const ZERO: Self = Self {
        major: 0,
        minor: 0,
        patch: 0,
    };

    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        Self::from_str(value)
    }
}

impl FromStr for CatalogVersion {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let mut parts = value.split('.');
        let (Some(major), Some(minor), Some(patch), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(format!(
                "catalog version '{value}' must use MAJOR.MINOR.PATCH"
            ));
        };

        let major = parse_component("major", major)?;
        let minor = parse_component("minor", minor)?;
        let patch = parse_component("patch", patch)?;

        Ok(Self {
            major,
            minor,
            patch,
        })
    }
}

fn parse_component(name: &str, value: &str) -> Result<u32, String> {
    if value.is_empty()
        || !value.bytes().all(|byte| byte.is_ascii_digit())
        || (value.len() > 1 && value.starts_with('0'))
    {
        return Err(format!(
            "catalog version {name} component '{value}' is not a canonical number"
        ));
    }

    value
        .parse::<u32>()
        .map_err(|_| format!("catalog version {name} component '{value}' is too large"))
}

impl fmt::Display for CatalogVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Metadata stored in `catalog_meta`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogMetadata {
    pub schema_version: i64,
    pub catalog_format: String,
    pub catalog_version: CatalogVersion,
    pub generated_at: String,
    pub source_policy_version: String,
    pub last_verified_at: Option<String>,
}

impl CatalogMetadata {
    /// Parses and validates metadata pairs read from `catalog_meta`.
    pub fn from_pairs<I>(pairs: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = (String, String)>,
    {
        let mut schema_version = None;
        let mut catalog_format = None;
        let mut catalog_version = None;
        let mut generated_at = None;
        let mut source_policy_version = None;
        let mut last_verified_at = None;

        for (key, value) in pairs {
            match key.as_str() {
                "schema_version" => schema_version = Some(value),
                "catalog_format" => catalog_format = Some(value),
                "catalog_version" => catalog_version = Some(value),
                "generated_at" => generated_at = Some(value),
                "source_policy_version" => source_policy_version = Some(value),
                "last_verified_at" => last_verified_at = Some(value),
                _ => {}
            }
        }

        let schema_version = required("schema_version", schema_version)?
            .parse::<i64>()
            .map_err(|_| "catalog metadata schema_version is not an integer".to_string())?;

        if schema_version <= 0 {
            return Err("catalog metadata schema_version must be positive".to_string());
        }

        let catalog_format = required("catalog_format", catalog_format)?;
        if catalog_format.trim().is_empty() {
            return Err("catalog metadata catalog_format is empty".to_string());
        }

        let catalog_version =
            CatalogVersion::parse(&required("catalog_version", catalog_version)?)?;

        let generated_at = required("generated_at", generated_at)?;
        if !is_rfc3339_timestamp(&generated_at) {
            return Err("catalog metadata generated_at is not an RFC 3339 timestamp".to_string());
        }

        let source_policy_version = required("source_policy_version", source_policy_version)?;
        if source_policy_version.trim().is_empty() {
            return Err("catalog metadata source_policy_version is empty".to_string());
        }

        if let Some(value) = &last_verified_at {
            if !is_rfc3339_timestamp(value) {
                return Err(
                    "catalog metadata last_verified_at is not an RFC 3339 timestamp".to_string(),
                );
            }
        }

        Ok(Self {
            schema_version,
            catalog_format,
            catalog_version,
            generated_at,
            source_policy_version,
            last_verified_at,
        })
    }

    /// Checks that this snapshot can be read by the current build.
    pub fn validate_runtime_compatibility(&self) -> Result<(), String> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(format!(
                "unsupported catalog schema version {}; this build reads schema version {}",
                self.schema_version, SCHEMA_VERSION
            ));
        }

        if self.catalog_format != CATALOG_FORMAT {
            return Err(format!(
                "unsupported catalog format '{}'; expected '{}'",
                self.catalog_format, CATALOG_FORMAT
            ));
        }

        Ok(())
    }
}

fn required(name: &str, value: Option<String>) -> Result<String, String> {
    value.ok_or_else(|| format!("catalog metadata is missing {name}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_orders_catalog_versions() {
        assert_eq!(
            CatalogVersion::parse("1.2.3").expect("version"),
            CatalogVersion::new(1, 2, 3)
        );
        assert!(CatalogVersion::parse("1.10.0").expect("version") > CatalogVersion::new(1, 9, 9));
        assert!(CatalogVersion::parse("2.0.0").expect("version") > CatalogVersion::new(1, 99, 99));
        assert_eq!(CatalogVersion::new(1, 2, 3).to_string(), "1.2.3");
    }

    #[test]
    fn rejects_non_canonical_versions() {
        for value in [
            "1.0",
            "1.0.0.0",
            "01.0.0",
            "1.00.0",
            "1.0.0-beta",
            "v1.0.0",
            "",
        ] {
            assert!(CatalogVersion::parse(value).is_err(), "{value}");
        }
    }

    #[test]
    fn reads_metadata_pairs() {
        let metadata = CatalogMetadata::from_pairs([
            ("schema_version".to_string(), "2".to_string()),
            ("catalog_format".to_string(), CATALOG_FORMAT.to_string()),
            ("catalog_version".to_string(), "2026.11.6".to_string()),
            (
                "generated_at".to_string(),
                "2026-11-06T00:00:00Z".to_string(),
            ),
            ("source_policy_version".to_string(), "1".to_string()),
            (
                "last_verified_at".to_string(),
                "2026-11-05T12:00:00Z".to_string(),
            ),
        ])
        .expect("metadata");

        assert_eq!(metadata.schema_version, 2);
        assert_eq!(metadata.catalog_version, CatalogVersion::new(2026, 11, 6));
        assert_eq!(
            metadata.last_verified_at.as_deref(),
            Some("2026-11-05T12:00:00Z")
        );
        metadata
            .validate_runtime_compatibility()
            .expect("compatible");
    }

    #[test]
    fn rejects_missing_or_invalid_metadata() {
        assert!(CatalogMetadata::from_pairs(Vec::<(String, String)>::new()).is_err());

        let base = [
            ("schema_version", "2"),
            ("catalog_format", CATALOG_FORMAT),
            ("catalog_version", "1.0.0"),
            ("generated_at", "2026-11-06T00:00:00Z"),
            ("source_policy_version", "1"),
        ];

        let missing = base
            .iter()
            .filter(|(key, _)| *key != "generated_at")
            .map(|(key, value)| (key.to_string(), value.to_string()));
        assert!(CatalogMetadata::from_pairs(missing).is_err());

        let invalid_date = base
            .iter()
            .map(|(key, value)| {
                (
                    key.to_string(),
                    if *key == "generated_at" {
                        "2026-02-30T00:00:00Z".to_string()
                    } else {
                        value.to_string()
                    },
                )
            })
            .collect::<Vec<_>>();
        assert!(CatalogMetadata::from_pairs(invalid_date).is_err());
    }

    #[test]
    fn rejects_incompatible_runtime_schema_or_format() {
        let metadata = CatalogMetadata {
            schema_version: 1,
            catalog_format: CATALOG_FORMAT.to_string(),
            catalog_version: CatalogVersion::new(1, 0, 0),
            generated_at: "2026-11-06T00:00:00Z".to_string(),
            source_policy_version: "1".to_string(),
            last_verified_at: None,
        };
        assert!(metadata.validate_runtime_compatibility().is_err());

        let metadata = CatalogMetadata {
            schema_version: 2,
            catalog_format: "opendrvr.catalog.v1".to_string(),
            ..metadata
        };
        assert!(metadata.validate_runtime_compatibility().is_err());
    }
}
