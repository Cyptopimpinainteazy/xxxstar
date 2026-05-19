import React, { useState } from 'react';
import {
  Shield,
  CheckCircle,
  AlertTriangle,
  XCircle,
  Clock,
  Server,
} from 'lucide-react';
import {
  ResponsiveContainer,
  LineChart,
  Line,
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
} from 'recharts';
import clsx from 'clsx';

type HealthTab = 'overview' | 'services' | 'metrics';
type Status = 'up' | 'degraded' | 'down';

interface HealthCard {
  name: string;
  status: Status;
  uptime: number;
  lastCheck: string;
}

interface ServiceRow {
  name: string;
  status: Status;
  responseTime: string;
  uptime: number;
  lastError: string;
}

const HEALTH_CARDS: HealthCard[] = [
  { name: 'API Gateway', status: 'up', uptime: 99.9, lastCheck: '2s ago' },
  { name: 'Blockchain Node', status: 'up', uptime: 100, lastCheck: '5s ago' },
  { name: 'IPFS Cluster', status: 'up', uptime: 98.2, lastCheck: '3s ago' },
  { name: 'GPU Swarm', status: 'degraded', uptime: 94.7, lastCheck: '8s ago' },
];

const SERVICES: ServiceRow[] = [
  { name: 'API Gateway', status: 'up', responseTime: '12ms', uptime: 99.9, lastError: 'None' },
  { name: 'Auth Service', status: 'up', responseTime: '8ms', uptime: 99.95, lastError: 'None' },
  { name: 'Block Indexer', status: 'up', responseTime: '45ms', uptime: 99.8, lastError: '2h ago — timeout' },
  { name: 'Comit Processor', status: 'up', responseTime: '23ms', uptime: 99.7, lastError: 'None' },
  { name: 'DNS Resolver', status: 'up', responseTime: '3ms', uptime: 100, lastError: 'None' },
  { name: 'EVM RPC', status: 'up', responseTime: '18ms', uptime: 99.9, lastError: 'None' },
  { name: 'IPFS Gateway', status: 'up', responseTime: '34ms', uptime: 98.2, lastError: '45m ago — 503' },
  { name: 'Metrics Collector', status: 'up', responseTime: '5ms', uptime: 99.99, lastError: 'None' },
  { name: 'Notification Service', status: 'up', responseTime: '15ms', uptime: 99.6, lastError: '6h ago — queue full' },
  { name: 'SVM RPC', status: 'degraded', responseTime: '67ms', uptime: 94.7, lastError: '10m ago — high latency' },
  { name: 'Swap Engine', status: 'up', responseTime: '28ms', uptime: 99.85, lastError: 'None' },
  { name: 'Treasury Service', status: 'up', responseTime: '11ms', uptime: 99.95, lastError: 'None' },
];

const generateResponseTimeData = () =>
  Array.from({ length: 24 }, (_, i) => ({
    time: `${String(i).padStart(2, '0')}:00`,
    'API Gateway': 8 + Math.random() * 10,
    'EVM RPC': 12 + Math.random() * 15,
    'SVM RPC': 30 + Math.random() * 50,
    'IPFS Gateway': 20 + Math.random() * 25,
  }));

const generateErrorRateData = () =>
  Array.from({ length: 24 }, (_, i) => ({
    time: `${String(i).padStart(2, '0')}:00`,
    errorRate: Math.random() * 2.5,
  }));

const RESPONSE_TIME_DATA = generateResponseTimeData();
const ERROR_RATE_DATA = generateErrorRateData();

const statusDot = (status: Status) => {
  if (status === 'up') return 'bg-green-400';
  if (status === 'degraded') return 'bg-yellow-400';
  return 'bg-red-400';
};

const statusBadge = (status: Status) => {
  if (status === 'up')
    return (
      <span className="flex items-center gap-1 text-xs text-green-400">
        <CheckCircle size={12} /> Healthy
      </span>
    );
  if (status === 'degraded')
    return (
      <span className="flex items-center gap-1 text-xs text-yellow-400">
        <AlertTriangle size={12} /> Degraded
      </span>
    );
  return (
    <span className="flex items-center gap-1 text-xs text-red-400">
      <XCircle size={12} /> Down
    </span>
  );
};

