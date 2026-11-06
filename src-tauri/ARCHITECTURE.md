# OpenDrvr backend architecture

This document describes the Rust backend as of **Phase 11**. The structure is
intentionally conservative: it reserves clear module boundaries without adding
abstractions that the application does not need yet. Phase 10 builds identifier
parsing, vendor resolution, and device classification on top of the Windows Plug
and Play enumerator exposed through `scan_hardware`. Phase 11 adds a versioned,
read-only driver catalog pipeline with strict import validation and provenance;
it still contains metadata only. Driver matching, manufacturer integrations,
downloads, and installations remain **not** implemented. See
[`HARDWARE_INTEGRATION.md`](HARDWARE_INTEGRATION.md) for the API assessment and
the catalog-matching boundary, and [`CATALOG.md`](CATALOG.md) for the catalog
schema, source policy, and import/update workflow.

## Layout

```text
src-tauri/src/
├── main.rs              Windows entry point (release builds hide the console)
├── lib.rs               Tauri builder, startup sequence, state management
├── config.rs            Typed configuration, validation, load/save
├── error.rs             AppError, CommandError, result aliases
├── logging.rs           tracing subscriber initialization
├── preferences.rs       Typed preferences, defaults, validation, key mapping
├── state.rs             AppState and module status types
├── commands/
│   ├── mod.rs           Command exports and the single registration macro
│   ├── app.rs           get_app_info
│   ├── database.rs      get_database_health
│   ├── hardware.rs      scan_hardware
│   ├── preferences.rs   get_preferences, update_preferences, reset_preferences
│   └── system.rs        get_backend_status
├── hardware/
│   ├── mod.rs           Hardware service and provider injection
│   ├── id.rs            Device instance IDs, bus types, PCI/USB/ACPI/HDAUDIO identifiers
│   ├── identify.rs      Identifier-based identity, confidence, and conflict notes
│   ├── vendors.rs       Curated PCI/USB/ACPI/HDAUDIO vendor tables
│   ├── status.rs        Device state, condition, and CM_PROB mapping
│   ├── category.rs      Conservative device categorization
│   ├── model.rs         Portable device, driver, location, snapshot models
│   └── provider.rs      DeviceEnumerator trait and typed errors
├── catalog/
│   ├── mod.rs           Catalog domain exports
│   ├── schema.rs        Schema version, table contract, embedded DDL
│   ├── metadata.rs      Catalog content versions and metadata validation
│   ├── model.rs         Import document model and enums
│   ├── source_policy.rs Official HTTPS domain policy
│   ├── time.rs          RFC 3339 and calendar-date validation
│   ├── validate.rs      Cross-reference, duplicate, and provenance validation
│   └── import.rs        Immutable snapshot builder
├── drivers/mod.rs       Driver matching boundary (planned)
├── sources/mod.rs       Official manufacturer sources boundary (planned)
├── database/
│   ├── mod.rs           Database service, startup degradation, health, preferences
│   ├── user.rs          Writable user database, migrations, recovery, preference I/O
│   ├── catalog.rs       Read-only catalog snapshots and update resolution
│   ├── models.rs        IPC-facing health DTOs
│   └── test_support.rs  Temporary directories for tests
├── downloads/mod.rs     Download orchestration boundary (planned)
├── platform/
│   ├── mod.rs           Portable platform helpers and device-enumerator factory
│   ├── windows.rs       Windows integration entry point
│   └── windows/
│       └── device_tree.rs  CfgMgr32 enumeration, properties, and status
└── utils/mod.rs         Small shared helpers such as input validation
```

Adjacent to `src/`:

```text
src-tauri/
├── migrations/0001_user_initial.sql       Versioned user-database schema
├── migrations/0002_user_configuration.sql Structured configuration table
├── resources/catalog/schema.sql           Catalog schema source
├── resources/catalog/catalog.sqlite3      Generated read-only snapshot
├── examples/build_catalog.rs              Schema-only snapshot regeneration tool
├── examples/import_catalog.rs             Validated import and update writer
├── CATALOG.md                              Catalog source policy and workflow
└── HARDWARE_INTEGRATION.md                 Windows API assessment and hardware design
```

Each directory is a Rust module with its own `mod.rs`. Keeping a directory
instead of a single file leaves room for submodules (for example
`platform/windows/wmi.rs` or `platform/windows/setupapi.rs`) without
restructuring the crate later.

## Request flow

