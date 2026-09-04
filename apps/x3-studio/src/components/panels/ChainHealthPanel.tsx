import { useState, useEffect } from 'react';
import { useWorkspaceStore, useChainStore } from '../../store';
import { pollChainStatus, startChainPolling, stopChainPolling } from '../../services/chainConnection';

export default function ChainHealthPanel() {
  const workspacePath = useWorkspaceStore(s => s.workspacePath);
  const chain = useChainStore(s => s.chain);
  const history = useChainStore(s => s.history);
  const [polling, setPolling] = useState(false);

  useEffect(() => {
    if (workspacePath) {
      startChainPolling(5000);
      setPolling(true);
      return () => { stopChainPolling(); setPolling(false); };
    }
  }, [workspacePath]);

  const handlePoll = async () => {
    await pollChainStatus();
  };

  const togglePolling = () => {
    if (polling) {
      stopChainPolling();
      setPolling(false);
    } else {
      startChainPolling(5000);
      setPolling(true);
    }
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <div className="panel-header">
        <span>Chain Health</span>
        <div style={{ display: 'flex', gap: 4 }}>
          <button className="btn" onClick={handlePoll} style={{ fontSize: 10, padding: '2px 6px' }}>Poll Now</button>
          <button className="btn" onClick={togglePolling} style={{ fontSize: 10, padding: '2px 6px' }}>
            {polling ? 'Stop' : 'Start'} Polling
          </button>
        </div>
      </div>
      <div className="panel-body" style={{ padding: '8px', fontSize: 'var(--font-size-sm)' }}>
        {!chain ? (
          <div style={{ color: 'var(--text-muted)', textAlign: 'center', padding: 16 }}>
            No chain data. Poll or wait for automatic polling.
          </div>
        ) : (
          <>
            <div className="dashboard-grid" style={{ gridTemplateColumns: '1fr 1fr' }}>
              <div className="dashboard-card">
                <h3>Status</h3>
                <div style={{ marginTop: 4 }}>
                  <span className={`badge badge-${chain.connected ? 'pass' : 'fail'}`}>
                    {chain.connected ? 'Connected' : 'Disconnected'}
                  </span>
                </div>
              </div>
              <div className="dashboard-card">
                <h3>Chain ID</h3>
                <div className="value" style={{ fontFamily: 'var(--font-mono)', fontSize: 14 }}>{chain.chainId}</div>
              </div>
              <div className="dashboard-card">
                <h3>Latest Block</h3>
                <div className="value" style={{ fontFamily: 'var(--font-mono)', fontSize: 14 }}>#{chain.blockNumber.toLocaleString()}</div>
              </div>
              <div className="dashboard-card">
                <h3>Latency</h3>
                <div className="value" style={{ fontFamily: 'var(--font-mono)', fontSize: 14, color: chain.latency > 1000 ? 'var(--red)' : 'var(--green)' }}>
                  {chain.latency}ms
                </div>
              </div>
            </div>

            <div className="section-title">RPC Endpoint</div>
            <div style={{ fontFamily: 'var(--font-mono)', fontSize: 11, color: 'var(--text-muted)', marginBottom: 8 }}>
              {chain.rpcUrl}
            </div>

            <div className="section-title">Last Checked</div>
            <div style={{ color: 'var(--text-muted)', marginBottom: 8 }}>
              {new Date(chain.lastChecked).toLocaleTimeString()}
            </div>

            {history.length > 1 && (
              <>
                <div className="section-title">Latency History (last {history.length})</div>
                <div style={{ display: 'flex', gap: 2, alignItems: 'flex-end', height: 40, marginBottom: 8 }}>
                  {history.map((h, i) => {
                    const maxLatency = Math.max(...history.map(x => x.latency), 1);
                    const height = Math.max(4, (h.latency / maxLatency) * 36);
                    return (
                      <div key={i} style={{
                        flex: 1, height, background: h.connected ? 'var(--green)' : 'var(--red)',
                        borderRadius: '2px 2px 0 0', opacity: 0.3 + (i / history.length) * 0.7,
                      }} title={`Block ${h.blockNumber}: ${h.latency}ms`} />
                    );
                  })}
                </div>
              </>
            )}

            {!polling && (
              <div style={{ color: 'var(--text-muted)', fontStyle: 'italic' }}>
                Polling stopped. Click "Start Polling" to auto-refresh.
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}
