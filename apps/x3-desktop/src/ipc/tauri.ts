/**
 * Tauri IPC bridge — typed invoke wrapper.
 * Provides a clean interface for calling Rust backend commands
 * from the TypeScript frontend.
 */

// @ts-ignore - Tauri API may not resolve in strict typecheck without @tauri-apps/api dependency
import { invoke as tauriInvoke } from '@tauri-apps/api/core';

/**
 * Typed invoke — calls a Tauri backend command and returns the result.
 *
 * @param cmd - The Rust backend command name
 * @param args - Arguments object
 * @returns Promise<T> - The typed response
 */
export async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    const result = await tauriInvoke<T>(cmd, args);
    return result;
  } catch (error) {
    console.error(`[IPC] invoke('${cmd}') failed:`, error);
    throw error;
  }
}

/**
 * Check if running inside Tauri (vs web browser).
 */
export function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

/**
 * Listen for Tauri events.
 */
// @ts-ignore
import { listen as tauriListen } from '@tauri-apps/api/event';

export async function listen<T>(event: string, callback: (payload: T) => void): Promise<() => void> {
  if (!isTauri()) return () => {};
  const unlisten = await tauriListen<T>(event, (event) => {
    callback(event.payload);
  });
  return unlisten;
}

/**
 * Emit a Tauri event from the frontend.
 */
// @ts-ignore
import { emit as tauriEmit } from '@tauri-apps/api/event';

export async function emit(event: string, payload?: unknown): Promise<void> {
  if (!isTauri()) return;
  await tauriEmit(event, payload);
}