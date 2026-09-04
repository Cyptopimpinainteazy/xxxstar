import { useState } from 'react';

export default function GraphQLExplorerPanel() {
  const [endpoint, setEndpoint] = useState('https://api.x3chain.xyz/graphql');
  const [query, setQuery] = useState('query { blocks(last: 5) { number hash timestamp } }');
  const [response, setResponse] = useState('');
  const [history, setHistory] = useState<string[]>([]);

  const runQuery = async () => {
    if (!endpoint || !query) return;
    setResponse('Running query...');
    try {
      const res = await fetch(endpoint, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ query }),
      });
      const data = await res.json();
      setResponse(JSON.stringify(data, null, 2));
      setHistory(prev => [`[${new Date().toLocaleTimeString()}] POST ${endpoint} - ${res.status}`, ...prev].slice(0, 20));
    } catch (e: any) {
      setResponse('Error: ' + e.message);
    }
  };

  const presets = [
    { name: 'Last 5 Blocks', q: 'query { blocks(last: 5) { number hash timestamp } }' },
    { name: 'System Properties', q: 'query { system { chain { name chainId } } }' },
    { name: 'Validator Set', q: 'query { validators { address stake status } }' },
    { name: 'Account Balance', q: 'query { account(address: "0x742d35Cc6634C0532925a3b844Bc9e7595f2bD18") { balance nonce } }' },
    { name: 'Recent Transfers', q: 'query { transfers(last: 10) { from to value hash } }' },
    { name: 'Chain Health', q: 'query { health { status blockHeight peers } }' },
  ];

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <div className="panel-header">GraphQL Explorer</div>
      <div style={{ padding: '4px 8px', borderBottom: '1px solid var(--border-color)' }}>
        <div className="form-group">
          <label style={{ fontSize: 'var(--font-size-sm)' }}>Endpoint</label>
          <input className="input-field" value={endpoint} onChange={e => setEndpoint(e.target.value)} placeholder="https://api.example.com/graphql" />
        </div>
        <div style={{ marginBottom: 4, display: 'flex', gap: 2, flexWrap: 'wrap' }}>
          {presets.map(p => (
            <button key={p.name} className="btn" style={{ fontSize: 9, padding: '2px 6px' }} onClick={() => setQuery(p.q)}>{p.name}</button>
          ))}
        </div>
      </div>
      <div style={{ flex: 1, display: 'flex', flexDirection: 'column' }}>
        <textarea className="input-field" style={{ flex: '0 0 120px', fontFamily: 'var(--font-mono)', fontSize: 10, border: 'none', borderBottom: '1px solid var(--border-color)', borderRadius: 0, resize: 'vertical' }}
          value={query} onChange={e => setQuery(e.target.value)} placeholder="GraphQL query..." />
        <button className="btn btn-primary" onClick={runQuery} style={{ borderRadius: 0 }}>▶ Run Query</button>
        <pre style={{ flex: 1, overflow: 'auto', padding: 8, fontSize: 10, margin: 0, fontFamily: 'var(--font-mono)', whiteSpace: 'pre-wrap' }}>
          {response || 'Run a query to see results.'}
        </pre>
      </div>
      {history.length > 0 && (
        <div style={{ maxHeight: 60, overflow: 'auto', borderTop: '1px solid var(--border-color)', padding: 4, fontSize: 10, color: 'var(--text-muted)' }}>
          {history.map((h, i) => <div key={i}>{h}</div>)}
        </div>
      )}
    </div>
  );
}
