import { useState } from 'react';
import { Sidebar, type PanelId } from './components/Sidebar';
import { ExplorerPanel } from './components/ExplorerPanel';
import { EditorPanel } from './components/EditorPanel';
import { TerminalPanel } from './components/TerminalPanel';
import { NetworkPanel } from './components/NetworkPanel';
import { AccountsPanel } from './components/AccountsPanel';
import { ContractsPanel } from './components/ContractsPanel';
import { FileExplorerPanel } from './components/FileExplorerPanel';
import { TemplatesPanel } from './components/TemplatesPanel';
import { ABIPanel } from './components/ABIPanel';
import { CompilerPanel } from './components/CompilerPanel';
import { RpcConsole } from './components/RpcConsole';
import { TxBuilderPanel } from './components/TxBuilderPanel';
import { DeployPanel } from './components/DeployPanel';
import { InspectorPanel } from './components/InspectorPanel';
import { EventsPanel } from './components/EventsPanel';
import { SearchBar } from './components/SearchBar';
import { Blocks, Layout } from 'lucide-react';
import './App.css';

function App() {
  const [activePanel, setActivePanel] = useState<PanelId>('files');

  const handleNavigate = (panel: string, id?: string) => {
    if (panel === 'explorer') setActivePanel('explorer');
    else if (panel === 'editor') setActivePanel('editor');
    else if (panel === 'network') setActivePanel('network');
  };

  const renderPanel = () => {
    switch (activePanel) {
      case 'explorer': return <ExplorerPanel />;
      case 'editor': return <EditorPanel />;
      case 'terminal': return <TerminalPanel />;
      case 'network': return <NetworkPanel />;
      case 'accounts': return <AccountsPanel />;
      case 'contracts': return <ContractsPanel />;
      case 'files': return <FileExplorerPanel />;
      case 'templates': return <TemplatesPanel />;
      case 'abis': return <ABIPanel />;
      case 'compiler': return <CompilerPanel />;
      case 'rpc': return <RpcConsole />;
      case 'txbuilder': return <TxBuilderPanel />;
      case 'deploy': return <DeployPanel />;
      case 'inspector': return <InspectorPanel />;
      case 'events': return <EventsPanel />;
      default: return <FileExplorerPanel />;
    }
  };

  const panelTitles: Record<PanelId, string> = {
    explorer: 'X3 Chain Explorer',
    editor: 'Code Editor',
    terminal: 'Terminal',
    network: 'Network',
    accounts: 'Accounts & Keys',
    contracts: 'Deployed Contracts',
    files: 'File Browser',
    templates: 'X3 Templates',
    abis: 'Contract ABIs',
    compiler: 'Compiler',
    rpc: 'RPC Console',
    txbuilder: 'Transaction Builder',
    deploy: 'Contract Deployer',
    inspector: 'State Inspector',
    events: 'Event Logs',
  };

  return (
    <div id="ide-container">
      <Sidebar active={activePanel} onSelect={setActivePanel} />
      <div id="main-area">
        <div id="title-bar">
          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <Blocks size={16} color="#569cd6" />
            <span id="app-title">Super IDE — {panelTitles[activePanel]}</span>
          </div>
          <SearchBar onNavigate={handleNavigate} />
        </div>
        <div id="content">
          {renderPanel()}
        </div>
      </div>
    </div>
  );
}

export default App;