```text
React invokeCommand("get_preferences")
        │
        ▼
commands::preferences::get_preferences    thin adapter, no business logic
        │
        ▼
AppState / Database service               owns application logic
        │
        ▼
AppError                                  internal, detailed error
        │
        ▼
CommandError                              sanitized serializable payload
        │
        ▼
React rejects with { code, message }
```

Commands are deliberately thin adapters. They call a service from `AppState`
and convert errors. Domain modules never import Tauri types, so they can be
unit-tested without a running application. Input validation for preference
values lives in `preferences.rs`, next to the defaults and key mapping, so the
rules cannot drift between the command and the database layer.

## Frontend contract

The React shell consumes the backend through typed wrappers in
`src/lib/tauri.ts`. Every call goes through `invokeCommand`, which checks for
the Tauri runtime and converts Rust's `{ code, message }` rejection payload into
an `IpcError`. There is one place to change IPC behavior.

On startup, `useBackendSnapshot` loads `get_app_info`, `get_backend_status`, and
`get_database_health` in parallel and exposes a single snapshot to the sidebar,
Overview, Settings, and About views. Refreshes repeat the same read-only calls.

The Settings page uses `usePreferences`, which exposes a small loading/ready/
error state machine plus `pendingAction`. Saving sends the complete typed
preference set to `update_preferences`; Rust trims, validates, writes the set in
one transaction, and returns the canonical stored value. The UI adopts that
returned value instead of assuming its draft was accepted. A ref guard prevents
overlapping writes, and the page tracks a local draft with explicit Save,
Discard, and Restore defaults actions.

The frontend never receives a SQL connection or SQL text. It sees only the IPC
DTOs defined in `commands/` and `database/models.rs`, which use camelCase Serde
fields. The Hardware page calls `scan_hardware` through `useHardwareScan`, which
exposes loading, ready, and error states and guards against an older scan
overwriting a newer refresh. The page renders only snapshot values, with search,
category and condition filters, summary counts, and a device detail panel.
Driver and download pages remain planned. They do not synthesize device or
transfer data.

The desktop shell uses hash navigation and a custom component library rather
than a router or a UI framework. This keeps the webview dependency surface small
and makes the IPC boundary the only coupling between React and Rust.

## Decisions

### Single instance

`tauri-plugin-single-instance` is registered before the rest of the builder. A
second launch exits immediately and focuses the existing window instead of
opening the same user database, running migrations, or competing for recovery
against the first process. WAL and the busy timeout remain a safety net, not the
primary concurrency control.

### Centralized immutable state

`AppState` is constructed once during Tauri `setup` and registered with
`app.manage`. It owns the configuration and one service per domain. The state is
immutable after startup, so commands do not need a global `Mutex`. When a
service needs mutable data (a download queue, a hardware cache), it should own
its own synchronization primitive rather than making the entire state mutable.

### Typed errors, sanitized IPC boundary

Internal code returns `AppResult<T>` with `AppError`. Variants distinguish
configuration, I/O, serialization, validation, database, hardware, and internal
failures. Detailed messages may contain paths or OS error strings, so they are
logged with `tracing` and never sent to the webview.

Tauri commands return `CommandResult<T>`. `CommandError` contains only a stable
machine-readable `code` and a fixed public `message`. This gives the frontend
predictable handling without leaking implementation details.

### Configuration

Configuration is loaded from `config.json` in Tauri's per-user app config
directory (`%APPDATA%\com.opendrvr.desktop` on Windows, based on the bundle
identifier). A missing file is normal and yields defaults. A malformed or
invalid file aborts startup; silently ignoring bad configuration would make
future driver behavior unpredictable.

`AppConfig::save` validates before writing so an invalid configuration cannot be
persisted. Paths are never accepted from the frontend. The database setting is a
file name only, not a path.

Example:

```json
{
  "logging": { "level": "info", "format": "pretty" },
  "database": { "fileName": "opendrvr.sqlite3" },
  "downloads": { "maxConcurrent": 3, "timeoutSeconds": 60 }
}
```

`RUST_LOG` overrides the configured log level, which is useful during
development.

### Application preferences

Preferences are typed in `preferences.rs` and stored as individual keys in the
`app_preferences` table. Individual keys keep the data inspectable and let the
schema grow one field at a time without a migration for every UI toggle.

