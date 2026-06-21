import { useEffect, useCallback } from 'react';
import { Group as PanelGroup, Panel, Separator as PanelResizeHandle } from 'react-resizable-panels';
import { useWorkspaceStore, useLayoutStore, useSettingsStore, useExtensionStore } from './store';
import Sidebar from './components/Sidebar';
import StatusBar from './components/StatusBar';
import BottomPanel from './components/BottomPanel';
import EditorPanel from './components/editor/EditorPanel';
import ControlCenter from './components/panels/ControlCenter';
import ProofPanel from './components/panels/ProofPanel';
import ScoreboardPanel from './components/panels/ScoreboardPanel';
import ScannerPanel from './components/panels/ScannerPanel';
import SecurityPanel from './components/panels/SecurityPanel';
import AdapterPanel from './components/panels/AdapterPanel';
import RelayerPanel from './components/panels/RelayerPanel';
import ValidatorPanel from './components/panels/ValidatorPanel';
import ProofLedgerPanel from './components/panels/ProofLedgerPanel';
import ChainHealthPanel from './components/panels/ChainHealthPanel';
import DebuggerPanel from './components/panels/DebuggerPanel';
import GitDiffPanel from './components/panels/GitDiffPanel';
import AiAgentPanel from './components/panels/AiAgentPanel';
import LaunchCockpit from './components/panels/LaunchCockpit';
import GitPanel from './components/panels/GitPanel';
import FileExplorer from './components/explorer/FileExplorer';
import OutputPanel from './components/panels/OutputPanel';
import SettingsPanel from './components/panels/SettingsPanel';
import KeybindingsPanel from './components/panels/KeybindingsPanel';
import ProblemsPanel from './components/panels/ProblemsPanel';
import ProjectPanel from './components/panels/ProjectPanel';
import ExtensionManagerPanel from './components/panels/ExtensionManagerPanel';
import NetworkProfilerPanel from './components/panels/NetworkProfilerPanel';
import ForgeCoveragePanel from './components/panels/ForgeCoveragePanel';
import PermissionsPanel from './components/panels/PermissionsPanel';
import TestRunnerPanel from './components/panels/TestRunnerPanel';
import ContractVerificationPanel from './components/panels/ContractVerificationPanel';
import GasProfilerPanel from './components/panels/GasProfilerPanel';
import MultiWindowPanel from './components/panels/MultiWindowPanel';
import GraphQLExplorerPanel from './components/panels/GraphQLExplorerPanel';
import DeploymentConfigPanel from './components/panels/DeploymentConfigPanel';
import DaoProposalPanel from './components/panels/DaoProposalPanel';
import AccountAbstractionPanel from './components/panels/AccountAbstractionPanel';
import TpsBenchmarkPanel from './components/panels/TpsBenchmarkPanel';
import CrossChainSimulator from './components/panels/CrossChainSimulator';
import ChainConfigPanel from './components/panels/ChainConfigPanel';
import SolidityCompilerPanel from './components/panels/SolidityCompilerPanel';
import WasmDebuggerPanel from './components/panels/WasmDebuggerPanel';
import RegistryMarketplacePanel from './components/panels/RegistryMarketplacePanel';
import CollabPanel from './components/panels/CollabPanel';
import ChainSyncPanel from './components/panels/ChainSyncPanel';
import TpsMeterPanel from './components/panels/TpsMeterPanel';

