# Driver catalog

OpenDrvr ships a **metadata catalog**, not driver packages. The catalog describes
hardware identifiers, manufacturers, driver package metadata, Windows
compatibility, and the official source that supports each claim. Driver binaries
are never bundled, downloaded by the catalog pipeline, or stored in SQLite.

This document describes schema version **2**, the import format, the official
source policy, and the update workflow. The bundled snapshot currently contains
schema and metadata only (`catalog_version` `0.0.0`); real records are added only
after they can be verified against an authoritative source.

## Principles

1. **Official sources only.** A record is accepted only when it cites an HTTPS
   URL on a manufacturer-controlled domain. Search engines, forums, and
   third-party driver aggregators are not authoritative, even when they mirror
   an official file.
2. **No fabricated data.** Missing metadata is rejected or reported as a warning;
   it is never invented. Compatibility claims require an explicit hardware ID
   and an official source.
3. **Immutable snapshots.** Updates are complete, versioned SQLite files. The
   runtime never mutates an installed snapshot.
4. **Strict separation.** Catalog snapshots never share a file, transaction, or
   directory with the writable user database. Catalog updates cannot overwrite
   preferences, scan history, download history, or cached metadata.
5. **Metadata only.** The catalog does not contain executables, INF payloads,
   checksums for unverified files, or download URLs that have not been checked
   against the source policy.

## Storage and lifecycle

| Component       | Location                                   | Access     |
| --------------- | ------------------------------------------ | ---------- |
| Bundled catalog | Tauri resource `catalog/catalog.sqlite3`   | Read-only  |
| Catalog updates | `app_data_dir()/catalog-updates/*.sqlite3` | Read-only  |
| User database   | `app_data_dir()/opendrvr.sqlite3`          | Read/write |

The catalog pool is opened with SQLite's `read_only(true)` and
`create_if_missing(false)` flags. Write attempts fail at the database layer.

At startup:

1. The bundled snapshot is opened and validated.
2. Every `.sqlite3` file in the update directory is opened read-only and
   validated independently.
3. The valid snapshot with the highest `catalog_version` wins.
4. Ties keep the bundled snapshot because shipped content is the trusted
   baseline.
5. Invalid, unreadable, or incompatible update files are logged and ignored.
6. If no snapshot is readable, the catalog component is reported as unavailable;
   the rest of the application continues to start.

Removing an update file rolls the application back to the bundled snapshot on
the next launch. No user data is touched.

### Snapshot validation

Before a snapshot is exposed, the runtime verifies:

- `catalog_meta` parses as complete metadata.
- `schema_version` matches the build's schema version.
- `catalog_format` is `opendrvr.catalog.v2`.
- Every required table exists. Extra tables are tolerated so an additive format
  can still be read.
- `PRAGMA integrity_check` returns `ok`.
- `PRAGMA foreign_key_check` reports no violations.

## Versions

Two versions are independent:

| Field             | Meaning                                       | Example     |
| ----------------- | --------------------------------------------- | ----------- |
| `schema_version`  | SQLite table layout understood by the build   | `2`         |
| `catalog_version` | Content snapshot version, `MAJOR.MINOR.PATCH` | `2026.11.6` |

A schema change does not imply a content change, and a content refresh does not
change the schema. `catalog_version` must be canonical: decimal components with
no leading zeros and no pre-release suffix.

The schema-only bundled snapshot uses:

- `catalog_version`: `0.0.0`
- `generated_at`: `1970-01-01T00:00:00Z`
- `last_verified_at`: absent

Any document that contains records must use a version greater than `0.0.0`.

## Schema

All tables live in the read-only catalog file. `resources/catalog/schema.sql` is
the source of truth; `catalog/schema.rs` embeds the same DDL for the import tool.

### `catalog_meta`

Key/value metadata. Required keys:

- `schema_version`
- `catalog_format` (`opendrvr.catalog.v2`)
- `catalog_version`
- `generated_at` (RFC 3339 with offset)
- `source_policy_version`

Optional:

- `last_verified_at` (RFC 3339 with offset)

### `manufacturers`

Stable manufacturer identity.

| Column        | Notes                       |
| ------------- | --------------------------- |
| `id`          | Lowercase slug, primary key |
| `name`        | Display name                |
| `website_url` | Optional official HTTPS URL |
| `support_url` | Optional official HTTPS URL |

