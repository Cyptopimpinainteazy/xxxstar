import { useState, useRef, useCallback, useMemo } from 'react';
import { useNetworkProfilerStore } from '../../store';

export default function NetworkProfilerPanel() {
  const requests = useNetworkProfilerStore(s => s.requests);
  const isRecording = useNetworkProfilerStore(s => s.isRecording);
  const setRecording = useNetworkProfilerStore(s => s.setRecording);
  const addRequest = useNetworkProfilerStore(s => s.addRequest);
  const clear = useNetworkProfilerStore(s => s.clear);
  const [filter, setFilter] = useState('');
  const [selected, setSelected] = useState<string | null>(null);

  const origFetch = useRef<typeof window.fetch | null>(null);

  const startRecording = useCallback(() => {
    if (origFetch.current) return;
    origFetch.current = window.fetch.bind(window);
    window.fetch = async (input, init) => {
      const start = Date.now();
      try {
        const response = await origFetch.current!(input, init);
        const duration = Date.now() - start;
        const cloned = response.clone();
        const body = await cloned.text();
        addRequest({
          id: `req-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
          url: typeof input === 'string' ? input : input.url,
          method: init?.method || 'GET',
          status: response.status,
          duration,
          timestamp: new Date().toISOString(),
          body: init?.body?.toString() || '',
          response: body.substring(0, 500),
        });
        return response;
      } catch (e: any) {
        const duration = Date.now() - start;
        addRequest({
          id: `req-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
          url: typeof input === 'string' ? input : input.url,
          method: init?.method || 'GET',
          status: 0,
          duration,
          timestamp: new Date().toISOString(),
          body: init?.body?.toString() || '',
          response: `Error: ${e.message}`,
        });
        throw e;
      }
    };
    setRecording(true);
  }, []);

  const stopRecording = useCallback(() => {
    if (origFetch.current) {
      window.fetch = origFetch.current;
      origFetch.current = null;
    }
    setRecording(false);
  }, []);

  const filtered = requests.filter(r =>
    !filter || r.url.toLowerCase().includes(filter.toLowerCase()) || r.method.toLowerCase().includes(filter.toLowerCase())
  );

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <div className="panel-header">Network Profiler</div>
      <div style={{ padding: '4px 8px', display: 'flex', gap: 4, alignItems: 'center', borderBottom: '1px solid var(--border-color)' }}>
        {isRecording ? (
          <button className="btn btn-danger" onClick={stopRecording} style={{ fontSize: 10 }}>■ Stop Recording</button>
        ) : (
          <button className="btn btn-primary" onClick={startRecording} style={{ fontSize: 10 }}>● Start Recording</button>
        )}
        <span className={`badge badge-${isRecording ? 'pass' : 'info'}`} style={{ fontSize: 10 }}>
          {isRecording ? 'Recording' : 'Idle'} ({requests.length})
        </span>
        <input className="input-field" style={{ flex: 1, fontSize: 10 }} value={filter} onChange={e => setFilter(e.target.value)} placeholder="Filter URL/method..." />
        <button className="btn" onClick={clear} style={{ fontSize: 10 }}>Clear</button>
      </div>
      {/* Timeline visualization */}
      {requests.length > 0 && (() => {
        const maxDur = Math.max(...requests.map(r => r.duration), 1);
        const recent = requests.slice(-50);
        return (
          <div style={{ padding: '4px 8px', borderBottom: '1px solid var(--border-color)' }}>
            <div className="section-title" style={{ fontSize: 9 }}>Response Timeline (last {recent.length})</div>
            <div style={{ display: 'flex', gap: 1, height: 40, alignItems: 'flex-end', marginTop: 4 }}>
              {recent.map((r, i) => {
                const h = Math.max(3, (r.duration / maxDur) * 36);
                const color = r.duration < 100 ? '#4ade80' : r.duration < 500 ? '#facc15' : r.duration < 2000 ? '#fb923c' : '#f87171';
                return (
                  <div key={r.id} style={{
                    width: Math.max(4, Math.min(10, 500 / recent.length)),
                    height: h,
                    background: color,
                    borderRadius: '1px 1px 0 0',
                    opacity: 0.85,
                    flexShrink: 0,
                    cursor: 'pointer',
                    transition: 'height 0.2s',
                    position: 'relative',
                  }} title={`${r.method} ${r.url.substring(0, 50)}... ${r.duration}ms`} />
                );
              })}
            </div>
            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: 9, color: 'var(--text-muted)', marginTop: 2 }}>
              <span>◄ older</span>
              <span>{maxDur >= 1000 ? `${(maxDur/1000).toFixed(1)}s max` : `${maxDur}ms max`}</span>
              <span>newer ►</span>
            </div>
          </div>
        );
      })()}

      <div style={{ overflow: 'auto', flex: 1 }}>
        <table className="data-table" style={{ fontSize: 10 }}>
          <thead><tr><th>Method</th><th>URL</th><th>Status</th><th>Duration</th><th>Time</th></tr></thead>
          <tbody>
            {filtered.map(req => (
              <tr key={req.id} onClick={() => setSelected(selected === req.id ? null : req.id)}
                style={{ cursor: 'pointer', background: selected === req.id ? 'var(--bg-surface)' : undefined }}>
                <td><span className={`badge badge-${req.method === 'GET' ? 'info' : 'warn'}`} style={{ fontSize: 9 }}>{req.method}</span></td>
                <td style={{ maxWidth: 200, overflow: 'hidden', textOverflow: 'ellipsis', fontFamily: 'var(--font-mono)', fontSize: 10 }}>{req.url.substring(0, 60)}</td>
                <td><span className={`badge badge-${req.status >= 200 && req.status < 400 ? 'pass' : 'fail'}`} style={{ fontSize: 9 }}>{req.status}</span></td>
                <td style={{ fontFamily: 'var(--font-mono)', fontSize: 10 }}>{req.duration}ms</td>
                <td style={{ fontSize: 10, color: 'var(--text-muted)' }}>{new Date(req.timestamp).toLocaleTimeString()}</td>
              </tr>
            ))}
            {filtered.length === 0 && (
              <tr><td colSpan={5} style={{ textAlign: 'center', color: 'var(--text-muted)', padding: 16 }}>
                {isRecording ? 'Waiting for network requests...' : 'Start recording to capture network requests'}
              </td></tr>
            )}
          </tbody>
        </table>
        {selected && (() => {
          const req = requests.find(r => r.id === selected);
          if (!req) return null;
          return (
            <div style={{ padding: 8, borderTop: '1px solid var(--border-color)' }}>
              <div className="section-title" style={{ fontSize: 10 }}>Request Details</div>
              <div style={{ fontSize: 10, fontFamily: 'var(--font-mono)', whiteSpace: 'pre-wrap', maxHeight: 200, overflow: 'auto', background: 'var(--bg-surface)', padding: 8, borderRadius: 'var(--radius)' }}>
                <div><strong>URL:</strong> {req.url}</div>
                <div><strong>Method:</strong> {req.method}</div>
                <div><strong>Status:</strong> {req.status}</div>
                <div><strong>Duration:</strong> {req.duration}ms</div>
                {req.body && <><div><strong>Body:</strong></div><div>{req.body.substring(0, 300)}</div></>}
                {req.response && <><div><strong>Response:</strong></div><div>{req.response.substring(0, 500)}</div></>}
              </div>
            </div>
          );
        })()}
      </div>
    </div>
  );
}
