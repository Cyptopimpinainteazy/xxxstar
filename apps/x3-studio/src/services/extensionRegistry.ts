import type { ExtensionPanel } from '../types';
import { useExtensionStore } from '../store';

const EXTENSIONS_KEY = 'x3studio-extensions';

export function registerPanel(panel: ExtensionPanel) {
  useExtensionStore.getState().registerPanel(panel);
  try {
    const stored = JSON.parse(localStorage.getItem(EXTENSIONS_KEY) || '[]');
    stored.push(panel);
    localStorage.setItem(EXTENSIONS_KEY, JSON.stringify(stored));
  } catch {}
}

export function unregisterPanel(id: string) {
  useExtensionStore.getState().unregisterPanel(id);
  try {
    const stored = JSON.parse(localStorage.getItem(EXTENSIONS_KEY) || '[]');
    localStorage.setItem(EXTENSIONS_KEY, JSON.stringify(stored.filter((p: any) => p.id !== id)));
  } catch {}
}

export function loadExtensions(): ExtensionPanel[] {
  try {
    return JSON.parse(localStorage.getItem(EXTENSIONS_KEY) || '[]');
  } catch { return []; }
}

export function getRegisteredPanels(): ExtensionPanel[] {
  return useExtensionStore.getState().panels;
}
