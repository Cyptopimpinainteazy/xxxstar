import { BarChart3, Clock, Download, Gauge, TrendingUp } from 'lucide-react';

interface RankEntry {
  rank: number;
  validatorId: string;
  name: string;
  chain: string;
  tps: number;
  latency: number;
  uptime: number;
  gasEfficiency: number;
}

interface MetricsSnapshot {
  timestamp: string;
  avgTps: number;
  avgLatency: number;
  avgUptime: number;
  avgGasEfficiency: number;
}

interface SummaryMetricsPanelProps {
  snapshots: MetricsSnapshot[];
}

interface SnapshotsPanelProps {
  snapshots: MetricsSnapshot[];
  adminOverride: boolean;
  onExportCSV: () => void;
  onAddSnapshot: () => void;
  onSetAdminOverride: (next: boolean) => void;
}

interface RankingsPanelProps {
  filteredRankings: RankEntry[];
  sortBy: 'tps' | 'latency' | 'uptime' | 'gasEfficiency';
  filterChain: 'all' | 'Ethereum' | 'Solana';
  onSetSortBy: (value: 'tps' | 'latency' | 'uptime' | 'gasEfficiency') => void;
  onSetFilterChain: (value: 'all' | 'Ethereum' | 'Solana') => void;
}

export function SummaryMetricsPanel({ snapshots }: SummaryMetricsPanelProps) {
  const latestSnapshot = snapshots[snapshots.length - 1];

  return (
    <div className="grid grid-cols-4 gap-4 mb-8">
      <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
        <div className="flex items-center gap-2 mb-2">
          <BarChart3 className="w-4 h-4 text-blue-400" />
          <p className="text-gray-400 text-xs font-semibold">AVG TPS</p>
        </div>
        <p className="text-2xl font-bold text-blue-400">{latestSnapshot ? Math.floor(latestSnapshot.avgTps) : 1587}</p>
        <p className="text-gray-500 text-xs mt-1">up 2.3% from last hour</p>
      </div>
      <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
        <div className="flex items-center gap-2 mb-2">
          <Clock className="w-4 h-4 text-green-400" />
          <p className="text-gray-400 text-xs font-semibold">AVG LATENCY</p>
        </div>
        <p className="text-2xl font-bold text-green-400">{latestSnapshot ? Math.floor(latestSnapshot.avgLatency) : 42} ms</p>
        <p className="text-gray-500 text-xs mt-1">down 1.2% from last hour</p>
      </div>
      <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
        <div className="flex items-center gap-2 mb-2">
          <TrendingUp className="w-4 h-4 text-purple-400" />
          <p className="text-gray-400 text-xs font-semibold">AVG UPTIME</p>
        </div>
        <p className="text-2xl font-bold text-purple-400">{latestSnapshot ? latestSnapshot.avgUptime.toFixed(2) : 99.83}%</p>
        <p className="text-gray-500 text-xs mt-1">Stable</p>
      </div>
      <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
        <div className="flex items-center gap-2 mb-2">
          <Gauge className="w-4 h-4 text-orange-400" />
          <p className="text-gray-400 text-xs font-semibold">GAS EFFICIENCY</p>
        </div>
        <p className="text-2xl font-bold text-orange-400">{latestSnapshot ? latestSnapshot.avgGasEfficiency.toFixed(1) : 87.0}%</p>
        <p className="text-gray-500 text-xs mt-1">up 0.8% from last hour</p>
      </div>
    </div>
  );
}

