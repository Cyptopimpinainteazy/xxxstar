import { useEffect, useState, useCallback } from 'react';
import { invoke } from '../../ipc/tauri';

interface ValidatorInfo {
  id: string;
  name: string;
  status: string;
  score: number;
  blocks: number;
  uptime: number;
  address: string;
}

function ValidatorGlobePanel() {
  const [validators, setValidators] = useState<ValidatorInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchValidators = useCallback(async () => {
    try {
      const result = await invoke<ValidatorInfo[]>('get_validators');
      if (Array.isArray(result)) {
        setValidators(result);
        setError(null);
      }
    } catch (err) {
      console.error('Failed to fetch validators:', err);
      setError('Node RPC unreachable — showing fallback data');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchValidators();
    const interval = setInterval(fetchValidators, 8000);
    return () => clearInterval(interval);
  }, [fetchValidators]);

  const onlineCount = validators.filter(v => v.status === 'online' || v.status === 'active').length;

  if (loading) {
    return (
      <div className="view p-6">
        <h2 className="text-xl font-bold text-white mb-2">Validator Network</h2>
        <div className="text-gray-400">Loading live validator data via Tauri RPC...</div>
      </div>
    );
  }

  return (
    <div className="view p-6">
      <div className="flex items-center justify-between mb-4">
        <div>
          <h2 className="text-xl font-bold text-white">Validator Network</h2>
          <p className="text-gray-400 text-sm">Live validator set from chain state</p>
        </div>
        <div className="flex items-center gap-2 bg-gray-800/50 rounded-lg px-3 py-2">
          <div className={`w-2 h-2 rounded-full ${onlineCount > 0 ? 'bg-green-400 animate-pulse' : 'bg-red-400'}`} />
          <span className="text-cyan-400 font-mono text-lg">{onlineCount}</span>
          <span className="text-gray-500 text-sm">/ {validators.length} online</span>
        </div>
      </div>

      {error && <div className="bg-yellow-900/30 border border-yellow-600/30 rounded-lg p-3 mb-4 text-yellow-300 text-sm">{error}</div>}

      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="text-left text-gray-400 border-b border-gray-700">
              <th className="py-2 px-3">ID</th>
              <th className="py-2 px-3">Name</th>
              <th className="py-2 px-3">Status</th>
              <th className="py-2 px-3 text-right">Score</th>
              <th className="py-2 px-3 text-right">Blocks</th>
              <th className="py-2 px-3 text-right">Uptime</th>
            </tr>
          </thead>
          <tbody>
            {validators.length > 0 ? validators.map((v, i) => (
              <tr key={v.id || i} className="border-b border-gray-800 hover:bg-gray-800/30">
                <td className="py-2 px-3 font-mono text-gray-300">{v.id?.slice(0, 12) || `#${i + 1}`}</td>
                <td className="py-2 px-3 text-white">{v.name || 'Unknown'}</td>
                <td className="py-2 px-3">
                  <span className={`px-2 py-0.5 rounded-full text-xs font-medium ${
                    v.status === 'online' || v.status === 'active' ? 'bg-green-900/40 text-green-400' :
                    v.status === 'syncing' ? 'bg-yellow-900/40 text-yellow-400' :
                    'bg-red-900/40 text-red-400'
                  }`}>
                    {v.status || 'offline'}
                  </span>
                </td>
                <td className="py-2 px-3 text-right font-mono text-cyan-400">{v.score ?? '-'}</td>
                <td className="py-2 px-3 text-right font-mono text-gray-300">{v.blocks?.toLocaleString() ?? '-'}</td>
                <td className="py-2 px-3 text-right font-mono text-green-400">{v.uptime != null ? `${v.uptime}%` : '-'}</td>
              </tr>
            )) : (
              <tr>
                <td colSpan={6} className="py-8 text-center text-gray-500">
                  No validators found. Start the node to populate chain state.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      <div className="mt-3 text-xs text-gray-600">
        Query: invoke('get_validators') → node RPC :9933 (validator_getValidators)
      </div>
    </div>
  );
}

export default ValidatorGlobePanel;