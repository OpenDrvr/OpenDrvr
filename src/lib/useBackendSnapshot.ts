import { useCallback, useEffect, useState } from "react";
import { describeError } from "./errors";
import {
  getAppInfo,
  getBackendStatus,
  getDatabaseHealth,
  type AppInfo,
  type BackendStatus,
  type DatabaseHealth,
} from "./tauri";

export interface BackendSnapshot {
  app: AppInfo;
  backend: BackendStatus;
  database: DatabaseHealth;
}

export type BackendSnapshotState =
  | { status: "loading" }
  | { status: "ready"; snapshot: BackendSnapshot }
  | { status: "error"; message: string };

/** Loads the real application, module and database state exposed by Rust. */
export function useBackendSnapshot() {
  const [state, setState] = useState<BackendSnapshotState>({ status: "loading" });

  const load = useCallback(async () => {
    try {
      const [app, backend, database] = await Promise.all([
        getAppInfo(),
        getBackendStatus(),
        getDatabaseHealth(),
      ]);
      setState({ status: "ready", snapshot: { app, backend, database } });
    } catch (error) {
      setState({ status: "error", message: describeError(error) });
    }
  }, []);

  useEffect(() => {
    // Initial IPC fetch. State is only updated after the async calls resolve, so this
    // does not cascade renders; the rule cannot see through the callback.
    // eslint-disable-next-line react-hooks/set-state-in-effect
    void load();
  }, [load]);

  const refresh = useCallback(async () => {
    setState({ status: "loading" });
    await load();
  }, [load]);

  return { state, refresh };
}
