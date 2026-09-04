import { useState, useCallback } from 'react';
import { useLayoutStore, useWorkspaceStore } from '../store';
import type { PanelId } from '../types';

const DEFAULT_ICON_ORDER: { id: PanelId; label: string; icon: string }[] = [
  { id: 'control-center', label: 'Control Center', icon: '⊕' },
  { id: 'explorer', label: 'File Explorer', icon: '◲' },
  { id: 'project', label: 'Project', icon: '📁' },
  { id: 'git', label: 'Git', icon: '⎇' },
  { id: 'git-diff', label: 'Git Diff', icon: '∆' },
  { id: 'proof', label: 'Proof Mode', icon: '✓' },
  { id: 'scoreboard', label: 'Scoreboard', icon: 'Σ' },
  { id: 'scanner', label: 'Fake-Code Scanner', icon: '🔍' },
  { id: 'security', label: 'Security', icon: '🛡' },
  { id: 'debugger', label: 'Debugger', icon: '▶' },
  { id: 'adapters', label: 'Adapters', icon: '🔌' },
  { id: 'relayers', label: 'Relayers', icon: '↔' },
  { id: 'validators', label: 'Validators', icon: '✓' },
  { id: 'proof-ledger', label: 'Proof Ledger', icon: '📋' },
  { id: 'chain-health', label: 'Chain Health', icon: '♥' },
  { id: 'ai-agent', label: 'AI Agent', icon: '✦' },
  { id: 'test-runner', label: 'Test Runner', icon: '🧪' },
  { id: 'forge-coverage', label: 'Forge Coverage', icon: '📊' },
  { id: 'network-profiler', label: 'Network Profiler', icon: '🌐' },
  { id: 'gas-profiler', label: 'Gas Profiler', icon: '⛽' },
  { id: 'tps-benchmark', label: 'TPS Benchmark', icon: '⚡' },
  { id: 'contract-verification', label: 'Contract Verify', icon: '✓' },
  { id: 'cross-chain-sim', label: 'Cross-Chain Sim', icon: '↔' },
  { id: 'launch-cockpit', label: 'Launch Cockpit', icon: '🚀' },
  { id: 'deployment-config', label: 'Deployment Config', icon: '📄' },
  { id: 'dao-proposal', label: 'DAO Proposals', icon: '🗳' },
  { id: 'account-abstraction', label: 'AA Wallet', icon: '🔐' },
  { id: 'chain-config', label: 'Chain Config', icon: '⚙' },
  { id: 'graphql-explorer', label: 'GraphQL Explorer', icon: '◈' },
  { id: 'multi-window', label: 'Multi-Window', icon: '⊞' },
  { id: 'extension-manager', label: 'Extensions', icon: '📦' },
  { id: 'permissions', label: 'Permissions', icon: '🔒' },
  { id: 'keybindings', label: 'Keybindings', icon: '⌨' },
  { id: 'solidity-compiler', label: 'Solidity Compiler', icon: '◈' },
  { id: 'wasm-debugger', label: 'WASM Debugger', icon: '⚡' },
  { id: 'collab', label: 'Collaboration', icon: '⇄' },
  { id: 'registry-marketplace', label: 'Extension Marketplace', icon: '🏪' },
  { id: 'chain-sync', label: 'Chain Sync', icon: '⟳' },
  { id: 'tps-meter', label: 'TPS Meter', icon: '📈' },
  { id: 'output', label: 'Output', icon: '≡' },
  { id: 'settings', label: 'Settings', icon: '⚙' },
  { id: 'problems', label: 'Problems', icon: '!' },
];

function loadIconOrder(): typeof DEFAULT_ICON_ORDER {
  try {
    const saved = localStorage.getItem('x3studio-icon-order');
    if (saved) {
      const parsed = JSON.parse(saved);
      const lookup = new Map(DEFAULT_ICON_ORDER.map(e => [e.id, e]));
      return parsed.map((id: string) => lookup.get(id) || { id, label: id, icon: '?' }).filter(Boolean);
    }
  } catch {}
  return [...DEFAULT_ICON_ORDER];
}

