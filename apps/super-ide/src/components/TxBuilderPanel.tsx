import { useState } from 'react';
import { Send, ArrowRight, Loader2, Copy, Fuel, Hash } from 'lucide-react';
import { api } from '../api/client';

export function TxBuilderPanel() {
  const [from, setFrom] = useState('');
  const [to, setTo] = useState('');
  const [value, setValue] = useState('0');
  const [data, setData] = useState('0x');
  const [gasLimit, setGasLimit] = useState('21000');
  const [gasPrice, setGasPrice] = useState('100000000000');
  const [nonce, setNonce] = useState('0');
  const [txResult, setTxResult] = useState<Record<string, string> | null>(null);
  const [loading, setLoading] = useState(false);
  const [estGas, setEstGas] = useState<number | null>(null);
  const [estLoading, setEstLoading] = useState(false);

  const build = async () => {
    setLoading(true);
    try {
      const result = await api.buildTx(from, to, value, data);
      setTxResult(result.unsigned);
    } catch (e) {
      console.error(e);
    } finally {
      setLoading(false);
    }
  };

  const estimate = async () => {
    setEstLoading(true);
    try {
      const result = await api.estimateGas(from, to, data, value);
      setEstGas(result.gasEstimate);
    } catch {
      setEstGas(null);
    } finally {
      setEstLoading(false);
    }
  };

  return (
    <div style={{ padding: 16, color: '#d4d4d4', height: '100%', overflow: 'auto' }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 16 }}>
        <Send size={20} />
        <h2 style={{ margin: 0, fontSize: 16, fontWeight: 600 }}>Transaction Builder</h2>
      </div>

      <div style={{ display: 'grid', gap: 10, maxWidth: 600 }}>
        <Field label="From Address" value={from} onChange={setFrom} placeholder="0x..." />
        <Field label="To Address" value={to} onChange={setTo} placeholder="0x... (leave empty for deploy)" />
        <Field label="Value (wei)" value={value} onChange={setValue} placeholder="0" />
        <div>
          <label style={{ fontSize: 12, color: '#888', display: 'block', marginBottom: 4 }}>Data (hex)</label>
          <textarea value={data} onChange={e => setData(e.target.value)}
            placeholder="0x..."
            rows={3}
            style={{ width: '100%', padding: '6px 8px', background: '#3c3c3c', border: '1px solid #555', borderRadius: 4, color: '#d4d4d4', fontSize: 12, fontFamily: 'monospace', outline: 'none', resize: 'vertical' }}
          />
        </div>
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: 8 }}>
          <Field label="Gas Limit" value={gasLimit} onChange={setGasLimit} placeholder="21000" />
          <Field label="Gas Price" value={gasPrice} onChange={setGasPrice} placeholder="100000000000" />
          <Field label="Nonce" value={nonce} onChange={setNonce} placeholder="0" />
        </div>

        <div style={{ display: 'flex', gap: 8, marginTop: 8 }}>
          <button onClick={build} disabled={loading || !from}
            style={{ display: 'flex', alignItems: 'center', gap: 4, padding: '8px 20px', border: 'none', borderRadius: 4, background: '#0e639c', color: '#fff', cursor: 'pointer', fontSize: 13, fontWeight: 600, opacity: loading ? 0.6 : 1 }}
          >{loading ? <Loader2 size={14} className="spin" /> : <Send size={14} />} Build Transaction</button>

          <button onClick={estimate} disabled={estLoading || !from}
            style={{ display: 'flex', alignItems: 'center', gap: 4, padding: '8px 14px', border: '1px solid #333', borderRadius: 4, background: '#2d2d2d', color: '#ccc', cursor: 'pointer', fontSize: 12 }}
          >{estLoading ? <Loader2 size={14} className="spin" /> : <Fuel size={14} />} Estimate Gas</button>
        </div>

        {estGas !== null && (
          <div style={{ padding: '8px 12px', background: '#252526', borderRadius: 6, border: '1px solid #333', fontSize: 13 }}>
            <span style={{ color: '#888' }}>Estimated Gas:</span> <span style={{ color: '#4ec9b0', fontFamily: 'monospace' }}>{estGas.toLocaleString()}</span> units
          </div>
        )}

        {txResult && (
          <div style={{ padding: 12, background: '#1a3a2a', borderRadius: 8, border: '1px solid #4ec9b0', fontFamily: 'monospace', fontSize: 12, lineHeight: 1.8 }}>
            <div style={{ color: '#4ec9b0', marginBottom: 8, fontWeight: 600 }}>Unsigned Transaction</div>
            {Object.entries(txResult).map(([k, v]) => (
              <div key={k}>
                <span style={{ color: '#888' }}>{k}:</span>{' '}
                <span style={{ color: '#d4d4d4', wordBreak: 'break-all' }}>{v}</span>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

function Field({ label, value, onChange, placeholder }: { label: string; value: string; onChange: (v: string) => void; placeholder?: string }) {
  return (
    <div>
      <label style={{ fontSize: 12, color: '#888', display: 'block', marginBottom: 4 }}>{label}</label>
      <input value={value} onChange={e => onChange(e.target.value)} placeholder={placeholder}
        style={{ width: '100%', padding: '6px 8px', background: '#3c3c3c', border: '1px solid #555', borderRadius: 4, color: '#d4d4d4', fontSize: 12, fontFamily: 'monospace', outline: 'none' }}
      />
    </div>
  );
}
