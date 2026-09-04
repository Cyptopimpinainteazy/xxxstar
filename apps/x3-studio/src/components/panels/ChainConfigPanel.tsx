import { useState } from 'react';
import { useChainConfigStore } from '../../store';
import type { ChainConfig } from '../../types';

export default function ChainConfigPanel() {
  const configs = useChainConfigStore(s => s.configs);
  const addConfig = useChainConfigStore(s => s.addConfig);
  const removeConfig = useChainConfigStore(s => s.removeConfig);
  const [name, setName] = useState('');
  const [chainId, setChainId] = useState('');
  const [rpcUrl, setRpcUrl] = useState('');
  const [explorerUrl, setExplorerUrl] = useState('');
  const [currency, setCurrency] = useState('ETH');
  const [type, setType] = useState<ChainConfig['type']>('evm');
  const [status, setStatus] = useState('');

  const addNewConfig = () => {
    if (!name || !chainId) return;
    addConfig({ name, chainId: parseInt(chainId), rpcUrl, explorerUrl, currency, type });
    setName(''); setChainId('');
    setStatus(`✓ Added ${name}`);
  };

  const exportConfigs = () => {
    const json = JSON.stringify(configs, null, 2);
    navigator.clipboard.writeText(json);
    setStatus('✓ Copied to clipboard');
  };

  const generateRpcGateway = () => {
    const gatewayConfig = {
      routes: configs.map(c => ({
        chain: c.name,
        chainId: c.chainId,
        rpcUrl: c.rpcUrl,
        type: c.type,
        rateLimit: 100,
        timeout: 5000,
      })),
      defaultChain: 'X3 Local',
      healthCheckInterval: 30000,
      quorum: { enabled: true, minResponses: 2, timeout: 3000 },
    };
    setStatus(JSON.stringify(gatewayConfig, null, 2));
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', padding: 8, overflow: 'auto' }}>
      <div className="panel-header" style={{ margin: '-8px -8px 8px -8px' }}>Chain Configuration Generator</div>
      <p style={{ fontSize: 11, color: 'var(--text-muted)', marginBottom: 8 }}>
        Manage chain configs and generate gateway/router configurations.
      </p>

      <div className="section-title">New Chain</div>
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8, marginBottom: 8 }}>
        <div className="form-group"><label style={{ fontSize: 'var(--font-size-sm)' }}>Name</label><input className="input-field" value={name} onChange={e => setName(e.target.value)} /></div>
        <div className="form-group"><label style={{ fontSize: 'var(--font-size-sm)' }}>Chain ID</label><input className="input-field" value={chainId} onChange={e => setChainId(e.target.value)} /></div>
        <div className="form-group"><label style={{ fontSize: 'var(--font-size-sm)' }}>RPC URL</label><input className="input-field" value={rpcUrl} onChange={e => setRpcUrl(e.target.value)} /></div>
        <div className="form-group"><label style={{ fontSize: 'var(--font-size-sm)' }}>Explorer URL</label><input className="input-field" value={explorerUrl} onChange={e => setExplorerUrl(e.target.value)} /></div>
        <div className="form-group"><label style={{ fontSize: 'var(--font-size-sm)' }}>Currency</label><input className="input-field" value={currency} onChange={e => setCurrency(e.target.value)} /></div>
        <div className="form-group"><label style={{ fontSize: 'var(--font-size-sm)' }}>Type</label>
          <select className="select-field" value={type} onChange={e => setType(e.target.value as any)}>
            <option value="evm">EVM</option><option value="svm">SVM</option><option value="substrate">Substrate</option><option value="cosmos">Cosmos</option>
          </select></div>
      </div>
      <button className="btn btn-primary" onClick={addNewConfig} disabled={!name || !chainId}>Add Chain</button>

      <div className="section-title">Configured Chains ({configs.length})</div>
      <table className="data-table" style={{ fontSize: 10 }}>
        <thead><tr><th>Name</th><th>Chain ID</th><th>Type</th><th>Currency</th><th>Actions</th></tr></thead>
        <tbody>
          {configs.map(c => (
            <tr key={c.name}>
              <td style={{ fontWeight: 600, fontSize: 10 }}>{c.name}</td>
              <td style={{ fontFamily: 'var(--font-mono)', fontSize: 10 }}>{c.chainId}</td>
              <td><span className="badge badge-info" style={{ fontSize: 9 }}>{c.type}</span></td>
              <td style={{ fontSize: 10 }}>{c.currency}</td>
              <td><button className="btn btn-danger" style={{ fontSize: 9, padding: '2px 6px' }} onClick={() => removeConfig(c.name)}>Remove</button></td>
            </tr>
          ))}
        </tbody>
      </table>

      <div style={{ display: 'flex', gap: 4, marginTop: 8 }}>
        <button className="btn" onClick={exportConfigs}>Export Configs</button>
        <button className="btn" onClick={generateRpcGateway}>Generate RPC Gateway Config</button>
      </div>

      {status && (
        <pre style={{ background: 'var(--bg-surface)', padding: 8, borderRadius: 'var(--radius)', fontSize: 10, maxHeight: 200, overflow: 'auto', whiteSpace: 'pre-wrap', marginTop: 8 }}>
          {status}
        </pre>
      )}
    </div>
  );
}
