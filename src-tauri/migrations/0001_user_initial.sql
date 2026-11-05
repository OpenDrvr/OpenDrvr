-- OpenDrvr user database schema (migration 1).
--
-- This database contains only user-specific data. The bundled catalog lives in
-- a separate read-only SQLite file and is never modified from here.

CREATE TABLE app_preferences (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE hardware_scans (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    started_at TEXT NOT NULL,
    completed_at TEXT,
    status TEXT NOT NULL CHECK (status IN ('running', 'completed', 'failed', 'cancelled')),
    summary_json TEXT,
    error_message TEXT
);

CREATE INDEX idx_hardware_scans_started_at ON hardware_scans (started_at DESC);

CREATE TABLE hardware_scan_devices (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    scan_id INTEGER NOT NULL REFERENCES hardware_scans (id) ON DELETE CASCADE,
    bus_type TEXT NOT NULL CHECK (bus_type IN ('pci', 'usb', 'other')),
    vendor_id TEXT,
    device_id TEXT,
    subsystem_vendor_id TEXT,
    subsystem_device_id TEXT,
    name TEXT,
    details_json TEXT
);

CREATE INDEX idx_hardware_scan_devices_scan ON hardware_scan_devices (scan_id);
CREATE INDEX idx_hardware_scan_devices_ids ON hardware_scan_devices (vendor_id, device_id);

CREATE TABLE download_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    driver_id TEXT,
    source_id TEXT,
    source_url TEXT NOT NULL,
    file_name TEXT,
    status TEXT NOT NULL CHECK (status IN ('queued', 'downloading', 'completed', 'failed', 'cancelled')),
    bytes_downloaded INTEGER NOT NULL DEFAULT 0 CHECK (bytes_downloaded >= 0),
    started_at TEXT NOT NULL,
    completed_at TEXT,
    error_message TEXT
);

CREATE INDEX idx_download_history_started_at ON download_history (started_at DESC);
CREATE INDEX idx_download_history_driver ON download_history (driver_id);

CREATE TABLE cached_driver_metadata (
    cache_key TEXT PRIMARY KEY NOT NULL,
    driver_id TEXT,
    payload_json TEXT NOT NULL,
    fetched_at TEXT NOT NULL,
    expires_at TEXT,
    etag TEXT
);

CREATE INDEX idx_cached_driver_metadata_driver ON cached_driver_metadata (driver_id);
CREATE INDEX idx_cached_driver_metadata_expires ON cached_driver_metadata (expires_at);
