import { useState } from 'react';
import { Eye, Loader2, Search, Copy, Database } from 'lucide-react';
import { api } from '../api/client';

export function InspectorPanel() {
  const [address, setAddress] = useState('');
  const [balance, setBalance] = useState<{ balance: string; error?: string } | null>(null);
  const [code, setCode] = useState<{ code: string; hasCode: boolean; error?: string } | null>(null);
  const [storageSlot, setStorageSlot] = useState('0x0');
  const [storageVal, setStorageVal] = useState<{ value: string; error?: string } | null>(null);
  const [loadingBal, setLoadingBal] = useState(false);
  const [loadingCode, setLoadingCode] = useState(false);
  const [loadingStorage, setLoadingStorage] = useState(false);

  const inspectBalance = async () => {
    if (!address) return;
    setLoadingBal(true);
    try {
      const res = await api.inspectBalance(address);
      setBalance(res);
    } catch (e) {
      setBalance({ balance: '0', error: String(e) });
    } finally {
      setLoadingBal(false);
    }
  };

  const inspectCode = async () => {
    if (!address) return;
    setLoadingCode(true);
    try {
      const res = await api.inspectCode(address);
      setCode(res);
    } catch (e) {
      setCode({ code: '0x', hasCode: false, error: String(e) });
    } finally {
      setLoadingCode(false);
    }
  };

  const inspectStorage = async () => {
    if (!address) return;
    setLoadingStorage(true);
    try {
      const res = await api.inspectStorage(address, storageSlot);
      setStorageVal(res);
    } catch (e) {
      setStorageVal({ value: '0x', error: String(e) });
    } finally {
      setLoadingStorage(false);
    }
  };

  return (
    <div style={{ padding: 16, color: '#d4d4d4', height: '100%', overflow: 'auto' }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 16 }}>
        <Eye size={20} />
        <h2 style={{ margin: 0, fontSize: 16, fontWeight: 600 }}>State Inspector</h2>
      </div>

      <div style={{ display: 'grid', gap: 10, maxWidth: 600 }}>
        <div>
          <label style={{ fontSize: 12, color: '#888', display: 'block', marginBottom: 4 }}>Contract Address</label>
          <div style={{ display: 'flex', gap: 8 }}>
            <input value={address} onChange={e => setAddress(e.target.value)} placeholder="0x..."
              style={{ flex: 1, padding: '6px 8px', background: '#3c3c3c', border: '1px solid #555', borderRadius: 4, color: '#d4d4d4', fontSize: 12, fontFamily: 'monospace', outline: 'none' }}
            />
            <button onClick={inspectBalance} disabled={loadingBal || !address}
              style={{ padding: '6px 12px', border: 'none', borderRadius: 4, background: '#0e639c', color: '#fff', cursor: 'pointer', fontSize: 12 }}
            >{loadingBal ? <Loader2 size={12} className="spin" /> : 'Balance'}</button>
            <button onClick={inspectCode} disabled={loadingCode || !address}
              style={{ padding: '6px 12px', border: '1px solid #333', borderRadius: 4, background: '#2d2d2d', color: '#ccc', cursor: 'pointer', fontSize: 12 }}
            >{loadingCode ? <Loader2 size={12} className="spin" /> : 'Code'}</button>
          </div>
        </div>

        {balance && (
          <div style={{ padding: 10, background: '#252526', borderRadius: 6, border: '1px solid #333', fontFamily: 'monospace', fontSize: 12 }}>
            <div style={{ color: '#888', marginBottom: 4 }}>Balance</div>
            {balance.error ? <div style={{ color: '#f48771' }}>{balance.error}</div> : (
              <div style={{ color: '#4ec9b0', fontSize: 18, fontWeight: 700 }}>{Number(balance.balance).toLocaleString()} wei</div>
            )}
          </div>
        )}

        {code && (
          <div style={{ padding: 10, background: '#252526', borderRadius: 6, border: '1px solid #333', fontFamily: 'monospace', fontSize: 12 }}>
            <div style={{ color: '#888', marginBottom: 4 }}>Bytecode</div>
            {code.error ? <div style={{ color: '#f48771' }}>{code.error}</div> : (
              <>
                <div style={{ marginBottom: 4 }}>
                  <span style={{ color: code.hasCode ? '#4ec9b0' : '#dcdcaa' }}>
                    {code.hasCode ? '✓ Contract deployed' : '✗ No contract code'}
                  </span>
                  <span style={{ color: '#888', marginLeft: 8 }}>({code.code.length / 2 - 1} bytes)</span>
                </div>
                <pre style={{ margin: 0, fontSize: 11, whiteSpace: 'pre-wrap', wordBreak: 'break-all', maxHeight: 80, overflow: 'auto' }}>
                  {code.code.slice(0, 200)}...
                </pre>
              </>
            )}
          </div>
        )}
      </div>

      <div style={{ marginTop: 20 }}>
        <h3 style={{ fontSize: 14, fontWeight: 600, margin: '0 0 10px', color: '#ccc' }}>Storage Query</h3>
        <div style={{ display: 'flex', gap: 8, alignItems: 'center', maxWidth: 500 }}>
          <div style={{ flex: 1 }}>
            <label style={{ fontSize: 12, color: '#888', display: 'block', marginBottom: 4 }}>Storage Slot</label>
            <input value={storageSlot} onChange={e => setStorageSlot(e.target.value)} placeholder="0x0"
              style={{ width: '100%', padding: '6px 8px', background: '#3c3c3c', border: '1px solid #555', borderRadius: 4, color: '#d4d4d4', fontSize: 12, fontFamily: 'monospace', outline: 'none' }}
            />
          </div>
          <button onClick={inspectStorage} disabled={loadingStorage || !address}
            style={{ marginTop: 20, padding: '6px 14px', border: 'none', borderRadius: 4, background: '#0e639c', color: '#fff', cursor: 'pointer', fontSize: 12 }}
          >{loadingStorage ? <Loader2 size={12} className="spin" /> : <Search size={12} />}</button>
        </div>
        {storageVal && (
          <div style={{ marginTop: 8, padding: 10, background: '#252526', borderRadius: 6, border: '1px solid #333', fontFamily: 'monospace', fontSize: 12, maxWidth: 500 }}>
            <div style={{ color: '#888', marginBottom: 4 }}>Slot {storageSlot}</div>
            {storageVal.error ? <div style={{ color: '#f48771' }}>{storageVal.error}</div> : (
              <div style={{ color: '#d4d4d4', wordBreak: 'break-all' }}>{storageVal.value}</div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
