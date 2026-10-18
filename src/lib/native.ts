import { invoke } from "@tauri-apps/api/core";

/**
 * Calls a native command whose failure doesn't matter to the user. Outside the desktop app
 * (a browser, the tests) there is nothing to call, and that is fine.
 */
export function callQuietly(command: string, args?: Record<string, unknown>): void {
  void Promise.resolve(invoke(command, args)).catch(() => {});
}
