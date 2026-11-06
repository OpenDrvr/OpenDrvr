# OpenDrvr

**Open Driver** — an open-source Windows utility for identifying computer hardware, discovering
compatible drivers, and providing downloads from official manufacturer sources.

> Status: **Phase 4 — desktop UI and design system.** OpenDrvr now has a dark-first desktop shell,
> a reusable React component library, and initial layouts for every navigation page. SQLite still
> runs inside the Rust backend with a read-only bundled driver catalog and a separate writable user
> database. Hardware detection, driver lookup, downloads, and installations are intentionally not
> implemented yet. See [`src-tauri/ARCHITECTURE.md`](src-tauri/ARCHITECTURE.md) for the backend
> design.

## Stack

| Layer    | Technology                                    |
| -------- | --------------------------------------------- |
| Desktop  | Tauri 2 (WebView2)                            |
| Backend  | Rust (stable, MSVC toolchain)                 |
| Frontend | React 19 + TypeScript 6 (strict mode)         |
| Database | SQLite via SQLx 0.8 (bundled, no server)      |
| Bundler  | Vite 8                                        |
| Styling  | Tailwind CSS 4 (dark-first design tokens)     |
| Icons    | Lucide React                                  |
| UI       | Custom React component library                |
| Tooling  | pnpm 12, ESLint 10, Prettier 3, Cargo/rustfmt |
| Target   | Windows 10 / Windows 11 x64                   |

## Prerequisites

- **Node.js** 20.19 or newer
- **pnpm** 12 (`npm install --global pnpm@latest`)
- **Rust** stable with the `x86_64-pc-windows-msvc` target
- **Visual Studio Build Tools 2022** with the “Desktop development with C++” workload
- **WebView2 Runtime** (included with Windows 11 and current Windows 10)

Verify the desktop toolchain at any time:

```powershell
pnpm tauri info
```

## Getting started

```powershell
# Install frontend dependencies
pnpm install

# Start the desktop app (Vite + Rust in development mode)
pnpm tauri:dev
```

Running `pnpm dev` alone serves the React frontend in a browser. The UI detects the missing Tauri
IPC bridge and shows a clear “backend unavailable” state instead of failing silently.

## Scripts

| Command            | Purpose                                             |
| ------------------ | --------------------------------------------------- |
| `pnpm tauri:dev`   | Run the Tauri desktop app with Vite HMR             |
| `pnpm tauri:build` | Build a Windows x64 release and MSI/NSIS installers |
| `pnpm dev`         | Run the frontend only in a browser                  |
| `pnpm build`       | Type-check and build the production frontend        |
| `pnpm typecheck`   | TypeScript project build with strict checks         |
| `pnpm lint`        | ESLint over the repository                          |
| `pnpm format`      | Format with Prettier                                |
| `pnpm check`       | Type-check, lint, and verify Prettier formatting    |
| `pnpm rust:check`  | Run `cargo check` on the Tauri backend              |
| `pnpm rust:clippy` | Run Clippy with warnings denied                     |
| `pnpm rust:test`   | Run backend unit tests                              |
| `pnpm check:all`   | Run all frontend and Rust checks                    |

Release artifacts are written under
`src-tauri/target/x86_64-pc-windows-msvc/release/bundle/` (`msi/` and `nsis/`).

## Project structure