| Key                            | Default     | Validation                             |
| ------------------------------ | ----------- | -------------------------------------- |
| `ui.theme`                     | `dark`      | Only `dark` is implemented             |
| `updates.channel`              | `stable`    | Only `stable` is implemented           |
| `downloads.directory`          | `Downloads` | Trimmed, 1–260 characters, single line |
| `updates.check_on_launch`      | `true`      | Boolean                                |
| `downloads.notify_on_complete` | `true`      | Boolean                                |
| `downloads.keep_history`       | `true`      | Boolean                                |

Rust owns the defaults, key mapping, validation, and normalization. Reads apply
defaults per field, so one malformed stored value cannot discard every other
preference. Writes normalize and validate first, then update every owned key in
one transaction. Reset deletes exactly the keys this build owns, which returns
the next read to application defaults. Unknown stored keys are ignored, and
unknown values are logged and replaced by the field default.

`update_preferences` returns the canonical stored value. The frontend adopts it
rather than treating its draft as saved, which keeps React state consistent with
SQLite even when the backend trims or rejects a value.

### Embedded database split

SQLite is embedded through SQLx 0.8 with the bundled SQLite build, so users
never install or configure a database server. Two independent databases keep
shipped data and user data from interfering:

| Database | Location                                                                                           | Mode            | Contents                                                                               |
| -------- | -------------------------------------------------------------------------------------------------- | --------------- | -------------------------------------------------------------------------------------- |
| User     | `app_data_dir()/opendrvr.sqlite3`                                                                  | read/write, WAL | preferences, structured configuration, scan history, download history, cached metadata |
| Catalog  | Tauri resource `catalog/catalog.sqlite3`, plus optional `app_data_dir()/catalog-updates/*.sqlite3` | read-only       | hardware identifiers, manufacturers, driver metadata, official sources                 |

The catalog is resolved through `platform::catalog_path`. Tauri's resource
directory is checked first, with a `CARGO_MANIFEST_DIR` fallback in debug builds
so `cargo run` works before resources are copied. The catalog pool sets
`read_only(true)` and `create_if_missing(false)`; accidental writes fail at the
SQLite layer.

Catalog updates are additional immutable snapshots, not in-place edits. At
startup the runtime scans the per-user update directory, validates every
candidate's metadata, required tables, integrity, and foreign keys, and selects
the highest `catalog_version` that is newer than the bundled snapshot. Invalid
updates are logged and ignored. Removing the update file rolls the application
back to the bundled snapshot without touching user data. See
[`CATALOG.md`](CATALOG.md) for the full workflow.

The user database lives in Tauri's per-user `app_data_dir`, which is writable
without administrator rights and survives upgrades. It never shares a file or
transaction with the catalog, so catalog updates cannot overwrite user data.
Schema changes are applied by SQLx migrations in `src-tauri/migrations/`; the
applied version is read from `_sqlx_migrations`.

### Database initialization and recovery

`Database::initialize` runs during Tauri `setup` and is intentionally
non-fatal. The user database creates its parent directory, opens a pool with WAL,
foreign keys, a 5-second busy timeout and at most five connections, then runs
migrations and `PRAGMA quick_check`.

Recovery is reserved for actual corruption. Failures are classified before the
file is touched:

- **Preserved (unavailable):** `SQLITE_PERM`, `SQLITE_BUSY`, `SQLITE_LOCKED`,
  `SQLITE_READONLY`, `SQLITE_IOERR`, `SQLITE_FULL`, `SQLITE_CANTOPEN`, pool
  timeouts, closed pools, and I/O errors. These are reported without renaming
  anything, so a second instance holding a lock cannot sideline a healthy
  database.
- **Quarantined (corruption):** integrity-check failures, `SQLITE_CORRUPT`,
  `SQLITE_NOTADB`, and migration failures not caused by an operational error.

A quarantined file and its `-wal`/`-shm` sidecars are renamed with a
`.corrupt-<unix>` suffix and a fresh database is created once. A second failure
leaves the component unavailable rather than aborting startup.

Health reports a live writability probe, not just a successful open: it acquires
a write transaction, rolls it back, and reports `writable: false` with an
unavailable status when the file cannot be written.

A missing or unreadable catalog is logged and reported as unavailable; the user
database and the rest of the application continue to work. Health commands
surface both states to the UI. Preference commands return a sanitized database
error when the user database is unavailable.

Connections stay inside Rust. Commands expose only health summaries, the typed
preference set, and narrow results; SQL text is never accepted from the
frontend. Queries use bound parameters (`?1`, `?2`) rather than string
interpolation, and indexes cover the lookup and ordering paths defined in the
initial schemas.

