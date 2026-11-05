import { invoke } from "@tauri-apps/api/core";

export interface AppInfo {
  name: string;
  version: string;
  platform: string;
  architecture: string;
  profile: "debug" | "release";
  backend: string;
}

/**
 * Tauri injects this global into the WebView. It is absent when the frontend
 * is opened in a regular browser via `pnpm dev`.
 */
export function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/**
 * Calls the Rust command defined in `src-tauri/src/lib.rs`.
 */
export async function getAppInfo(): Promise<AppInfo> {
  if (!isTauriRuntime()) {
    throw new Error(
      "Tauri IPC is unavailable in a plain browser. Launch the desktop shell with pnpm tauri:dev.",
    );
  }

  return invoke<AppInfo>("get_app_info");
}
