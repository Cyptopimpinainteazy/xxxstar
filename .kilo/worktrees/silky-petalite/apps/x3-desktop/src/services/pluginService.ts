/**
 * pluginService.ts — Unified facade for official Tauri v2 plugins.
 *
 * Re-exports helpers so the rest of the app can import from one place
 * instead of remembering 14 different package names.
 */

// ── Autostart ──────────────────────────────────────
import { enable as asEnable, disable as asDisable, isEnabled as asIsEnabled } from "@tauri-apps/plugin-autostart";

export const autostart = {
  enable: asEnable,
  disable: asDisable,
  isEnabled: asIsEnabled,
} as const;

// ── Clipboard ──────────────────────────────────────
import { readText as clipRead, writeText as clipWrite } from "@tauri-apps/plugin-clipboard-manager";

export const clipboard = {
  readText: clipRead,
  writeText: clipWrite,
} as const;

// ── Dialog ─────────────────────────────────────────
export { open, save, message, ask, confirm } from "@tauri-apps/plugin-dialog";

// ── FS ─────────────────────────────────────────────
export {
  readTextFile,
  writeTextFile,
  readDir,
  exists,
  mkdir,
  remove,
} from "@tauri-apps/plugin-fs";

// ── Global Shortcut ────────────────────────────────
export {
  register as registerShortcut,
  unregister as unregisterShortcut,
  isRegistered as isShortcutRegistered,
} from "@tauri-apps/plugin-global-shortcut";

// ── Log ────────────────────────────────────────────
import {
  trace as logTrace,
  debug as logDebug,
  info as logInfo,
  warn as logWarn,
  error as logError,
} from "@tauri-apps/plugin-log";

export const log = {
  trace: logTrace,
  debug: logDebug,
  info: logInfo,
  warn: logWarn,
  error: logError,
} as const;

// ── Notification ───────────────────────────────────
export {
  sendNotification,
  requestPermission as requestNotificationPermission,
  isPermissionGranted as isNotificationPermissionGranted,
} from "@tauri-apps/plugin-notification";

// ── Opener ─────────────────────────────────────────
export { openUrl, openPath } from "@tauri-apps/plugin-opener";

// ── OS ─────────────────────────────────────────────
export { platform, arch, version as osVersion, locale } from "@tauri-apps/plugin-os";

// ── Process ────────────────────────────────────────
export { exit, relaunch } from "@tauri-apps/plugin-process";

// ── Shell ──────────────────────────────────────────
export { Command } from "@tauri-apps/plugin-shell";

// ── Store (persistent key-value) ───────────────────
import { load as storeLoad } from "@tauri-apps/plugin-store";

export const store = {
  load: storeLoad,
} as const;

// ── Window State ───────────────────────────────────
// window-state is fully automatic (saves/restores geometry on close/open).
// No JS API needed — just having the plugin registered is sufficient.