### Driver catalog pipeline

The catalog is metadata only: hardware identifiers, manufacturers, driver
package metadata, Windows compatibility, and provenance. Driver binaries are
never bundled or stored in SQLite.

Two versions are deliberately independent:

- `schema_version` is the SQLite layout version (`2` in this build). A snapshot
  with a different schema version is rejected.
- `catalog_version` is the content version (`MAJOR.MINOR.PATCH`). It orders
  snapshots and decides whether an update supersedes the bundled file.

The shipped snapshot is schema-only with `catalog_version = 0.0.0` and a
deterministic `generated_at` of `1970-01-01T00:00:00Z`. It contains no fabricated
manufacturer, hardware, or driver rows. Real records are imported only from
verified sources.

`catalog/import.rs` builds a snapshot in a temporary file, applies the embedded
schema, inserts normalized rows in one transaction, runs `PRAGMA integrity_check`
and `PRAGMA foreign_key_check`, verifies row counts, and atomically moves the
finished file into place. The runtime never uses this writer.

`catalog/validate.rs` rejects malformed metadata, unknown cross-references,
duplicate identities, unsafe file names, invalid checksums, and non-official
URLs before any output is created. Hardware IDs must be concrete PCI, USB, ACPI,
or HDAUDIO identifiers; class-only identifiers are too broad to justify a
per-driver compatibility claim. A missing release date is a warning because it
must not be inferred from a version string. Every driver requires at least one
hardware match, one Windows/architecture pair, and one official source
reference.

`catalog/source_policy.rs` accepts only HTTPS URLs on a curated set of
manufacturer-controlled domains. Windows Update entries must stay on
Microsoft-controlled domains. Search engines, forums, and third-party driver
aggregators are not accepted even when they mirror an official file. The domain
list is intentionally curated and not exhaustive; adding a domain is a policy
decision, not a convenience.

The import document is described in [`CATALOG.md`](CATALOG.md) and is consumed by
`examples/import_catalog.rs`. `examples/build_catalog.rs` regenerates the
schema-only bundled snapshot. Catalog health reports the selected origin
(`bundled` or `update`), both versions, generated and last-verified timestamps,
and record counts so the UI can show exactly what data is loaded.

### Structured logging

The backend uses `tracing` and `tracing-subscriber`. Logs go to stderr so they
never mix with Tauri IPC. Both human-readable and JSON output are available
through configuration. Logging is initialized before `AppState` is created so
startup failures and command errors are recorded consistently.

### Serde boundary

Only types intended for the webview derive `Serialize`; internal service state
does not. IPC-facing structs use `#[serde(rename_all = "camelCase")]` so
TypeScript receives idiomatic property names while Rust keeps snake_case.
Deserialized configuration and preference payloads use the same convention.
`AppPreferences` also applies per-field defaults when a field is missing, which
keeps older frontends and stored payloads readable as the preference set grows.

### Command registration

All commands are listed once in the `generate_command_handlers!` macro in
`commands/mod.rs`. `lib.rs` installs that list with a single
`.invoke_handler(...)` call. Adding a command requires one function and one
macro entry, which makes omissions visible in review.

The registered surface is intentionally narrow: application info, backend
status, database health, hardware scanning, and the three preference operations.
The earlier `ping` and `database_test_query` diagnostics were removed once real
preference commands replaced them.

### Windows hardware enumeration

Phase 10 exposes the native hardware layer through `scan_hardware`. CfgMgr32 is
the primary provider because it enumerates the installed Plug and Play tree
without class GUIDs and returns authoritative instance IDs, status flags,
problem codes, and typed device properties. SetupAPI is reserved for future
read-only driver-package and INF inspection, and WMI/CIM for supplemental
CPU/GPU/motherboard/BIOS/memory facts.

The hardware domain (`hardware/`) owns a portable model and a `DeviceEnumerator`
trait; it never imports Win32 or Tauri. `platform::windows::device_tree` is the
only module that calls CfgMgr32. `AppState` injects the platform enumerator into
`HardwareService`, inverting the dependency so tests and future providers can
supply a fake.