### `hardware_devices`

One row per concrete hardware identifier. `hardware_id` is canonical uppercase
and unique.

| Column                                       | Notes                                               |
| -------------------------------------------- | --------------------------------------------------- |
| `bus_type`                                   | `pci`, `usb`, `acpi`, or `hdaudio`                  |
| `hardware_id`                                | Full Windows hardware ID without an instance suffix |
| `vendor_id`, `device_id`                     | Parsed identity when available                      |
| `subsystem_vendor_id`, `subsystem_device_id` | Optional subsystem identity                         |
| `revision`, `class_code`                     | Optional parsed fields                              |
| `name`                                       | Optional descriptive name                           |
| `manufacturer_id`                            | Optional reference to `manufacturers.id`            |

Class-only identifiers such as `USB\CLASS_03&SUBCLASS_01&PROT_02` are rejected:
they are too broad to justify a per-driver compatibility claim.

### `driver_sources`

A source describes where driver information comes from.

| Column             | Notes                                                                |
| ------------------ | -------------------------------------------------------------------- |
| `id`               | Lowercase slug, primary key                                          |
| `manufacturer_id`  | Optional reference to `manufacturers.id`                             |
| `name`             | Display name                                                         |
| `kind`             | `manufacturer`, `windows-update`, `oem`, `component-vendor`, `other` |
| `base_url`         | Official HTTPS URL                                                   |
| `official`         | Must be `1`; non-official sources cannot be imported                 |
| `attribution`      | Human-readable credit shown with results                             |
| `last_verified_at` | Optional RFC 3339 timestamp                                          |

### `drivers`

Driver package metadata. `provider` is the package provider and is separate from
the device or system manufacturer.

| Column             | Notes                                                                     |
| ------------------ | ------------------------------------------------------------------------- |
| `id`               | Lowercase slug, primary key                                               |
| `manufacturer_id`  | Required reference to `manufacturers.id`                                  |
| `provider`         | Driver provider reported by the package                                   |
| `name`             | Driver display name                                                       |
| `version`          | Driver version string                                                     |
| `release_date`     | Optional `YYYY-MM-DD`; never inferred from the version                    |
| `description`      | Optional text                                                             |
| `package_type`     | `installer`, `inf`, `cab`, `msi`, `exe`, `zip`, `windows-update`, `other` |
| `file_name`        | Optional plain file name, no path separators                              |
| `download_url`     | Optional official HTTPS URL                                               |
| `size_bytes`       | Optional non-negative size                                                |
| `checksum_sha256`  | Optional 64-character hexadecimal SHA-256                                 |
| `last_verified_at` | Required RFC 3339 timestamp                                               |

`(manufacturer_id, name, version)` is unique.

### Relationship tables

- `driver_hardware` links a driver to a hardware device with `match_quality`
  (`exact`, `subsystem`, `compatible`) and optional evidence.
- `driver_windows_compat` records supported `windows_version` (`windows-10`,
  `windows-11`) and `architecture` (`x86`, `x64`, `arm64`).
- `driver_source_refs` records the exact URL, retrieval timestamp, and
  attribution for each driver citation. `is_official` must be `1`.

## Import document

The import tool consumes one JSON document. Field names are camelCase and unknown
fields are rejected. An empty document is valid only for the schema-only
`0.0.0` snapshot:

```json
{
  "catalogVersion": "0.0.0",
  "generatedAt": "1970-01-01T00:00:00Z",
  "sourcePolicyVersion": "1",
  "manufacturers": [],
  "hardwareDevices": [],
  "driverSources": [],
  "drivers": []
}
```

### Top level

| Field                 | Required | Notes                                          |
| --------------------- | -------- | ---------------------------------------------- |
| `catalogVersion`      | Yes      | Canonical `MAJOR.MINOR.PATCH`                  |
| `generatedAt`         | Yes      | RFC 3339 with offset                           |
| `sourcePolicyVersion` | Yes      | Must match the importer's policy version (`1`) |
| `lastVerifiedAt`      | No       | RFC 3339 with offset                           |
| `manufacturers`       | No       | Array of manufacturer records                  |
| `hardwareDevices`     | No       | Array of hardware device records               |
| `driverSources`       | No       | Array of source records                        |
| `drivers`             | No       | Array of driver records                        |

