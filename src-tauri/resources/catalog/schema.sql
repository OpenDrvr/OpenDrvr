-- OpenDrvr bundled catalog schema (schema version 2).
--
-- This file is the source used to build resources/catalog/catalog.sqlite3.
-- The shipped database is opened read-only at runtime and must never contain
-- user data. The catalog stores driver metadata and provenance only; driver
-- packages are never bundled.
--
-- `catalog_meta` rows are inserted by the build/import tooling so the Rust
-- constants in `src/catalog/schema.rs` are the single source of truth. Do not
-- add hardware or driver rows without verified source data.

PRAGMA foreign_keys = ON;

CREATE TABLE catalog_meta (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
);

CREATE TABLE manufacturers (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    website_url TEXT,
    support_url TEXT
);

-- One row per concrete PCI, USB, ACPI, or HD Audio hardware identifier.
-- `hardware_id` is stored in canonical uppercase form and is unique so the
-- importer can reject duplicate claims before writing.
CREATE TABLE hardware_devices (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    bus_type TEXT NOT NULL CHECK (bus_type IN ('pci', 'usb', 'acpi', 'hdaudio')),
    hardware_id TEXT NOT NULL UNIQUE CHECK (length(trim(hardware_id)) > 0),
    vendor_id TEXT,
    device_id TEXT,
    subsystem_vendor_id TEXT,
    subsystem_device_id TEXT,
    revision TEXT,
    class_code TEXT,
    name TEXT,
    manufacturer_id TEXT REFERENCES manufacturers (id) ON DELETE SET NULL
);

CREATE INDEX idx_hardware_devices_lookup ON hardware_devices (bus_type, vendor_id, device_id);
CREATE INDEX idx_hardware_devices_vendor ON hardware_devices (vendor_id);
CREATE INDEX idx_hardware_devices_manufacturer ON hardware_devices (manufacturer_id);

-- Only official sources are accepted. `attribution` is shown with catalog
-- results so the user can see where the information came from.
CREATE TABLE driver_sources (
    id TEXT PRIMARY KEY NOT NULL,
    manufacturer_id TEXT REFERENCES manufacturers (id) ON DELETE SET NULL,
    name TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (
        kind IN ('manufacturer', 'windows-update', 'oem', 'component-vendor', 'other')
    ),
    base_url TEXT NOT NULL,
    official INTEGER NOT NULL DEFAULT 1 CHECK (official = 1),
    attribution TEXT NOT NULL,
    last_verified_at TEXT
);

CREATE INDEX idx_driver_sources_manufacturer ON driver_sources (manufacturer_id);

-- Driver metadata only. `provider` is the package provider and is separate from
-- `manufacturer_id`. Direct package URLs are optional because Windows Update
-- packages are identified by their update identity instead of a public file URL.
CREATE TABLE drivers (
    id TEXT PRIMARY KEY NOT NULL,
    manufacturer_id TEXT NOT NULL REFERENCES manufacturers (id) ON DELETE RESTRICT,
    provider TEXT NOT NULL,
    name TEXT NOT NULL,
    version TEXT NOT NULL,
    release_date TEXT,
    description TEXT,
    package_type TEXT NOT NULL CHECK (
        package_type IN ('installer', 'inf', 'cab', 'msi', 'exe', 'zip', 'windows-update', 'other')
    ),
    file_name TEXT,
    download_url TEXT,
    size_bytes INTEGER CHECK (size_bytes IS NULL OR size_bytes >= 0),
    checksum_sha256 TEXT,
    last_verified_at TEXT NOT NULL,
    UNIQUE (manufacturer_id, name, version)
);

CREATE INDEX idx_drivers_manufacturer ON drivers (manufacturer_id);
CREATE INDEX idx_drivers_name ON drivers (name);

CREATE TABLE driver_hardware (
    driver_id TEXT NOT NULL REFERENCES drivers (id) ON DELETE CASCADE,
    hardware_device_id INTEGER NOT NULL REFERENCES hardware_devices (id) ON DELETE CASCADE,
    match_quality TEXT NOT NULL DEFAULT 'exact' CHECK (
        match_quality IN ('exact', 'subsystem', 'compatible')
    ),
    evidence TEXT,
    PRIMARY KEY (driver_id, hardware_device_id)
);

CREATE INDEX idx_driver_hardware_device ON driver_hardware (hardware_device_id);

CREATE TABLE driver_windows_compat (
    driver_id TEXT NOT NULL REFERENCES drivers (id) ON DELETE CASCADE,
    windows_version TEXT NOT NULL CHECK (windows_version IN ('windows-10', 'windows-11')),
    architecture TEXT NOT NULL CHECK (architecture IN ('x86', 'x64', 'arm64')),
    PRIMARY KEY (driver_id, windows_version, architecture)
);

CREATE INDEX idx_driver_windows_compat_version
    ON driver_windows_compat (windows_version, architecture);

-- Per-driver provenance. `retrieved_at` records when the citation was checked
-- and `attribution` records the exact wording to show to the user.
CREATE TABLE driver_source_refs (
    driver_id TEXT NOT NULL REFERENCES drivers (id) ON DELETE CASCADE,
    source_id TEXT NOT NULL REFERENCES driver_sources (id) ON DELETE CASCADE,
    url TEXT NOT NULL,
    is_official INTEGER NOT NULL DEFAULT 1 CHECK (is_official = 1),
    retrieved_at TEXT NOT NULL,
    attribution TEXT NOT NULL,
    PRIMARY KEY (driver_id, source_id, url)
);

CREATE INDEX idx_driver_source_refs_source ON driver_source_refs (source_id);
