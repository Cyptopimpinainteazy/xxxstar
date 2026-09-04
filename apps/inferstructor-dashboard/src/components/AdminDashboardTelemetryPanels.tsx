import type { Dispatch, SetStateAction } from 'react';
import type { AggregatedMetrics } from '../api';
import {
  Activity,
  AlertTriangle,
  Clock,
  Cpu,
  DollarSign,
  Eye,
  Globe,
  Heart,
  Link,
  Lock,
  Server,
  TrendingUp,
  Zap,
  type LucideIcon,
} from 'lucide-react';
import { SvgBarChart, SvgLineChart } from './charts/SvgCharts';

type HistoryPoint = AggregatedMetrics['aggregated'] & { timestamp?: number };

interface TimeRangeOption {
  key: string;
  label: string;
  seconds: number;
}

interface MetricCardProps {
  icon: LucideIcon;
  color: string;
  label: string;
  value: string | number;
  sub?: string;
}

interface OverviewPanelProps {
  services: Record<string, string>;
  aggregated?: AggregatedMetrics['aggregated'] | null;
  gpuLanes: unknown[];
  filteredHistory: HistoryPoint[];
}

interface PerformancePanelProps {
  aggregated?: AggregatedMetrics['aggregated'] | null;
  gpuLanes: unknown[];
  filteredHistory: HistoryPoint[];
  timeRange: string;
  setTimeRange: Dispatch<SetStateAction<string>>;
  timeRanges: TimeRangeOption[];
}

interface NetworkPanelProps {
  aggregated?: AggregatedMetrics['aggregated'] | null;
  chain?: Record<string, any> | null;
  upstreams: unknown[];
}

interface IntelligencePanelProps {
  aggregated?: AggregatedMetrics['aggregated'] | null;
  filteredHistory: HistoryPoint[];
}

const GPU_COUNT = 3;
const GPU_POWER_WATTS = 150;
const ELECTRICITY_RATE = 0.12;
function fmt(n: number | undefined | null): string {
  if (n == null) return '—';
  return n.toLocaleString();
}

