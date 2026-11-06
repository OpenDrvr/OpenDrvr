# OpenDrvr backend architecture

This document describes the Rust backend as of **Phase 4**. The structure is
intentionally conservative: it reserves clear module boundaries without adding
abstractions that the application does not need yet. Hardware scanning, driver
discovery, manufacturer integrations, downloads, and installations are **not**
implemented. Phase 4 added the desktop UI shell and design system on top of the
Phase 3 database foundation; the IPC surface is unchanged.

## Layout

```text
src-tauri/src/
├── main.rs              Windows entry point (release builds hide the console)
├── lib.rs               Tauri builder, startup sequence, state management
├── config.rs            Typed configuration, validation, load/save
├── error.rs             AppError, CommandError, result aliases
├── logging.rs           tracing subscriber initialization
├── state.rs             AppState and module status types
├── commands/
│   ├── mod.rs           Command exports and the single registration macro
│   ├── app.rs           get_app_info
│   ├── database.rs      get_database_health, database_test_query
│   └── system.rs        get_backend_status, ping
├── hardware/mod.rs      Hardware identification boundary (planned)
├── drivers/mod.rs       Driver matching boundary (planned)
├── sources/mod.rs       Official manufacturer sources boundary (planned)
├── database/
│   ├── mod.rs           Database service, startup degradation, health, test query
│   ├── user.rs          Writable user database, migrations, recovery
│   ├── catalog.rs       Read-only bundled catalog
│   ├── models.rs        IPC-facing health and test DTOs
│   └── test_support.rs  Temporary directories for tests
├── downloads/mod.rs     Download orchestration boundary (planned)
├── platform/
│   ├── mod.rs           Portable platform helpers and support check
│   └── windows.rs       Windows-only integration surface (future Win32 calls)
└── utils/mod.rs         Small shared helpers such as input validation
```

Adjacent to `src/`:

```text
src-tauri/
├── migrations/0001_user_initial.sql       Versioned user-database schema
├── migrations/0002_user_configuration.sql Structured configuration table
├── resources/catalog/schema.sql           Catalog schema source
├── resources/catalog/catalog.sqlite3      Generated read-only snapshot
└── examples/build_catalog.rs              Catalog regeneration tool
```

Each directory is a Rust module with its own `mod.rs`. Keeping a directory
instead of a single file leaves room for submodules (for example
`hardware/wmi.rs` or `platform/windows/setupapi.rs`) without restructuring the
crate later.

## Request flow

```text
React invoke("ping")
        │
        ▼
commands::system::ping        validate input, no business logic
        │
        ▼
AppState / domain service     owns application logic
        │
        ▼
AppError                      internal, detailed error
        │
        ▼
CommandError                  sanitized serializable payload
        │
        ▼
React rejects with { code, message }
```

Commands are deliberately thin adapters. They validate frontend input, call a
service from `AppState`, and convert errors. Domain modules never import Tauri
types, so they can be unit-tested without a running application.

## Frontend contract

The React shell consumes the backend through typed wrappers in
`src/lib/tauri.ts`. On startup, `useBackendSnapshot` loads `get_app_info`,
`get_backend_status`, and `get_database_health` in parallel and exposes a single
snapshot to the sidebar, Overview, Settings, and About views. Refreshes repeat
the same read-only calls; no UI control writes to the database or starts a
background operation.

The frontend never receives a SQL connection or SQL text. It sees only the
IPC DTOs defined in `commands/` and `database/models.rs`, which use camelCase
Serde fields. Hardware, driver, and download pages render planned empty states
until their backend modules exist; they do not synthesize device or transfer
data. Settings preferences are explicitly marked as a session-only preview and
are not persisted yet.

The desktop shell uses hash navigation and a custom component library rather
than a router or a UI framework. This keeps the webview dependency surface small
and makes the IPC boundary the only coupling between React and Rust.

## Decisions

### Centralized immutable state

`AppState` is constructed once during Tauri `setup` and registered with
`app.manage`. It owns the configuration and one service per domain. The state is
immutable after startup, so commands do not need a global `Mutex`. When a
service needs mutable data (a download queue, a hardware cache), it should own
its own synchronization primitive rather than making the entire state mutable.

### Typed errors, sanitized IPC boundary

Internal code returns `AppResult<T>` with `AppError`. Variants distinguish
configuration, I/O, serialization, validation, and internal failures. Detailed
messages may contain paths or OS error strings, so they are logged with
`tracing` and never sent to the webview.

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

### Embedded database split

