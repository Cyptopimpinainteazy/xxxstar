import { Globe, Zap, Link } from 'lucide-react';
import type { ChainStats } from '../../api';
import { SECONDS_PER_HOUR, SECONDS_PER_MINUTE } from '../../constants';

interface SolanaChainSectionProps {
  chainStats: ChainStats | null;
}

export function SolanaChainSection({ chainStats }: SolanaChainSectionProps) {
  if (!chainStats) return null;

  const formatNumber = (num: number) => num.toLocaleString();
  
  const formatUptime = (seconds: number) => {
    const hours = Math.floor(seconds / SECONDS_PER_HOUR);
    const minutes = Math.floor((seconds % SECONDS_PER_HOUR) / SECONDS_PER_MINUTE);
    return `${hours}h ${minutes}m`;
  };

  return (
    <div className="card mb-8">
      <div className="flex items-center gap-2 mb-6">
        <Globe className="w-5 h-5 text-purple-400" />
        <h2 className="text-xl font-bold text-white">Solana Chain — Live</h2>
        <span className="w-2 h-2 bg-green-400 rounded-full animate-pulse"></span>
        {chainStats.chain.version && (
          <span className="ml-auto text-xs text-gray-400 font-mono">
            Solana Core {chainStats.chain.version['solana-core']}
          </span>
        )}
      </div>

      {/* Chain overview cards */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
        <div className="bg-gray-800/50 rounded-lg p-4 border border-gray-700">
          <p className="text-xs text-gray-400 mb-1">Slot</p>
          <p className="text-xl font-bold text-white font-mono">
            {chainStats.chain.slot?.toLocaleString() ?? '—'}
          </p>
        </div>
        <div className="bg-gray-800/50 rounded-lg p-4 border border-gray-700">
          <p className="text-xs text-gray-400 mb-1">Epoch</p>
          <p className="text-xl font-bold text-white font-mono">
            {chainStats.chain.epoch?.epoch?.toLocaleString() ?? '—'}
          </p>
          {chainStats.chain.epoch && (
            <div className="mt-2">
              <div className="flex justify-between text-[10px] text-gray-400 mb-1">
                <span>{chainStats.chain.epoch.slotIndex.toLocaleString()}</span>
                <span>{chainStats.chain.epoch.slotsInEpoch.toLocaleString()}</span>
              </div>
              <div className="w-full bg-gray-700 rounded-full h-1.5">
                <div
                  className="bg-purple-500 h-1.5 rounded-full transition-all"
                  style={{ width: `${(chainStats.chain.epoch.slotIndex / chainStats.chain.epoch.slotsInEpoch * 100).toFixed(1)}%` }}
                ></div>
              </div>
            </div>
          )}
        </div>
        <div className="bg-gray-800/50 rounded-lg p-4 border border-gray-700">
          <p className="text-xs text-gray-400 mb-1">Block Height</p>
          <p className="text-xl font-bold text-white font-mono">
            {chainStats.chain.block_height?.toLocaleString() ?? '—'}
          </p>
        </div>
        <div className="bg-gray-800/50 rounded-lg p-4 border border-gray-700">
          <p className="text-xs text-gray-400 mb-1">Network Txns</p>
          <p className="text-xl font-bold text-white font-mono">
            {chainStats.chain.epoch?.transactionCount
              ? `${(chainStats.chain.epoch.transactionCount / 1_000_000_000).toFixed(2)}B`
              : '—'}
          </p>
        </div>
      </div>

      {/* Blockhash */}
      {chainStats.chain.latest_blockhash && (
        <div className="bg-gray-900/50 rounded-lg p-3 mb-6 font-mono text-xs">
          <span className="text-gray-400 mr-2">Latest Blockhash:</span>
          <span className="text-green-400 break-all">{chainStats.chain.latest_blockhash}</span>
        </div>
      )}

      {/* Upstream RPCs + Proxy metrics */}
      <div className="grid md:grid-cols-2 gap-4">
        {/* Upstreams */}
        <div className="bg-gray-800/50 rounded-lg p-4 border border-gray-700">
          <div className="flex items-center gap-2 mb-3">
            <Link className="w-4 h-4 text-blue-400" />
            <h3 className="text-sm font-semibold text-white">Upstream RPCs</h3>
          </div>
          <div className="space-y-2">
            {chainStats.upstreams.map((u) => (
              <div key={u.name} className="flex items-center justify-between text-sm">
                <div className="flex items-center gap-2">
                  <span className={`w-2 h-2 rounded-full ${u.healthy ? 'bg-green-400' : 'bg-red-400'}`}></span>
                  <span className="text-white">{u.name}</span>
                </div>
                <span className="text-gray-400 font-mono">{u.latency_ms.toFixed(0)}ms</span>
              </div>
            ))}
          </div>
        </div>

        {/* Proxy metrics */}
        <div className="bg-gray-800/50 rounded-lg p-4 border border-gray-700">
          <div className="flex items-center gap-2 mb-3">
            <Zap className="w-4 h-4 text-yellow-400" />
            <h3 className="text-sm font-semibold text-white">GPU RPC Proxy</h3>
          </div>
          <div className="space-y-2 text-sm">
            <div className="flex justify-between">
              <span className="text-gray-400">Total Requests</span>
              <span className="text-white font-mono">{formatNumber(chainStats.proxy.total_requests)}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-400">Cache Hit Rate</span>
              <span className="text-green-400 font-mono">{chainStats.proxy.cache_hit_rate}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-400">GPU Verified</span>
              <span className="text-purple-400 font-mono">{formatNumber(chainStats.gpu_verifier.total_verified)}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-400">Uptime</span>
              <span className="text-white font-mono">{formatUptime(chainStats.proxy.uptime_seconds)}</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
