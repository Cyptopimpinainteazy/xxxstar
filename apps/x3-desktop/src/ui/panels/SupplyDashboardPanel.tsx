import { useEffect, useState, useCallback } from 'react';
import { invoke } from '../../ipc/tauri';

interface SupplyData {
  total_supply: string;
  circulating_supply: string;
  locked_supply: string;
}

function SupplyDashboardPanel() {
  const [supply, setSupply] = useState<SupplyData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchSupply = useCallback(async () => {
    try {
      const result = await invoke<SupplyData>('get_supply_data');
      if (result) {
        setSupply(result);
        setError(null);
      }
    } catch (err) {
      console.error('Failed to fetch supply data:', err);
      setError('Node RPC unreachable');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchSupply();
    const interval = setInterval(fetchSupply, 10_000);
    return () => clearInterval(interval);
  }, [fetchSupply]);

  if (loading) {
    return (
      <div className="view p-6">
        <h2 className="text-xl font-bold text-white mb-2">Token Supply</h2>
        <div className="text-gray-400">Loading supply data via Tauri → node RPC...</div>
      </div>
    );
  }

  const total = supply?.total_supply || '0';
  const circ = supply?.circulating_supply || '0';
  const locked = supply?.locked_supply || '0';

  const totalNum = parseFloat(total) || 0;
  const circNum = parseFloat(circ) || 0;
  const lockedNum = parseFloat(locked) || 0;
  const circPct = totalNum > 0 ? ((circNum / totalNum) * 100).toFixed(1) : '0';
  const lockedPct = totalNum > 0 ? ((lockedNum / totalNum) * 100).toFixed(1) : '0';

  return (
    <div className="view p-6">
      <div className="mb-4">
        <h2 className="text-xl font-bold text-white">Token Supply</h2>
        <p className="text-gray-400 text-sm">Circulating + locked supply from chain state</p>
      </div>

      {error && <div className="bg-yellow-900/30 border border-yellow-600/30 rounded-lg p-3 mb-4 text-yellow-300 text-sm">{error}</div>}

      <div className="grid grid-cols-3 gap-4 mb-6">
        <div className="bg-gray-800/40 rounded-lg p-5 border border-gray-700/50">
          <div className="text-gray-500 text-xs mb-1">Total Supply</div>
          <div className="text-white font-mono text-xl font-bold">{Number(total).toLocaleString()}</div>
          <div className="text-gray-500 text-xs mt-1">X3 tokens</div>
        </div>
        <div className="bg-gray-800/40 rounded-lg p-5 border border-green-700/30">
          <div className="text-gray-500 text-xs mb-1">Circulating</div>
          <div className="text-green-400 font-mono text-xl font-bold">{Number(circ).toLocaleString()}</div>
          <div className="text-gray-500 text-xs mt-1">{circPct}% of total</div>
        </div>
        <div className="bg-gray-800/40 rounded-lg p-5 border border-yellow-700/30">
          <div className="text-gray-500 text-xs mb-1">Locked</div>
          <div className="text-yellow-400 font-mono text-xl font-bold">{Number(locked).toLocaleString()}</div>
          <div className="text-gray-500 text-xs mt-1">{lockedPct}% of total</div>
        </div>
      </div>

      <div className="bg-gray-800/30 rounded-lg p-4 border border-gray-700/50">
        <h3 className="text-sm font-medium text-gray-300 mb-2">Supply Distribution</h3>
        <div className="w-full h-4 bg-gray-700 rounded-full overflow-hidden">
          {totalNum > 0 ? (
            <>
              <div className="h-full bg-green-500 float-left" style={{ width: `${circPct}%` }} />
              <div className="h-full bg-yellow-600 float-left" style={{ width: `${lockedPct}%` }} />
            </>
          ) : (
            <div className="h-full bg-gray-600 w-full" />
          )}
        </div>
        <div className="flex gap-4 mt-2 text-xs text-gray-500">
          <div className="flex items-center gap-1"><div className="w-2 h-2 rounded-full bg-green-500" /> Circulating</div>
          <div className="flex items-center gap-1"><div className="w-2 h-2 rounded-full bg-yellow-600" /> Locked</div>
        </div>
      </div>

      <div className="mt-3 text-xs text-gray-600">
        Query: invoke('get_supply_data') → node RPC :9933 (token_getSupply)
      </div>
    </div>
  );
}

export default SupplyDashboardPanel;