SQLite is embedded through SQLx 0.8 with the bundled SQLite build, so users
never install or configure a database server. Two independent databases keep
shipped data and user data from interfering:

| Database | Location                                 | Mode            | Contents                                                                               |
| -------- | ---------------------------------------- | --------------- | -------------------------------------------------------------------------------------- |
| User     | `app_data_dir()/opendrvr.sqlite3`        | read/write, WAL | preferences, structured configuration, scan history, download history, cached metadata |
| Catalog  | Tauri resource `catalog/catalog.sqlite3` | read-only       | hardware identifiers, manufacturers, driver metadata, official sources                 |

The catalog is resolved through `platform::catalog_path`. Tauri's resource
directory is checked first, with a `CARGO_MANIFEST_DIR` fallback in debug builds
so `cargo run` works before resources are copied. The catalog pool sets
`read_only(true)` and `create_if_missing(false)`; accidental writes fail at the
SQLite layer. Future incremental catalog updates should ship a new immutable
snapshot instead of mutating the installed file.

The user database lives in Tauri's per-user `app_data_dir`, which is writable
without administrator rights and survives upgrades. It never shares a file or
transaction with the catalog, so catalog updates cannot overwrite user data.
Schema changes are applied by SQLx migrations in `src-tauri/migrations/`; the
applied version is read from `_sqlx_migrations`.

### Database initialization and recovery

`Database::initialize` runs during Tauri `setup` and is intentionally
non-fatal. The user database creates its parent directory, opens a pool with WAL,
foreign keys, a 5-second busy timeout and at most five connections, then runs
migrations and `PRAGMA quick_check`. If any step fails, the file and its
`-wal`/`-shm` sidecars are renamed with a `.corrupt-<unix>` suffix and a fresh
database is created once. A second failure leaves the component unavailable
rather than aborting startup.

A missing or unreadable catalog is logged and reported as unavailable; the user
database and the rest of the application continue to work. Health and test-query
commands surface both states to the UI.

Connections stay inside Rust. Commands expose only health summaries and narrow
results; SQL text is never accepted from the frontend. Queries use bound
parameters (`?1`, `?2`) rather than string interpolation, and indexes cover the
lookup and ordering paths defined in the initial schemas.

### Structured logging

The backend uses `tracing` and `tracing-subscriber`. Logs go to stderr so they
never mix with Tauri IPC. Both human-readable and JSON output are available
through configuration. Logging is initialized before `AppState` is created so
startup failures and command errors are recorded consistently.

### Serde boundary

Only types intended for the webview derive `Serialize`; internal service state
does not. IPC-facing structs use `#[serde(rename_all = "camelCase")]` so
TypeScript receives idiomatic property names while Rust keeps snake_case.
Deserialized configuration uses the same convention.

### Command registration

All commands are listed once in the `generate_command_handlers!` macro in
`commands/mod.rs`. `lib.rs` installs that list with a single
`.invoke_handler(...)` call. Adding a command requires one function and one
macro entry, which makes omissions visible in review.

### Windows isolation

The application targets Windows 10/11 x64, but Win32 APIs are confined to
`platform::windows`. General logic, commands, and domain services stay portable
and safe. Future WMI, SetupAPI, CfgMgr32, and registry code belongs under
`platform`, keeping the unsafe surface small and auditable.

`platform::is_supported` reports whether the current target is Windows x64. The
UI-facing status command exposes this as `supportedTarget`.

### Input validation

Frontend input is untrusted. Commands validate length and reject control
characters through `utils::validate_user_text` before using a value. The `ping`
command exists solely to demonstrate this pattern end to end; it has no side
effects.

## Tests

Unit tests cover configuration validation and disk round-tripping, error
sanitization, module status reporting, input validation, and the `ping` command.
Database tests cover initial creation, reopening with persisted preferences,
migration output, corrupt-file recovery, catalog read-only access, missing
catalog degradation, bundled/user separation, and the parameterized diagnostic
query. Run them with:

```powershell
cargo test --all-targets
```

Domain services are Tauri-free by design, so future tests can exercise them
directly without a webview.

## Deliberate omissions

- No hardware enumeration, WMI queries, or SetupAPI calls.
- No driver matching, manufacturer integrations, or source scraping.
- No network requests, downloads, or installers.
- No real catalog hardware or driver rows yet; only schema and metadata ship.
- No persisted UI preferences; the Settings controls are a labelled preview.
- No Tauri events; commands return values only.

These are boundaries, not placeholders for hidden behavior.
