/**
 * File system service — wraps Tauri FS plugin for config and file I/O.
 */

import { ipcInvoke } from "./ipcService";

/**
 * Read a text file from the application data directory.
 */
export async function readConfigFile(filename: string): Promise<string | null> {
  try {
    const { readTextFile, BaseDirectory } = await import(
      "@tauri-apps/plugin-fs"
    );
    return await readTextFile(filename, { baseDir: BaseDirectory.AppConfig });
  } catch {
    console.warn(`[FS] Failed to read config file: ${filename}`);
    return null;
  }
}

/**
 * Write a text file to the application data directory.
 */
export async function writeConfigFile(
  filename: string,
  content: string,
): Promise<boolean> {
  try {
    const { writeTextFile, BaseDirectory } = await import(
      "@tauri-apps/plugin-fs"
    );
    await writeTextFile(filename, content, {
      baseDir: BaseDirectory.AppConfig,
    });
    return true;
  } catch {
    console.warn(`[FS] Failed to write config file: ${filename}`);
    return false;
  }
}

/**
 * Get system information from the backend.
 */
export async function getSystemInfo(): Promise<{
  os: string;
  arch: string;
  hostname: string;
}> {
  try {
    return await ipcInvoke("get_system_info");
  } catch {
    return { os: "unknown", arch: "unknown", hostname: "localhost" };
  }
}
