-- OpenDrvr bundled catalog schema (schema version 1).
--
-- This file is the source used to build resources/catalog/catalog.sqlite3.
-- The shipped database is opened read-only at runtime and must never contain
-- user data. Do not add hardware or driver rows without real catalog data.

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

-- One row per PCI or USB hardware identifier. Subsystem IDs are optional
-- because not every bus/device exposes them.
CREATE TABLE hardware_devices (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    bus_type TEXT NOT NULL CHECK (bus_type IN ('pci', 'usb')),
    vendor_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    subsystem_vendor_id TEXT,
    subsystem_device_id TEXT,
    revision TEXT,
    class_code TEXT,
    name TEXT,
    manufacturer_id TEXT REFERENCES manufacturers (id) ON DELETE SET NULL
);

CREATE INDEX idx_hardware_devices_ids ON hardware_devices (bus_type, vendor_id, device_id);
CREATE INDEX idx_hardware_devices_vendor ON hardware_devices (vendor_id);
CREATE INDEX idx_hardware_devices_manufacturer ON hardware_devices (manufacturer_id);

CREATE TABLE driver_sources (
    id TEXT PRIMARY KEY NOT NULL,
    manufacturer_id TEXT REFERENCES manufacturers (id) ON DELETE SET NULL,
    name TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('manufacturer', 'windows-update', 'oem', 'other')),
    base_url TEXT,
    official INTEGER NOT NULL DEFAULT 0 CHECK (official IN (0, 1)),
    last_verified_at TEXT
);

CREATE INDEX idx_driver_sources_manufacturer ON driver_sources (manufacturer_id);

CREATE TABLE drivers (
    id TEXT PRIMARY KEY NOT NULL,
    manufacturer_id TEXT REFERENCES manufacturers (id) ON DELETE SET NULL,
    name TEXT NOT NULL,
    version TEXT,
    release_date TEXT,
    description TEXT,
    file_name TEXT,
    download_url TEXT,
    size_bytes INTEGER CHECK (size_bytes IS NULL OR size_bytes >= 0),
    checksum_sha256 TEXT
);

CREATE INDEX idx_drivers_manufacturer ON drivers (manufacturer_id);
CREATE INDEX idx_drivers_name ON drivers (name);

CREATE TABLE driver_hardware (
    driver_id TEXT NOT NULL REFERENCES drivers (id) ON DELETE CASCADE,
    hardware_device_id INTEGER NOT NULL REFERENCES hardware_devices (id) ON DELETE CASCADE,
    match_quality TEXT NOT NULL DEFAULT 'exact' CHECK (match_quality IN ('exact', 'subsystem', 'compatible')),
    PRIMARY KEY (driver_id, hardware_device_id)
);

CREATE INDEX idx_driver_hardware_device ON driver_hardware (hardware_device_id);

CREATE TABLE driver_windows_compat (
    driver_id TEXT NOT NULL REFERENCES drivers (id) ON DELETE CASCADE,
    windows_version TEXT NOT NULL,
    architecture TEXT NOT NULL CHECK (architecture IN ('x86', 'x64', 'arm64')),
    PRIMARY KEY (driver_id, windows_version, architecture)
);

CREATE INDEX idx_driver_windows_compat_version ON driver_windows_compat (windows_version, architecture);

CREATE TABLE driver_source_refs (
    driver_id TEXT NOT NULL REFERENCES drivers (id) ON DELETE CASCADE,
    source_id TEXT NOT NULL REFERENCES driver_sources (id) ON DELETE CASCADE,
    url TEXT NOT NULL,
    is_official INTEGER NOT NULL DEFAULT 1 CHECK (is_official IN (0, 1)),
    PRIMARY KEY (driver_id, source_id, url)
);

CREATE INDEX idx_driver_source_refs_source ON driver_source_refs (source_id);

INSERT INTO catalog_meta (key, value) VALUES
    ('schema_version', '1'),
    ('catalog_format', 'opendrvr.catalog.v1');
