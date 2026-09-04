import { api } from '../../api';
import type { ValidatorStats, BridgeStats } from '../../api';

interface DetailsGridProps {
  stats: ValidatorStats | null;
  bridgeStats: BridgeStats | null;
}

export function DetailsGrid({ stats, bridgeStats }: DetailsGridProps) {
  const formatNumber = (num: number) => num.toLocaleString();

  return (
    <div className="grid md:grid-cols-2 gap-6">
      {/* Validator Info */}
      <div className="card">
        <h3 className="text-lg font-bold text-white mb-4">Validator Information</h3>
        <div className="space-y-3">
          <div className="flex justify-between py-2 border-b border-gray-700">
            <span className="text-gray-400">Chain</span>
            <span className="text-white font-semibold capitalize">{stats?.chain}</span>
          </div>
          <div className="flex justify-between py-2 border-b border-gray-700">
            <span className="text-gray-400">Status</span>
            <span className={`
              px-2 py-1 rounded text-xs font-semibold
              ${stats?.status === 'enabled' ? 'bg-green-500/20 text-green-300' : 'bg-red-500/20 text-red-300'}
            `}>
              {stats?.status}
            </span>
          </div>
          <div className="flex justify-between py-2 border-b border-gray-700">
            <span className="text-gray-400">API Key</span>
            <code className="text-white font-mono text-xs">{api.getApiKey()?.slice(0, 20)}...</code>
          </div>
          <div className="flex justify-between py-2">
            <span className="text-gray-400">Last Used</span>
            <span className="text-white">
              {stats?.usage.last_used
                ? new Date(stats.usage.last_used * 1000).toLocaleString()
                : 'Never'}
            </span>
          </div>
        </div>
      </div>

      {/* Bridge Statistics */}
      <div className="card">
        <h3 className="text-lg font-bold text-white mb-4">Bridge Statistics</h3>
        <div className="space-y-3">
          <div className="flex justify-between py-2 border-b border-gray-700">
            <span className="text-gray-400">Total Received</span>
            <span className="text-white font-semibold">{formatNumber(bridgeStats?.total_received || 0)}</span>
          </div>
          <div className="flex justify-between py-2 border-b border-gray-700">
            <span className="text-gray-400">Total Forwarded</span>
            <span className="text-white font-semibold">{formatNumber(bridgeStats?.total_forwarded || 0)}</span>
          </div>
          <div className="flex justify-between py-2 border-b border-gray-700">
            <span className="text-gray-400">Total Failed</span>
            <span className="text-white font-semibold">{formatNumber(bridgeStats?.total_failed || 0)}</span>
          </div>
          <div className="flex justify-between py-2">
            <span className="text-gray-400">Success Rate</span>
            <span className="text-green-400 font-semibold">
              {bridgeStats?.total_received
                ? ((bridgeStats.total_forwarded / bridgeStats.total_received) * 100).toFixed(2) + '%'
                : '0%'}
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}