const overallStatus: Status = HEALTH_CARDS.some((c) => c.status === 'down')
  ? 'down'
  : HEALTH_CARDS.some((c) => c.status === 'degraded')
    ? 'degraded'
    : 'up';

const HealthDashboardPanel: React.FC = () => {
  const [tab, setTab] = useState<HealthTab>('overview');

  const renderOverview = () => (
    <div className="grid grid-cols-2 gap-4">
      {HEALTH_CARDS.map((card) => (
        <div key={card.name} className="bg-[#111111] rounded-xl p-4 border border-[#1a1a1a]">
          <div className="flex items-center justify-between mb-3">
            <div className="flex items-center gap-2">
              <span className={clsx('w-2.5 h-2.5 rounded-full', statusDot(card.status))} />
              <span className="text-sm font-semibold text-white">{card.name}</span>
            </div>
            {statusBadge(card.status)}
          </div>
          <div className="mb-2">
            <div className="flex justify-between text-xs mb-1">
              <span className="text-gray-500">Uptime</span>
              <span
                className={clsx(
                  'font-medium',
                  card.uptime >= 99 ? 'text-green-400' : card.uptime >= 95 ? 'text-yellow-400' : 'text-red-400',
                )}
              >
                {card.uptime}%
              </span>
            </div>
            <div className="h-1.5 rounded-full bg-[#0a0a0f] overflow-hidden">
              <div
                className={clsx(
                  'h-full rounded-full',
                  card.uptime >= 99 ? 'bg-green-500' : card.uptime >= 95 ? 'bg-yellow-500' : 'bg-red-500',
                )}
                style={{ width: `${card.uptime}%` }}
              />
            </div>
          </div>
          <div className="flex items-center gap-1 text-[10px] text-gray-600">
            <Clock size={10} /> Last check: {card.lastCheck}
          </div>
        </div>
      ))}
    </div>
  );

  const renderServices = () => (
    <div className="bg-[#111111] rounded-xl border border-[#1a1a1a] overflow-hidden">
      <table className="w-full text-sm">
        <thead>
          <tr className="border-b border-[#1a1a1a] text-gray-500 text-xs">
            <th className="text-left p-3">Service</th>
            <th className="text-left p-3">Status</th>
            <th className="text-right p-3">Response Time</th>
            <th className="text-right p-3">Uptime</th>
            <th className="text-right p-3">Last Error</th>
          </tr>
        </thead>
        <tbody>
          {SERVICES.map((svc) => (
            <tr
              key={svc.name}
              className="border-b border-[#1a1a1a] last:border-0 hover:bg-[#0f0f14] transition-colors"
            >
              <td className="p-3">
                <div className="flex items-center gap-2">
                  <Server size={12} className="text-gray-500" />
                  <span className="text-white font-medium">{svc.name}</span>
                </div>
              </td>
              <td className="p-3">{statusBadge(svc.status)}</td>
              <td className="p-3 text-right text-white">{svc.responseTime}</td>
              <td
                className={clsx(
                  'p-3 text-right font-medium',
                  svc.uptime >= 99.5 ? 'text-green-400' : svc.uptime >= 95 ? 'text-yellow-400' : 'text-red-400',
                )}
              >
                {svc.uptime}%
              </td>
              <td className="p-3 text-right text-gray-500 text-xs">{svc.lastError}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );

  const renderMetrics = () => (
    <div className="space-y-4">
      {/* Response time chart */}
      <div className="bg-[#111111] rounded-xl p-4 border border-[#1a1a1a]">
        <h3 className="text-sm font-semibold mb-3">Response Time (ms)</h3>
        <ResponsiveContainer width="100%" height={200}>
          <LineChart data={RESPONSE_TIME_DATA}>
            <CartesianGrid strokeDasharray="3 3" stroke="#1a1a1a" />
            <XAxis dataKey="time" tick={{ fontSize: 10, fill: '#666' }} />
            <YAxis tick={{ fontSize: 10, fill: '#666' }} />
            <Tooltip contentStyle={{ background: '#111', border: '1px solid #1a1a1a', borderRadius: 8, fontSize: 12 }} />
            <Legend wrapperStyle={{ fontSize: 10 }} />
            <Line type="monotone" dataKey="API Gateway" stroke="#22c55e" strokeWidth={1.5} dot={false} />
            <Line type="monotone" dataKey="EVM RPC" stroke="#3b82f6" strokeWidth={1.5} dot={false} />
            <Line type="monotone" dataKey="SVM RPC" stroke="#f59e0b" strokeWidth={1.5} dot={false} />
            <Line type="monotone" dataKey="IPFS Gateway" stroke="#a855f7" strokeWidth={1.5} dot={false} />
          </LineChart>
        </ResponsiveContainer>
      </div>

      <div className="grid grid-cols-2 gap-4">
        {/* Error rate chart */}
        <div className="bg-[#111111] rounded-xl p-4 border border-[#1a1a1a]">
          <h3 className="text-sm font-semibold mb-3">Error Rate (%)</h3>
          <ResponsiveContainer width="100%" height={160}>
            <AreaChart data={ERROR_RATE_DATA}>
              <defs>
                <linearGradient id="errGrad" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stopColor="#ef4444" stopOpacity={0.3} />
                  <stop offset="100%" stopColor="#ef4444" stopOpacity={0} />
                </linearGradient>
              </defs>
              <CartesianGrid strokeDasharray="3 3" stroke="#1a1a1a" />
              <XAxis dataKey="time" tick={{ fontSize: 10, fill: '#666' }} />
              <YAxis domain={[0, 5]} tick={{ fontSize: 10, fill: '#666' }} />
              <Tooltip contentStyle={{ background: '#111', border: '1px solid #1a1a1a', borderRadius: 8, fontSize: 12 }} />
              <Area type="monotone" dataKey="errorRate" stroke="#ef4444" fill="url(#errGrad)" strokeWidth={2} />
            </AreaChart>
          </ResponsiveContainer>
        </div>

        {/* Throughput */}
        <div className="bg-[#111111] rounded-xl p-4 border border-[#1a1a1a]">
          <h3 className="text-sm font-semibold mb-3">Throughput</h3>
          <div className="flex items-baseline gap-2 mb-2">
            <span className="text-3xl font-bold text-white">4,821</span>
            <span className="text-xs text-gray-500">req/sec</span>
          </div>
          <div className="text-xs text-green-400 mb-3">+12.3% from last hour</div>
          <div className="flex items-end gap-px h-16">
            {Array.from({ length: 24 }, (_, i) => {
              const h = 20 + Math.random() * 80;
              return (
                <div
                  key={i}
                  className="flex-1 rounded-t bg-gradient-to-t from-blue-500/40 to-blue-500 transition-all"
                  style={{ height: `${h}%` }}
                />
              );
            })}
          </div>
        </div>
      </div>
    </div>
  );

  return (
    <div className="h-full flex flex-col bg-[#0a0a0f] text-white overflow-auto">
      {/* Header */}
      <div className="flex items-center justify-between px-5 py-4 border-b border-[#1a1a1a]">
        <div className="flex items-center gap-3">
          <Shield size={18} className="text-green-400" />
          <h1 className="text-lg font-bold">System Health Dashboard</h1>
          <span
            className={clsx(
              'text-xs px-2 py-0.5 rounded-full font-medium',
              overallStatus === 'up' && 'bg-green-500/20 text-green-400',
              overallStatus === 'degraded' && 'bg-yellow-500/20 text-yellow-400',
              overallStatus === 'down' && 'bg-red-500/20 text-red-400',
            )}
          >
            {overallStatus === 'up' ? 'Healthy' : overallStatus === 'degraded' ? 'Degraded' : 'Down'}
          </span>
        </div>
        <div className="flex items-center gap-1 bg-[#111111] rounded-lg p-1 border border-[#1a1a1a]">
          {(['overview', 'services', 'metrics'] as HealthTab[]).map((t) => (
            <button
              key={t}
              onClick={() => setTab(t)}
              className={clsx(
                'px-3 py-1.5 rounded-md text-xs font-medium transition-colors capitalize',
                tab === t ? 'bg-green-500/20 text-green-400' : 'text-gray-500 hover:text-white',
              )}
            >
              {t}
            </button>
          ))}
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 p-5 overflow-auto">
        {tab === 'overview' && renderOverview()}
        {tab === 'services' && renderServices()}
        {tab === 'metrics' && renderMetrics()}
      </div>
    </div>
  );
};

export default HealthDashboardPanel;
