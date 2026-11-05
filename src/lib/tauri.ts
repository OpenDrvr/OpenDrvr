import { invoke } from "@tauri-apps/api/core";

export interface AppInfo {
  name: string;
  version: string;
  platform: string;
  architecture: string;
  profile: "debug" | "release";
  backend: string;
}

export type DatabaseComponentStatus = "ok" | "unavailable";

export interface UserDatabaseHealth {
  status: DatabaseComponentStatus;
  schemaVersion: number;
  tables: number;
  writable: boolean;
  message: string | null;
}

export interface CatalogDatabaseHealth {
  status: DatabaseComponentStatus;
  schemaVersion: number | null;
  tables: number;
  manufacturerCount: number;
  readOnly: boolean;
  message: string | null;
}

export interface DatabaseHealth {
  user: UserDatabaseHealth;
  catalog: CatalogDatabaseHealth;
}

export interface DatabaseTestResult {
  userDatabaseReady: boolean;
  catalogDatabaseReady: boolean;
  preference: string | null;
  catalogSchemaVersion: number | null;
  catalogManufacturerCount: number | null;
}

/**
 * Tauri injects this global into the WebView. It is absent when the frontend
 * is opened in a regular browser via `pnpm dev`.
 */
export function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function requireTauriRuntime(): void {
  if (!isTauriRuntime()) {
    throw new Error(
      "Tauri IPC is unavailable in a plain browser. Launch the desktop shell with pnpm tauri:dev.",
    );
  }
}

/**
 * Calls the Rust command defined in `src-tauri/src/lib.rs`.
 */
export async function getAppInfo(): Promise<AppInfo> {
  requireTauriRuntime();
  return invoke<AppInfo>("get_app_info");
}

/**
 * Returns live health for the writable user database and the bundled catalog.
 */
export async function getDatabaseHealth(): Promise<DatabaseHealth> {
  requireTauriRuntime();
  return invoke<DatabaseHealth>("get_database_health");
}

/**
 * Runs a harmless parameterized diagnostic query through the Rust backend.
 */
export async function databaseTestQuery(key?: string): Promise<DatabaseTestResult> {
  requireTauriRuntime();
  return invoke<DatabaseTestResult>("database_test_query", { key });
}
