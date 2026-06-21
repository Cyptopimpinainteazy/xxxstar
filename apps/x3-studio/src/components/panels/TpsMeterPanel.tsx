import { useState, useEffect } from 'react';
import { useTpsMeterStore, useSettingsStore } from '../../store';

export default function TpsMeterPanel() {
  const snapshots = useTpsMeterStore(s => s.snapshots);
  const currentTps = useTpsMeterStore(s => s.currentTps);
  const currentBlock = useTpsMeterStore(s => s.currentBlock);
  const isPolling = useTpsMeterStore(s => s.isPolling);
  const addSnapshot = useTpsMeterStore(s => s.addSnapshot);
  const setCurrentTps = useTpsMeterStore(s => s.setCurrentTps);
  const setCurrentBlock = useTpsMeterStore(s => s.setCurrentBlock);
  const setPolling = useTpsMeterStore(s => s.setPolling);
  const rpcUrl = useSettingsStore(s => s.chainRpcUrl);
  const [localPolling, setLocalPolling] = useState(false);

  const poll = async () => {
    try {
      const result = await (window as any).x3studio.chain.monitorBlock(rpcUrl);
      if (result) {
        const tps = result.tps ?? 0;
        const blockNumber = result.blockNumber ?? 0;
        const txCount = result.txCount ?? 0;
        setCurrentTps(tps);
        setCurrentBlock(blockNumber);
        addSnapshot({ blockNumber, timestamp: Date.now(), tps, txCount });
      }
    } catch {
      // ignore polling errors
    }
  };

  useEffect(() => {
    if (localPolling) {
      poll();
      const interval = setInterval(poll, 3000);
      return () => clearInterval(interval);
    }
  }, [localPolling, rpcUrl]);

  const togglePolling = () => {
    const next = !localPolling;
    setLocalPolling(next);
    setPolling(next);
  };

  const reset = () => {
    useTpsMeterStore.setState({ snapshots: [], currentTps: 0, currentBlock: 0 });
  };

  const recent = snapshots.slice(-30);
  const minTps = recent.length > 0 ? Math.min(...recent.map(s => s.tps)) : 0;
  const maxTps = recent.length > 0 ? Math.max(...recent.map(s => s.tps)) : 0;
  const barMax = Math.max(maxTps, 1);

  const lastTxCount = snapshots.length > 0 ? snapshots[snapshots.length - 1].txCount : 0;

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <div className="panel-header">
        <span>TPS Meter</span>
        <div style={{ display: 'flex', gap: 4 }}>
          <button className="btn" onClick={togglePolling} style={{ fontSize: 10, padding: '2px 6px' }}>
            {localPolling ? 'Stop Polling' : 'Start Polling'}
          </button>
          <button className="btn" onClick={reset} style={{ fontSize: 10, padding: '2px 6px' }}>Reset</button>
        </div>
      </div>
      <div className="panel-body" style={{ padding: '8px', fontSize: 'var(--font-size-sm)' }}>
        {snapshots.length === 0 && !localPolling && (
          <div style={{ color: 'var(--text-muted)', textAlign: 'center', padding: 16 }}>
            No data. Click "Start Polling" to monitor TPS.
          </div>
        )}
        <div className="dashboard-grid" style={{ gridTemplateColumns: '1fr 1fr' }}>
          <div className="dashboard-card">
            <h3>Current TPS</h3>
            <div className="value" style={{ fontFamily: 'var(--font-mono)', fontSize: 24 }}>{currentTps.toFixed(1)}</div>
          </div>
          <div className="dashboard-card">
            <h3>Block</h3>
            <div className="value" style={{ fontFamily: 'var(--font-mono)', fontSize: 14 }}>#{currentBlock.toLocaleString()}</div>
          </div>
          <div className="dashboard-card">
            <h3>Last Tx Count</h3>
            <div className="value" style={{ fontFamily: 'var(--font-mono)', fontSize: 14 }}>{lastTxCount}</div>
          </div>
          <div className="dashboard-card">
            <h3>Snapshots</h3>
            <div className="value" style={{ fontFamily: 'var(--font-mono)', fontSize: 14 }}>{snapshots.length}</div>
          </div>
        </div>

        <div className="section-title">TPS History (last 30)</div>
        {recent.length > 0 && (
          <div style={{ marginBottom: 8 }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: 10, color: 'var(--text-muted)' }}>
              <span>min: {minTps.toFixed(1)}</span>
              <span>current: {currentTps.toFixed(1)}</span>
              <span>max: {maxTps.toFixed(1)}</span>
            </div>
            <div style={{ display: 'flex', gap: 1, alignItems: 'flex-end', height: 28, marginTop: 4 }}>
              {recent.map((s, i) => {
                const h = Math.max(3, (s.tps / barMax) * 24);
                return (
                  <div key={i} style={{
                    flex: 1, height, background: s.tps === currentTps ? 'var(--accent)' : 'var(--blue)',
                    borderRadius: '1px 1px 0 0', opacity: 0.4 + (i / recent.length) * 0.6,
                  }} title={`Block ${s.blockNumber}: ${s.tps.toFixed(1)} TPS`} />
                );
              })}
            </div>
            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: 9, color: 'var(--text-muted)', marginTop: 2 }}>
              <span>ago</span>
              <span>now</span>
            </div>
          </div>
        )}

        <div className="section-title">Recent Snapshots (last 10)</div>
        <div className="data-table">
          <table style={{ width: '100%', fontSize: 11 }}>
            <thead>
              <tr>
                <th>Block</th>
                <th>TPS</th>
                <th>Tx Count</th>
                <th>Time</th>
              </tr>
            </thead>
            <tbody>
              {snapshots.slice(-10).reverse().map((s, i) => (
                <tr key={i}>
                  <td>#{s.blockNumber}</td>
                  <td>{s.tps.toFixed(1)}</td>
                  <td>{s.txCount}</td>
                  <td>{new Date(s.timestamp).toLocaleTimeString()}</td>
                </tr>
              ))}
              {snapshots.length === 0 && (
                <tr><td colSpan={4} style={{ textAlign: 'center', color: 'var(--text-muted)' }}>No snapshots yet</td></tr>
              )}
            </tbody>
          </table>
        </div>

        {!localPolling && snapshots.length > 0 && (
          <div style={{ color: 'var(--text-muted)', fontStyle: 'italic', marginTop: 4 }}>
            Polling stopped. Click "Start Polling" to resume.
          </div>
        )}
      </div>
    </div>
  );
}
