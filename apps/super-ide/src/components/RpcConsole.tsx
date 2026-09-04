import { useState } from 'react';
import { Terminal, Send, Trash2, Loader2, Copy } from 'lucide-react';

const RPC_METHODS = [
  'system_health', 'system_chain', 'system_name', 'system_version',
  'system_properties', 'system_peers',
  'chain_getBlock', 'chain_getBlockHash', 'chain_getFinalizedHead',
  'chain_getHeader',
  'state_getRuntimeVersion', 'state_getMetadata',
  'eth_blockNumber', 'eth_chainId', 'eth_getBalance',
  'eth_getCode', 'eth_getStorageAt', 'eth_call',
  'eth_estimateGas', 'eth_gasPrice',
  'eth_getTransactionCount', 'eth_getTransactionReceipt',
  'eth_getTransactionByHash', 'eth_getLogs',
  'net_version', 'web3_clientVersion',
  'x3_getCanonicalBalance', 'x3_getWrappedAccounting',
  'wallet_getBalance', 'wallet_listWallets', 'wallet_getNetworks',
  'validator_getStatus', 'validator_getLeaderboard',
  'gateway_getPendingTransfers',
  'swarm_getMetrics',
];

export function RpcConsole() {
  const [method, setMethod] = useState('eth_blockNumber');
  const [params, setParams] = useState('[]');
  const [response, setResponse] = useState('');
  const [sending, setSending] = useState(false);
  const [history, setHistory] = useState<{ method: string; params: string; result: string }[]>([]);

  const send = async () => {
    setSending(true);
    setResponse('');
    try {
      let parsed: unknown[];
      try { parsed = JSON.parse(params); } catch { parsed = [params]; }
      const res = await fetch('http://127.0.0.1:8765/api/rpc', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ jsonrpc: '2.0', method, params: parsed, id: Date.now() }),
      });
      const data = await res.json();
      const formatted = JSON.stringify(data, null, 2);
      setResponse(formatted);
      setHistory(prev => [{ method, params, result: formatted }, ...prev].slice(0, 50));
    } catch (e) {
      setResponse(`Error: ${e}`);
    } finally {
      setSending(false);
    }
  };

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column', color: '#d4d4d4' }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '6px 16px', borderBottom: '1px solid #333', background: '#252526', fontSize: 12 }}>
        <Terminal size={14} /> <span>RPC Console</span>
      </div>

      <div style={{ display: 'flex', gap: 8, padding: '10px 16px', borderBottom: '1px solid #333', alignItems: 'center', flexWrap: 'wrap' }}>
        <select value={method} onChange={e => setMethod(e.target.value)}
          style={{
            padding: '5px 8px', background: '#3c3c3c', border: '1px solid #555',
            borderRadius: 4, color: '#d4d4d4', fontSize: 12, fontFamily: 'monospace',
            maxWidth: 280,
          }}
        >
          {RPC_METHODS.map(m => <option key={m} value={m}>{m}</option>)}
        </select>
        <input value={params} onChange={e => setParams(e.target.value)}
          placeholder='["0x..."]]'
          style={{
            flex: 1, padding: '5px 8px', background: '#3c3c3c', border: '1px solid #555',
            borderRadius: 4, color: '#d4d4d4', fontSize: 12, fontFamily: 'monospace',
            outline: 'none', minWidth: 100,
          }}
        />
        <button onClick={send} disabled={sending}
          style={{
            display: 'flex', alignItems: 'center', gap: 4, padding: '5px 12px',
            border: 'none', borderRadius: 4, background: '#0e639c', color: '#fff',
            cursor: 'pointer', fontSize: 12, opacity: sending ? 0.6 : 1,
          }}
        >
          {sending ? <Loader2 size={14} className="spin" /> : <Send size={14} />} Send
        </button>
        <button onClick={() => setResponse('')}
          style={{ padding: '5px 8px', border: '1px solid #333', borderRadius: 4, background: '#2d2d2d', color: '#ccc', cursor: 'pointer', fontSize: 12 }}
        >Clear</button>
      </div>

      <div style={{ flex: 1, display: 'flex', minHeight: 0, overflow: 'hidden' }}>
        <div style={{ flex: 1, overflow: 'auto', padding: 12, fontFamily: 'monospace', fontSize: 12 }}>
          {response ? (
            <pre style={{ margin: 0, whiteSpace: 'pre-wrap', wordBreak: 'break-all' }}>
              <span style={{ color: response.startsWith('Error') ? '#f48771' : '#4ec9b0' }}>{response}</span>
            </pre>
          ) : (
            <div style={{ color: '#666', fontStyle: 'italic' }}>Send an RPC call to see the response...</div>
          )}
        </div>

        {history.length > 0 && (
          <div style={{ width: 250, borderLeft: '1px solid #333', overflow: 'auto', background: '#1e1e1e', flexShrink: 0 }}>
            <div style={{ padding: '6px 10px', fontSize: 11, color: '#888', borderBottom: '1px solid #333', background: '#252526' }}>History</div>
            {history.map((h, i) => (
              <div key={i} onClick={() => { setMethod(h.method); setParams(h.params); setResponse(h.result); }}
                style={{ padding: '6px 10px', borderBottom: '1px solid #2a2a2a', cursor: 'pointer', fontSize: 11 }}
                onMouseEnter={e => e.currentTarget.style.background = '#2a2a2a'}
                onMouseLeave={e => e.currentTarget.style.background = 'transparent'}
              >
                <div style={{ color: '#569cd6' }}>{h.method}</div>
                <div style={{ color: '#888', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{h.params}</div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