```text
.
├── src/                      React + TypeScript frontend
│   ├── App.tsx               Persistent shell: sidebar, page routing, backend snapshot
│   ├── main.tsx              React entry point
│   ├── index.css             Tailwind import, dark/purple tokens, base styles
│   ├── components/
│   │   ├── layout/           Sidebar, page headers, About dialog
│   │   └── ui/               Buttons, inputs, tables, dialogs, toasts, status, empty states
│   ├── pages/                Overview, Hardware, Drivers, Downloads, Settings
│   └── lib/
│       ├── tauri.ts          Typed wrapper around Tauri IPC
│       ├── navigation.ts     Page definitions and hash parsing
│       ├── useAppNavigation.ts  Hash-based desktop navigation
│       ├── useBackendSnapshot.ts  App, module, and database status loading
│       ├── toast.ts          Toast context contract
│       └── cn.ts             Conditional class names
├── src-tauri/
│   ├── src/lib.rs            Tauri builder, startup, and state registration
│   ├── src/main.rs           Windows desktop entry point
│   ├── src/commands/         Thin IPC adapters and command registration
│   ├── src/hardware/         Hardware identification boundary
│   ├── src/drivers/          Driver discovery boundary
│   ├── src/sources/          Official source registry boundary
│   ├── src/database/         Embedded SQLite: user database and bundled catalog
│   ├── src/downloads/        Download orchestration boundary
│   ├── src/platform/         Platform helpers and Windows-only integration
│   ├── src/config.rs         Typed configuration and validation
│   ├── src/error.rs          Internal and IPC error types
│   ├── src/logging.rs        Structured logging setup
│   ├── src/state.rs          Central application state
│   ├── src/utils/            Shared validation helpers
│   ├── migrations/           Versioned user-database migrations
│   ├── resources/catalog/    Catalog schema and read-only SQLite snapshot
│   ├── examples/             Maintenance tools (catalog regeneration)
│   ├── ARCHITECTURE.md       Backend design and decisions
│   ├── capabilities/         Tauri 2 window capabilities
│   ├── icons/                Windows icon assets
│   ├── build.rs              Tauri build script
│   ├── Cargo.toml            Rust crate configuration
│   └── tauri.conf.json       Window, security, and bundle configuration
├── index.html                Vite HTML entry
├── vite.config.ts            Vite + React + Tailwind configuration
├── tsconfig*.json            Strict TypeScript project references
├── eslint.config.js          ESLint flat configuration
├── .prettierrc.json          Formatting rules
├── rust-toolchain.toml       Stable Rust + Windows x64 target + fmt/clippy
└── package.json              Frontend dependencies and scripts
```

## Frontend ↔ Rust communication

The frontend calls the Rust command through Tauri's IPC bridge:

```ts
// src/lib/tauri.ts
import { invoke } from "@tauri-apps/api/core";

export const getAppInfo = () => invoke<AppInfo>("get_app_info");
```

```rust
// src-tauri/src/commands/app.rs
#[tauri::command]
pub fn get_app_info(app: tauri::AppHandle) -> AppInfo {
    // Returns the application name, version, OS, architecture, and build profile.
}
```

Commands are registered in one place, `src-tauri/src/commands/mod.rs`, and installed with a single
call in `lib.rs`:

```rust
// src-tauri/src/commands/mod.rs
macro_rules! generate_command_handlers {
    () => {
        tauri::generate_handler![
            $crate::commands::app::get_app_info,
            $crate::commands::database::database_test_query,
            $crate::commands::database::get_database_health,
            $crate::commands::system::get_backend_status,
            $crate::commands::system::ping,
        ]
    };
}

// src-tauri/src/lib.rs
.invoke_handler(crate::generate_command_handlers!())
```

The UI loads these values once into a backend snapshot and presents them on the Overview page, in the
Settings local-data panel, in the sidebar status card, and in the About dialog. `get_backend_status`
reports each domain module's lifecycle state, and `ping` is a harmless round-trip command that
demonstrates input validation and typed error handling.

Errors cross the IPC boundary as `{ code, message }`. Internal details (paths, OS errors) are
logged with `tracing` and never exposed to the webview.

## Desktop UI

The frontend is a Windows desktop shell, not a website. It uses a persistent 236 px sidebar with
application branding, grouped navigation (Overview, Hardware, Drivers, Downloads, Settings), live
backend and local-data status, the application version, and an About dialog. The main content area
scrolls independently and the layout is verified down to the configured minimum window size.

Design system:

- Near-black and charcoal surfaces with a faint violet undertone.
- Electric purple (`#8b5cf6`) as the primary accent.
- Segoe UI Variable typography, subtle 1 px borders, restrained shadows, and small radii.
- Tokens live in `src/index.css` through Tailwind's `@theme`; no shadcn/ui or dashboard template is
  used.
- Custom components in `src/components/ui`: buttons, icon buttons, inputs, search fields, dropdowns,
  tabs, tables, status indicators, native dialogs, tooltips, empty states, spinners/skeletons, switches,
  and toast notifications.
- Layout components in `src/components/layout`: sidebar, page/section headers, and About dialog.

Navigation is hash-based (`#/overview`, `#/hardware`, and so on) through `useAppNavigation`, so
browser-style back/forward works without a router dependency. The current page is also exposed with
`aria-current="page"` for assistive technology.

The initial pages are deliberately honest:

