import { useCallback, useEffect, useRef, useState } from "react";
import { describeError } from "./errors";
import { getPreferences, resetPreferences, updatePreferences, type AppPreferences } from "./tauri";

export type PreferencesState =
  | { status: "loading" }
  | { status: "ready"; preferences: AppPreferences }
  | { status: "error"; message: string };

export type PreferenceAction = "save" | "reset";

/**
 * Owns the persisted preference resource for the Settings page.
 *
 * Reads are exposed as a small state machine. Writes return the canonical value
 * from Rust and update the state in place, so the UI never invents a value the
 * database did not accept.
 */
export function usePreferences() {
  const [state, setState] = useState<PreferencesState>({ status: "loading" });
  const [pendingAction, setPendingAction] = useState<PreferenceAction | null>(null);
  const pendingRef = useRef(false);

  const load = useCallback(async () => {
    try {
      const preferences = await getPreferences();
      setState({ status: "ready", preferences });
    } catch (error) {
      setState({ status: "error", message: describeError(error) });
    }
  }, []);

  useEffect(() => {
    // Initial IPC fetch. State is only updated after the async call resolves, so
    // this does not cascade renders; the rule cannot see through the callback.
    // eslint-disable-next-line react-hooks/set-state-in-effect
    void load();
  }, [load]);

  const refresh = useCallback(async () => {
    setState({ status: "loading" });
    await load();
  }, [load]);

  const save = useCallback(async (preferences: AppPreferences): Promise<AppPreferences | null> => {
    if (pendingRef.current) {
      return null;
    }

    pendingRef.current = true;
    setPendingAction("save");

    try {
      const saved = await updatePreferences(preferences);
      setState({ status: "ready", preferences: saved });
      return saved;
    } finally {
      pendingRef.current = false;
      setPendingAction(null);
    }
  }, []);

  const reset = useCallback(async (): Promise<AppPreferences | null> => {
    if (pendingRef.current) {
      return null;
    }

    pendingRef.current = true;
    setPendingAction("reset");

    try {
      const defaults = await resetPreferences();
      setState({ status: "ready", preferences: defaults });
      return defaults;
    } finally {
      pendingRef.current = false;
      setPendingAction(null);
    }
  }, []);

  return { state, pendingAction, save, reset, refresh };
}
