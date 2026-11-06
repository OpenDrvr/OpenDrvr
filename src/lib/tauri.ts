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

export type ModuleState = "ready" | "planned" | "unavailable";

export interface ModuleStatus {
  name: string;
  state: ModuleState;
  detail: string;
}

export interface BackendStatus {
  version: string;
  platform: string;
  supportedTarget: boolean;
  modules: ModuleStatus[];
}

export type ThemePreference = "dark";
export type UpdateChannel = "stable";

export interface AppPreferences {
  theme: ThemePreference;
  updateChannel: UpdateChannel;
  downloadDirectory: string;
  checkCatalogUpdates: boolean;
  notifyDownloads: boolean;
  keepDownloadHistory: boolean;
}

/** Error payload returned by Rust commands. */
export interface CommandErrorPayload {
  code: string;
  message: string;
}

/** Normalized IPC failure with the backend's stable error code attached. */
export class IpcError extends Error {
  readonly code: string;

  constructor(payload: CommandErrorPayload) {
    super(payload.message);
    this.name = "IpcError";
    this.code = payload.code;
  }
}

function isCommandErrorPayload(value: unknown): value is CommandErrorPayload {
  if (typeof value !== "object" || value === null) {
    return false;
  }

  const candidate = value as { code?: unknown; message?: unknown };
  return typeof candidate.code === "string" && typeof candidate.message === "string";
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
 * Single IPC entry point. Every command goes through here so runtime checks and
 * error normalization cannot drift between callers.
 */
async function invokeCommand<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  requireTauriRuntime();

  try {
    return await invoke<T>(command, args);
  } catch (error) {
    if (isCommandErrorPayload(error)) {
      throw new IpcError(error);
    }

    throw error;
  }
}

/** Safe application metadata used to confirm the Rust backend is reachable. */
export async function getAppInfo(): Promise<AppInfo> {
  return invokeCommand<AppInfo>("get_app_info");
}

/** Returns live health for the writable user database and the bundled catalog. */
export async function getDatabaseHealth(): Promise<DatabaseHealth> {
  return invokeCommand<DatabaseHealth>("get_database_health");
}

/** Returns the backend version, platform and per-module lifecycle state. */
export async function getBackendStatus(): Promise<BackendStatus> {
  return invokeCommand<BackendStatus>("get_backend_status");
}

/** Reads persisted application preferences. Missing values use defaults. */
export async function getPreferences(): Promise<AppPreferences> {
  return invokeCommand<AppPreferences>("get_preferences");
}

/** Validates and persists the complete preference set. */
export async function updatePreferences(preferences: AppPreferences): Promise<AppPreferences> {
  return invokeCommand<AppPreferences>("update_preferences", { preferences });
}

/** Removes saved preferences and returns the application defaults. */
export async function resetPreferences(): Promise<AppPreferences> {
  return invokeCommand<AppPreferences>("reset_preferences");
}
