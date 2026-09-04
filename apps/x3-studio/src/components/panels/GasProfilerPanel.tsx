import { useState, useCallback } from 'react';
import { useGasProfilerStore, useSettingsStore, useChainStore } from '../../store';

export default function GasProfilerPanel() {
  const entries = useGasProfilerStore(s => s.entries);
  const addEntry = useGasProfilerStore(s => s.addEntry);
  const clear = useGasProfilerStore(s => s.clear);
  const chainRpcUrl = useSettingsStore(s => s.chainRpcUrl);
  const chain = useChainStore(s => s.chain);
  const [contract, setContract] = useState('0x...');
  const [method, setMethod] = useState('eth_estimateGas');
  const [params, setParams] = useState('[{"to": "0x...", "data": "0x..."}]');
  const [output, setOutput] = useState('');

  const estimateGas = useCallback(async () => {
    if (!chainRpcUrl) return;
    setOutput('Estimating gas...');
    try {
      let parsedParams: any[];
      try { parsedParams = JSON.parse(params); } catch { parsedParams = [params]; }
      const start = Date.now();
      const result = await window.x3studio.chain.rpcCall(chainRpcUrl, method, parsedParams);
      const duration = Date.now() - start;
      setOutput(JSON.stringify(result, null, 2));

      if (result.result) {
        const gasUsed = typeof result.result === 'string' ? result.result : JSON.stringify(result.result);
        const gasPrice = chain?.chainId ? await window.x3studio.chain.rpcCall(chainRpcUrl, 'eth_gasPrice', []) : { result: '0x0' };
        const gpResult = gasPrice.result || '0x0';
        const used = parseInt(gasUsed, 16);
        const price = parseInt(gpResult, 16);
        addEntry({
          id: `gas-${Date.now()}`,
          method,
          contract,
          gasUsed: used.toString(),
          gasPrice: price.toString(),
          cost: ((used * price) / 1e18).toFixed(6) + ' ETH',
          timestamp: new Date().toISOString(),
        });
      }
    } catch (e: any) { setOutput('Error: ' + e.message); }
  }, [chainRpcUrl, method, params, contract]);

  const runBatch = useCallback(async () => {
    const methods = ['eth_estimateGas', 'eth_gasPrice', 'eth_feeHistory'];
    for (const m of methods) {
      setMethod(m);
      setOutput(`Running ${m}...`);
      try {
        const p = m === 'eth_feeHistory' ? '[4, "latest", [25, 50, 75]]' : m === 'eth_estimateGas' ? params : '[]';
        const start = Date.now();
        const result = await window.x3studio.chain.rpcCall(chainRpcUrl, m, JSON.parse(p));
        const duration = Date.now() - start;
        addEntry({
          id: `gas-${Date.now()}-${m}`,
          method: m,
          contract: m,
          gasUsed: result.result?.toString() || '0',
          gasPrice: result.result?.toString() || '0',
          cost: `${duration}ms`,
          timestamp: new Date().toISOString(),
        });
      } catch {}
    }
    setOutput('Batch complete');
  }, [chainRpcUrl, params]);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <div className="panel-header">Gas Profiler</div>
      <div style={{ padding: 8, borderBottom: '1px solid var(--border-color)' }}>
        <div className="form-group">
          <label style={{ fontSize: 'var(--font-size-sm)' }}>Contract Address</label>
          <input className="input-field" value={contract} onChange={e => setContract(e.target.value)} placeholder="0x..." />
        </div>
        <div className="form-group">
          <label style={{ fontSize: 'var(--font-size-sm)' }}>RPC Method</label>
          <select className="select-field" value={method} onChange={e => setMethod(e.target.value)}>
            <option value="eth_estimateGas">eth_estimateGas</option>
            <option value="eth_gasPrice">eth_gasPrice</option>
            <option value="eth_feeHistory">eth_feeHistory</option>
            <option value="eth_maxPriorityFeePerGas">eth_maxPriorityFeePerGas</option>
          </select>
        </div>
        <div className="form-group">
          <label style={{ fontSize: 'var(--font-size-sm)' }}>Parameters (JSON array)</label>
          <textarea className="input-field" rows={3} value={params} onChange={e => setParams(e.target.value)} style={{ fontFamily: 'var(--font-mono)', fontSize: 10 }} />
        </div>
        <div style={{ display: 'flex', gap: 4 }}>
          <button className="btn btn-primary" onClick={estimateGas}>Estimate Gas</button>
          <button className="btn" onClick={runBatch}>Run Batch</button>
          <button className="btn" onClick={clear}>Clear History</button>
        </div>
      </div>

      <div style={{ flex: 1, overflow: 'auto', padding: 4 }}>
        <table className="data-table" style={{ fontSize: 10 }}>
          <thead><tr><th>Method</th><th>Gas Used</th><th>Gas Price</th><th>Cost/Duration</th><th>Time</th></tr></thead>
          <tbody>
            {entries.map(e => (
              <tr key={e.id}>
                <td style={{ fontFamily: 'var(--font-mono)', fontSize: 10 }}>{e.method}</td>
                <td style={{ fontFamily: 'var(--font-mono)', fontSize: 10 }}>{parseInt(e.gasUsed).toLocaleString()}</td>
                <td style={{ fontFamily: 'var(--font-mono)', fontSize: 10 }}>{parseInt(e.gasPrice).toLocaleString()} wei</td>
                <td style={{ fontSize: 10 }}>{e.cost}</td>
                <td style={{ fontSize: 10, color: 'var(--text-muted)' }}>{new Date(e.timestamp).toLocaleTimeString()}</td>
              </tr>
            ))}
            {entries.length === 0 && (
              <tr><td colSpan={5} style={{ textAlign: 'center', color: 'var(--text-muted)', padding: 16 }}>No gas estimates yet.</td></tr>
            )}
          </tbody>
        </table>
      </div>

      {output && (
        <div style={{ maxHeight: 100, overflow: 'auto', borderTop: '1px solid var(--border-color)' }}>
          <pre style={{ padding: 8, fontSize: 10, whiteSpace: 'pre-wrap' }}>{output}</pre>
        </div>
      )}
    </div>
  );
}
