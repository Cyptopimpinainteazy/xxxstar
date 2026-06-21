import React, { useState, useCallback, lazy, Suspense } from 'react';
import { SceneManager } from './scene/SceneManager';
import { AgentPanel } from './ui/AgentPanel';
import { Overlay } from './ui/Overlay';
import { FoundryPanel } from './ui/FoundryPanel';
import { useAgentStore } from './agents/AgentStore';
import { useBlockStore } from './blockchain/BlockStore';
import DesktopIcons, { PanelTab } from './ui/DesktopIcons';

// Lazy-loaded panels — only loaded when user navigates to them
const ValidatorGlobePanel = lazy(() => import('./ui/panels/ValidatorGlobePanel'));
const NetworkOverviewPanel = lazy(() => import('./ui/panels/NetworkOverviewPanel'));
const SwarmActivityPanel = lazy(() => import('./ui/panels/SwarmActivityPanel'));
const SupplyDashboardPanel = lazy(() => import('./ui/panels/SupplyDashboardPanel'));
const CrossVmActivityPanel = lazy(() => import('./ui/panels/CrossVmActivityPanel'));
const Phase5GovernancePanel = lazy(() => import('./ui/panels/Phase5GovernancePanel'));
const WalletPanel = lazy(() => import('./ui/panels/WalletPanel'));
const SwapPanel = lazy(() => import('./ui/panels/SwapPanel'));
const IntelligencePanel = lazy(() => import('./ui/panels/IntelligencePanel'));
const ExplorerPanel = lazy(() => import('./ui/panels/ExplorerPanel'));

const TABS: { id: PanelTab; label: string; icon: string }[] = [
  { id: 'arena', label: 'Arena', icon: '🎮' },
  { id: 'validators', label: 'Validators', icon: '🌐' },
  { id: 'network', label: 'Network', icon: '🔗' },
  { id: 'swarm', label: 'Swarm', icon: '🐝' },
  { id: 'supply', label: 'Supply', icon: '💰' },
  { id: 'crossvm', label: 'Cross-VM', icon: '🌉' },
  { id: 'phase5', label: 'Governance', icon: '🏛️' },
  { id: 'foundry', label: 'Foundry', icon: '⚒️' },
  { id: 'wallet', label: 'Wallet', icon: '💼' },
  { id: 'swap', label: 'Swap', icon: '🔄' },
  { id: 'intelligence', label: 'Intelligence', icon: '🧠' },
  { id: 'explorer', label: 'Explorer', icon: '🔍' },
];

function PanelLoader({ children }: { children: React.ReactNode }) {
  return (
    <Suspense
      fallback={
        <div className="w-full h-full flex items-center justify-center text-gray-400">
          Loading panel...
        </div>
      }
    >
      {children}
    </Suspense>
  );
}

function ActivePanel({ tab }: { tab: PanelTab }) {
  switch (tab) {
    case 'validators':
      return <PanelLoader><ValidatorGlobePanel /></PanelLoader>;
    case 'network':
      return <PanelLoader><NetworkOverviewPanel /></PanelLoader>;
    case 'swarm':
      return <PanelLoader><SwarmActivityPanel /></PanelLoader>;
    case 'supply':
      return <PanelLoader><SupplyDashboardPanel /></PanelLoader>;
    case 'crossvm':
      return <PanelLoader><CrossVmActivityPanel /></PanelLoader>;
    case 'phase5':
      return <PanelLoader><Phase5GovernancePanel /></PanelLoader>;
    case 'foundry':
      return <PanelLoader><FoundryPanel onClose={() => {}} /></PanelLoader>;
    case 'wallet':
      return <PanelLoader><WalletPanel /></PanelLoader>;
    case 'swap':
      return <PanelLoader><SwapPanel /></PanelLoader>;
    case 'intelligence':
      return <PanelLoader><IntelligencePanel /></PanelLoader>;
    case 'explorer':
      return <PanelLoader><ExplorerPanel /></PanelLoader>;
    default:
      return null;
  }
}

export function App() {
  const [selectedAgentId, setSelectedAgentId] = useState<string | null>(null);
  const [showOverlay, setShowOverlay] = useState(true);
  const [activeTab, setActiveTab] = useState<PanelTab>('arena');
  const agents = useAgentStore((s) => s.agents);
  const blocks = useBlockStore((s) => s.recentBlocks);

  const handleAgentSelect = useCallback((id: string) => {
    setSelectedAgentId(id);
  }, []);

  const handleAgentDeselect = useCallback(() => {
    setSelectedAgentId(null);
  }, []);

  const selectedAgent = selectedAgentId
    ? agents.find((a) => a.id === selectedAgentId) ?? null
    : null;

  const showArena = activeTab === 'arena';

  return (
    <div className="relative w-screen h-screen overflow-hidden bg-transparent">
      {/* Tab bar — top nav */}
      <div className="absolute top-0 left-0 right-0 z-30 flex items-center gap-1 px-4 py-2 bg-black/70 backdrop-blur border-b border-white/5">
        {TABS.map((tab) => (
          <button
            key={tab.id}
            onClick={() => setActiveTab(tab.id)}
            className={`px-3 py-1 text-xs rounded transition-colors flex items-center gap-1.5 ${
              activeTab === tab.id
                ? 'bg-cyan-500/20 text-cyan-300 border border-cyan-500/30'
                : 'text-gray-500 hover:text-gray-300 hover:bg-white/5'
            }`}
          >
            <span>{tab.icon}</span>
            <span>{tab.label}</span>
          </button>
        ))}
        {/* Toggle HUD only visible in arena mode */}
        {showArena && (
          <button
            className="ml-auto px-2 py-1 text-xs text-gray-500 hover:text-gray-300"
            onClick={() => setShowOverlay(!showOverlay)}
          >
            {showOverlay ? 'Hide HUD' : 'Show HUD'}
          </button>
        )}
      </div>

      {/* Three.js Scene — full viewport, only when arena tab active */}
      {showArena && (
        <div className="absolute inset-0 z-0">
          <SceneManager
            selectedAgentId={selectedAgentId}
            onAgentSelect={handleAgentSelect}
            onAgentDeselect={handleAgentDeselect}
          />
        </div>
      )}

      {/* Panel views */}
      {!showArena && (
        <div className="absolute inset-0 top-[37px] z-10 overflow-y-auto bg-gray-950">
          <ActivePanel tab={activeTab} />
        </div>
      )}

      {/* HUD Overlay — top bar (arena only) */}
      {showArena && showOverlay && (
        <div className="absolute top-[37px] left-0 right-0 z-10 pointer-events-none">
          <Overlay
            agents={agents}
            blocks={blocks}
            onToggleOverlay={() => setShowOverlay(false)}
          />
        </div>
      )}

      {/* Agent Panel — side bar (arena only) */}
      {showArena && selectedAgent && (
        <div className="absolute top-[37px] right-0 z-20 h-full pointer-events-none">
          <AgentPanel
            agent={selectedAgent}
            onClose={handleAgentDeselect}
          />
        </div>
      )}

      {/* Desktop Icons Dock — always on top */}
      <DesktopIcons onNavigate={(tab) => setActiveTab(tab)} />
    </div>
  );
}