import React from 'react';
import { WebSocketProvider } from './services/websocket';
import ASTHeatmap from './components/ASTHeatmap';
import NodeVotes from './components/NodeVotes';
import RollbackButton from './components/RollbackButton';
import CounterfactualOverlay from './components/CounterfactualOverlay';
import MarketBlockIndicator from './components/MarketBlockIndicator';
import PipelineStatus from './components/PipelineStatus';

const App: React.FC = () => {
  return (
    <WebSocketProvider>
      <div style={{ display: 'flex', flexDirection: 'column', height: '100vh', padding: '10px', gap: '10px' }}>
        <header style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <h1 style={{ margin: 0 }}>🔥 md_supervisor Cockpit</h1>
          <div id="connection-status" style={{ fontSize: '12px', color: 'gray' }}>disconnected</div>
        </header>
        <div style={{ display: 'flex', flex: 1, gap: '10px' }}>
          <div style={{ flex: 3, display: 'flex', flexDirection: 'column', gap: '10px' }}>
            <ASTHeatmap />
            <PipelineStatus />
          </div>
          <div style={{ flex: 2, display: 'flex', flexDirection: 'column', gap: '5px' }}>
            <NodeVotes />
            <CounterfactualOverlay />
            <MarketBlockIndicator />
            <RollbackButton />
          </div>
        </div>
      </div>
    </WebSocketProvider>
  );
};

export default App;