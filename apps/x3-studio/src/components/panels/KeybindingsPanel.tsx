import { useState } from 'react';
import { useKeybindingStore, useKeybindImportStore } from '../../store';
import { saveKeybindings, formatKeys } from '../../services/keybindings';

interface ImportEntry {
  key: string;
  command: string;
  when?: string;
  label: string;
  mappedId: string | null;
}

const VSCODE_TO_INTERNAL: Record<string, { id: string; label: string; command: string }> = {
  'workbench.action.files.save': { id: 'save', label: 'Save File', command: 'editor.save' },
  'workbench.action.files.saveAll': { id: 'save-all', label: 'Save All', command: 'editor.saveAll' },
  'actions.find': { id: 'find', label: 'Find', command: 'editor.find' },
  'editor.action.startFindReplaceAction': { id: 'replace', label: 'Replace', command: 'editor.replace' },
  'workbench.action.gotoLine': { id: 'go-to-line', label: 'Go to Line', command: 'editor.goToLine' },
  'workbench.action.toggleSidebarVisibility': { id: 'toggle-sidebar', label: 'Toggle Sidebar', command: 'layout.toggleSidebar' },
  'workbench.action.terminal.toggleTerminal': { id: 'toggle-terminal', label: 'Toggle Terminal', command: 'layout.toggleTerminal' },
  'workbench.action.showCommands': { id: 'command-palette', label: 'Command Palette', command: 'editor.commandPalette' },
  'workbench.action.closeActiveEditor': { id: 'close-tab', label: 'Close Tab', command: 'editor.closeTab' },
  'workbench.action.togglePanel': { id: 'toggle-bottom', label: 'Toggle Bottom Panel', command: 'layout.toggleBottom' },
};

function cmdLabel(name: string): string {
  const last = name.split('.').pop() || name;
  return last.replace(/([A-Z])/g, ' $1').replace(/^./, c => c.toUpperCase()).replace(/[_-]/g, ' ').trim();
}

function cmdId(name: string): string {
  return name.replace(/[^a-zA-Z0-9-_.]/g, '-').toLowerCase();
}

function convertKey(vscodeKey: string): string {
  return vscodeKey.split('+').map(part => {
    const lower = part.toLowerCase();
    if (lower === 'ctrl' || lower === 'control') return 'Ctrl';
    if (lower === 'shift') return 'Shift';
    if (lower === 'alt') return 'Alt';
    if (lower === 'meta' || lower === 'cmd' || lower === 'win') return 'Meta';
    return part.length === 1 ? part.toUpperCase() : part;
  }).join('+');
}

