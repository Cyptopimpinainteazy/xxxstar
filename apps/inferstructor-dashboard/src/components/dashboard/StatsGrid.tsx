import { Server, Zap, TrendingUp, Activity } from 'lucide-react';
import type { ValidatorStats, BridgeStats } from '../../api';
import { SECONDS_PER_HOUR, SECONDS_PER_MINUTE } from '../../constants';

interface StatsGridProps {
  stats: ValidatorStats | null;
  bridgeStats: BridgeStats | null;
}

export function StatsGrid({ stats, bridgeStats }: StatsGridProps) {
  const formatNumber = (num: number) => num.toLocaleString();
  
  const formatUptime = (seconds: number) => {
    const hours = Math.floor(seconds / SECONDS_PER_HOUR);
    const minutes = Math.floor((seconds % SECONDS_PER_HOUR) / SECONDS_PER_MINUTE);
    return `${hours}h ${minutes}m`;
  };

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-8">
      {/* SLA Tier */}
      <div className="card">
        <div className="flex items-center justify-between mb-2">
          <Server className="w-5 h-5 text-blue-400" />
          <span className={`
            px-2 py-1 rounded text-xs font-semibold
            ${stats?.sla_tier === 'enterprise' ? 'bg-purple-500/20 text-purple-300' : ''}
            ${stats?.sla_tier === 'pro' ? 'bg-blue-500/20 text-blue-300' : ''}
            ${stats?.sla_tier === 'basic' ? 'bg-gray-500/20 text-gray-300' : ''}
          `}>
            {stats?.sla_tier?.toUpperCase()}
          </span>
        </div>
        <p className="text-2xl font-bold text-white">{formatNumber(stats?.max_tps || 0)}</p>
        <p className="text-sm text-gray-400">Max TPS</p>
      </div>

      {/* Current TPS */}
      <div className="card">
        <div className="flex items-center justify-between mb-2">
          <Zap className="w-5 h-5 text-yellow-400" />
          <span className="w-2 h-2 bg-green-400 rounded-full animate-pulse"></span>
        </div>
        <p className="text-2xl font-bold text-white">{formatNumber(bridgeStats?.current_tps || 0)}</p>
        <p className="text-sm text-gray-400">Current TPS</p>
      </div>

      {/* Total Transactions */}
      <div className="card">
        <TrendingUp className="w-5 h-5 text-green-400 mb-2" />
        <p className="text-2xl font-bold text-white">{formatNumber(stats?.usage.total_tx || 0)}</p>
        <p className="text-sm text-gray-400">Total Transactions</p>
      </div>

      {/* Uptime */}
      <div className="card">
        <Activity className="w-5 h-5 text-blue-400 mb-2" />
        <p className="text-2xl font-bold text-white">{formatUptime(bridgeStats?.uptime_seconds || 0)}</p>
        <p className="text-sm text-gray-400">Uptime</p>
      </div>
    </div>
  );
}
