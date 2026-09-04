import { useState } from 'react';
import { Rocket, Loader2, CheckCircle, XCircle } from 'lucide-react';

export function DeployPanel() {
  const [name, setName] = useState('');
  const [bytecode, setBytecode] = useState('');
  const [from, setFrom] = useState('');
  const [abi, setAbi] = useState('[]');
  const [deploying, setDeploying] = useState(false);
  const [result, setResult] = useState<{ address: string; txHash: string; name: string } | null>(null);
  const [error, setError] = useState('');

  const deploy = async () => {
    if (!name || !bytecode || !from) return;
    setDeploying(true);
    setError('');
    setResult(null);
    try {
      const res = await fetch('http://127.0.0.1:8765/api/contracts/deploy', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name, bytecode, from_address: from, abi }),
      });
      const data = await res.json();
      if (data.address) setResult(data);
      else setError(JSON.stringify(data));
    } catch (e) {
      setError(String(e));
    } finally {
      setDeploying(false);
    }
  };

  const fillFlashloan = () => {
    setName('X3Flashloan');
    setBytecode('0x608060405260043610...');
    setFrom('0x0000000000000000000000000000000000000001');
  };

  return (
    <div style={{ padding: 16, color: '#d4d4d4', height: '100%', overflow: 'auto' }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 16 }}>
        <Rocket size={20} />
        <h2 style={{ margin: 0, fontSize: 16, fontWeight: 600 }}>Contract Deployer</h2>
      </div>

      <div style={{ display: 'grid', gap: 10, maxWidth: 600 }}>
        <div style={{ display: 'flex', gap: 8 }}>
          <div style={{ flex: 1 }}>
            <label style={{ fontSize: 12, color: '#888', display: 'block', marginBottom: 4 }}>Contract Name</label>
            <input value={name} onChange={e => setName(e.target.value)} placeholder="MyContract"
              style={{ width: '100%', padding: '6px 8px', background: '#3c3c3c', border: '1px solid #555', borderRadius: 4, color: '#d4d4d4', fontSize: 12, fontFamily: 'monospace', outline: 'none' }}
            />
          </div>
          <div style={{ flex: 2 }}>
            <label style={{ fontSize: 12, color: '#888', display: 'block', marginBottom: 4 }}>From Address</label>
            <input value={from} onChange={e => setFrom(e.target.value)} placeholder="0x..."
              style={{ width: '100%', padding: '6px 8px', background: '#3c3c3c', border: '1px solid #555', borderRadius: 4, color: '#d4d4d4', fontSize: 12, fontFamily: 'monospace', outline: 'none' }}
            />
          </div>
        </div>

        <div>
          <label style={{ fontSize: 12, color: '#888', display: 'block', marginBottom: 4 }}>Bytecode (hex)</label>
          <textarea value={bytecode} onChange={e => setBytecode(e.target.value)}
            placeholder="0x608060..."
            rows={4}
            style={{ width: '100%', padding: '6px 8px', background: '#3c3c3c', border: '1px solid #555', borderRadius: 4, color: '#d4d4d4', fontSize: 12, fontFamily: 'monospace', outline: 'none', resize: 'vertical' }}
          />
        </div>

        <div>
          <label style={{ fontSize: 12, color: '#888', display: 'block', marginBottom: 4 }}>ABI (JSON, optional)</label>
          <textarea value={abi} onChange={e => setAbi(e.target.value)}
            rows={3}
            style={{ width: '100%', padding: '6px 8px', background: '#3c3c3c', border: '1px solid #555', borderRadius: 4, color: '#d4d4d4', fontSize: 12, fontFamily: 'monospace', outline: 'none', resize: 'vertical' }}
          />
        </div>

        <div style={{ display: 'flex', gap: 8 }}>
          <button onClick={deploy} disabled={deploying || !name || !bytecode || !from}
            style={{ display: 'flex', alignItems: 'center', gap: 4, padding: '8px 20px', border: 'none', borderRadius: 4, background: '#0e639c', color: '#fff', cursor: 'pointer', fontSize: 13, fontWeight: 600, opacity: deploying ? 0.6 : 1 }}
          >{deploying ? <Loader2 size={14} className="spin" /> : <Rocket size={14} />} Deploy</button>
          <button onClick={fillFlashloan}
            style={{ padding: '8px 14px', border: '1px solid #333', borderRadius: 4, background: '#2d2d2d', color: '#ccc', cursor: 'pointer', fontSize: 12 }}
          >Fill Example</button>
        </div>

        {result && (
          <div style={{ padding: 12, background: '#1a3a2a', borderRadius: 8, border: '1px solid #4ec9b0', fontFamily: 'monospace', fontSize: 12, lineHeight: 1.8 }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 6, color: '#4ec9b0', marginBottom: 8 }}>
              <CheckCircle size={14} /> Deployed Successfully
            </div>
            <div><span style={{ color: '#888' }}>Contract:</span> <span style={{ color: '#569cd6' }}>{result.name}</span></div>
            <div><span style={{ color: '#888' }}>Address:</span> <span style={{ color: '#d4d4d4', wordBreak: 'break-all' }}>{result.address}</span></div>
            <div><span style={{ color: '#888' }}>Tx Hash:</span> <span style={{ wordBreak: 'break-all' }}>{result.txHash}</span></div>
          </div>
        )}

        {error && (
          <div style={{ padding: 12, background: '#3a1a1a', borderRadius: 8, border: '1px solid #f48771', fontSize: 12, color: '#f48771' }}>
            <XCircle size={14} style={{ marginRight: 6 }} /> {error}
          </div>
        )}
      </div>
    </div>
  );
}
