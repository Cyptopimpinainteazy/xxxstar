import { useState, useCallback } from 'react';
import { invoke } from '../../ipc/tauri';

const CHAINS = [
  { id: 'evm', label: 'EVM' },
  { id: 'solana', label: 'Solana' },
  { id: 'substrate', label: 'Substrate' },
  { id: 'bitcoin', label: 'Bitcoin' },
];

interface CrossSwapResponse {
  intent_id: string;
  tx_hash: string;
  from_chain: string;
  to_chain: string;
  amount_units: string;
  status: string;
}

function SwapPanel() {
  const [fromChain, setFromChain] = useState('evm');
  const [toChain, setToChain] = useState('solana');
  const [amount, setAmount] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<CrossSwapResponse | null>(null);

  const handleSwap = useCallback(async () => {
    if (!amount || parseFloat(amount) <= 0) {
      setError('Enter a valid amount');
      return;
    }
    if (fromChain === toChain) {
      setError('Source and destination must differ');
      return;
    }
    setLoading(true);
    setError(null);
    setResult(null);
    try {
      const res = await invoke<CrossSwapResponse>('submit_cross_swap', {
        fromChain,
        toChain,
        amount,
      });
      setResult(res);
    } catch (err) {
      console.error('Swap failed:', err);
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, [fromChain, toChain, amount]);

  return (
    <div className="view p-6">
      <div className="mb-4">
        <h2 className="text-xl font-bold text-white">Cross-Chain Swap</h2>
        <p className="text-gray-400 text-sm">Atomic swap across EVM, Solana, Substrate & Bitcoin</p>
      </div>

      {error && <div className="bg-red-900/30 border border-red-600/30 rounded-lg p-3 mb-4 text-red-300 text-sm">{error}</div>}

      <div className="bg-gray-800/40 rounded-lg p-5 border border-gray-700/50 max-w-lg">
        {/* From */}
        <div className="mb-4">
          <label className="text-gray-400 text-xs mb-1 block">From</label>
          <select
            className="w-full bg-gray-900/60 border border-gray-700 rounded-lg px-3 py-2 text-white text-sm focus:outline-none focus:border-cyan-500/50"
            value={fromChain}
            onChange={(e) => setFromChain(e.target.value)}
          >
            {CHAINS.map((c) => (
              <option key={c.id} value={c.id}>{c.label}</option>
            ))}
          </select>
        </div>

        {/* To */}
        <div className="mb-4">
          <label className="text-gray-400 text-xs mb-1 block">To</label>
          <select
            className="w-full bg-gray-900/60 border border-gray-700 rounded-lg px-3 py-2 text-white text-sm focus:outline-none focus:border-cyan-500/50"
            value={toChain}
            onChange={(e) => setToChain(e.target.value)}
          >
            {CHAINS.map((c) => (
              <option key={c.id} value={c.id}>{c.label}</option>
            ))}
          </select>
        </div>

        {/* Amount */}
        <div className="mb-4">
          <label className="text-gray-400 text-xs mb-1 block">Amount</label>
          <input
            type="number"
            step="0.001"
            min="0"
            className="w-full bg-gray-900/60 border border-gray-700 rounded-lg px-3 py-2 text-white text-sm focus:outline-none focus:border-cyan-500/50"
            placeholder="0.0"
            value={amount}
            onChange={(e) => setAmount(e.target.value)}
          />
        </div>

        <button
          className="w-full px-4 py-2 text-sm bg-cyan-600/30 border border-cyan-500/40 text-cyan-300 rounded-lg hover:bg-cyan-600/50 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          onClick={handleSwap}
          disabled={loading}
        >
          {loading ? 'Submitting...' : 'Submit Swap'}
        </button>
      </div>

      {result && (
        <div className="mt-4 bg-gray-800/40 rounded-lg p-4 border border-gray-700/50 max-w-lg">
          <div className="text-gray-400 text-xs mb-2">Swap Result</div>
          <div className="grid grid-cols-2 gap-2 text-sm">
            <div className="text-gray-500">Intent ID</div>
            <div className="text-cyan-400 font-mono text-xs break-all">{result.intent_id}</div>
            <div className="text-gray-500">Tx Hash</div>
            <div className="text-cyan-400 font-mono text-xs break-all">{result.tx_hash}</div>
            <div className="text-gray-500">From</div>
            <div className="text-gray-300">{result.from_chain}</div>
            <div className="text-gray-500">To</div>
            <div className="text-gray-300">{result.to_chain}</div>
            <div className="text-gray-500">Amount</div>
            <div className="text-gray-300">{result.amount_units}</div>
            <div className="text-gray-500">Status</div>
            <div className="text-yellow-400">{result.status}</div>
          </div>
        </div>
      )}

      <div className="mt-3 text-xs text-gray-600">
        {`Query: invoke('submit_cross_swap', { fromChain, toChain, amount }) → intent + tx hash`}
      </div>
    </div>
  );
}

export default SwapPanel;
