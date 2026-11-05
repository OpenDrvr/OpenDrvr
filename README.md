# OpenDrvr

**Open Driver** — an open-source Windows utility for identifying computer hardware, discovering
compatible drivers, and providing downloads from official manufacturer sources.

> Status: **Phase 2 — backend architecture.** Hardware detection, driver lookup, downloads, and
> installations are intentionally not implemented yet. See
> [`src-tauri/ARCHITECTURE.md`](src-tauri/ARCHITECTURE.md) for the backend design.

## Stack

| Layer    | Technology                                    |
| -------- | --------------------------------------------- |
| Desktop  | Tauri 2 (WebView2)                            |
| Backend  | Rust (stable, MSVC toolchain)                 |
| Frontend | React 19 + TypeScript 6 (strict mode)         |
| Bundler  | Vite 8                                        |
| Styling  | Tailwind CSS 4                                |
| Icons    | Lucide React                                  |
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
│   ├── App.tsx               Application shell and backend status panel
│   ├── main.tsx              React entry point
│   ├── index.css             Tailwind import, theme, base styles
│   └── lib/tauri.ts          Typed wrapper around Tauri IPC
├── src-tauri/
│   ├── src/lib.rs            Tauri builder, startup, and state registration
│   ├── src/main.rs           Windows desktop entry point
│   ├── src/commands/         Thin IPC adapters and command registration
│   ├── src/hardware/         Hardware identification boundary
│   ├── src/drivers/          Driver discovery boundary
│   ├── src/sources/          Official source registry boundary
│   ├── src/database/         Local persistence boundary
│   ├── src/downloads/        Download orchestration boundary
│   ├── src/platform/         Platform helpers and Windows-only integration
│   ├── src/config.rs         Typed configuration and validation
│   ├── src/error.rs          Internal and IPC error types
│   ├── src/logging.rs        Structured logging setup
│   ├── src/state.rs          Central application state
│   ├── src/utils/            Shared validation helpers
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
            $crate::commands::system::get_backend_status,
            $crate::commands::system::ping,
        ]
    };
}

// src-tauri/src/lib.rs
.invoke_handler(crate::generate_command_handlers!())
```

The UI displays the returned values in the **Rust backend connection** panel, which doubles as an
end-to-end IPC check. `get_backend_status` reports each domain module's lifecycle state, and `ping`
is a harmless round-trip command that demonstrates input validation and typed error handling.

Errors cross the IPC boundary as `{ code, message }`. Internal details (paths, OS errors) are
logged with `tracing` and never exposed to the webview.

## Windows x64 notes

- The build script pins the target to `x86_64-pc-windows-msvc`.
- Bundles are configured for **MSI** and **NSIS**.
- WebView2 is installed with the `downloadBootstrapper` mode when missing.
- The release profile enables LTO, strips symbols, and optimizes for size.
- A production Content Security Policy only allows Tauri IPC and local assets; a separate `devCsp`
  additionally allows the Vite dev server and HMR WebSocket.

## License

[AGPL-3.0](LICENSE) © 2026 Open Driver
