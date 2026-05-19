/**
 * localStorage wrapper with safe JSON serialization and type safety.
 */

const PREFIX = "x3-desktop:";

/**
 * Save a value to localStorage with automatic JSON serialisation.
 */
export function save<T>(key: string, value: T): void {
  try {
    localStorage.setItem(PREFIX + key, JSON.stringify(value));
  } catch (err) {
    console.warn(`[Storage] Failed to save "${key}":`, err);
  }
}

/**
 * Load a value from localStorage with JSON parsing.
 * Returns `defaultValue` if key is missing or parsing fails.
 */
export function load<T>(key: string, defaultValue: T): T {
  try {
    const raw = localStorage.getItem(PREFIX + key);
    if (raw === null) return defaultValue;
    return JSON.parse(raw) as T;
  } catch {
    return defaultValue;
  }
}

/**
 * Remove a key from localStorage.
 */
export function remove(key: string): void {
  localStorage.removeItem(PREFIX + key);
}

/**
 * Clear all x3-desktop prefixed keys.
 */
export function clearAll(): void {
  const keys: string[] = [];
  for (let i = 0; i < localStorage.length; i++) {
    const k = localStorage.key(i);
    if (k?.startsWith(PREFIX)) keys.push(k);
  }
  keys.forEach((k) => localStorage.removeItem(k));
}
