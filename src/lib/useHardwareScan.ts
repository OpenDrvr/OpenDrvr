import { useCallback, useEffect, useRef, useState } from "react";
import { describeError } from "./errors";
import { scanHardware, type HardwareSnapshot } from "./tauri";

export type HardwareScanState =
  | { status: "loading" }
  | { status: "ready"; snapshot: HardwareSnapshot }
  | { status: "error"; message: string };

/**
 * Runs a real hardware scan against the Rust backend.
 *
 * Each request gets a monotonically increasing id so a slower, older scan can
 * never overwrite the result of a newer refresh.
 */
export function useHardwareScan() {
  const [state, setState] = useState<HardwareScanState>({ status: "loading" });
  const requestId = useRef(0);

  const run = useCallback(async () => {
    const currentRequest = requestId.current + 1;
    requestId.current = currentRequest;

    try {
      const snapshot = await scanHardware();
      if (requestId.current === currentRequest) {
        setState({ status: "ready", snapshot });
      }
    } catch (error) {
      if (requestId.current === currentRequest) {
        setState({ status: "error", message: describeError(error) });
      }
    }
  }, []);

  useEffect(() => {
    // Initial scan. State is only updated after the async call resolves, so this
    // does not cascade renders; the rule cannot see through the callback.
    // eslint-disable-next-line react-hooks/set-state-in-effect
    void run();
  }, [run]);

  const refresh = useCallback(async () => {
    setState({ status: "loading" });
    await run();
  }, [run]);

  return { state, refresh };
}
