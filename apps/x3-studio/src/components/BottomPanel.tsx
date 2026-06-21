import { useLayoutStore } from '../store';
import XTerminal from './terminal/XTerminal';
import ProblemsPanel from './panels/ProblemsPanel';
import OutputPanel from './panels/OutputPanel';
import ProofPanel from './panels/ProofPanel';
import SecurityPanel from './panels/SecurityPanel';
import DebuggerPanel from './panels/DebuggerPanel';
import AdapterPanel from './panels/AdapterPanel';
import RelayerPanel from './panels/RelayerPanel';
import ValidatorPanel from './panels/ValidatorPanel';
import ChainHealthPanel from './panels/ChainHealthPanel';
import NetworkProfilerPanel from './panels/NetworkProfilerPanel';
import TestRunnerPanel from './panels/TestRunnerPanel';
import ForgeCoveragePanel from './panels/ForgeCoveragePanel';
import TpsBenchmarkPanel from './panels/TpsBenchmarkPanel';
import PermissionsPanel from './panels/PermissionsPanel';
import MultiWindowPanel from './panels/MultiWindowPanel';

const DEFAULT_BOTTOM_TABS = [
  { id: 'terminal', label: 'TERMINAL' },
  { id: 'problems', label: 'PROBLEMS' },
  { id: 'output', label: 'OUTPUT' },
  { id: 'proof', label: 'PROOF' },
];

const BOTTOM_PANEL_MAP: Record<string, React.ReactNode> = {
  'terminal': <XTerminal />,
  'problems': <ProblemsPanel />,
  'output': <OutputPanel />,
  'proof': <div style={{ padding: 8, fontSize: 'var(--font-size-sm)', color: 'var(--text-secondary)' }}>Switch to Proof Mode sidebar for full proof interface.</div>,
  'security': <SecurityPanel />,
  'debugger': <DebuggerPanel />,
  'adapters': <AdapterPanel />,
  'relayers': <RelayerPanel />,
  'validators': <ValidatorPanel />,
  'chain-health': <ChainHealthPanel />,
  'network-profiler': <NetworkProfilerPanel />,
  'test-runner': <TestRunnerPanel />,
  'forge-coverage': <ForgeCoveragePanel />,
  'tps-benchmark': <TpsBenchmarkPanel />,
  'permissions': <PermissionsPanel />,
  'multi-window': <MultiWindowPanel />,
};

const MOVABLE_PANEL_NAMES: Record<string, { id: string; label: string }> = {
  'security': { id: 'security', label: 'SECURITY' },
  'debugger': { id: 'debugger', label: 'DEBUGGER' },
  'adapters': { id: 'adapters', label: 'ADAPTERS' },
  'relayers': { id: 'relayers', label: 'RELAYERS' },
  'validators': { id: 'validators', label: 'VALIDATORS' },
  'chain-health': { id: 'chain-health', label: 'CHAIN HEALTH' },
  'network-profiler': { id: 'network-profiler', label: 'NETWORK' },
  'test-runner': { id: 'test-runner', label: 'TESTS' },
  'forge-coverage': { id: 'forge-coverage', label: 'COVERAGE' },
  'tps-benchmark': { id: 'tps-benchmark', label: 'TPS BENCH' },
  'permissions': { id: 'permissions', label: 'PERMISSIONS' },
  'multi-window': { id: 'multi-window', label: 'WINDOWS' },
};

export default function BottomPanel() {
  const bottomPanel = useLayoutStore(s => s.bottomPanel);
  const setBottomPanel = useLayoutStore(s => s.setBottomPanel);
  const toggleBottom = useLayoutStore(s => s.toggleBottom);
  const bottomPanels = useLayoutStore(s => s.bottomPanels);

  const allTabs = [
    ...DEFAULT_BOTTOM_TABS,
    ...bottomPanels
      .filter(id => MOVABLE_PANEL_NAMES[id])
      .map(id => MOVABLE_PANEL_NAMES[id]),
  ];

  const currentContent = BOTTOM_PANEL_MAP[bottomPanel] || (
    <div style={{ padding: 8, fontSize: 'var(--font-size-sm)', color: 'var(--text-secondary)' }}>
      Select a panel from the bottom tabs.
    </div>
  );

  return (
    <div className="bottom-panel">
      <div className="bottom-tabs">
        {allTabs.map(t => (
          <div
            key={t.id}
            className={`bottom-tab ${bottomPanel === t.id ? 'active' : ''}`}
            onClick={() => setBottomPanel(t.id)}
          >
            {t.label}
          </div>
        ))}
        <div style={{ flex: 1 }} />
        {bottomPanels.length > 0 && (
          <span style={{ fontSize: 9, color: 'var(--text-muted)', marginRight: 8 }}>
            {bottomPanels.length} pinned
          </span>
        )}
        <div className="bottom-tab" onClick={toggleBottom} style={{ color: 'var(--text-muted)' }}>
          ✕
        </div>
      </div>
      <div className="bottom-content">
        {currentContent}
      </div>
    </div>
  );
}
