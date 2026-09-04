import { useEffect, useState, useCallback, useRef } from 'react';
import { invoke, listen } from '../../ipc/tauri';

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

interface NetworkOverview {
  chain: ChainInfo;
  health: HealthInfo;
}

interface BlockEvent {
  height?: number;
  number?: number;
  timestamp?: string;
  hash?: string;
}

interface ValidatorInfo {
  id?: string;
  accountId?: string;
  name?: string;
  stake?: string;
  commission?: number;
}

interface SupplyData {
  total_supply: string;
  circulating_supply: string;
  locked_supply: string;
}

function ExplorerPanel() {
  const [chainInfo, setChainInfo] = useState<ChainInfo | null>(null);
  const [health, setHealth] = useState<HealthInfo | null>(null);
  const [blocks, setBlocks] = useState<BlockEvent[]>([]);
  const [validators, setValidators] = useState<ValidatorInfo[]>([]);
  const [validatorCount, setValidatorCount] = useState(0);
  const [supply, setSupply] = useState<SupplyData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const blockFeedRef = useRef<HTMLDivElement>(null);

  const fetchAll = useCallback(async () => {
    try {
      const [overview, vResult, sResult] = await Promise.all([
        invoke<NetworkOverview>('get_network_overview'),
        invoke<ValidatorInfo[]>('get_validators'),
        invoke<SupplyData>('get_supply_data'),
      ]);

      if (overview) {
        setChainInfo(overview.chain);
        setHealth(overview.health);
      }
      if (Array.isArray(vResult)) {
        setValidators(vResult);
        setValidatorCount(vResult.length);
      }
      if (sResult) {
        setSupply(sResult);
      }
      setError(null);
    } catch (err) {
      console.error('Explorer fetch error:', err);
      setError('Some data sources unreachable');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchAll();
    const interval = setInterval(fetchAll, 8000);
    return () => clearInterval(interval);
  }, [fetchAll]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen<BlockEvent>('block:new', (payload) => {
      setBlocks((prev) => {
        const next = [payload, ...prev].slice(0, 50);
        return next;
      });
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  useEffect(() => {
    if (blockFeedRef.current) {
      blockFeedRef.current.scrollTop = 0;
    }
  }, [blocks]);

  const chainName = chainInfo?.chain || chainInfo?.chainName || chainInfo?.name || 'Unknown';
  const peerCount = health?.peers ?? 0;
  const isSyncing = health?.is_syncing ?? health?.isSyncing ?? false;

  if (loading) {
    return (
      <div className="view p-6">
        <h2 className="text-xl font-bold text-white mb-2">Blockchain Explorer</h2>
        <div className="text-gray-400">Loading chain data via Tauri RPC...</div>
      </div>
    );
  }

  return (
    <div className="view p-6">
      <div className="mb-4">
        <h2 className="text-xl font-bold text-white">Blockchain Explorer</h2>
        <p className="text-gray-400 text-sm">Chain info, blocks, validators and supply</p>
      </div>

      {error && (
        <div className="bg-yellow-900/30 border border-yellow-600/30 rounded-lg p-3 mb-4 text-yellow-300 text-sm">
          {error}
        </div>
      )}

      {/* Section 1: Chain Info */}
      <div className="mb-6">
        <h3 className="text-sm font-bold text-white mb-3">Chain Info</h3>
        <div className="grid grid-cols-3 gap-4">
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
            <div
              className={`font-mono font-bold text-lg ${isSyncing ? 'text-yellow-400' : 'text-green-400'}`}
            >
              {isSyncing ? 'Syncing' : 'Synced'}
            </div>
          </div>
        </div>
      </div>

      {/* Section 2: Recent Blocks */}
      <div className="mb-6">
        <h3 className="text-sm font-bold text-white mb-3">Recent Blocks (live)</h3>
        <div
          ref={blockFeedRef}
          className="bg-gray-800/30 rounded-lg border border-gray-700/50 max-h-40 overflow-y-auto"
        >
          {blocks.length === 0 ? (
            <div className="p-4 text-gray-500 text-sm text-center">Waiting for new blocks...</div>
          ) : (
            <table className="w-full text-sm">
              <thead>
                <tr className="text-left text-gray-400 border-b border-gray-700">
                  <th className="py-2 px-3">Height</th>
                  <th className="py-2 px-3">Timestamp</th>
                </tr>
              </thead>
              <tbody>
                {blocks.map((block, i) => (
                  <tr key={block.hash || i} className="border-b border-gray-800 hover:bg-gray-800/30">
                    <td className="py-2 px-3 font-mono text-xs text-cyan-400">
                      #{block.height ?? block.number ?? '?'}
                    </td>
                    <td className="py-2 px-3 font-mono text-xs text-gray-400">
                      {block.timestamp
                        ? new Date(block.timestamp).toLocaleTimeString()
                        : '—'}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      </div>

      {/* Section 3: Validators */}
      <div className="mb-6">
        <h3 className="text-sm font-bold text-white mb-3">Validators ({validatorCount})</h3>
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
          {validators.length === 0 ? (
            <div className="text-gray-500 text-sm col-span-2">No validator data available.</div>
          ) : (
            validators.slice(0, 10).map((v, i) => (
              <div
                key={v.id || v.accountId || i}
                className="bg-gray-800/40 rounded-lg p-3 border border-gray-700/50"
              >
                <div className="text-white font-mono text-xs truncate">
                  {v.name || v.id || v.accountId || `Validator #${i + 1}`}
                </div>
                <div className="text-gray-400 text-[10px] mt-1">
                  {v.stake ? `Stake: ${v.stake}` : ''}
                  {v.commission !== undefined ? ` | Commission: ${v.commission}%` : ''}
                </div>
              </div>
            ))
          )}
          {validators.length > 10 && (
            <div className="text-gray-500 text-xs col-span-2 text-center mt-1">
              +{validators.length - 10} more
            </div>
          )}
        </div>
      </div>

      {/* Section 4: Supply */}
      <div className="mb-6">
        <h3 className="text-sm font-bold text-white mb-3">Supply</h3>
        <div className="grid grid-cols-3 gap-4">
          <div className="bg-gray-800/40 rounded-lg p-4 border border-gray-700/50">
            <div className="text-gray-400 text-xs mb-1">Total</div>
            <div className="text-white font-mono font-bold text-lg">
              {supply?.total_supply ? Number(supply.total_supply).toLocaleString() : '—'}
            </div>
          </div>
          <div className="bg-gray-800/40 rounded-lg p-4 border border-green-700/30">
            <div className="text-gray-400 text-xs mb-1">Circulating</div>
            <div className="text-green-400 font-mono font-bold text-lg">
              {supply?.circulating_supply ? Number(supply.circulating_supply).toLocaleString() : '—'}
            </div>
          </div>
          <div className="bg-gray-800/40 rounded-lg p-4 border border-yellow-700/30">
            <div className="text-gray-400 text-xs mb-1">Locked</div>
            <div className="text-yellow-400 font-mono font-bold text-lg">
              {supply?.locked_supply ? Number(supply.locked_supply).toLocaleString() : '—'}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

export default ExplorerPanel;
