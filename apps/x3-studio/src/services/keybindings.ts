import type { Keybinding } from '../types';
import { useKeybindingStore } from '../store';

const KEYBINDINGS_KEY = 'x3studio-keybindings';

export function saveKeybindings() {
  try {
    localStorage.setItem(KEYBINDINGS_KEY, JSON.stringify(useKeybindingStore.getState().bindings));
  } catch {}
}

export function loadKeybindings(): Keybinding[] {
  try {
    const stored = localStorage.getItem(KEYBINDINGS_KEY);
    if (stored) {
      const bindings = JSON.parse(stored);
      useKeybindingStore.getState().setBindings(bindings);
      return bindings;
    }
  } catch {}
  return useKeybindingStore.getState().bindings;
}

export function formatKeys(keys: string): string {
  return keys.split('+').map(k => {
    const map: Record<string, string> = {
      Ctrl: '⌃', Cmd: '⌘', Shift: '⇧', Alt: '⌥',
      Backquote: '`', Slash: '/',
    };
    return map[k] || k;
  }).join('');
}