Enumeration covers all installed device nodes (`CM_GETIDLIST_FILTER_NONE`).
Present nodes are located with `CM_LOCATE_DEVNODE_NORMAL`; non-present nodes fall
back to `CM_LOCATE_DEVNODE_PHANTOM`. Instance IDs are deduplicated
case-insensitively and sorted before the snapshot is returned. Only failure to
read the device ID list is fatal; per-device and per-property failures degrade
into `ScanDiagnostics` while the rest of the snapshot is returned. Raw instance
IDs and hardware IDs are authoritative; identifier parsing is best-effort and
malformed identifiers are preserved verbatim. Devices with no driver metadata,
no hardware IDs, or an unknown problem code are still recorded.

`hardware/id.rs` parses PCI (`VEN`, `DEV`, `SUBSYS`, `REV`, `CC`), USB (`VID`,
`PID`, `REV`, `MI`, class triplet), ACPI (`VEN`/`DEV` and compact EISA-style
IDs), and HD Audio (`FUNC`, `VEN`, `DEV`, `SUBSYS`, `REV`) fields. The raw string
is always retained. `hardware/vendors.rs` maps public vendor identifiers to
names through small curated tables; adding a vendor or identifier family only
changes that module. `hardware/identify.rs` resolves a preferred manufacturer
(subsystem vendor when present, otherwise the hardware vendor), keeps the
hardware and subsystem vendors separate, preserves Windows-reported names, and
records notes. Confidence is `confirmed` when a known device vendor matched and
reported metadata does not conflict, `partial` for subsystem-only, incomplete, or
conflicting identity, and `unresolved` when no known vendor identifier matched.
Reported names never establish identity on their own.

`DeviceCategory` derives processor, graphics, motherboard, chipset, network,
audio, Bluetooth, USB, storage, input, and other groups from the Windows class,
PCI/USB class codes, and a small set of well-known ACPI IDs. Friendly names are
never used for classification.

`DeviceCondition` combines presence, lifecycle state, problem code, and driver
metadata into `healthy`, `disabled`, `problem`, `missing-driver`, `non-present`,
or `unknown`. This is a presentation classification; the raw status flags and
problem code remain on every record.

The command runs the synchronous scan on `tauri::async_runtime::spawn_blocking`
so Configuration Manager cannot block the async runtime. Scans are not persisted,
no catalog match is attempted, and the hardware module is `Ready`. Driver
matching and installation belong to later phases.

### Windows isolation

The application targets Windows 10/11 x64, but Win32 APIs are confined to
`platform::windows`. General logic, commands, and domain services stay portable
and safe. CfgMgr32 enumeration now lives in `platform::windows::device_tree`;
future WMI, SetupAPI, and registry code also belongs under `platform`, keeping
the unsafe surface small and auditable.

`platform::is_supported` reports whether the current target is Windows x64. The
UI-facing status command exposes this as `supportedTarget`.

### Input validation

Frontend input is untrusted. Commands validate preference values through the
typed preference layer, which trims text, enforces length limits, and rejects
control characters through `utils::validate_user_text` before writing. Values
are stored with bound parameters, never interpolated into SQL.

## Tests

Unit tests cover configuration validation and disk round-tripping, error
sanitization, module status reporting, input validation, and preference
defaults, parsing, normalization, and rejection rules.

Hardware tests cover PCI/USB/ACPI/HDAUDIO identifier parsing, malformed-ID
preservation, instance-ID validation, device status and `CM_PROB_*` mapping,
condition classification, category classification, vendor lookup and aliases,
identity resolution and confidence, conflicting manufacturer metadata,
incomplete and unknown identifiers, snapshot helpers, provider injection,
multi-string decoding, GUID formatting, FILETIME conversion, case-insensitive
instance-ID deduplication, command error sanitization, and a real-hardware
enumeration of the local device tree.

Database tests cover initial creation, reopening with persisted preferences,
migration output, corrupt-file recovery, locked-database classification,
live read-only detection, catalog read-only access, missing catalog degradation,
bundled/user separation, atomic preference writes, reset, invalid-input rejection
before writing, and preference persistence across a reopen. Run them with:

```powershell
cargo test --all-targets
```

Domain services are Tauri-free by design, so future tests can exercise them
directly without a webview.

## Deliberate omissions

- No persisted scan, scan history write, or catalog matching.
- No WMI queries, SetupAPI driver-package inspection, or driver installation.
- No driver matching, manufacturer integrations, or source scraping.
- No network requests, downloads, or installers.
- No real catalog hardware or driver rows yet; only schema and metadata ship.
- No additional themes or update channels; only the implemented values are
  accepted.
- No Tauri events; commands return values only.

These are boundaries, not placeholders for hidden behavior.
