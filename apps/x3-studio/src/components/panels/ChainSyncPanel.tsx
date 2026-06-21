import { useState } from 'react';
import { useChainConfigStore, useSettingsStore } from '../../store';

interface SyncedConfig {
  name: string;
  chainId: number;
  rpcUrl: string;
  explorerUrl: string;
  currency: string;
  type: 'evm' | 'svm' | 'substrate' | 'cosmos';
}

export default function ChainSyncPanel() {
  const chainRpcUrl = useSettingsStore(s => s.chainRpcUrl);
  const configs = useChainConfigStore(s => s.configs);
  const addConfig = useChainConfigStore(s => s.addConfig);
  const removeConfig = useChainConfigStore(s => s.removeConfig);
  const [rpcUrl, setRpcUrl] = useState(chainRpcUrl);
  const [synced, setSynced] = useState<SyncedConfig[]>([]);
  const [status, setStatus] = useState('');

  const handleSync = async () => {
    setStatus('Syncing...');
    try {
      const result = await window.x3studio.chain.syncConfigs(rpcUrl);
      setSynced(result ?? []);
      setStatus(`✓ Synced ${result?.length ?? 0} configs`);
    } catch (e: any) {
      setStatus(`✗ Sync failed: ${e.message ?? e}`);
    }
  };

  const handleAdd = (cfg: SyncedConfig) => {
    addConfig(cfg);
    setStatus(`✓ Added ${cfg.name}`);
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
    navigator.clipboard.writeText(JSON.stringify(gatewayConfig, null, 2));
    setStatus('✓ RPC Gateway JSON copied to clipboard');
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <div className="panel-header">Chain Sync</div>
      <div className="panel-body" style={{ padding: 8 }}>
        <div className="section-title">Sync from Validator</div>
        <div style={{ display: 'flex', gap: 4, marginBottom: 8 }}>
          <input className="input-field" style={{ flex: 1 }} value={rpcUrl} onChange={e => setRpcUrl(e.target.value)} placeholder="RPC URL" />
          <button className="btn btn-primary" onClick={handleSync} disabled={!rpcUrl}>Sync</button>
        </div>

        {synced.length > 0 && (
          <>
            <div className="section-title">Synced Configs ({synced.length})</div>
            <table className="data-table" style={{ fontSize: 10 }}>
              <thead>
                <tr>
                  <th>Name</th>
                  <th>Chain ID</th>
                  <th>RPC URL</th>
                  <th>Explorer URL</th>
                  <th>Currency</th>
                  <th>Type</th>
                  <th>Action</th>
                </tr>
              </thead>
              <tbody>
                {synced.map(c => (
                  <tr key={c.name}>
                    <td style={{ fontWeight: 600 }}>{c.name}</td>
                    <td style={{ fontFamily: 'var(--font-mono)' }}>{c.chainId}</td>
                    <td style={{ fontFamily: 'var(--font-mono)', maxWidth: 160, overflow: 'hidden', textOverflow: 'ellipsis' }}>{c.rpcUrl}</td>
                    <td style={{ fontFamily: 'var(--font-mono)', maxWidth: 160, overflow: 'hidden', textOverflow: 'ellipsis' }}>{c.explorerUrl}</td>
                    <td>{c.currency}</td>
                    <td><span className="badge badge-info" style={{ fontSize: 9 }}>{c.type}</span></td>
                    <td>
                      <button className="btn" style={{ fontSize: 9, padding: '2px 6px' }} onClick={() => handleAdd(c)}>
                        Add to Configs
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </>
        )}

        <div className="section-title" style={{ marginTop: 12 }}>Current Configs ({configs.length})</div>
        <table className="data-table" style={{ fontSize: 10 }}>
          <thead>
            <tr>
              <th>Name</th>
              <th>Chain ID</th>
              <th>RPC URL</th>
              <th>Explorer URL</th>
              <th>Currency</th>
              <th>Type</th>
              <th>Action</th>
            </tr>
          </thead>
          <tbody>
            {configs.map(c => (
              <tr key={c.name}>
                <td style={{ fontWeight: 600 }}>{c.name}</td>
                <td style={{ fontFamily: 'var(--font-mono)' }}>{c.chainId}</td>
                <td style={{ fontFamily: 'var(--font-mono)', maxWidth: 160, overflow: 'hidden', textOverflow: 'ellipsis' }}>{c.rpcUrl}</td>
                <td style={{ fontFamily: 'var(--font-mono)', maxWidth: 160, overflow: 'hidden', textOverflow: 'ellipsis' }}>{c.explorerUrl}</td>
                <td>{c.currency}</td>
                <td><span className="badge badge-info" style={{ fontSize: 9 }}>{c.type}</span></td>
                <td>
                  <button className="btn btn-danger" style={{ fontSize: 9, padding: '2px 6px' }} onClick={() => removeConfig(c.name)}>
                    Remove
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>

        <div style={{ display: 'flex', gap: 4, marginTop: 8 }}>
          <button className="btn" onClick={generateRpcGateway} disabled={configs.length === 0}>
            Generate RPC Gateway JSON
          </button>
        </div>

        {status && (
          <pre style={{
            background: 'var(--bg-surface)', padding: 8, borderRadius: 'var(--radius)',
            fontSize: 10, maxHeight: 200, overflow: 'auto', whiteSpace: 'pre-wrap', marginTop: 8,
          }}>
            {status}
          </pre>
        )}
      </div>
    </div>
  );
}
