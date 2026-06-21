import { useState } from 'react';
import { Activity, Loader2, Search, RefreshCw } from 'lucide-react';

export function EventsPanel() {
  const [address, setAddress] = useState('');
  const [fromBlock, setFromBlock] = useState('0x0');
  const [toBlock, setToBlock] = useState('latest');
  const [logs, setLogs] = useState<unknown[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  const fetchLogs = async () => {
    setLoading(true);
    setError('');
    setLogs(null);
    try {
      const res = await fetch('http://127.0.0.1:8765/api/events', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          address: address || undefined,
          fromBlock,
          toBlock,
          topics: [],
        }),
      });
      const data = await res.json();
      if (Array.isArray(data)) setLogs(data);
      else setError(JSON.stringify(data));
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div style={{ padding: 16, color: '#d4d4d4', height: '100%', overflow: 'auto' }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 16 }}>
        <Activity size={20} />
        <h2 style={{ margin: 0, fontSize: 16, fontWeight: 600 }}>Event Logs</h2>
      </div>

      <div style={{ display: 'flex', gap: 8, marginBottom: 12, alignItems: 'end', flexWrap: 'wrap' }}>
        <div style={{ flex: 1, minWidth: 200 }}>
          <label style={{ fontSize: 12, color: '#888', display: 'block', marginBottom: 4 }}>Contract Address (optional)</label>
          <input value={address} onChange={e => setAddress(e.target.value)} placeholder="0x..."
            style={{ width: '100%', padding: '6px 8px', background: '#3c3c3c', border: '1px solid #555', borderRadius: 4, color: '#d4d4d4', fontSize: 12, fontFamily: 'monospace', outline: 'none' }}
          />
        </div>
        <div style={{ width: 120 }}>
          <label style={{ fontSize: 12, color: '#888', display: 'block', marginBottom: 4 }}>From Block</label>
          <input value={fromBlock} onChange={e => setFromBlock(e.target.value)} placeholder="0x0"
            style={{ width: '100%', padding: '6px 8px', background: '#3c3c3c', border: '1px solid #555', borderRadius: 4, color: '#d4d4d4', fontSize: 12, fontFamily: 'monospace', outline: 'none' }}
          />
        </div>
        <div>
          <button onClick={fetchLogs} disabled={loading}
            style={{ display: 'flex', alignItems: 'center', gap: 4, padding: '6px 16px', border: 'none', borderRadius: 4, background: '#0e639c', color: '#fff', cursor: 'pointer', fontSize: 12, opacity: loading ? 0.6 : 1 }}
          >{loading ? <Loader2 size={14} className="spin" /> : <Search size={14} />} Fetch</button>
        </div>
      </div>

      {error && <div style={{ padding: 12, background: '#3a1a1a', borderRadius: 8, border: '1px solid #f48771', color: '#f48771', fontSize: 12 }}>{error}</div>}

      {logs && (
        <div>
          <div style={{ fontSize: 12, color: '#888', marginBottom: 8 }}>{logs.length} log entries</div>
          {logs.length === 0 && <div style={{ color: '#666', fontStyle: 'italic' }}>No events found</div>}
          {logs.map((log: unknown, i) => {
            const l = log as Record<string, unknown>;
            return (
              <div key={i} style={{ padding: 10, marginBottom: 6, background: '#252526', borderRadius: 6, border: '1px solid #333', fontFamily: 'monospace', fontSize: 11 }}>
                <div style={{ display: 'flex', gap: 8, marginBottom: 4 }}>
                  <span style={{ color: '#569cd6' }}>{(l.address as string || '').slice(0, 14)}...</span>
                  <span style={{ color: '#888' }}>block: {l.blockNumber as string}</span>
                  <span style={{ color: '#888' }}>tx: {(l.transactionHash as string || '').slice(0, 12)}...</span>
                </div>
                {Array.isArray(l.topics) && (l.topics as string[]).slice(0, 3).map((t, j) => (
                  <div key={j} style={{ color: '#666', marginLeft: 8 }}>topic[{j}]: {t.slice(0, 24)}...</div>
                ))}
                <div style={{ color: '#888', marginTop: 4 }}>
                  data: {(l.data as string || '').slice(0, 40)}...
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
