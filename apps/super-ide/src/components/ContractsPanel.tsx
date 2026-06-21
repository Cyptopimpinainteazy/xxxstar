import { useState } from 'react';
import { Search, BadgeCheck, Loader2, ExternalLink, FileText } from 'lucide-react';
import { useApi } from '../hooks/useApi';
import { api, type Contract } from '../api/client';

export function ContractsPanel() {
  const { data: contracts, loading, error, refresh } = useApi(() => api.contracts(), []);
  const [selected, setSelected] = useState<Contract | null>(null);

  return (
    <div style={{ padding: 16, color: '#d4d4d4', height: '100%', overflow: 'auto' }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 16 }}>
        <Search size={20} />
        <h2 style={{ margin: 0, fontSize: 16, fontWeight: 600 }}>Deployed Contracts</h2>
      </div>

      {loading && <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}><Loader2 size={14} className="spin" /> Loading...</div>}
      {error && <div style={{ color: '#f48771' }}>{error}</div>}

      {!selected && contracts?.map(c => (
        <div key={c.address} onClick={() => setSelected(c)}
          style={{ padding: '10px 12px', borderBottom: '1px solid #2a2a2a', cursor: 'pointer', borderRadius: 4 }}
          onMouseEnter={e => e.currentTarget.style.background = '#2a2a2a'}
          onMouseLeave={e => e.currentTarget.style.background = 'transparent'}
        >
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <span style={{ fontWeight: 600, color: '#569cd6' }}>{c.name || 'Unnamed'}</span>
            <div style={{ display: 'flex', gap: 6, alignItems: 'center' }}>
              {c.compiler && <span style={{ color: '#888', fontSize: 11, padding: '1px 4px', background: '#2a2a2a', borderRadius: 3 }}>{c.compiler}</span>}
              {c.verified && <BadgeCheck size={14} color="#4ec9b0" />}
            </div>
          </div>
          <div style={{ fontSize: 12, color: '#888', fontFamily: 'monospace', marginTop: 2 }}>
            {c.address.slice(0, 16)}... owner: {c.owner?.slice(0, 10)}...
          </div>
        </div>
      ))}

      {selected && (
        <div>
          <button onClick={() => setSelected(null)} style={{ marginBottom: 12, padding: '4px 10px', border: '1px solid #333', borderRadius: 4, background: '#2d2d2d', color: '#ccc', cursor: 'pointer', fontSize: 12 }}>← Back</button>
          <div style={{ fontFamily: 'monospace', fontSize: 13, lineHeight: 1.8, padding: 12, background: '#252526', borderRadius: 8, border: '1px solid #333' }}>
            <div><span style={{ color: '#888' }}>Name:</span> {selected.name || '—'}</div>
            <div><span style={{ color: '#888' }}>Address:</span> <span style={{ fontSize: 12, wordBreak: 'break-all' }}>{selected.address}</span></div>
            <div><span style={{ color: '#888' }}>Owner:</span> {selected.owner || '—'}</div>
            <div><span style={{ color: '#888' }}>Compiler:</span> {selected.compiler || '—'}</div>
            <div><span style={{ color: '#888' }}>Verified:</span> <span style={{ color: selected.verified ? '#4ec9b0' : '#dcdcaa' }}>{selected.verified ? 'Yes' : 'No'}</span></div>
            <div><span style={{ color: '#888' }}>Source:</span> {selected.sourcePath || '—'}</div>
            <div><span style={{ color: '#888' }}>Tx Hash:</span> {selected.txHash ? <span style={{ fontSize: 12 }}>{selected.txHash.slice(0, 20)}...</span> : '—'}</div>
            <div><span style={{ color: '#888' }}>Deployed:</span> {new Date(selected.deployedAt).toLocaleString()}</div>
          </div>
        </div>
      )}

      <button onClick={refresh} style={{ marginTop: 12, padding: '6px 16px', border: '1px solid #333', borderRadius: 4, background: '#2d2d2d', color: '#ccc', cursor: 'pointer', fontSize: 12 }}>Refresh</button>
    </div>
  );
}