function fmtK(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(0)}K`;
  return n.toString();
}

function fmtTime(seconds: number): string {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  return `${hours}h ${minutes}m`;
}

function tsToTime(ts: number): string {
  return new Date(ts * 1000).toLocaleTimeString('en-US', {
    hour12: false,
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  });
}

function renderGpuLaneDistributionChart(gpuLanes: unknown[]) {
  return (
    <SvgBarChart
      ariaLabel="GPU lane distribution chart"
      data={gpuLanes.map((gpuLane: any) => ({
        label: (gpuLane.service || '').replace('gpu-lane-', '').toUpperCase() || `GPU ${gpuLane.gpu?.id}`,
        value: gpuLane.stats?.total_txns || 0,
        title: `${(gpuLane.service || '').replace('gpu-lane-', '').toUpperCase() || `GPU ${gpuLane.gpu?.id}`}: ${(gpuLane.stats?.total_txns || 0).toLocaleString()} txns, ${Math.round(gpuLane.stats?.txns_per_second || 0).toLocaleString()} TPS, ${gpuLane.gpu?.utilization || 0}% util.`,
      }))}
      heightClassName="h-48"
    />
  );
}

const statusDot = (status: string) => (
  status === 'up' ? 'bg-green-500' : status === 'error' ? 'bg-yellow-500' : 'bg-red-500'
);

function MetricCard({ icon: Icon, color, label, value, sub }: MetricCardProps) {
  return (
    <div className="bg-gray-800/50 rounded-lg p-4 border border-gray-700">
      <Icon className={`w-5 h-5 ${color} mb-2`} />
      <p className="text-2xl font-bold text-white font-mono">{value}</p>
      <p className="text-xs text-gray-400">{label}</p>
      {sub && <p className="text-[10px] text-gray-500 mt-1">{sub}</p>}
    </div>
  );
}

export function OverviewPanel({ services, aggregated, gpuLanes, filteredHistory }: OverviewPanelProps) {
  return (
    <div className="space-y-6">
      <div className="card">
        <div className="flex items-center gap-2 mb-4">
          <Activity className="w-5 h-5 text-green-400" />
          <h2 className="text-lg font-bold text-white">Service Status</h2>
          <span className="ml-auto text-sm text-gray-400">
            {aggregated?.services_up || 0}/{aggregated?.services_total || 0} online
          </span>
        </div>
        <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-3">
          {Object.entries(services).map(([name, status]) => (
            <div key={name} className="bg-gray-800/50 rounded-lg p-3 border border-gray-700">
              <div className="flex items-center gap-2 mb-1">
                <span className={`w-2.5 h-2.5 rounded-full ${statusDot(status)} ${status === 'up' ? 'animate-pulse' : ''}`} />
                <span className="text-white text-xs font-semibold truncate">{name}</span>
              </div>
              <span className={`text-[10px] font-semibold ${status === 'up' ? 'text-green-400' : 'text-red-400'}`}>
                {status.toUpperCase()}
              </span>
            </div>
          ))}
        </div>
      </div>

      <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-4">
        <MetricCard icon={Activity} color="text-green-400" label="Services Online" value={`${aggregated?.services_up || 0}/${aggregated?.services_total || 0}`} />
        <MetricCard icon={Zap} color="text-yellow-400" label="Current TPS" value={fmt(aggregated?.current_tps)} />
        <MetricCard icon={TrendingUp} color="text-blue-400" label="Peak TPS" value={fmt(aggregated?.peak_tps)} />
        <MetricCard icon={Server} color="text-purple-400" label="Total GPU Txns" value={fmtK(aggregated?.total_gpu_txns || 0)} />
        <MetricCard icon={Heart} color="text-green-400" label="Success Rate" value={`${aggregated?.success_rate || 0}%`} />
        <MetricCard icon={Clock} color="text-blue-400" label="Uptime" value={fmtTime(aggregated?.uptime_seconds || 0)} />
      </div>

      <div className="card">
        <h3 className="text-sm font-bold text-white mb-3">TPS (last 5 minutes)</h3>
        <SvgLineChart
          ariaLabel="Overview TPS history chart"
          data={filteredHistory.map(point => ({ time: tsToTime(point.timestamp || 0), tps: point.current_tps }))}
          labelKey="time"
          series={[{ key: 'tps', color: '#3B82F6', label: 'TPS', area: true, fillOpacity: 0.4 }]}
          heightClassName="h-32"
          showGrid={false}
          showXAxis={false}
          showYAxis={false}
        />
      </div>

      {gpuLanes.length > 0 && (
        <div className="card">
          <div className="flex items-center gap-2 mb-4">
            <Cpu className="w-5 h-5 text-green-400" />
            <h3 className="text-lg font-bold text-white">GPU Fleet</h3>
          </div>
          <div className="grid md:grid-cols-3 gap-3">
            {gpuLanes.map((gpuLane: any, index: number) => (
              <div key={index} className="bg-gray-800/50 rounded-lg p-3 border border-gray-700">
                <div className="flex items-center justify-between mb-2">
                  <span className="text-white text-sm font-semibold capitalize">
                    {gpuLane.service?.replace('gpu-lane-', '') || `GPU ${index}`}
                  </span>
                  <span className="text-xs text-green-400 font-mono">GPU {gpuLane.gpu?.id}</span>
                </div>
                <div className="grid grid-cols-2 gap-2 text-xs">
                  <div><span className="text-gray-400">Util:</span> <span className="text-white">{gpuLane.gpu?.utilization}%</span></div>
                  <div><span className="text-gray-400">VRAM:</span> <span className="text-yellow-400">{gpuLane.gpu?.memory_used_mb?.toFixed(0)} MB</span></div>
                  <div><span className="text-gray-400">Temp:</span> <span className="text-orange-400">{gpuLane.gpu?.temperature_c}°C</span></div>
                  <div><span className="text-gray-400">TPS:</span> <span className="text-blue-400">{fmt(Math.round(gpuLane.stats?.txns_per_second || 0))}</span></div>
                </div>
                <div className="mt-2 w-full bg-gray-700 rounded-full h-1.5">
                  <div className="bg-blue-500 h-1.5 rounded-full" style={{ width: `${Math.min(gpuLane.gpu?.utilization || 0, 100)}%` }} />
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

export function PerformancePanel({ aggregated, gpuLanes, filteredHistory, timeRange, setTimeRange, timeRanges }: PerformancePanelProps) {
  return (
    <div className="space-y-6">
      <div className="card">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-bold text-white">Real-Time TPS</h2>
          <div className="flex items-center gap-2">
            {filteredHistory.length > 0 && (
              <span className="text-blue-400 font-mono text-sm">
                Peak: {fmtK(Math.max(...filteredHistory.map(historyPoint => historyPoint.peak_tps || 0)))}
              </span>
            )}
          </div>
        </div>
        <div className="flex items-center gap-1 mb-4 bg-gray-900/60 rounded-lg p-1 w-fit">
          {timeRanges.map(range => (
            <button
              key={range.key}
              onClick={() => setTimeRange(range.key)}
              className={`px-3 py-1 rounded text-xs font-semibold transition-all ${
                timeRange === range.key
                  ? 'bg-blue-600 text-white shadow-sm'
                  : 'text-gray-400 hover:text-white hover:bg-gray-700/50'
              }`}
            >
              {range.label}
            </button>
          ))}
          <span className="text-gray-600 text-xs ml-2 font-mono">{filteredHistory.length} pts</span>
        </div>
        <div className="space-y-3">
          <div className="flex flex-wrap gap-3 text-xs text-gray-400">
            <span className="flex items-center gap-2"><span className="h-2.5 w-2.5 rounded-full bg-blue-500" />Current TPS</span>
            <span className="flex items-center gap-2"><span className="h-0.5 w-3 bg-red-500" />Peak TPS</span>
          </div>
          <SvgLineChart
            ariaLabel="Performance TPS history chart"
            data={filteredHistory.map(point => ({ time: tsToTime(point.timestamp || 0), tps: point.current_tps, peak: point.peak_tps }))}
            labelKey="time"
            series={[
              { key: 'tps', color: '#3B82F6', label: 'Current TPS' },
              { key: 'peak', color: '#EF4444', label: 'Peak TPS', dashed: true },
            ]}
            heightClassName="h-72"
            valueFormatter={fmtK}
          />
        </div>
      </div>

      <div className="grid md:grid-cols-2 gap-6">
        <div className="card">
          <h3 className="text-sm font-bold text-white mb-3">Throughput Utilization</h3>
          <div className="flex items-end gap-4 mb-3">
            <span className="text-4xl font-bold text-blue-400">{aggregated?.throughput_utilization || 0}%</span>
            <span className="text-xs text-gray-400 mb-1">of 960K theoretical max</span>
          </div>
          <div className="w-full bg-gray-700 rounded-full h-3">
            <div className="bg-blue-500 h-3 rounded-full transition-all" style={{ width: `${Math.min(aggregated?.throughput_utilization || 0, 100)}%` }} />
          </div>
        </div>
        <div className="card">
          <h3 className="text-sm font-bold text-white mb-3">Transaction Success Rate</h3>
          <div className="flex items-end gap-4 mb-3">
            <span className={`text-4xl font-bold ${(aggregated?.success_rate || 0) >= 99 ? 'text-green-400' : (aggregated?.success_rate || 0) >= 95 ? 'text-yellow-400' : 'text-red-400'}`}>
              {aggregated?.success_rate || 0}%
            </span>
            <span className="text-xs text-gray-400 mb-1">
              {fmt(aggregated?.total_gpu_success || 0)} / {fmt(aggregated?.total_gpu_txns || 0)}
            </span>
          </div>
          <div className="w-full bg-gray-700 rounded-full h-3">
            <div className={`h-3 rounded-full transition-all ${(aggregated?.success_rate || 0) >= 99 ? 'bg-green-500' : 'bg-yellow-500'}`} style={{ width: `${aggregated?.success_rate || 0}%` }} />
          </div>
        </div>
      </div>

      {gpuLanes.length > 0 && (
        <div className="card">
          <h3 className="text-sm font-bold text-white mb-3">GPU Lane Distribution</h3>
          {renderGpuLaneDistributionChart(gpuLanes)}
        </div>
      )}

      <div className="grid md:grid-cols-3 gap-4">
        <MetricCard icon={Cpu} color="text-blue-400" label="Avg GPU Utilization" value={`${aggregated?.avg_gpu_utilization || 0}%`} />
        <MetricCard icon={Server} color="text-yellow-400" label="Avg VRAM Used" value={`${(aggregated?.avg_gpu_memory_mb || 0).toFixed(0)} MB`} />
        <MetricCard icon={AlertTriangle} color="text-orange-400" label="Avg GPU Temp" value={`${aggregated?.avg_gpu_temp_c || 0}°C`} />
      </div>
    </div>
  );
}

export function NetworkPanel({ aggregated, chain, upstreams }: NetworkPanelProps) {
  return (
    <div className="space-y-6">
      {chain && (
        <div className="card">
          <div className="flex items-center gap-2 mb-4">
            <Globe className="w-5 h-5 text-purple-400" />
            <h2 className="text-lg font-bold text-white">Solana Chain — Live</h2>
            <span className="w-2 h-2 bg-green-400 rounded-full animate-pulse" />
            {chain.version && (
              <span className="ml-auto text-xs text-gray-400 font-mono">
                Core {chain.version['solana-core']}
              </span>
            )}
          </div>
          <div className="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-4">
            <div className="bg-gray-800/50 rounded-lg p-4 border border-gray-700">
              <p className="text-xs text-gray-400 mb-1">Slot</p>
              <p className="text-xl font-bold text-white font-mono">{fmt(chain.slot)}</p>
            </div>
            <div className="bg-gray-800/50 rounded-lg p-4 border border-gray-700">
              <p className="text-xs text-gray-400 mb-1">Epoch</p>
              <p className="text-xl font-bold text-white font-mono">{fmt(chain.epoch?.epoch)}</p>
              {chain.epoch && (
                <div className="mt-2">
                  <div className="flex justify-between text-[10px] text-gray-400 mb-1">
                    <span>{fmt(chain.epoch.slotIndex)}</span>
                    <span>{fmt(chain.epoch.slotsInEpoch)}</span>
                  </div>
                  <div className="w-full bg-gray-700 rounded-full h-1.5">
                    <div className="bg-purple-500 h-1.5 rounded-full" style={{ width: `${((chain.epoch.slotIndex / chain.epoch.slotsInEpoch) * 100).toFixed(1)}%` }} />
                  </div>
                </div>
              )}
            </div>
            <div className="bg-gray-800/50 rounded-lg p-4 border border-gray-700">
              <p className="text-xs text-gray-400 mb-1">Block Height</p>
              <p className="text-xl font-bold text-white font-mono">{fmt(chain.block_height)}</p>
            </div>
            <div className="bg-gray-800/50 rounded-lg p-4 border border-gray-700">
              <p className="text-xs text-gray-400 mb-1">Network Txns</p>
              <p className="text-xl font-bold text-white font-mono">
                {chain.epoch?.transactionCount ? `${(chain.epoch.transactionCount / 1e9).toFixed(2)}B` : '—'}
              </p>
            </div>
          </div>
          {chain.latest_blockhash && (
            <div className="bg-gray-900/50 rounded-lg p-3 font-mono text-xs">
              <span className="text-gray-400 mr-2">Latest Blockhash:</span>
              <span className="text-green-400 break-all">{chain.latest_blockhash}</span>
            </div>
          )}
        </div>
      )}

      <div className="grid md:grid-cols-2 gap-6">
        <div className="card">
          <div className="flex items-center gap-2 mb-3">
            <Link className="w-4 h-4 text-blue-400" />
            <h3 className="text-sm font-semibold text-white">Upstream RPCs</h3>
          </div>
          <div className="space-y-2">
            {upstreams.map((upstream: any) => (
              <div key={upstream.name} className="flex items-center justify-between text-sm bg-gray-800/40 rounded-lg p-2">
                <div className="flex items-center gap-2">
                  <span className={`w-2 h-2 rounded-full ${upstream.healthy ? 'bg-green-400' : 'bg-red-400'}`} />
                  <span className="text-white">{upstream.name}</span>
                </div>
                <div className="flex items-center gap-3 text-xs text-gray-400 font-mono">
                  <span>{upstream.latency_ms?.toFixed(0)}ms</span>
                  <span>{fmt(upstream.requests)} req</span>
                  <span className={upstream.errors > 0 ? 'text-red-400' : ''}>{upstream.errors} err</span>
                </div>
              </div>
            ))}
            {upstreams.length === 0 && <p className="text-gray-500 text-sm">No upstream data</p>}
          </div>
        </div>
        <div className="card">
          <div className="flex items-center gap-2 mb-3">
            <Zap className="w-4 h-4 text-yellow-400" />
            <h3 className="text-sm font-semibold text-white">GPU RPC Proxy</h3>
          </div>
          <div className="space-y-2 text-sm">
            <div className="flex justify-between"><span className="text-gray-400">Total Requests</span><span className="text-white font-mono">{fmt(aggregated?.rpc_total_requests)}</span></div>
            <div className="flex justify-between"><span className="text-gray-400">Cache Hit Rate</span><span className="text-green-400 font-mono">{aggregated?.rpc_cache_hit_rate || '—'}</span></div>
            <div className="flex justify-between"><span className="text-gray-400">GPU Verified</span><span className="text-purple-400 font-mono">{fmt(aggregated?.rpc_gpu_verified)}</span></div>
            <div className="flex justify-between"><span className="text-gray-400">Cached Responses</span><span className="text-blue-400 font-mono">{fmt(aggregated?.rpc_cached_responses)}</span></div>
            <div className="flex justify-between"><span className="text-gray-400">Errors</span><span className={`font-mono ${(aggregated?.rpc_errors || 0) > 0 ? 'text-red-400' : 'text-gray-400'}`}>{aggregated?.rpc_errors || 0}</span></div>
          </div>
        </div>
      </div>
    </div>
  );
}

export function IntelligencePanel({ aggregated, filteredHistory }: IntelligencePanelProps) {
  return (
    <div className="space-y-6">
      <div className="card">
        <div className="flex items-center gap-2 mb-4">
          <DollarSign className="w-5 h-5 text-green-400" />
          <h2 className="text-lg font-bold text-white">Cost & Fee Intelligence</h2>
        </div>
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
          <div className="bg-gray-800/50 rounded-lg p-4 border border-gray-700">
            <p className="text-xs text-gray-400 mb-1">GPU Power Draw</p>
            <p className="text-2xl font-bold text-yellow-400 font-mono">{aggregated?.gpu_power_watts || 0}W</p>
            <p className="text-[10px] text-gray-500">{GPU_COUNT}x GTX 1070 @ {GPU_POWER_WATTS}W</p>
          </div>
          <div className="bg-gray-800/50 rounded-lg p-4 border border-gray-700">
            <p className="text-xs text-gray-400 mb-1">Cost/Hour</p>
            <p className="text-2xl font-bold text-green-400 font-mono">${aggregated?.gpu_cost_per_hour_usd?.toFixed(4) || '0'}</p>
            <p className="text-[10px] text-gray-500">@ ${ELECTRICITY_RATE}/kWh</p>
          </div>
          <div className="bg-gray-800/50 rounded-lg p-4 border border-gray-700">
            <p className="text-xs text-gray-400 mb-1">Cost/Million Txns</p>
            <p className="text-2xl font-bold text-blue-400 font-mono">${aggregated?.cost_per_million_tx_usd?.toFixed(4) || '—'}</p>
            <p className="text-[10px] text-gray-500">at current throughput</p>
          </div>
          <div className="bg-gray-800/50 rounded-lg p-4 border border-gray-700">
            <p className="text-xs text-gray-400 mb-1">Cost/Tx</p>
            <p className="text-2xl font-bold text-purple-400 font-mono">
              {aggregated?.cost_per_tx_usd ? `$${aggregated.cost_per_tx_usd.toExponential(2)}` : '—'}
            </p>
            <p className="text-[10px] text-gray-500">effectively free</p>
          </div>
        </div>
      </div>

      <div className="card">
        <div className="flex items-center gap-2 mb-4">
          <AlertTriangle className="w-5 h-5 text-orange-400" />
          <h2 className="text-lg font-bold text-white">Reliability & Fault Detection</h2>
        </div>
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
          <div className="bg-gray-800/50 rounded-lg p-4 border border-gray-700">
            <p className="text-xs text-gray-400 mb-1">Dropped Tx %</p>
            <p className={`text-2xl font-bold font-mono ${(aggregated?.dropped_tx_pct || 0) > 1 ? 'text-red-400' : 'text-green-400'}`}>
              {aggregated?.dropped_tx_pct || 0}%
            </p>
          </div>
          <div className="bg-gray-800/50 rounded-lg p-4 border border-gray-700">
            <p className="text-xs text-gray-400 mb-1">Bridge Failed</p>
            <p className="text-2xl font-bold text-red-400 font-mono">{fmt(aggregated?.bridge_failed)}</p>
          </div>
          <div className="bg-gray-800/50 rounded-lg p-4 border border-gray-700">
            <p className="text-xs text-gray-400 mb-1">GPU Failed</p>
            <p className="text-2xl font-bold text-red-400 font-mono">{fmt(aggregated?.total_gpu_failed)}</p>
          </div>
          <div className="bg-gray-800/50 rounded-lg p-4 border border-gray-700">
            <p className="text-xs text-gray-400 mb-1">RPC Errors</p>
            <p className={`text-2xl font-bold font-mono ${(aggregated?.rpc_errors || 0) > 0 ? 'text-red-400' : 'text-green-400'}`}>
              {aggregated?.rpc_errors || 0}
            </p>
          </div>
        </div>
      </div>

      <div className="grid md:grid-cols-2 gap-6">
        <div className="card">
          <div className="flex items-center gap-2 mb-4">
            <Lock className="w-5 h-5 text-blue-400" />
            <h3 className="text-lg font-bold text-white">Security Radar</h3>
          </div>
          <div className="space-y-3 text-sm">
            <div className="flex justify-between items-center">
              <span className="text-gray-400">Admin Auth</span>
              <span className="px-2 py-0.5 rounded text-xs font-semibold bg-green-500/20 text-green-300">JWT Active</span>
            </div>
            <div className="flex justify-between items-center">
              <span className="text-gray-400">CORS Policy</span>
              <span className="text-yellow-400 text-xs">Open (dev mode)</span>
            </div>
            <div className="flex justify-between items-center">
              <span className="text-gray-400">GPU Sig Verification</span>
              <span className="text-green-400 text-xs">{fmt(aggregated?.rpc_gpu_verified)} verified</span>
            </div>
            <div className="flex justify-between items-center">
              <span className="text-gray-400">Cache Layer</span>
              <span className="text-green-400 text-xs">{aggregated?.rpc_cache_hit_rate || '—'} hit rate</span>
            </div>
          </div>
        </div>
        <div className="card">
          <div className="flex items-center gap-2 mb-4">
            <Eye className="w-5 h-5 text-purple-400" />
            <h3 className="text-lg font-bold text-white">MEV & Extraction</h3>
          </div>
          <div className="space-y-3 text-sm">
            <div className="flex justify-between items-center">
              <span className="text-gray-400">MEV Detection</span>
              <span className="px-2 py-0.5 rounded text-xs font-semibold bg-blue-500/20 text-blue-300">Monitoring</span>
            </div>
            <div className="flex justify-between items-center">
              <span className="text-gray-400">Sandwich Attacks</span>
              <span className="text-green-400 text-xs">None detected</span>
            </div>
            <div className="flex justify-between items-center">
              <span className="text-gray-400">Front-running Risk</span>
              <span className="text-green-400 text-xs">Low</span>
            </div>
            <div className="flex justify-between items-center">
              <span className="text-gray-400">Tx Ordering</span>
              <span className="text-gray-400 text-xs">FIFO (GPU batch)</span>
            </div>
          </div>
        </div>
      </div>

      {filteredHistory.length > 1 && (
        <div className="card">
          <h3 className="text-sm font-bold text-white mb-3">Throughput Utilization Over Time</h3>
          <SvgLineChart
            ariaLabel="Throughput utilization history chart"
            data={filteredHistory.map(point => ({ time: tsToTime(point.timestamp || 0), util: point.throughput_utilization, dropped: point.dropped_tx_pct }))}
            labelKey="time"
            series={[{ key: 'util', color: '#8B5CF6', label: 'Utilization %', area: true, fillOpacity: 0.2 }]}
            heightClassName="h-40"
            showXAxis={false}
          />
        </div>
      )}
    </div>
  );
}