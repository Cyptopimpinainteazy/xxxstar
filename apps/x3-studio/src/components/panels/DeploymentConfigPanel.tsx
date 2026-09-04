import { useState } from 'react';
import { useDeploymentConfigStore, useWorkspaceStore } from '../../store';

export default function DeploymentConfigPanel() {
  const wp = useWorkspaceStore(s => s.workspacePath);
  const configs = useDeploymentConfigStore(s => s.configs);
  const addConfig = useDeploymentConfigStore(s => s.addConfig);
  const removeConfig = useDeploymentConfigStore(s => s.removeConfig);
  const [name, setName] = useState('');
  const [chain, setChain] = useState('Ethereum');
  const [rpcUrl, setRpcUrl] = useState('http://localhost:8545');
  const [contract, setContract] = useState('');
  const [bytecode, setBytecode] = useState('');
  const [abi, setAbi] = useState('');
  const [constructorArgs, setConstructorArgs] = useState('');
  const [gasLimit, setGasLimit] = useState('3000000');
  const [status, setStatus] = useState('');

  const saveConfig = async () => {
    if (!name || !contract) return;
    const config = {
      name, chain, rpcUrl, contract, bytecode, abi,
      constructorArgs: constructorArgs.split(',').map(s => s.trim()),
      gasLimit, timestamp: new Date().toISOString(),
    };
    addConfig(config);
    if (wp) {
      try {
        const configPath = wp + '/x3-proof/deployment-' + name + '.json';
        await window.x3studio.fs.writeFile(configPath, JSON.stringify(config, null, 2));
        setStatus(`✓ Config saved to ${configPath}`);
      } catch {}
    }
    setName(''); setContract('');
  };

  const deploy = async (config: typeof configs[0]) => {
    setStatus(`Deploying ${config.name} to ${config.chain}...`);
    try {
      let abiObj: any[];
      try { abiObj = JSON.parse(config.abi); } catch { abiObj = []; }

      const constructorAbi = abiObj.find((a: any) => a.type === 'constructor');
      const args = constructorAbi ? config.constructorArgs : [];

      const tx: any = {
        data: config.bytecode,
        gas: '0x' + parseInt(config.gasLimit).toString(16),
      };

      if (args.length > 0) {
        const encoded = await window.x3studio.chain.rpcCall(config.rpcUrl, 'eth_encodeAbi', [config.abi, JSON.stringify(args)]);
        tx.data = config.bytecode + (encoded.result || '').substring(2);
      }

      const result = await window.x3studio.chain.rpcCall(config.rpcUrl, 'eth_sendRawTransaction', [tx.data]);
      setStatus(`✓ Deploy tx sent: ${JSON.stringify(result)}`);
    } catch (e: any) { setStatus('Error: ' + e.message); }
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', padding: 8 }}>
      <div className="panel-header" style={{ margin: '-8px -8px 8px -8px' }}>Deployment Config Manager</div>

      <div className="section-title">New Deployment Config</div>
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8, marginBottom: 8 }}>
        <div className="form-group"><label style={{ fontSize: 'var(--font-size-sm)' }}>Name</label><input className="input-field" value={name} onChange={e => setName(e.target.value)} /></div>
        <div className="form-group"><label style={{ fontSize: 'var(--font-size-sm)' }}>Chain</label>
          <select className="select-field" value={chain} onChange={e => setChain(e.target.value)}>
            <option>Ethereum</option><option>Base</option><option>Arbitrum</option><option>Polygon</option><option>X3 Local</option><option>X3 Testnet</option>
          </select></div>
        <div className="form-group"><label style={{ fontSize: 'var(--font-size-sm)' }}>RPC URL</label><input className="input-field" value={rpcUrl} onChange={e => setRpcUrl(e.target.value)} /></div>
        <div className="form-group"><label style={{ fontSize: 'var(--font-size-sm)' }}>Contract Name</label><input className="input-field" value={contract} onChange={e => setContract(e.target.value)} /></div>
        <div className="form-group"><label style={{ fontSize: 'var(--font-size-sm)' }}>Gas Limit</label><input className="input-field" value={gasLimit} onChange={e => setGasLimit(e.target.value)} /></div>
        <div className="form-group"><label style={{ fontSize: 'var(--font-size-sm)' }}>Constructor Args (comma-sep)</label><input className="input-field" value={constructorArgs} onChange={e => setConstructorArgs(e.target.value)} /></div>
      </div>
      <div className="form-group"><label style={{ fontSize: 'var(--font-size-sm)' }}>Bytecode (hex)</label>
        <textarea className="input-field" style={{ fontFamily: 'var(--font-mono)', fontSize: 10, height: 50 }} value={bytecode} onChange={e => setBytecode(e.target.value)} /></div>
      <div className="form-group"><label style={{ fontSize: 'var(--font-size-sm)' }}>ABI (JSON)</label>
        <textarea className="input-field" style={{ fontFamily: 'var(--font-mono)', fontSize: 10, height: 60 }} value={abi} onChange={e => setAbi(e.target.value)} /></div>
      <button className="btn btn-primary" onClick={saveConfig} disabled={!name || !contract}>Save Config</button>

      <div className="section-title">Saved Configs ({configs.length})</div>
      {configs.map(c => (
        <div key={c.name} style={{ background: 'var(--bg-surface)', borderRadius: 'var(--radius)', padding: 8, marginBottom: 6 }}>
          <div style={{ fontWeight: 600, fontSize: 'var(--font-size-sm)' }}>{c.name} <span style={{ color: 'var(--text-muted)', fontWeight: 400 }}>→ {c.chain}</span></div>
          <div style={{ fontSize: 11 }}>{c.contract}</div>
          <div style={{ fontSize: 10, color: 'var(--text-muted)' }}>RPC: {c.rpcUrl}</div>
          <div style={{ display: 'flex', gap: 4, marginTop: 4 }}>
            <button className="btn" style={{ fontSize: 9, padding: '2px 6px' }} onClick={() => deploy(c)}>Deploy</button>
            <button className="btn btn-danger" style={{ fontSize: 9, padding: '2px 6px' }} onClick={() => removeConfig(c.name)}>Delete</button>
          </div>
        </div>
      ))}
      {configs.length === 0 && <div style={{ color: 'var(--text-muted)', fontSize: 11 }}>No deployment configs saved.</div>}
      {status && <div style={{ fontSize: 11, color: status.includes('✓') ? 'var(--pass-color)' : 'var(--fail-color)', marginTop: 8 }}>{status}</div>}
    </div>
  );
}