export default function KeybindingsPanel() {
  const bindings = useKeybindingStore(s => s.bindings);
  const updateBinding = useKeybindingStore(s => s.updateBinding);
  const { importedBindings, setImportedBindings, clearImported } = useKeybindImportStore();
  const [editingId, setEditingId] = useState<string | null>(null);
  const [importText, setImportText] = useState('');
  const [importError, setImportError] = useState('');

  const handleKeyDown = (e: React.KeyboardEvent, id: string) => {
    e.preventDefault();
    e.stopPropagation();
    const parts: string[] = [];
    if (e.ctrlKey || e.metaKey) parts.push('Ctrl');
    if (e.shiftKey) parts.push('Shift');
    if (e.altKey) parts.push('Alt');
    const key = e.key;
    if (!['Control', 'Shift', 'Alt', 'Meta'].includes(key)) {
      parts.push(key.length === 1 ? key.toUpperCase() : key);
    }
    if (parts.length > 0) {
      updateBinding(id, parts.join('+'));
      saveKeybindings();
      setEditingId(null);
    }
  };

  const resetBindings = () => {
    const defaultBindings = [
      { id: 'save', label: 'Save File', keys: 'Ctrl+S', command: 'editor.save' },
      { id: 'save-all', label: 'Save All', keys: 'Ctrl+Shift+S', command: 'editor.saveAll' },
      { id: 'find', label: 'Find', keys: 'Ctrl+F', command: 'editor.find' },
      { id: 'replace', label: 'Replace', keys: 'Ctrl+H', command: 'editor.replace' },
      { id: 'go-to-line', label: 'Go to Line', keys: 'Ctrl+G', command: 'editor.goToLine' },
      { id: 'toggle-sidebar', label: 'Toggle Sidebar', keys: 'Ctrl+B', command: 'layout.toggleSidebar' },
      { id: 'toggle-terminal', label: 'Toggle Terminal', keys: 'Ctrl+`', command: 'layout.toggleTerminal' },
      { id: 'command-palette', label: 'Command Palette', keys: 'Ctrl+Shift+P', command: 'editor.commandPalette' },
      { id: 'close-tab', label: 'Close Tab', keys: 'Ctrl+W', command: 'editor.closeTab' },
    ];
    useKeybindingStore.getState().setBindings(defaultBindings);
    saveKeybindings();
  };

  const handleImport = () => {
    setImportError('');
    try {
      const parsed: { key: string; command: string; when?: string }[] = JSON.parse(importText);
      if (!Array.isArray(parsed)) {
        setImportError('Invalid format: expected an array of keybindings.');
        return;
      }
      const mapped: ImportEntry[] = parsed
        .filter(item => item.command && !item.command.startsWith('-'))
        .map(item => {
          const mapping = VSCODE_TO_INTERNAL[item.command];
          if (mapping) {
            return { key: convertKey(item.key), command: mapping.command, when: item.when, label: mapping.label, mappedId: mapping.id };
          }
          return { key: convertKey(item.key), command: item.command, when: item.when, label: cmdLabel(item.command), mappedId: null };
        });
      setImportedBindings(mapped);
    } catch {
      setImportError('Invalid JSON. Please paste a valid keybindings.json array.');
    }
  };

  const handleApply = () => {
    const merged = [...bindings];
    const entries = importedBindings as ImportEntry[];
    for (const imp of entries) {
      const id = imp.mappedId || cmdId(imp.command);
      const idx = merged.findIndex(b => b.id === id);
      if (idx >= 0) {
        merged[idx] = { ...merged[idx], keys: imp.key };
      } else {
        merged.push({ id, label: imp.label, keys: imp.key, command: imp.command, when: imp.when });
      }
    }
    useKeybindingStore.getState().setBindings(merged);
    saveKeybindings();
  };

  return (
    <div className="panel-body" style={{ padding: '8px', height: '100%', overflowY: 'auto' }}>
      <div className="panel-header">
        <span>Keybindings</span>
        <button className="btn" onClick={resetBindings} style={{ fontSize: 10 }}>Reset Defaults</button>
      </div>

      <div style={{ marginBottom: 12 }}>
        <div style={{ fontWeight: 600, fontSize: 'var(--font-size-sm)', marginBottom: 4 }}>Import VS Code Keybindings</div>
        <textarea
          value={importText}
          onChange={e => { setImportText(e.target.value); setImportError(''); }}
          placeholder={'Paste your VS Code keybindings.json here...'}
          rows={4}
          style={{ width: '100%', fontSize: 'var(--font-size-sm)', fontFamily: 'var(--font-mono)', resize: 'vertical' }}
        />
        <div style={{ display: 'flex', gap: 6, marginTop: 4 }}>
          <button className="btn" onClick={handleImport}>Import</button>
          {importedBindings.length > 0 && (
            <button className="btn" onClick={clearImported}>Clear Imported</button>
          )}
        </div>
        {importError && (
          <div style={{ color: 'var(--error-color)', fontSize: 'var(--font-size-sm)', marginTop: 4 }}>{importError}</div>
        )}
      </div>

      {importedBindings.length > 0 && (
        <div style={{ marginBottom: 12 }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 4 }}>
            <div style={{ fontWeight: 600, fontSize: 'var(--font-size-sm)' }}>Imported from VS Code ({importedBindings.length})</div>
            <button className="btn" onClick={handleApply} style={{ fontSize: 10 }}>Apply Imported</button>
          </div>
          <div style={{ maxHeight: 160, overflowY: 'auto', border: '1px solid var(--border-color)', borderRadius: 4 }}>
            {(importedBindings as ImportEntry[]).map((imp, i) => (
              <div key={i} style={{
                display: 'flex', justifyContent: 'space-between', alignItems: 'center',
                padding: '4px 8px', borderBottom: '1px solid var(--border-color)',
                fontSize: 'var(--font-size-sm)',
              }}>
                <span>{imp.label}</span>
                <span style={{ fontFamily: 'var(--font-mono)', color: 'var(--text-muted)' }}>{imp.key}</span>
              </div>
            ))}
          </div>
        </div>
      )}

      <div style={{ marginBottom: 8, fontSize: 'var(--font-size-sm)', color: 'var(--text-muted)' }}>
        Click a keybinding to edit. Press the desired key combination.
      </div>

      {bindings.map(b => (
        <div key={b.id} style={{
          display: 'flex', justifyContent: 'space-between', alignItems: 'center',
          padding: '6px 8px', borderBottom: '1px solid var(--border-color)',
          fontSize: 'var(--font-size-sm)',
        }}>
          <span>{b.label}</span>
          {editingId === b.id ? (
            <span
              className="badge badge-info"
              tabIndex={0}
              onKeyDown={(e) => handleKeyDown(e, b.id)}
              style={{ cursor: 'pointer', fontFamily: 'var(--font-mono)' }}
            >
              Press keys...
            </span>
          ) : (
            <span
              className="badge badge-info"
              onClick={() => setEditingId(b.id)}
              style={{ cursor: 'pointer', fontFamily: 'var(--font-mono)' }}
            >
              {formatKeys(b.keys)}
            </span>
          )}
        </div>
      ))}
    </div>
  );
}