function App() {
  const workspacePath = useWorkspaceStore(s => s.workspacePath);
  const sidebarPanel = useLayoutStore(s => s.sidebarPanel);
  const sidebarVisible = useLayoutStore(s => s.sidebarVisible);
  const bottomVisible = useLayoutStore(s => s.bottomVisible);
  const theme = useSettingsStore(s => s.theme);
  const extPanels = useExtensionStore(s => s.panels);

  useEffect(() => { document.documentElement.setAttribute('data-theme', theme); }, [theme]);

  const renderSidebarContent = useCallback(() => {
    if (!workspacePath) return (
      <div className="welcome-message">
        <div className="welcome-logo">X3</div>
        <h2>X3 Studio</h2>
        <p>Blockchain IDE for the X3 ecosystem</p>
        <button className="btn btn-primary welcome-btn" onClick={async () => {
          const dir = await window.x3studio.dialog.openDirectory();
          if (dir) useWorkspaceStore.getState().setWorkspace(dir);
        }}>📂 Open Folder</button>
        <p className="welcome-hint">Or click the <strong>X3</strong> logo in the activity bar</p>
        <div className="welcome-features">
          <span>✦ Monaco Editor</span>
          <span>✦ x3-lang Support</span>
          <span>✦ File Explorer</span>
          <span>✦ Integrated Terminal</span>
          <span>✦ AI Agent</span>
          <span>✦ Proof Mode</span>
        </div>
      </div>
    );

    const panelMap: Record<string, React.ReactNode> = {
      'control-center': <ControlCenter />,
      'project': <ProjectPanel />,
      'explorer': <FileExplorer />,
      'proof': <ProofPanel />,
      'scoreboard': <ScoreboardPanel />,
      'scanner': <ScannerPanel />,
      'security': <SecurityPanel />,
      'adapters': <AdapterPanel />,
      'relayers': <RelayerPanel />,
      'validators': <ValidatorPanel />,
      'proof-ledger': <ProofLedgerPanel />,
      'chain-health': <ChainHealthPanel />,
      'debugger': <DebuggerPanel />,
      'git-diff': <GitDiffPanel />,
      'ai-agent': <AiAgentPanel />,
      'launch-cockpit': <LaunchCockpit />,
      'git': <GitPanel />,
      'output': <OutputPanel />,
      'settings': <SettingsPanel />,
      'keybindings': <KeybindingsPanel />,
      'problems': <ProblemsPanel />,
      'extension-manager': <ExtensionManagerPanel />,
      'network-profiler': <NetworkProfilerPanel />,
      'forge-coverage': <ForgeCoveragePanel />,
      'permissions': <PermissionsPanel />,
      'test-runner': <TestRunnerPanel />,
      'contract-verification': <ContractVerificationPanel />,
      'gas-profiler': <GasProfilerPanel />,
      'multi-window': <MultiWindowPanel />,
      'graphql-explorer': <GraphQLExplorerPanel />,
      'deployment-config': <DeploymentConfigPanel />,
      'dao-proposal': <DaoProposalPanel />,
      'account-abstraction': <AccountAbstractionPanel />,
      'tps-benchmark': <TpsBenchmarkPanel />,
      'cross-chain-sim': <CrossChainSimulator />,
      'chain-config': <ChainConfigPanel />,
      'solidity-compiler': <SolidityCompilerPanel />,
      'wasm-debugger': <WasmDebuggerPanel />,
      'registry-marketplace': <RegistryMarketplacePanel />,
      'collab': <CollabPanel />,
      'chain-sync': <ChainSyncPanel />,
      'tps-meter': <TpsMeterPanel />,
    };

    if (panelMap[sidebarPanel]) return panelMap[sidebarPanel];

    const extPanel = extPanels.find(p => p.id === sidebarPanel);
    if (extPanel) {
      return <div className="panel-body" style={{ padding: 16 }}><h3>{extPanel.icon} {extPanel.label}</h3><p style={{ fontSize: 'var(--font-size-sm)', color: 'var(--text-muted)' }}>{extPanel.description}</p><p style={{ fontSize: 11, color: 'var(--text-muted)' }}>Extension panel loaded dynamically. v{extPanel.version}</p></div>;
    }

    return <ControlCenter />;
  }, [workspacePath, sidebarPanel, extPanels]);

  return (
    <div className="app-shell">
      <PanelGroup direction="horizontal" className="app-body">
        {/* Activity Bar - fixed width */}
        <div className="sidebar-icons-wrapper">
          <Sidebar />
        </div>
        {/* Sidebar Panel - resizable */}
        {sidebarVisible && (
          <>
            <Panel defaultSize={22} minSize={12} maxSize={50}>
              <div className="sidebar-content">
                {renderSidebarContent()}
              </div>
            </Panel>
            <PanelResizeHandle className="resize-handle" />
          </>
        )}
        {/* Editor + Bottom Panel */}
        <Panel minSize={30}>
          <PanelGroup direction="vertical" style={{ height: '100%' }}>
            <Panel minSize={20}>
              <EditorPanel />
            </Panel>
            {bottomVisible && (
              <>
                <PanelResizeHandle className="resize-handle resize-handle-horizontal" />
                <Panel defaultSize={25} minSize={8} maxSize={60}>
                  <BottomPanel />
                </Panel>
              </>
            )}
          </PanelGroup>
        </Panel>
      </PanelGroup>
      <StatusBar />
    </div>
  );
}

export default App;
