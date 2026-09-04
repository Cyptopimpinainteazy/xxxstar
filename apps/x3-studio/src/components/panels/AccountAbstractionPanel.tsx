import { useState } from 'react';
import type { AccountAbstractionWallet } from '../../types';

export default function AccountAbstractionPanel() {
  const [wallets, setWallets] = useState<AccountAbstractionWallet[]>([]);
  const [owner, setOwner] = useState('0x...');
  const [guardians, setGuardians] = useState('0x...,0x...');
  const [threshold, setThreshold] = useState('2');
  const [output, setOutput] = useState('');

  const buildWallet = () => {
    const gList = guardians.split(',').map(s => s.trim());
    const wallet: AccountAbstractionWallet = {
      address: `0x${Array.from({ length: 40 }, () => Math.floor(Math.random() * 16).toString(16)).join('')}`,
      owner, guardians: gList, threshold: parseInt(threshold), deployed: false,
    };
    setWallets(prev => [wallet, ...prev].slice(0, 20));
    setOutput(JSON.stringify(wallet, null, 2));
  };

  const deployWallet = async (w: AccountAbstractionWallet) => {
    setOutput(`Simulating AA wallet deployment for ${w.address}...\n\nThis would deploy an ERC-4337 compatible smart account.\nOwner: ${w.owner}\nGuardians: ${w.guardians.join(', ')}\nThreshold: ${w.threshold}\n\nERC-4337 EntryPoint: 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789\nWallet Factory: 0x9406Cc6185a346906296840746125a0E44976454\n\nNote: Real deployment requires:\n1. Running eth_sendRawTransaction with wallet factory bytecode\n2. Sufficient gas on the target chain\n3. The ERC-4337 entry point contract`);
    setWallets(prev => prev.map(wl => wl.address === w.address ? { ...wl, deployed: true } : wl));
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', padding: 8 }}>
      <div className="panel-header" style={{ margin: '-8px -8px 8px -8px' }}>Account Abstraction (ERC-4337)</div>
      <p style={{ fontSize: 11, color: 'var(--text-muted)', marginBottom: 8 }}>
        Build ERC-4337 smart accounts with guardian recovery.
      </p>

      <div className="form-group"><label style={{ fontSize: 'var(--font-size-sm)' }}>Owner Address</label>
        <input className="input-field" value={owner} onChange={e => setOwner(e.target.value)} placeholder="0x..." /></div>
      <div className="form-group"><label style={{ fontSize: 'var(--font-size-sm)' }}>Guardians (comma-sep)</label>
        <input className="input-field" value={guardians} onChange={e => setGuardians(e.target.value)} placeholder="0x...,0x..." /></div>
      <div className="form-group"><label style={{ fontSize: 'var(--font-size-sm)' }}>Signature Threshold</label>
        <input className="input-field" value={threshold} onChange={e => setThreshold(e.target.value)} /></div>

      <div style={{ display: 'flex', gap: 4, marginBottom: 8 }}>
        <button className="btn btn-primary" onClick={buildWallet}>Build Smart Account</button>
      </div>

      <div className="section-title">Smart Accounts ({wallets.length})</div>
      {wallets.map(w => (
        <div key={w.address} style={{ background: 'var(--bg-surface)', borderRadius: 'var(--radius)', padding: 8, marginBottom: 6 }}>
          <div style={{ fontWeight: 600, fontSize: 'var(--font-size-sm)', fontFamily: 'var(--font-mono)' }}>{w.address}</div>
          <div style={{ fontSize: 10, color: 'var(--text-muted)' }}>Owner: {w.owner}</div>
          <div style={{ fontSize: 10, color: 'var(--text-muted)' }}>Guardians: {w.guardians.length} | Threshold: {w.threshold}/{w.guardians.length}</div>
          <div><span className={`badge badge-${w.deployed ? 'pass' : 'info'}`} style={{ fontSize: 9 }}>{w.deployed ? 'Deployed' : 'Not Deployed'}</span></div>
          {!w.deployed && <button className="btn" style={{ fontSize: 9, padding: '2px 6px', marginTop: 4 }} onClick={() => deployWallet(w)}>Simulate Deploy</button>}
        </div>
      ))}

      {output && (
        <pre style={{ background: 'var(--bg-surface)', padding: 8, borderRadius: 'var(--radius)', fontSize: 10, maxHeight: 200, overflow: 'auto', whiteSpace: 'pre-wrap' }}>{output}</pre>
      )}
    </div>
  );
}
