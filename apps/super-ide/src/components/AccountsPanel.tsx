import { useState } from 'react';
import { Database, Copy, Loader2, Plus, Key } from 'lucide-react';
import { useApi } from '../hooks/useApi';
import { api, type Account } from '../api/client';

export function AccountsPanel() {
  const { data: accounts, loading, error, refresh } = useApi(() => api.accounts(), []);
  const [selected, setSelected] = useState<Account | null>(null);
  const [showCreate, setShowCreate] = useState(false);
  const [newKeyType, setNewKeyType] = useState('ed25519');
  const [newLabel, setNewLabel] = useState('');
  const [newKey, setNewKey] = useState<{ address: string; publicKey: string; label: string; seed: string } | null>(null);
  const [creating, setCreating] = useState(false);

  const copy = (text: string) => navigator.clipboard.writeText(text);

  const createKey = async () => {
    setCreating(true);
    try {
      const result = await api.generateKey(newKeyType, newLabel);
      setNewKey(result);
      refresh();
    } catch (e) {
      console.error(e);
    } finally {
      setCreating(false);
    }
  };

  return (
    <div style={{ padding: 16, color: '#d4d4d4', height: '100%', overflow: 'auto' }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 16 }}>
        <Database size={20} />
        <h2 style={{ margin: 0, fontSize: 16, fontWeight: 600 }}>Accounts & Keys</h2>
        <button onClick={() => { setShowCreate(!showCreate); setNewKey(null); }}
          style={{ marginLeft: 'auto', display: 'flex', alignItems: 'center', gap: 4, padding: '4px 10px',
            border: '1px solid #0e639c', borderRadius: 4, background: 'transparent', color: '#0e639c', cursor: 'pointer', fontSize: 12 }}
        >
          <Key size={12} /> Generate
        </button>
      </div>

      {showCreate && (
        <div style={{ padding: 12, background: '#252526', borderRadius: 8, border: '1px solid #333', marginBottom: 12 }}>
          <div style={{ fontSize: 12, color: '#888', marginBottom: 8 }}>Generate New Key</div>
          <div style={{ display: 'flex', gap: 8, marginBottom: 8, alignItems: 'center' }}>
            <select value={newKeyType} onChange={e => setNewKeyType(e.target.value)}
              style={{ padding: '4px 8px', background: '#3c3c3c', border: '1px solid #555', borderRadius: 4, color: '#d4d4d4', fontSize: 12 }}
            >
              <option value="ed25519">Ed25519</option>
              <option value="secp256k1">Secp256k1</option>
              <option value="sr25519">Sr25519</option>
            </select>
            <input value={newLabel} onChange={e => setNewLabel(e.target.value)}
              placeholder="Label (optional)"
              style={{ flex: 1, padding: '4px 8px', background: '#3c3c3c', border: '1px solid #555', borderRadius: 4, color: '#d4d4d4', fontSize: 12, outline: 'none' }}
            />
            <button onClick={createKey} disabled={creating}
              style={{ padding: '4px 12px', border: 'none', borderRadius: 4, background: '#0e639c', color: '#fff', cursor: 'pointer', fontSize: 12, opacity: creating ? 0.6 : 1 }}
            >{creating ? '...' : 'Create'}</button>
          </div>
          {newKey && (
            <div style={{ fontFamily: 'monospace', fontSize: 12, lineHeight: 1.8 }}>
              <div><span style={{ color: '#888' }}>Address:</span> <span style={{ color: '#569cd6' }}>{newKey.address}</span> <button onClick={() => copy(newKey.address)} style={{ background: 'none', border: 'none', cursor: 'pointer', color: '#569cd6' }}><Copy size={10} /></button></div>
              <div><span style={{ color: '#888' }}>Public Key:</span> {newKey.publicKey.slice(0, 20)}...</div>
              <div><span style={{ color: '#dcdcaa' }}>⚠ Seed:</span> <span style={{ color: '#f48771' }}>{newKey.seed.slice(0, 20)}... (SAVE THIS!)</span> <button onClick={() => copy(newKey.seed)} style={{ background: 'none', border: 'none', cursor: 'pointer', color: '#f48771' }}><Copy size={10} /></button></div>
            </div>
          )}
        </div>
      )}

      {loading && <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}><Loader2 size={14} className="spin" /> Loading...</div>}
      {error && <div style={{ color: '#f48771' }}>{error}</div>}

      {!selected && accounts?.map(acc => (
        <div key={acc.address} onClick={() => setSelected(acc)}
          style={{ padding: '10px 12px', borderBottom: '1px solid #2a2a2a', cursor: 'pointer', borderRadius: 4 }}
          onMouseEnter={e => e.currentTarget.style.background = '#2a2a2a'}
          onMouseLeave={e => e.currentTarget.style.background = 'transparent'}
        >
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <span style={{ fontWeight: 600 }}>{acc.label || 'Unnamed'}</span>
            <span style={{ color: '#4ec9b0', fontFamily: 'monospace', fontSize: 13 }}>{Number(acc.balance).toLocaleString()} X3</span>
          </div>
          <div style={{ display: 'flex', gap: 8, fontSize: 12, color: '#888', fontFamily: 'monospace', marginTop: 2 }}>
            <span>{acc.address.slice(0, 16)}...</span>
            <span style={{ color: '#569cd6' }}>{acc.keyType}</span>
            <span style={{ color: '#666' }}>nonce: {acc.nonce}</span>
          </div>
        </div>
      ))}

      {selected && (
        <div>
          <button onClick={() => setSelected(null)} style={{
            marginBottom: 12, padding: '4px 10px', border: '1px solid #333',
            borderRadius: 4, background: '#2d2d2d', color: '#ccc', cursor: 'pointer', fontSize: 12,
          }}>← Back</button>
          <div style={{ fontFamily: 'monospace', fontSize: 13, lineHeight: 1.8, padding: 12, background: '#252526', borderRadius: 8, border: '1px solid #333' }}>
            <div><span style={{ color: '#888' }}>Label:</span> {selected.label || '—'}</div>
            <div><span style={{ color: '#888' }}>Address:</span>
              <span style={{ fontSize: 12, wordBreak: 'break-all', marginLeft: 4 }}>{selected.address}</span>
              <button onClick={() => copy(selected.address)} style={{ background: 'none', border: 'none', cursor: 'pointer', color: '#569cd6', marginLeft: 4 }}><Copy size={12} /></button>
            </div>
            <div><span style={{ color: '#888' }}>Public Key:</span> {selected.publicKey?.slice(0, 24)}...</div>
            <div><span style={{ color: '#888' }}>Key Type:</span> <span style={{ color: '#569cd6' }}>{selected.keyType}</span></div>
            <div><span style={{ color: '#888' }}>Balance:</span> <span style={{ color: '#4ec9b0' }}>{Number(selected.balance).toLocaleString()} X3</span></div>
            <div><span style={{ color: '#888' }}>Nonce:</span> {selected.nonce}</div>
            <div><span style={{ color: '#888' }}>Network:</span> {selected.network}</div>
          </div>
        </div>
      )}

      <button onClick={refresh} style={{ marginTop: 12, padding: '6px 16px', border: '1px solid #333', borderRadius: 4, background: '#2d2d2d', color: '#ccc', cursor: 'pointer', fontSize: 12 }}>Refresh</button>
    </div>
  );
}
