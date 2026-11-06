import { IpcError } from "./tauri";

function isErrorPayload(value: unknown): value is { message: string } {
  if (typeof value !== "object" || value === null) {
    return false;
  }

  return typeof (value as { message?: unknown }).message === "string";
}

/** Converts an unknown IPC/browser error into a short, safe message. */
export function describeError(error: unknown): string {
  if (error instanceof IpcError) {
    return error.message;
  }

  if (error instanceof Error) {
    return error.message;
  }

  if (typeof error === "string") {
    return error;
  }

  if (isErrorPayload(error)) {
    return error.message;
  }

  return "The Rust backend did not respond to the IPC request.";
}