### Manufacturer

| Field        | Required | Notes              |
| ------------ | -------- | ------------------ |
| `id`         | Yes      | Lowercase slug     |
| `name`       | Yes      | 1–200 characters   |
| `websiteUrl` | No       | Official HTTPS URL |
| `supportUrl` | No       | Official HTTPS URL |

### Hardware device

| Field            | Required | Notes                                         |
| ---------------- | -------- | --------------------------------------------- |
| `hardwareId`     | Yes      | Concrete PCI, USB, ACPI, or HDAUDIO ID        |
| `name`           | No       | 1–300 characters                              |
| `manufacturerId` | No       | Must reference a manufacturer in the document |

### Driver source

| Field            | Required | Notes                                         |
| ---------------- | -------- | --------------------------------------------- |
| `id`             | Yes      | Lowercase slug                                |
| `manufacturerId` | No       | Must reference a manufacturer in the document |
| `name`           | Yes      | 1–200 characters                              |
| `kind`           | Yes      | Source kind enum                              |
| `baseUrl`        | Yes      | Official HTTPS URL                            |
| `official`       | No       | Defaults to `true`; `false` is rejected       |
| `attribution`    | Yes      | 1–500 characters                              |
| `lastVerifiedAt` | No       | RFC 3339 with offset                          |

### Driver

| Field            | Required | Notes                                         |
| ---------------- | -------- | --------------------------------------------- |
| `id`             | Yes      | Lowercase slug                                |
| `manufacturerId` | Yes      | Must reference a manufacturer in the document |
| `provider`       | Yes      | 1–200 characters                              |
| `name`           | Yes      | 1–300 characters                              |
| `version`        | Yes      | 1–128 characters                              |
| `releaseDate`    | No       | `YYYY-MM-DD`; missing is a warning            |
| `description`    | No       | Up to 4000 characters                         |
| `packageType`    | Yes      | Package type enum                             |
| `fileName`       | No       | Plain file name                               |
| `downloadUrl`    | No       | Official HTTPS URL                            |
| `sizeBytes`      | No       | Non-negative                                  |
| `checksumSha256` | No       | 64 hexadecimal characters                     |
| `lastVerifiedAt` | Yes      | RFC 3339 with offset                          |
| `hardware`       | Yes      | At least one entry                            |
| `windows`        | Yes      | At least one entry                            |
| `sources`        | Yes      | At least one entry                            |

### Driver hardware match

| Field          | Required | Notes                                   |
| -------------- | -------- | --------------------------------------- |
| `hardwareId`   | Yes      | Must already exist in `hardwareDevices` |
| `matchQuality` | Yes      | `exact`, `subsystem`, or `compatible`   |
| `evidence`     | No       | Up to 1000 characters                   |

A `subsystem` match against an ID without subsystem fields produces a warning.

### Windows compatibility

| Field            | Required | Notes                        |
| ---------------- | -------- | ---------------------------- |
| `windowsVersion` | Yes      | `windows-10` or `windows-11` |
| `architecture`   | Yes      | `x86`, `x64`, or `arm64`     |

### Driver source reference

| Field         | Required | Notes                                   |
| ------------- | -------- | --------------------------------------- |
| `sourceId`    | Yes      | Must reference a source in the document |
| `url`         | Yes      | Official HTTPS URL                      |
| `retrievedAt` | Yes      | RFC 3339 with offset                    |
| `attribution` | Yes      | 1–500 characters                        |

## Validation rules

Validation reports every issue it can find. Any error prevents output creation.

- Metadata fields must be present and well formed.
- `sourcePolicyVersion` must match the importer's policy version.
- IDs are lowercase slugs: letters, digits, `.`, `_`, or `-`; no leading or
  trailing separator and no repeated separators.
- Duplicate manufacturer IDs, source IDs, driver IDs, hardware IDs, source
  references, and `(manufacturer, name, version)` combinations are rejected.
  Hardware ID comparison is case-insensitive.
- Hardware IDs are trimmed and uppercased. They must contain exactly one
  enumerator prefix and no instance suffix, and must parse as PCI, USB, ACPI, or
  HDAUDIO with the required vendor/device fields.
- Every driver must list at least one known hardware ID, one Windows/architecture
  pair, and one official source reference.
