// Comprehensive keyboard shortcut system with customization and cheatsheet display

import React, { useState, useEffect, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';

export interface KeyCombo {
  ctrl: boolean;
  shift: boolean;
  alt: boolean;
  key: string;
}

export interface ShortcutAction {
  id: string;
  name: string;
  description: string;
  combo: KeyCombo;
  category: string;
  enabled: boolean;
  customizable: boolean;
}

interface ShortcutState {
  actions: ShortcutAction[];
  categories: string[];
  customizations: Map<string, KeyCombo>;
  conflicts: Map<string, string[]>;
  recordingConflict: string | null;
}

const DEFAULT_SHORTCUTS: ShortcutAction[] = [
  // Window management
  {
    id: 'window.snap.2x2',
    name: 'Window Snap: 2×2',
    description: 'Arrange windows in 2×2 grid',
    combo: { ctrl: true, shift: true, alt: false, key: '1' },
    category: 'Window Management',
    enabled: true,
    customizable: true,
  },
  {
    id: 'window.snap.1plus2',
    name: 'Window Snap: 1+2',
    description: 'Arrange 1 large + 2 small windows',
    combo: { ctrl: true, shift: true, alt: false, key: '2' },
    category: 'Window Management',
    enabled: true,
    customizable: true,
  },
  {
    id: 'window.snap.fullscreen',
    name: 'Window Snap: Fullscreen',
    description: 'Maximize current window',
    combo: { ctrl: true, shift: true, alt: false, key: 'f' },
    category: 'Window Management',
    enabled: true,
    customizable: true,
  },
  {
    id: 'window.cycle',
    name: 'Cycle Windows',
    description: 'Switch between open windows',
    combo: { ctrl: true, shift: false, alt: false, key: 'Tab' },
    category: 'Window Management',
    enabled: true,
    customizable: true,
  },
  {
    id: 'window.center',
    name: 'Center Window',
    description: 'Center window on screen',
    combo: { ctrl: true, shift: true, alt: false, key: 'c' },
    category: 'Window Management',
    enabled: true,
    customizable: true,
  },

  // Navigation
  {
    id: 'nav.validator',
    name: 'Go to Validators',
    description: 'Jump to validators panel',
    combo: { ctrl: true, shift: false, alt: true, key: '1' },
    category: 'Navigation',
    enabled: true,
    customizable: true,
  },
  {
    id: 'nav.wallet',
    name: 'Go to Wallet',
    description: 'Jump to wallet panel',
    combo: { ctrl: true, shift: false, alt: true, key: '2' },
    category: 'Navigation',
    enabled: true,
    customizable: true,
  },
  {
    id: 'nav.dex',
    name: 'Go to DEX',
    description: 'Jump to DEX trading panel',
    combo: { ctrl: true, shift: false, alt: true, key: '3' },
    category: 'Navigation',
    enabled: true,
    customizable: true,
  },
  {
    id: 'nav.terminal',
    name: 'Go to Terminal',
    description: 'Jump to terminal panel',
    combo: { ctrl: true, shift: false, alt: true, key: 't' },
    category: 'Navigation',
    enabled: true,
    customizable: true,
  },
  {
    id: 'nav.search',
    name: 'Global Search',
    description: 'Open global search',
    combo: { ctrl: true, shift: false, alt: false, key: 'k' },
    category: 'Navigation',
    enabled: true,
    customizable: true,
  },

  // DEX Trading
  {
    id: 'dex.new.swap',
    name: 'New Swap',
    description: 'Open new swap dialog',
    combo: { ctrl: true, shift: true, alt: false, key: 's' },
    category: 'DEX Trading',
    enabled: true,
    customizable: true,
  },
  {
    id: 'dex.new.limit',
    name: 'New Limit Order',
    description: 'Create limit order',
    combo: { ctrl: true, shift: true, alt: false, key: 'l' },
    category: 'DEX Trading',
    enabled: true,
    customizable: true,
  },
  {
    id: 'dex.quick.swap',
    name: 'Quick Swap',
    description: 'Execute last used swap',
    combo: { ctrl: true, shift: false, alt: true, key: 's' },
    category: 'DEX Trading',
    enabled: true,
    customizable: true,
  },
  {
    id: 'dex.portfolio',
    name: 'View Portfolio',
    description: 'Show portfolio summary',
    combo: { ctrl: true, shift: false, alt: false, key: 'p' },
    category: 'DEX Trading',
    enabled: true,
    customizable: true,
  },

  // Wallet
  {
    id: 'wallet.send',
    name: 'Send',
    description: 'Open send dialog',
    combo: { ctrl: true, shift: true, alt: false, key: 'x' },
    category: 'Wallet',
    enabled: true,
    customizable: true,
  },
  {
    id: 'wallet.receive',
    name: 'Receive',
    description: 'Show receive address',
    combo: { ctrl: true, shift: true, alt: false, key: 'r' },
    category: 'Wallet',
    enabled: true,
    customizable: true,
  },
  {
    id: 'wallet.history',
    name: 'Transaction History',
    description: 'View transaction history',
    combo: { ctrl: true, shift: false, alt: false, key: 'h' },
    category: 'Wallet',
    enabled: true,
    customizable: true,
  },

  // Application
  {
    id: 'app.preferences',
    name: 'Preferences',
    description: 'Open settings',
    combo: { ctrl: true, shift: false, alt: false, key: ',' },
    category: 'Application',
    enabled: true,
    customizable: true,
  },
  {
    id: 'app.help',
    name: 'Help',
    description: 'Show help/keyboard shortcuts',
    combo: { ctrl: false, shift: false, alt: false, key: 'F1' },
    category: 'Application',
    enabled: true,
    customizable: true,
  },
  {
    id: 'app.reload',
    name: 'Reload',
    description: 'Reload application',
    combo: { ctrl: true, shift: false, alt: false, key: 'r' },
    category: 'Application',
    enabled: true,
    customizable: true,
  },
  {
    id: 'app.devtools',
    name: 'Developer Tools',
    description: 'Toggle developer console',
    combo: { ctrl: true, shift: true, alt: false, key: 'i' },
    category: 'Application',
    enabled: true,
    customizable: false, // System shortcut
  },
  {
    id: 'app.fullscreen',
    name: 'Toggle Fullscreen',
    description: 'Toggle fullscreen mode',
    combo: { ctrl: true, shift: false, alt: false, key: 'F11' },
    category: 'Application',
    enabled: true,
    customizable: true,
  },
];

export const KeyboardShortcutMap: React.FC = () => {
  const [state, setState] = useState<ShortcutState>({
    actions: DEFAULT_SHORTCUTS,
    categories: [...new Set(DEFAULT_SHORTCUTS.map(s => s.category))],
    customizations: new Map(),
    conflicts: new Map(),
    recordingConflict: null,
  });

  const [showCheatsheet, setShowCheatsheet] = useState(false);
  const [selectedCategory, setSelectedCategory] = useState('');
  const [recordingShortcut, setRecordingShortcut] = useState<string | null>(null);
  const [recordedCombo, setRecordedCombo] = useState<KeyCombo | null>(null);

  // Load customizations from storage
  useEffect(() => {
    const loadCustomizations = async () => {
      try {
        const customizations = await invoke<Record<string, KeyCombo>>(
          'load_shortcut_customizations'
        );
        setState(prev => {
          const customMap = new Map(Object.entries(customizations || {}));
          const updatedActions = prev.actions.map(action => ({
            ...action,
            combo: customMap.get(action.id) || action.combo,
          }));
          return {
            ...prev,
            customizations: customMap,
            actions: updatedActions,
            categories: prev.categories,
            conflicts: prev.conflicts,
            recordingConflict: prev.recordingConflict,
          };
        });
      } catch (error) {
        console.error('Failed to load customizations:', error);
      }
    };

    loadCustomizations();
  }, []);

  // Detect key presses when recording
  useEffect(() => {
    if (!recordingShortcut) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      e.preventDefault();

      const combo: KeyCombo = {
        ctrl: e.ctrlKey,
        shift: e.shiftKey,
        alt: e.altKey,
        key: e.key.toUpperCase(),
      };

      setRecordedCombo(combo);
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [recordingShortcut]);

  // Check for shortcut conflicts
  const checkConflicts = (combo: KeyCombo): string[] => {
    const conflicts: string[] = [];
    for (const action of state.actions) {
      if (
        action.combo.ctrl === combo.ctrl &&
        action.combo.shift === combo.shift &&
        action.combo.alt === combo.alt &&
        action.combo.key === combo.key
      ) {
        conflicts.push(action.id);
      }
    }
    return conflicts;
  };

  // Save shortcut customization
  const saveShortcut = async (actionId: string, combo: KeyCombo) => {
    const conflicts = checkConflicts(combo);

    if (conflicts.length > 1) {
      setState(prev => ({
        ...prev,
        recordingConflict: actionId,
        conflicts: new Map([[actionId, conflicts]]),
      }));
      return;
    }

    try {
      setState(prev => ({
        ...prev,
        customizations: new Map(prev.customizations).set(actionId, combo),
        actions: prev.actions.map(a =>
          a.id === actionId ? { ...a, combo } : a
        ),
        recordingConflict: null,
        conflicts: new Map(),
      }));

      await invoke('save_shortcut_customization', {
        action_id: actionId,
        combo: combo,
      });

      setRecordingShortcut(null);
      setRecordedCombo(null);
    } catch (error) {
      console.error('Failed to save shortcut:', error);
    }
  };

  // Reset shortcut to default
  const resetShortcut = (actionId: string) => {
    const defaultAction = DEFAULT_SHORTCUTS.find(a => a.id === actionId);
    if (!defaultAction) return;

    setState(prev => ({
      ...prev,
      customizations: new Map(prev.customizations),
      actions: prev.actions.map(a =>
        a.id === actionId ? { ...defaultAction } : a
      ),
    }));

    invoke('reset_shortcut_to_default', { action_id: actionId });
  };

  // Format key combo for display
  const formatCombo = (combo: KeyCombo): string => {
    const parts: string[] = [];
    if (combo.ctrl) parts.push('Ctrl');
    if (combo.shift) parts.push('Shift');
    if (combo.alt) parts.push('Alt');
    parts.push(combo.key);
    return parts.join('+');
  };

  // Get shortcuts for category
  const shortcutsByCategory = useMemo(() => {
    const grouped = new Map<string, ShortcutAction[]>();
    for (const action of state.actions) {
      if (!grouped.has(action.category)) {
        grouped.set(action.category, []);
      }
      grouped.get(action.category)!.push(action);
    }
    return grouped;
  }, [state.actions]);

  return (
    <div className="keyboard-shortcut-map">
      {/* Cheatsheet Toggle */}
      <div className="cheatsheet-header">
        <h3>⌨️ Keyboard Shortcuts</h3>
        <button
          className="btn-cheatsheet"
          onClick={() => setShowCheatsheet(!showCheatsheet)}
          title="Ctrl+Shift+? or F1"
        >
          {showCheatsheet ? 'Hide' : 'Show'} Cheatsheet
        </button>
      </div>

      {showCheatsheet && (
        <div className="cheatsheet-overlay">
          <div className="cheatsheet-content">
            <div className="cheatsheet-header-inner">
              <h2>Keyboard Shortcuts Cheatsheet</h2>
              <button
                className="btn-close"
                onClick={() => setShowCheatsheet(false)}
              >
                ✕
              </button>
            </div>

            <div className="cheatsheet-categories">
              {Array.from(shortcutsByCategory.entries()).map(([category, shortcuts]) => (
                <div key={category} className="cheatsheet-category">
                  <h4>{category}</h4>
                  <div className="shortcut-grid">
                    {shortcuts.map(shortcut => (
                      <div key={shortcut.id} className="shortcut-row">
                        <div className="shortcut-keys">
                          <kbd>{formatCombo(shortcut.combo)}</kbd>
                        </div>
                        <div className="shortcut-desc">
                          <div className="shortcut-name">{shortcut.name}</div>
                          <div className="shortcut-help">{shortcut.description}</div>
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>
      )}

      {/* Customization Panel */}
      <div className="customization-panel">
        <h4>Customize Shortcuts</h4>

        {/* Category Filter */}
        <div className="category-filter">
          <select
            value={selectedCategory}
            onChange={e => setSelectedCategory(e.target.value)}
          >
            <option value="">All Categories</option>
            {state.categories.map(cat => (
              <option key={cat} value={cat}>
                {cat}
              </option>
            ))}
          </select>
        </div>

        {/* Shortcuts List */}
        <div className="shortcuts-list">
          {state.actions
            .filter(s => !selectedCategory || s.category === selectedCategory)
            .map(action => (
              <div key={action.id} className="shortcut-item">
                <div className="shortcut-info">
                  <div className="shortcut-name">{action.name}</div>
                  <div className="shortcut-desc">{action.description}</div>
                  <div className="shortcut-category">{action.category}</div>
                </div>

                <div className="shortcut-control">
                  {recordingShortcut === action.id ? (
                    <div className="recording-input">
                      <div className="recording-status">
                        Recording... Press any key combination
                      </div>
                      {recordedCombo && (
                        <div className="recorded-combo">
                          <code>{formatCombo(recordedCombo)}</code>
                          <div className="recording-buttons">
                            <button
                              className="btn-confirm"
                              onClick={() => saveShortcut(action.id, recordedCombo)}
                            >
                              ✓ Confirm
                            </button>
                            <button
                              className="btn-cancel"
                              onClick={() => {
                                setRecordingShortcut(null);
                                setRecordedCombo(null);
                              }}
                            >
                              ✕ Cancel
                            </button>
                          </div>
                        </div>
                      )}
                    </div>
                  ) : (
                    <>
                      <kbd className="display-combo">
                        {formatCombo(action.combo)}
                      </kbd>
                      {action.customizable && (
                        <div className="shortcut-buttons">
                          <button
                            className="btn-edit"
                            onClick={() => setRecordingShortcut(action.id)}
                            title="Edit shortcut"
                          >
                            ✏️
                          </button>
                          <button
                            className="btn-reset"
                            onClick={() => resetShortcut(action.id)}
                            title="Reset to default"
                          >
                            ↶
                          </button>
                        </div>
                      )}
                    </>
                  )}
                </div>
              </div>
            ))}
        </div>

        {/* Conflict Warning */}
        {state.recordingConflict && state.conflicts.size > 0 && (
          <div className="conflict-warning">
            <h4>⚠️ Shortcut Conflict</h4>
            <p>This key combination is already used by:</p>
            <ul>
              {Array.from(state.conflicts.values())
                .flat()
                .map(conflictId => {
                  const conflictAction = state.actions.find(a => a.id === conflictId);
                  return (
                    <li key={conflictId}>
                      <strong>{conflictAction?.name}</strong>
                      {conflictId !== state.recordingConflict && ' (will be replaced)'}
                    </li>
                  );
                })}
            </ul>
            <div className="conflict-buttons">
              <button
                className="btn-confirm"
                onClick={() => {
                  if (recordedCombo) {
                    saveShortcut(state.recordingConflict!, recordedCombo);
                  }
                }}
              >
                Replace
              </button>
              <button
                className="btn-cancel"
                onClick={() => {
                  setState(prev => ({
                    ...prev,
                    recordingConflict: null,
                    conflicts: new Map(),
                  }));
                  setRecordingShortcut(null);
                  setRecordedCombo(null);
                }}
              >
                Cancel
              </button>
            </div>
          </div>
        )}

        {/* Quick Tips */}
        <div className="quick-tips">
          <h5>💡 Quick Tips</h5>
          <ul>
            <li>Press <strong>F1</strong> or <kbd>Ctrl+?</kbd> anywhere to show this cheatsheet</li>
            <li>Edit shortcut by clicking the ✏️ icon and pressing your desired key combination</li>
            <li>Conflicts are automatically detected and can be resolved</li>
            <li>Use <kbd>Ctrl</kbd>, <kbd>Shift</kbd>, <kbd>Alt</kbd> modifiers for power users</li>
          </ul>
        </div>
      </div>

      <style>{`
        .keyboard-shortcut-map {
          background: white;
          border-radius: 8px;
          padding: 20px;
        }

        .cheatsheet-header {
          display: flex;
          justify-content: space-between;
          align-items: center;
          margin-bottom: 20px;
        }

        .btn-cheatsheet {
          padding: 8px 16px;
          background: #0066cc;
          color: white;
          border: none;
          border-radius: 4px;
          cursor: pointer;
          font-size: 14px;
        }

        .btn-cheatsheet:hover {
          background: #0052a3;
        }

        .cheatsheet-overlay {
          position: fixed;
          top: 0;
          left: 0;
          right: 0;
          bottom: 0;
          background: rgba(0, 0, 0, 0.5);
          display: flex;
          align-items: center;
          justify-content: center;
          z-index: 10000;
          padding: 20px;
        }

        .cheatsheet-content {
          background: white;
          border-radius: 8px;
          max-width: 900px;
          max-height: 80vh;
          overflow-y: auto;
          padding: 20px;
        }

        .cheatsheet-header-inner {
          display: flex;
          justify-content: space-between;
          align-items: center;
          margin-bottom: 20px;
        }

        .btn-close {
          background: none;
          border: none;
          font-size: 24px;
          cursor: pointer;
          color: #666;
        }

        .cheatsheet-categories {
          display: grid;
          grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
          gap: 20px;
        }

        .cheatsheet-category h4 {
          margin: 0 0 12px 0;
          font-size: 14px;
          font-weight: 600;
          text-transform: uppercase;
          color: #333;
        }

        .shortcut-grid {
          display: flex;
          flex-direction: column;
          gap: 8px;
        }

        .shortcut-row {
          display: flex;
          gap: 12px;
          padding: 8px;
          background: #f9f9f9;
          border-radius: 4px;
        }

        .shortcut-keys {
          flex-shrink: 0;
          min-width: 120px;
        }

        .shortcut-keys kbd {
          display: inline-block;
          padding: 4px 8px;
          background: white;
          border: 1px solid #ddd;
          border-radius: 3px;
          font-family: monospace;
          font-size: 11px;
          font-weight: 500;
          color: #333;
        }

        .shortcut-desc {
          flex: 1;
        }

        .shortcut-name {
          font-weight: 600;
          font-size: 12px;
          margin-bottom: 2px;
        }

        .shortcut-help {
          font-size: 11px;
          color: #666;
        }

        .customization-panel {
          margin-top: 20px;
        }

        .customization-panel h4 {
          margin: 0 0 12px 0;
          font-size: 16px;
          font-weight: 600;
        }

        .category-filter {
          margin-bottom: 16px;
        }

        .category-filter select {
          width: 200px;
          padding: 8px;
          border: 1px solid #ddd;
          border-radius: 4px;
          font-size: 14px;
        }

        .shortcuts-list {
          display: flex;
          flex-direction: column;
          gap: 12px;
          max-height: 400px;
          overflow-y: auto;
        }

        .shortcut-item {
          display: flex;
          justify-content: space-between;
          align-items: center;
          padding: 12px;
          background: #f9f9f9;
          border-radius: 6px;
          border-left: 3px solid #0066cc;
        }

        .shortcut-info {
          flex: 1;
        }

        .shortcut-item .shortcut-name {
          font-weight: 600;
          margin-bottom: 4px;
        }

        .shortcut-item .shortcut-desc {
          font-size: 12px;
          color: #666;
          margin-bottom: 4px;
        }

        .shortcut-category {
          font-size: 11px;
          color: #999;
        }

        .shortcut-control {
          display: flex;
          align-items: center;
          gap: 8px;
        }

        .display-combo {
          display: inline-block;
          padding: 4px 12px;
          background: white;
          border: 1px solid #ddd;
          border-radius: 4px;
          font-family: monospace;
          font-size: 13px;
          font-weight: 500;
          min-width: 150px;
          text-align: center;
        }

        .shortcut-buttons {
          display: flex;
          gap: 4px;
        }

        .btn-edit,
        .btn-reset {
          padding: 4px 8px;
          background: white;
          border: 1px solid #ddd;
          border-radius: 4px;
          cursor: pointer;
          font-size: 14px;
          transition: all 0.2s;
        }

        .btn-edit:hover,
        .btn-reset:hover {
          background: #f0f0f0;
        }

        .recording-input {
          background: #fffacd;
          border: 2px solid #ffd700;
          border-radius: 4px;
          padding: 12px;
          min-width: 300px;
        }

        .recording-status {
          text-align: center;
          font-size: 14px;
          font-weight: 600;
          margin-bottom: 12px;
          color: #ff6600;
        }

        .recorded-combo {
          display: flex;
          align-items: center;
          gap: 12px;
        }

        .recorded-combo code {
          flex: 1;
          display: block;
          padding: 8px;
          background: white;
          border: 1px solid #ddd;
          border-radius: 4px;
          font-family: monospace;
          text-align: center;
          font-size: 14px;
          font-weight: 500;
        }

        .recording-buttons {
          display: flex;
          gap: 8px;
        }

        .btn-confirm,
        .btn-cancel {
          padding: 6px 12px;
          border: none;
          border-radius: 4px;
          cursor: pointer;
          font-size: 12px;
          font-weight: 500;
          transition: all 0.2s;
        }

        .btn-confirm {
          background: #28a745;
          color: white;
        }

        .btn-confirm:hover {
          background: #218838;
        }

        .btn-cancel {
          background: #dc3545;
          color: white;
        }

        .btn-cancel:hover {
          background: #c82333;
        }

        .conflict-warning {
          background: #fff3cd;
          border: 1px solid #ffc107;
          border-radius: 6px;
          padding: 12px;
          margin-top: 12px;
        }

        .conflict-warning h4 {
          margin: 0 0 8px 0;
          color: #856404;
        }

        .conflict-warning p {
          margin: 0 0 8px 0;
          font-size: 13px;
          color: #856404;
        }

        .conflict-warning ul {
          list-style-position: inside;
          margin: 0 0 12px 0;
          padding-left: 0;
        }

        .conflict-warning li {
          font-size: 12px;
          color: #856404;
          margin-bottom: 4px;
        }

        .conflict-buttons {
          display: flex;
          gap: 8px;
        }

        .quick-tips {
          background: #e7f3ff;
          border: 1px solid #b3d9ff;
          border-radius: 6px;
          padding: 12px;
          margin-top: 12px;
        }

        .quick-tips h5 {
          margin: 0 0 8px 0;
          font-size: 13px;
          color: #004085;
        }

        .quick-tips ul {
          list-style: none;
          margin: 0;
          padding: 0;
        }

        .quick-tips li {
          font-size: 12px;
          color: #004085;
          margin-bottom: 6px;
          line-height: 1.4;
        }

        .quick-tips kbd {
          display: inline-block;
          padding: 2px 6px;
          background: white;
          border: 1px solid #b3d9ff;
          border-radius: 2px;
          font-family: monospace;
          font-size: 11px;
          margin: 0 2px;
        }
      `}</style>
    </div>
  );
};

export default KeyboardShortcutMap;
