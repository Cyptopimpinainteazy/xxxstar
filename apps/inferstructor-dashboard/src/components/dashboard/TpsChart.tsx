import { Clock } from 'lucide-react';
import { TIME_RANGE_MS, LARGE_NUMBER_THRESHOLD } from '../../constants';
import { SvgLineChart } from '../charts/SvgCharts';

interface TpsPoint {
  time: string;
  ts: number;
  tps: number;
  forwarded: number;
  received: number;
}

interface TpsChartProps {
  tpsHistory: TpsPoint[];
  timeRange: '1m' | '5m' | '15m' | '30m' | '1h' | 'all';
  onTimeRangeChange: (range: '1m' | '5m' | '15m' | '30m' | '1h' | 'all') => void;
}

export function TpsChart({ tpsHistory, timeRange, onTimeRangeChange }: TpsChartProps) {
  const latestTimestamp = tpsHistory[tpsHistory.length - 1]?.ts ?? 0;
  const filteredHistory = timeRange === 'all'
    ? tpsHistory
    : tpsHistory.filter(point => point.ts >= latestTimestamp - TIME_RANGE_MS[timeRange]);

  const timeRangeOptions = [
    { key: '1m' as const, label: '1m' },
    { key: '5m' as const, label: '5m' },
    { key: '15m' as const, label: '15m' },
    { key: '30m' as const, label: '30m' },
    { key: '1h' as const, label: '1H' },
    { key: 'all' as const, label: 'ALL' },
  ];

  return (
    <div className="card mb-8">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-xl font-bold text-white">Real-Time TPS Performance</h2>
        <div className="flex items-center gap-3 text-sm">
          {filteredHistory.length > 0 && (
            <span className="text-blue-400 font-mono">
              Peak: {Math.max(...filteredHistory.map(h => h.tps)).toLocaleString()} TPS
            </span>
          )}
          <span className="flex items-center gap-1 text-gray-500">
            <Clock className="w-3.5 h-3.5" />
            2s
          </span>
        </div>
      </div>

      {/* TradingView-style time range bar */}
      <div className="flex items-center gap-1 mb-4 bg-gray-900/60 rounded-lg p-1 w-fit">
        {timeRangeOptions.map(opt => (
          <button
            key={opt.key}
            onClick={() => onTimeRangeChange(opt.key)}
            className={`px-3 py-1 rounded text-xs font-semibold transition-all ${
              timeRange === opt.key
                ? 'bg-blue-600 text-white shadow-sm'
                : 'text-gray-400 hover:text-white hover:bg-gray-700/50'
            }`}
          >
            {opt.label}
          </button>
        ))}
        <span className="text-gray-600 text-xs ml-2 font-mono">
          {filteredHistory.length} pts
        </span>
      </div>
      
      <div className="h-72">
        <SvgLineChart
          ariaLabel="Dashboard TPS history chart"
          data={filteredHistory}
          labelKey="time"
          series={[{ key: 'tps', color: '#3B82F6', label: 'TPS' }]}
          heightClassName="h-72"
          valueFormatter={(value) => value >= LARGE_NUMBER_THRESHOLD ? `${(value / LARGE_NUMBER_THRESHOLD).toFixed(0)}K` : `${Math.round(value)}`}
        />
      </div>
    </div>
  );
}
