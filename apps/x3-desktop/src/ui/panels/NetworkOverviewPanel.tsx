import { useEffect, useState, useCallback } from 'react';
import { invoke } from '../../ipc/tauri';

interface ChainInfo {
  chain?: string;
  name?: string;
  version?: string;
  chainName?: string;
}

interface HealthInfo {
  peers: number;
  is_syncing?: boolean;
  isSyncing?: boolean;
}

interface PeerInfo {
  peerId: string;
  roles?: string;
  bestHash?: string;
  bestNumber?: number;
}

interface NetworkOverview {
  chain: ChainInfo;
  health: HealthInfo;
  peers: PeerInfo[];
}

function NetworkOverviewPanel() {
  const [data, setData] = useState<NetworkOverview | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchOverview = useCallback(async () => {
    try {
      const result = await invoke<NetworkOverview>('get_network_overview');
      if (result) {
        setData(result);
        setError(null);
      }
    } catch (err) {
      console.error('Failed to fetch network overview:', err);
      setError('Node RPC unreachable');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchOverview();
    const interval = setInterval(fetchOverview, 8000);
    return () => clearInterval(interval);
  }, [fetchOverview]);

  if (loading) {
    return (
      <div className="view p-6">
        <h2 className="text-xl font-bold text-white mb-2">Network Overview</h2>
        <div className="text-gray-400">Loading chain + peer data via Tauri RPC...</div>
      </div>
    );
  }

  const chainName = data?.chain?.chain || data?.chain?.chainName || data?.chain?.name || 'Unknown';
  const peerCount = data?.health?.peers ?? 0;
  const isSyncing = data?.health?.is_syncing ?? data?.health?.isSyncing ?? false;
  const peerList = Array.isArray(data?.peers) ? data.peers : [];

  return (
    <div className="view p-6">
      <div className="mb-4">
        <h2 className="text-xl font-bold text-white">Network Overview</h2>
        <p className="text-gray-400 text-sm">Chain state + live peers from system RPC</p>
      </div>

      {error && <div className="bg-red-900/30 border border-red-600/30 rounded-lg p-3 mb-4 text-red-300 text-sm">{error}</div>}

      <div className="grid grid-cols-3 gap-4 mb-6">
        <div className="bg-gray-800/40 rounded-lg p-4 border border-gray-700/50">
          <div className="text-gray-400 text-xs mb-1">Chain</div>
          <div className="text-cyan-400 font-mono font-bold text-lg">{chainName}</div>
        </div>
        <div className="bg-gray-800/40 rounded-lg p-4 border border-gray-700/50">
          <div className="text-gray-400 text-xs mb-1">Peers</div>
          <div className="text-green-400 font-mono font-bold text-lg">{peerCount}</div>
        </div>
        <div className="bg-gray-800/40 rounded-lg p-4 border border-gray-700/50">
          <div className="text-gray-400 text-xs mb-1">Sync Status</div>
          <div className={`font-mono font-bold text-lg ${isSyncing ? 'text-yellow-400' : 'text-green-400'}`}>
            {isSyncing ? 'Syncing' : 'Synced'}
          </div>
        </div>
      </div>

      <h3 className="text-sm font-bold text-white mb-3">Connected Peers ({peerList.length})</h3>
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="text-left text-gray-400 border-b border-gray-700">
              <th className="py-2 px-3">Peer ID</th>
              <th className="py-2 px-3">Roles</th>
              <th className="py-2 px-3">Best Block</th>
            </tr>
          </thead>
          <tbody>
            {peerList.length > 0 ? peerList.map((peer, i) => (
              <tr key={peer.peerId || i} className="border-b border-gray-800 hover:bg-gray-800/30">
                <td className="py-2 px-3 font-mono text-xs text-gray-300">{peer.peerId?.slice(0, 20) || `#${i}`}...</td>
                <td className="py-2 px-3 font-mono text-xs text-gray-400">{peer.roles || '-'}</td>
                <td className="py-2 px-3 font-mono text-xs text-cyan-400">#{peer.bestNumber || '?'}</td>
              </tr>
            )) : (
              <tr>
                <td colSpan={3} className="py-8 text-center text-gray-500">
                  No peers connected. Start the node to begin P2P discovery.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      <div className="mt-3 text-xs text-gray-600">
        Query: invoke('get_network_overview') → system_chain + system_health + system_peers
      </div>
    </div>
  );
}

export default NetworkOverviewPanel;