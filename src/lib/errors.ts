/** Converts an unknown IPC/browser error into a short, safe message. */
export function describeError(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }

  if (typeof error === "string") {
    return error;
  }

  return "The Rust backend did not respond to the IPC request.";
}