- A missing `releaseDate` is a warning. A malformed date is an error.
- URLs must use HTTPS, contain a DNS name rather than an IP literal, avoid
  credentials, and match the official domain policy.
- `fileName` must be a plain file name without path separators, reserved
  characters, or a trailing dot/space.
- `sizeBytes` must not be negative.
- `checksumSha256` must be exactly 64 hexadecimal characters.
- A document containing records must not use `catalog_version` `0.0.0`.

## Official source policy

Accepted source families include:

- Microsoft Update Catalog and Microsoft-controlled update domains.
- Manufacturer download centers and support portals, such as Intel, AMD, NVIDIA,
  Realtek, Dell, HP, Lenovo, ASUS, MSI, Gigabyte, and similar publishers.
- Component vendor sites for chipsets, network, audio, storage, and input
  controllers.

Not accepted:

- Third-party driver aggregators and "driver updater" sites.
- Search engines, forums, social posts, and mirrors.
- HTTP URLs, IP literals, URL shorteners, and links containing credentials.
- A manufacturer domain lookalike such as `intel.com.example` or `notintel.com`.

The curated allowlist lives in `catalog/source_policy.rs`. Adding a domain is a
deliberate policy change: verify that the manufacturer controls the domain before
adding it. A source reference must include attribution text that identifies the
publisher and the page or document being cited.

## Import workflow

Run from `src-tauri`:

```powershell
# Validate only; no file is created
cargo run --example import_catalog -- --input catalog.json --validate-only

# Write a standalone snapshot
cargo run --example import_catalog -- --input catalog.json --output catalog.sqlite3

# Install a versioned snapshot into the per-user update directory
cargo run --example import_catalog -- --input catalog.json --updates-dir "$env:APPDATA\com.opendrvr.desktop\catalog-updates"
```

Options:

- `--output <path>` writes to an explicit path.
- `--updates-dir <dir>` writes `catalog-<catalogVersion>.sqlite3` into the
  directory and creates it if needed.
- `--validate-only` validates and reports counts without writing.
- `--force` replaces an existing output file. Without it, the tool refuses to
  overwrite.
- `--help` prints usage.

The writer:

1. Validates the complete document.
2. Normalizes and sorts records.
3. Creates a temporary SQLite file next to the output.
4. Applies the schema and inserts rows in one transaction.
5. Runs integrity and foreign-key checks.
6. Verifies expected row counts.
7. Atomically moves the finished file into place.

A validation failure creates no output. A failed build removes its temporary file.

## Update and rollback

To publish an update:

1. Import a document with a `catalogVersion` greater than the current snapshot.
2. Place the resulting `.sqlite3` file in the per-user update directory.
3. Restart OpenDrvr. The runtime selects the highest valid snapshot.
4. The Settings and About views report the selected origin and catalog version.

To roll back, remove the update file and restart. The bundled snapshot is used
again. The update directory is separate from `opendrvr.sqlite3`; none of these
operations touch preferences, scan history, download history, or cached metadata.

Never edit the bundled `catalog.sqlite3` in place. Regenerate it with the build
tool, or ship a new versioned resource.

## Regenerating the bundled snapshot

After changing the schema:

```powershell
cd src-tauri
cargo run --example build_catalog
```

This writes a schema-only snapshot with deterministic metadata. The file is
bundled through `tauri.conf.json` as `catalog/catalog.sqlite3`.

## Verification

The catalog tests cover:

- Metadata parsing, canonical versions, and runtime compatibility.
- RFC 3339 and calendar-date validation.
- Official URL and domain policy, including lookalike and credential cases.
- Hardware ID normalization and rejection of class-only identifiers.
- Duplicate detection and cross-reference validation.
- Atomic snapshot creation, read-only reopening, and integrity checks.
- Bundled catalog health, version, origin, and empty record counts.
- Selection of the highest valid update, invalid-update fallback, and
  bundled-preferred ties.
- Separation of catalog updates from user preferences.

Run the repository checks from the project root:

```powershell
pnpm check:all
pnpm build
```

## Out of scope

- Downloading, verifying, or installing driver packages.
- Scraping manufacturer sites at runtime.
- Accepting third-party aggregator data.
- Inventing records, URLs, versions, release dates, or compatibility claims.
- Matching detected hardware against the catalog (a later phase).