function saveIconOrder(order: typeof DEFAULT_ICON_ORDER) {
  try { localStorage.setItem('x3studio-icon-order', JSON.stringify(order.map(e => e.id))); } catch {}
}

export default function Sidebar() {
  const sidebarPanel = useLayoutStore(s => s.sidebarPanel);
  const setSidebarPanel = useLayoutStore(s => s.setSidebarPanel);
  const workspacePath = useWorkspaceStore(s => s.workspacePath);
  const moveToBottom = useLayoutStore(s => s.moveToBottom);
  const bottomPanels = useLayoutStore(s => s.bottomPanels);
  const moveToSidebar = useLayoutStore(s => s.moveToSidebar);
  const [iconOrder, setIconOrder] = useState(loadIconOrder);
  const [dragIdx, setDragIdx] = useState<number | null>(null);
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; id: string } | null>(null);

  const handleOpenFolder = async () => {
    const dir = await window.x3studio.dialog.openDirectory();
    if (dir) {
      useWorkspaceStore.getState().setWorkspace(dir);
      await detectProject(dir);
    }
  };

  const handleDragStart = useCallback((e: React.DragEvent, idx: number) => {
    setDragIdx(idx);
    e.dataTransfer.effectAllowed = 'move';
    e.dataTransfer.setData('text/plain', String(idx));
  }, []);

  const handleDragOver = useCallback((e: React.DragEvent, idx: number) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = 'move';
    if (dragIdx === null || dragIdx === idx) return;
    const newOrder = [...iconOrder];
    const [moved] = newOrder.splice(dragIdx, 1);
    newOrder.splice(idx, 0, moved);
    setIconOrder(newOrder);
    setDragIdx(idx);
  }, [dragIdx, iconOrder]);

  const handleDragEnd = useCallback(() => {
    setDragIdx(null);
    saveIconOrder(iconOrder);
  }, [iconOrder]);

  const handleDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    setDragIdx(null);
    saveIconOrder(iconOrder);
  }, [iconOrder]);

  const handleContextMenu = useCallback((e: React.MouseEvent, id: string) => {
    e.preventDefault();
    setContextMenu({ x: e.clientX, y: e.clientY, id });
  }, []);

  const handleCloseContextMenu = useCallback(() => setContextMenu(null), []);

  return (
    <div className="sidebar-icons" onDragEnd={handleDragEnd} onDrop={handleDrop} onClick={handleCloseContextMenu}>
      <div className="sidebar-icon x3-logo" onClick={handleOpenFolder} title="Open Workspace">X3</div>
      <div className="sidebar-divider" />
      {iconOrder.map(({ id, label, icon }, idx) => {
        const isInBottom = bottomPanels.includes(id);
        return (
          <div
            key={id}
            className={`sidebar-icon ${sidebarPanel === id ? 'active' : ''} ${dragIdx === idx ? 'dragging' : ''} ${isInBottom ? 'in-bottom' : ''}`}
            onClick={() => setSidebarPanel(id)}
            onContextMenu={(e) => handleContextMenu(e, id)}
            title={label + (isInBottom ? ' (in bottom panel)' : '')}
            draggable
            onDragStart={(e) => handleDragStart(e, idx)}
            onDragOver={(e) => handleDragOver(e, idx)}
          >
            <span style={{ fontSize: 16 }}>{icon}</span>
            <span className="icon-label">{label}</span>
            {isInBottom && <span className="badge-bottom" title="Also in bottom panel">▽</span>}
          </div>
        );
      })}
      {contextMenu && (
        <div
          className="context-menu"
          style={{ position: 'fixed', left: contextMenu.x, top: contextMenu.y, zIndex: 1000 }}
          onClick={(e) => e.stopPropagation()}
        >
          {bottomPanels.includes(contextMenu.id) ? (
            <div className="context-menu-item" onClick={() => { moveToSidebar(contextMenu.id); setContextMenu(null); }}>
              Move to Sidebar
            </div>
          ) : (
            <div className="context-menu-item" onClick={() => { moveToBottom(contextMenu.id); setContextMenu(null); }}>
              Move to Bottom Panel
            </div>
          )}
          <div className="context-menu-item" onClick={() => { setSidebarPanel(contextMenu.id); setContextMenu(null); }}>
            Open Panel
          </div>
          <div className="context-menu-divider" />
          <div className="context-menu-item" onClick={handleCloseContextMenu}>Cancel</div>
        </div>
      )}
    </div>
  );
}