export function SnapshotsPanel({ snapshots, adminOverride, onExportCSV, onAddSnapshot, onSetAdminOverride }: SnapshotsPanelProps) {
  return (
    <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6 mb-8">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-xl font-bold text-white">Hourly Snapshots</h2>
        <div className="flex gap-2">
          <button
            onClick={onExportCSV}
            className="px-3 py-1 bg-blue-600 hover:bg-blue-700 text-white rounded-lg text-sm font-medium flex items-center gap-2 transition-colors"
          >
            <Download className="w-4 h-4" />
            Export CSV
          </button>
          <button
            onClick={onAddSnapshot}
            className="px-3 py-1 bg-green-600 hover:bg-green-700 text-white rounded-lg text-sm font-medium transition-colors"
          >
            + Add Snapshot
          </button>
          {!adminOverride ? (
            <button
              onClick={() => onSetAdminOverride(true)}
              className="px-3 py-1 bg-gray-700 hover:bg-gray-600 text-white rounded-lg text-sm font-medium transition-colors"
            >
              Admin Mode
            </button>
          ) : (
            <button
              onClick={() => onSetAdminOverride(false)}
              className="px-3 py-1 bg-red-700 hover:bg-red-600 text-white rounded-lg text-sm font-medium transition-colors"
            >
              Exit Admin
            </button>
          )}
        </div>
      </div>
      {adminOverride && (
        <div className="mb-4 p-3 bg-yellow-900/20 border border-yellow-700 rounded-lg">
          <p className="text-yellow-300 text-sm">Admin Mode Active: Snapshots can be added and exported</p>
        </div>
      )}
      <div className="overflow-x-auto">
        <table className="w-full">
          <thead>
            <tr className="border-b border-[#2a2a35]">
              <th className="px-4 py-2 text-left text-sm font-semibold text-gray-300">Timestamp</th>
              <th className="px-4 py-2 text-left text-sm font-semibold text-gray-300">Avg TPS</th>
              <th className="px-4 py-2 text-left text-sm font-semibold text-gray-300">Avg Latency</th>
              <th className="px-4 py-2 text-left text-sm font-semibold text-gray-300">Avg Uptime</th>
              <th className="px-4 py-2 text-left text-sm font-semibold text-gray-300">Gas Efficiency</th>
            </tr>
          </thead>
          <tbody>
            {snapshots.map((snapshot, index) => (
              <tr key={index} className="border-b border-[#2a2a35] hover:bg-[#2a2a35] transition-colors">
                <td className="px-4 py-2 text-gray-300 text-sm">{snapshot.timestamp}</td>
                <td className="px-4 py-2 text-white text-sm font-medium">{snapshot.avgTps}</td>
                <td className="px-4 py-2 text-white text-sm font-medium">{snapshot.avgLatency} ms</td>
                <td className="px-4 py-2 text-white text-sm font-medium">{snapshot.avgUptime.toFixed(2)}%</td>
                <td className="px-4 py-2 text-white text-sm font-medium">{snapshot.avgGasEfficiency.toFixed(1)}%</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

export function RankingsPanel({ filteredRankings, sortBy, filterChain, onSetSortBy, onSetFilterChain }: RankingsPanelProps) {
  return (
    <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-xl font-bold text-white">Validator Rankings</h2>
        <div className="flex gap-2">
          <select
            value={sortBy}
            onChange={(event) => onSetSortBy(event.target.value as RankingsPanelProps['sortBy'])}
            className="px-3 py-1 bg-[#0a0a0f] border border-[#2a2a35] rounded text-sm text-gray-300 focus:outline-none"
          >
            <option value="tps">Sort by TPS</option>
            <option value="latency">Sort by Latency</option>
            <option value="uptime">Sort by Uptime</option>
            <option value="gasEfficiency">Sort by Gas Efficiency</option>
          </select>
          <select
            value={filterChain}
            onChange={(event) => onSetFilterChain(event.target.value as RankingsPanelProps['filterChain'])}
            className="px-3 py-1 bg-[#0a0a0f] border border-[#2a2a35] rounded text-sm text-gray-300 focus:outline-none"
          >
            <option value="all">All Chains</option>
            <option value="Ethereum">Ethereum</option>
            <option value="Solana">Solana</option>
          </select>
        </div>
      </div>

      <div className="overflow-x-auto">
        <table className="w-full">
          <thead>
            <tr className="border-b border-[#2a2a35]">
              <th className="px-4 py-2 text-left text-sm font-semibold text-gray-300">Rank</th>
              <th className="px-4 py-2 text-left text-sm font-semibold text-gray-300">Validator</th>
              <th className="px-4 py-2 text-left text-sm font-semibold text-gray-300">Chain</th>
              <th className="px-4 py-2 text-left text-sm font-semibold text-gray-300">TPS</th>
              <th className="px-4 py-2 text-left text-sm font-semibold text-gray-300">Latency</th>
              <th className="px-4 py-2 text-left text-sm font-semibold text-gray-300">Uptime</th>
              <th className="px-4 py-2 text-left text-sm font-semibold text-gray-300">Gas Eff.</th>
            </tr>
          </thead>
          <tbody>
            {filteredRankings.map((entry) => (
              <tr key={entry.validatorId} className="border-b border-[#2a2a35] hover:bg-[#2a2a35] transition-colors">
                <td className="px-4 py-2 text-white font-bold">{entry.rank}</td>
                <td className="px-4 py-2">
                  <div>
                    <p className="text-white font-medium">{entry.name}</p>
                    <p className="text-gray-500 text-xs">{entry.validatorId}</p>
                  </div>
                </td>
                <td className="px-4 py-2 text-gray-300">{entry.chain}</td>
                <td className="px-4 py-2 text-blue-400 font-medium">{entry.tps}</td>
                <td className="px-4 py-2 text-green-400 font-medium">{entry.latency} ms</td>
                <td className="px-4 py-2 text-purple-400 font-medium">{entry.uptime.toFixed(2)}%</td>
                <td className="px-4 py-2 text-orange-400 font-medium">{entry.gasEfficiency.toFixed(1)}%</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}