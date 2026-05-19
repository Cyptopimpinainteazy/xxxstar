/**
 * IPC service — wraps Tauri invoke/listen with error handling, retry logic,
 * timeout management, and request/response logging.
 *
 * All backend communication flows through this single service layer.
 */

import type { IpcResponse, IpcError, IpcCommand } from "@/types/ipc";

/* ── Configuration ─────────────────────────────────────────── */
const DEFAULT_TIMEOUT_MS = 30_000;
const MAX_RETRIES = 3;
const RETRY_BASE_MS = 500;

/* ── Custom Error ──────────────────────────────────────────── */
export class AppError extends Error {
  code: string;
  details?: string;

  constructor(code: string, message: string, details?: string) {
    super(message);
    this.name = "AppError";
    this.code = code;
    this.details = details;
  }
}

/* ── Logging ───────────────────────────────────────────────── */
type LogLevel = "DEBUG" | "INFO" | "WARN" | "ERROR";

interface LogEntry {
  level: LogLevel;
  timestamp: string;
  command: string;
  duration?: number;
  error?: string;
}

const LOG_KEY = "x3-desktop-ipc-log";
const MAX_LOG_ENTRIES = 200;

function appendLog(entry: LogEntry): void {
  try {
    const raw = localStorage.getItem(LOG_KEY);
    const entries: LogEntry[] = raw ? JSON.parse(raw) : [];
    entries.push(entry);
    // Keep the most recent entries
    if (entries.length > MAX_LOG_ENTRIES) {
      entries.splice(0, entries.length - MAX_LOG_ENTRIES);
    }
    localStorage.setItem(LOG_KEY, JSON.stringify(entries));
  } catch {
    // Silently fail — logging should never crash the app
  }
}

/* ── Timeout utility ───────────────────────────────────────── */
function withTimeout<T>(promise: Promise<T>, ms: number): Promise<T> {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(
      () => reject(new AppError("TIMEOUT", `IPC call timed out after ${ms}ms`)),
      ms,
    );
    promise.then(
      (val) => {
        clearTimeout(timer);
        resolve(val);
      },
      (err) => {
        clearTimeout(timer);
        reject(err);
      },
    );
  });
}

/* ── Sleep for retry backoff ───────────────────────────────── */
function sleep(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms));
}

/* ── Tauri invoke wrapper ──────────────────────────────────── */

/**
 * Lazy-import the Tauri API so the app doesn't crash in a browser context
 * (useful during development without the Tauri runtime).
 */
async function tauriInvoke<T>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<T> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<T>(cmd, args);
  } catch {
    // Fallback: mock response when not running inside Tauri
    console.warn(`[IPC] Tauri not available — stubbing "${cmd}"`);
    return { success: true, data: null, timestamp: new Date().toISOString() } as T;
  }
}

/* ── Public API ────────────────────────────────────────────── */

/**
 * Invoke a Tauri IPC command with retry, timeout, and logging.
 *
 * @param command - The Tauri command name
 * @param payload - Optional arguments
 * @param options - Timeout and retry overrides
 * @returns The typed response data
 * @throws {AppError} on timeout, max retries, or backend error
 */
export async function ipcInvoke<T>(
  command: IpcCommand | string,
  payload?: Record<string, unknown>,
  options?: { timeout?: number; retries?: number },
): Promise<T> {
  const timeout = options?.timeout ?? DEFAULT_TIMEOUT_MS;
  const maxRetries = options?.retries ?? MAX_RETRIES;

  let lastError: AppError | null = null;
  const start = performance.now();

  for (let attempt = 0; attempt <= maxRetries; attempt++) {
    try {
      const response = await withTimeout(
        tauriInvoke<IpcResponse<T>>(command, payload),
        timeout,
      );

      const duration = Math.round(performance.now() - start);

      appendLog({
        level: "INFO",
        timestamp: new Date().toISOString(),
        command,
        duration,
      });

      if (response.success) {
        return response.data as T;
      }

      // Backend returned a structured error
      const err = response.error as IpcError;
      throw new AppError(
        err?.code ?? "BACKEND_ERROR",
        err?.message ?? "Unknown backend error",
        err?.details,
      );
    } catch (err) {
      lastError =
        err instanceof AppError
          ? err
          : new AppError("IPC_ERROR", String(err));

      // Don't retry on non-transient errors
      if (
        lastError.code !== "TIMEOUT" &&
        lastError.code !== "IPC_ERROR"
      ) {
        break;
      }

      // Exponential backoff
      if (attempt < maxRetries) {
        await sleep(RETRY_BASE_MS * 2 ** attempt);
      }
    }
  }

  const duration = Math.round(performance.now() - start);
  appendLog({
    level: "ERROR",
    timestamp: new Date().toISOString(),
    command,
    duration,
    error: lastError?.message,
  });

  throw lastError ?? new AppError("UNKNOWN", "IPC call failed");
}

/**
 * Listen for an event from the Tauri backend.
 *
 * @returns An unlisten function to detach the listener.
 */
export async function ipcListen<T>(
  event: string,
  handler: (payload: T) => void,
): Promise<() => void> {
  try {
    const { listen } = await import("@tauri-apps/api/event");
    const unlisten = await listen<T>(event, (e) => handler(e.payload));
    return unlisten;
  } catch {
    console.warn(`[IPC] Tauri not available — ignoring listen("${event}")`);
    return () => {};
  }
}

/** Clear the IPC log from localStorage */
export function clearIpcLog(): void {
  localStorage.removeItem(LOG_KEY);
}

/** Retrieve the IPC log entries */
export function getIpcLog(): LogEntry[] {
  try {
    const raw = localStorage.getItem(LOG_KEY);
    return raw ? JSON.parse(raw) : [];
  } catch {
    return [];
  }
}
