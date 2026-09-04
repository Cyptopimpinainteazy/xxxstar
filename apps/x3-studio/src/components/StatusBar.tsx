import { useEffect } from 'react';
import { useWorkspaceStore, useLayoutStore, useProofStore, useScoreboardStore, useEditorStore, useChainStore, useTpsMeterStore } from '../store';
import { pollChainStatus } from '../services/chainConnection';

export default function StatusBar() {
  const workspacePath = useWorkspaceStore(s => s.workspacePath);
  const branch = useWorkspaceStore(s => s.branch);
  const sidebarVisible = useLayoutStore(s => s.sidebarVisible);
  const toggleSidebar = useLayoutStore(s => s.toggleSidebar);
  const proofCount = useProofStore(s => s.records.length);
  const score = useScoreboardStore(s => s.totalScore);
  const tabs = useEditorStore(s => s.tabs);
  const dirtyCount = tabs.filter(t => t.dirty).length;
  const chain = useChainStore(s => s.chain);
  const tps = useTpsMeterStore(s => s.currentTps);
  const setSidebarPanel = useLayoutStore(s => s.setSidebarPanel);

  useEffect(() => {
    if (workspacePath) {
      pollChainStatus();
      const interval = setInterval(pollChainStatus, 10000);
      return () => clearInterval(interval);
    }
  }, [workspacePath]);

  if (!workspacePath) {
    return (
      <div className="status-bar">
        <span className="item">X3 Studio v0.1.0</span>
        <div className="spacer" />
        <span className="item">No workspace</span>
      </div>
    );
  }

  return (
    <div className="status-bar">
      <span className="item" onClick={toggleSidebar} style={{ cursor: 'pointer' }}>
        {sidebarVisible ? '◀' : '▶'} {branch}
      </span>
      <span className="item">
        <span className={`status-dot ${dirtyCount > 0 ? 'warn' : 'ok'}`} />
        {dirtyCount > 0 ? `${dirtyCount} unsaved` : 'saved'}
      </span>
      {chain && (
        <span className="item" onClick={() => setSidebarPanel('chain-health')} style={{ cursor: 'pointer' }}
          title={`${chain.chainId} | Block ${chain.blockNumber} | ${chain.latency}ms`}>
          <span className={`status-dot ${chain.connected ? 'ok' : 'fail'}`} />
          {chain.connected ? `${chain.chainId} #${chain.blockNumber}` : 'Disconnected'}
        </span>
      )}
      {proofCount > 0 && <span className="item">✓ {proofCount} proofs</span>}
      <div className="spacer" />
      <span className="item">Score: {score}%</span>
      {tps > 0 && <span className="item" onClick={() => setSidebarPanel('tps-meter')} style={{ cursor: 'pointer' }} title="Current TPS">TPS: {tps.toFixed(1)}</span>}
      <span className="item">{branch}</span>
    </div>
  );
}