- **Overview** introduces OpenDrvr, its principles, and the real backend module states.
- **Hardware**, **Drivers**, and **Downloads** show planned empty states; no hardware, driver status,
  or transfer data is invented.
- **Settings** demonstrates the preferences layout with a clear “preview” notice. Controls are not
  persisted yet, while the local-data panel shows real schema and catalog values from Rust.

`__APP_VERSION__` is injected from `package.json` by `vite.config.ts` and used by the sidebar and
About dialog.

## Embedded database

OpenDrvr uses embedded SQLite through SQLx 0.8 with the bundled SQLite build. No external database
server or user setup is required. All SQL stays inside Rust; React only calls typed commands and
never receives a connection or executes a query.

The database is split into two independent components:

| Component              | Location                                                       | Access    | Purpose                                                                     |
| ---------------------- | -------------------------------------------------------------- | --------- | --------------------------------------------------------------------------- |
| User database          | `%APPDATA%\com.opendrvr.desktop\opendrvr.sqlite3`              | Writable  | Preferences, configuration, scan history, download history, cached metadata |
| Bundled driver catalog | `<install>\resources\catalog\catalog.sqlite3` (Tauri resource) | Read-only | Hardware identifiers, manufacturers, driver metadata, official sources      |

The user database file name is configurable through `database.fileName` but is always resolved
inside Tauri's per-user app data directory. It survives application upgrades and is never touched
by catalog updates.

### Schemas

User database (migrations `0001_user_initial.sql` and `0002_user_configuration.sql`, schema
version 2):

- `app_preferences` — key/value preferences with update timestamps.
- `hardware_scans` — scan runs with status and summary.
- `hardware_scan_devices` — per-scan PCI/USB devices and optional details.
- `download_history` — download attempts, progress and outcome.
- `cached_driver_metadata` — cached driver payloads with expiry and ETag.
- `user_configuration` — structured JSON configuration documents.

Bundled catalog (schema version 1, `resources/catalog/schema.sql`):

- `catalog_meta` — catalog format and schema version.
- `manufacturers` — vendor metadata.
- `hardware_devices` — PCI/USB identifiers, subsystem IDs, class and revision.
- `driver_sources` — official manufacturer/Windows Update/OEM sources.
- `drivers` — driver name, version, release date, download metadata and checksum.
- `driver_hardware` — driver-to-hardware matches.
- `driver_windows_compat` — supported Windows versions and architectures (`x86`, `x64`, `arm64`).
- `driver_source_refs` — official source URLs per driver.

The shipped catalog contains schema and metadata only. Hardware and driver rows will be added when
real catalog data is available; no records are invented. Future catalog updates should ship a new
immutable snapshot, not mutate the installed file.

### Initialization and recovery

- User database: creates the directory and file, enables WAL, foreign keys, a 5-second busy
  timeout and a bounded connection pool, then runs SQLx migrations and `PRAGMA quick_check`.
  If initialization fails, the file (and `-wal`/`-shm` sidecars) is renamed to
  `opendrvr.sqlite3.corrupt-<unix>` and a fresh database is created once.
- Bundled catalog: opened with SQLite's `read_only` flag. A missing or broken catalog is logged and
  reported through health checks; it does not prevent the application from starting.
- Startup never crashes because a database is unavailable. The UI shows the affected component as
  unavailable instead.

### Commands

- `get_database_health` — live status, schema version, table count, writability and catalog
  manufacturer count.
- `database_test_query` — harmless parameterized preference lookup that also reports catalog
  availability. No SQL text is accepted from the frontend.

### Regenerating the catalog

After editing `src-tauri/resources/catalog/schema.sql`:

```powershell
cd src-tauri
cargo run --example build_catalog
```

This rewrites `resources/catalog/catalog.sqlite3` from the schema. The generated file is bundled
through `tauri.conf.json` as `catalog/catalog.sqlite3`.

## Windows x64 notes

- The build script pins the target to `x86_64-pc-windows-msvc`.
- Bundles are configured for **MSI** and **NSIS**.
- WebView2 is installed with the `downloadBootstrapper` mode when missing.
- The release profile enables LTO, strips symbols, and optimizes for size.
- A production Content Security Policy only allows Tauri IPC and local assets; a separate `devCsp`
  additionally allows the Vite dev server and HMR WebSocket.

## License

[AGPL-3.0](LICENSE) © 2026 Open Driver