async function detectProject(dir: string) {
  const { exec } = window.x3studio.shell;
  const { exists } = window.x3studio.fs;
  const checks = {
    hasCargo: await exists(dir + '/Cargo.toml'),
    hasPackageJson: await exists(dir + '/package.json'),
    hasHardhat: await exists(dir + '/hardhat.config.ts') || await exists(dir + '/hardhat.config.js'),
    hasFoundry: await exists(dir + '/foundry.toml'),
    hasAnchor: await exists(dir + '/Anchor.toml'),
    hasPallets: await exists(dir + '/pallets'),
    hasContracts: await exists(dir + '/contracts') || await exists(dir + '/X3-contracts'),
    hasX3Files: false,
    hasX3Lang: await exists(dir + '/x3-lang'),
    hasRelayer: await exists(dir + '/relayer'),
    hasAdapters: await exists(dir + '/adapters'),
    hasProofLedger: await exists(dir + '/proof-ledger') || await exists(dir + '/x3-proof'),
    hasValidator: await exists(dir + '/validator'),
    hasDocker: await exists(dir + '/Dockerfile') || await exists(dir + '/docker-compose.yml'),
    hasGit: false,
    hasSubstrate: await exists(dir + '/runtime') || await exists(dir + '/node'),
    modules: [] as string[],
  };

  const { stdout } = await exec('find . -name "*.x3" -maxdepth 3 2>/dev/null | head -5', dir);
  checks.hasX3Files = stdout.trim().length > 0;
  const { stdout: gitOut } = await exec('git rev-parse --git-dir 2>/dev/null', dir);
  checks.hasGit = gitOut.trim().length > 0;

  const modules: string[] = [];
  if (checks.hasCargo) modules.push('Rust Workspace');
  if (checks.hasPackageJson) modules.push('Node.js');
  if (checks.hasHardhat) modules.push('Hardhat');
  if (checks.hasFoundry) modules.push('Foundry');
  if (checks.hasAnchor) modules.push('Anchor/SVM');
  if (checks.hasSubstrate) modules.push('Substrate');
  if (checks.hasX3Files) modules.push('x3-lang');
  if (checks.hasPallets) modules.push('Pallets');
  if (checks.hasContracts) modules.push('Smart Contracts');
  if (checks.hasX3Lang) modules.push('X3 Language');
  if (checks.hasRelayer) modules.push('Relayer');
  if (checks.hasAdapters) modules.push('Adapters');
  if (checks.hasProofLedger) modules.push('Proof Ledger');
  if (checks.hasValidator) modules.push('Validator');
  checks.modules = modules;
  useWorkspaceStore.getState().setDetection(checks);

  if (checks.hasGit) {
    const { stdout: branch } = await exec('git rev-parse --abbrev-ref HEAD', dir);
    useWorkspaceStore.getState().setBranch(branch.trim());
    const { stdout: status } = await exec('git status --porcelain', dir);
    const lines = status.trim().split('\n').filter(Boolean);
    useWorkspaceStore.getState().setGitStatus(lines.map((l: string) => ({ status: l.substring(0, 2).trim(), file: l.substring(3) })));
  }
}
