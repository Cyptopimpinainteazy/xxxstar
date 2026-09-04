import { useState, useCallback } from 'react';
import { useTpsBenchmarkStore, useSettingsStore } from '../../store';

export default function TpsBenchmarkPanel() {
  const results = useTpsBenchmarkStore(s => s.results);
  const addResult = useTpsBenchmarkStore(s => s.addResult);
  const clear = useTpsBenchmarkStore(s => s.clear);
  const chainRpcUrl = useSettingsStore(s => s.chainRpcUrl);
  const [requests, setRequests] = useState('50');
  const [concurrency, setConcurrency] = useState('5');
  const [method, setMethod] = useState('eth_blockNumber');
  const [output, setOutput] = useState('');
  const [running, setRunning] = useState(false);

  const runBenchmark = useCallback(async () => {
    if (!chainRpcUrl) return;
    setRunning(true);
    const count = parseInt(requests);
    const conc = parseInt(concurrency);
    setOutput(`Benchmarking ${method} on ${chainRpcUrl}...\nRequests: ${count}, Concurrency: ${conc}`);

    const latencies: number[] = [];
    let errors = 0;
    const start = Date.now();

    const sendRequest = async () => {
      const reqStart = Date.now();
      try {
        const result = await window.x3studio.chain.rpcCall(chainRpcUrl, method, []);
        if (result.error) errors++;
        latencies.push(Date.now() - reqStart);
      } catch { errors++; latencies.push(Date.now() - reqStart); }
    };

    const batch = async () => {
      const promises: Promise<void>[] = [];
      for (let i = 0; i < count; i += conc) {
        const batchSize = Math.min(conc, count - i);
        for (let j = 0; j < batchSize; j++) {
          promises.push(sendRequest());
        }
        await Promise.all(promises.splice(0, batchSize));
      }
      await Promise.all(promises);
    };

    try {
      await batch();
      const duration = (Date.now() - start) / 1000;
      const tps = count / duration;
      const sorted = [...latencies].sort((a, b) => a - b);
      const avg = sorted.reduce((a, b) => a + b, 0) / sorted.length;
      const p95 = sorted[Math.floor(sorted.length * 0.95)];

      const result = {
        id: `tps-${Date.now()}`,
        method,
        chain: chainRpcUrl,
        requests: count,
        duration,
        tps: Math.round(tps),
        errors,
        latencyAvg: Math.round(avg),
        latencyP95: p95 || 0,
        timestamp: new Date().toISOString(),
      };
      addResult(result);
      setOutput(`Benchmark Complete!\n\nTPS: ${result.tps}/s\nRequests: ${count}\nDuration: ${duration.toFixed(1)}s\nErrors: ${errors}\nAvg Latency: ${Math.round(avg)}ms\nP95 Latency: ${p95}ms\n\nMethod: ${method}\nChain: ${chainRpcUrl}`);
    } catch (e: any) {
      setOutput('Benchmark error: ' + e.message);
    }
    setRunning(false);
  }, [chainRpcUrl, method, requests, concurrency]);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', padding: 8 }}>
      <div className="panel-header" style={{ margin: '-8px -8px 8px -8px' }}>TPS Benchmark</div>
      <p style={{ fontSize: 11, color: 'var(--text-muted)', marginBottom: 8 }}>
        Measure RPC throughput and latency.
      </p>

      <div className="form-group"><label style={{ fontSize: 'var(--font-size-sm)' }}>RPC Method</label>
        <select className="select-field" value={method} onChange={e => setMethod(e.target.value)}>
          <option value="eth_blockNumber">eth_blockNumber</option>
          <option value="eth_chainId">eth_chainId</option>
          <option value="eth_gasPrice">eth_gasPrice</option>
          <option value="net_version">net_version</option>
          <option value="web3_clientVersion">web3_clientVersion</option>
        </select></div>
      <div style={{ display: 'flex', gap: 8, marginBottom: 8 }}>
        <div className="form-group" style={{ flex: 1 }}><label style={{ fontSize: 'var(--font-size-sm)' }}>Request Count</label>
          <input className="input-field" value={requests} onChange={e => setRequests(e.target.value)} /></div>
        <div className="form-group" style={{ flex: 1 }}><label style={{ fontSize: 'var(--font-size-sm)' }}>Concurrency</label>
          <input className="input-field" value={concurrency} onChange={e => setConcurrency(e.target.value)} /></div>
      </div>

      <button className="btn btn-primary" onClick={runBenchmark} disabled={running || !chainRpcUrl}>
        {running ? 'Running...' : '▶ Run Benchmark'}
      </button>

      <div className="section-title">Results ({results.length})</div>
      <table className="data-table" style={{ fontSize: 10 }}>
        <thead><tr><th>TPS</th><th>Method</th><th>Requests</th><th>Duration</th><th>Avg Lat</th><th>P95</th><th>Errors</th></tr></thead>
        <tbody>
          {results.map(r => (
            <tr key={r.id}>
              <td style={{ fontWeight: 600, fontFamily: 'var(--font-mono)', fontSize: 10 }}>{r.tps}/s</td>
              <td style={{ fontFamily: 'var(--font-mono)', fontSize: 10 }}>{r.method}</td>
              <td style={{ fontSize: 10 }}>{r.requests}</td>
              <td style={{ fontSize: 10 }}>{r.duration.toFixed(1)}s</td>
              <td style={{ fontSize: 10 }}>{r.latencyAvg}ms</td>
              <td style={{ fontSize: 10 }}>{r.latencyP95}ms</td>
              <td><span className={`badge badge-${r.errors === 0 ? 'pass' : 'fail'}`} style={{ fontSize: 9 }}>{r.errors}</span></td>
            </tr>
          ))}
          {results.length === 0 && <tr><td colSpan={7} style={{ textAlign: 'center', color: 'var(--text-muted)', padding: 16 }}>No benchmarks run yet.</td></tr>}
        </tbody>
      </table>

      {results.length > 0 && <button className="btn" onClick={clear} style={{ marginTop: 4 }}>Clear History</button>}

      {output && (
        <pre style={{ background: 'var(--bg-surface)', padding: 8, borderRadius: 'var(--radius)', fontSize: 10, maxHeight: 150, overflow: 'auto', whiteSpace: 'pre-wrap', marginTop: 8 }}>
          {output}
        </pre>
      )}
    </div>
  );
}
