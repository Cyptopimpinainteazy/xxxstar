import { useState } from 'react';

export default function CrossChainSimulator() {
  const [sourceTx, setSourceTx] = useState('');
  const [sourceChain, setSourceChain] = useState('Ethereum');
  const [destChain, setDestChain] = useState('X3');
  const [amount, setAmount] = useState('1.0');
  const [token, setToken] = useState('ETH');
  const [output, setOutput] = useState('');
  const [logs, setLogs] = useState<string[]>([]);

  const simulateSwap = async () => {
    setLogs(prev => [`[${new Date().toLocaleTimeString()}] Simulating cross-chain swap...`, ...prev].slice(0, 50));
    setOutput('Running cross-chain simulation...');

    try {
      const steps = [
        'Initializing source chain connection...',
        'Building lock transaction...',
        'Simulating source chain execution...',
        'Generating proof of lock...',
        'Submitting proof to destination chain...',
        'Executing mint on destination...',
        'Verifying balance...',
      ];

      for (let i = 0; i < steps.length; i++) {
        setOutput(steps[i]);
        setLogs(prev => [`[${new Date().toLocaleTimeString()}] ${steps[i]}`, ...prev].slice(0, 50));
        await new Promise(r => setTimeout(r, 300));
      }

      const simulatedTx = `0x${Array.from({ length: 64 }, () => Math.floor(Math.random() * 16).toString(16)).join('')}`;
      setOutput(`Simulation complete!\n\nSource Chain: ${sourceChain}\nDestination: ${destChain}\nAmount: ${amount} ${token}\nSource Tx: ${sourceTx || 'N/A'}\nSimulated Dest Tx: ${simulatedTx}\nStatus: SUCCESS (simulated)\nFees: 0.001 ${token}\nTime: ~12s\n\nNote: This is a simulation. Real cross-chain swaps require running relayers and proof verification.`);
      setLogs(prev => [`[${new Date().toLocaleTimeString()}] ✓ Simulation complete: ${amount} ${token} from ${sourceChain} → ${destChain}`, ...prev].slice(0, 50));
    } catch (e: any) {
      setOutput('Simulation error: ' + e.message);
    }
  };

  const simulateProofLedger = async () => {
    setLogs(prev => [`[${new Date().toLocaleTimeString()}] Simulating proof ledger write...`, ...prev].slice(0, 50));
    try {
      const proofHash = `0x${Array.from({ length: 40 }, () => Math.floor(Math.random() * 16).toString(16)).join('')}`;
      const report = {
        simulation: 'cross-chain-swap',
        sourceChain, destChain, amount, token,
        sourceTxHash: sourceTx || 'simulated',
        proofHash,
        merkleRoot: `0x${Array.from({ length: 64 }, () => Math.floor(Math.random() * 16).toString(16)).join('')}`,
        timestamp: new Date().toISOString(),
        status: 'simulated',
      };
      setOutput(JSON.stringify(report, null, 2));
      setLogs(prev => [`[${new Date().toLocaleTimeString()}] Proof ledger entry generated: ${proofHash}`, ...prev].slice(0, 50));
    } catch (e: any) {
      setOutput('Error: ' + e.message);
    }
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', padding: 8 }}>
      <div className="panel-header" style={{ margin: '-8px -8px 8px -8px' }}>Cross-Chain Transaction Simulator</div>
      <p style={{ fontSize: 11, color: 'var(--text-muted)', marginBottom: 8 }}>
        Simulate cross-chain swaps between EVM, SVM, and X3 chains.
      </p>

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8, marginBottom: 8 }}>
        <div className="form-group"><label style={{ fontSize: 'var(--font-size-sm)' }}>Source Chain</label>
          <select className="select-field" value={sourceChain} onChange={e => setSourceChain(e.target.value)}>
            <option>Ethereum</option><option>Base</option><option>Arbitrum</option>
            <option>Polygon</option><option>Solana</option><option>X3</option>
          </select></div>
        <div className="form-group"><label style={{ fontSize: 'var(--font-size-sm)' }}>Destination Chain</label>
          <select className="select-field" value={destChain} onChange={e => setDestChain(e.target.value)}>
            <option>X3</option><option>Ethereum</option><option>Base</option>
            <option>Arbitrum</option><option>Solana</option><option>Polygon</option>
          </select></div>
        <div className="form-group"><label style={{ fontSize: 'var(--font-size-sm)' }}>Amount</label>
          <input className="input-field" value={amount} onChange={e => setAmount(e.target.value)} /></div>
        <div className="form-group"><label style={{ fontSize: 'var(--font-size-sm)' }}>Token</label>
          <select className="select-field" value={token} onChange={e => setToken(e.target.value)}>
            <option>ETH</option><option>wX3</option><option>USDC</option><option>USDT</option><option>SOL</option>
          </select></div>
      </div>
      <div className="form-group"><label style={{ fontSize: 'var(--font-size-sm)' }}>Source Tx Hash (optional)</label>
        <input className="input-field" value={sourceTx} onChange={e => setSourceTx(e.target.value)} placeholder="0x..." /></div>

      <div style={{ display: 'flex', gap: 4, marginBottom: 8 }}>
        <button className="btn btn-primary" onClick={simulateSwap}>Simulate Swap</button>
        <button className="btn" onClick={simulateProofLedger}>Generate Proof Ledger Entry</button>
      </div>

      {output && (
        <pre style={{ background: 'var(--bg-surface)', padding: 8, borderRadius: 'var(--radius)', fontSize: 10, maxHeight: 200, overflow: 'auto', whiteSpace: 'pre-wrap', marginBottom: 8 }}>
          {output}
        </pre>
      )}

      <div className="section-title">Simulation Log</div>
      <div style={{ background: 'var(--bg-surface)', borderRadius: 'var(--radius)', padding: 8, maxHeight: 150, overflow: 'auto', fontSize: 10, fontFamily: 'var(--font-mono)' }}>
        {logs.map((log, i) => <div key={i} style={{ color: i === 0 ? 'var(--pass-color)' : 'var(--text-muted)' }}>{log}</div>)}
        {logs.length === 0 && <div style={{ color: 'var(--text-muted)' }}>No simulations yet.</div>}
      </div>
    </div>
  );
}
