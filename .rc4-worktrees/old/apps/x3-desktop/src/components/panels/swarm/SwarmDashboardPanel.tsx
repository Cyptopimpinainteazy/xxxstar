import React, { useState, useEffect, useCallback } from 'react';
import {
  RefreshCw,
  Cpu,
  Activity,
  Layers,
  Timer,
  AlertTriangle,
  Info,
  AlertCircle,
  X,
} from 'lucide-react';
import {
  ResponsiveContainer,
  AreaChart,
  Area,
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
} from 'recharts';
import clsx from 'clsx';

const generateUtilData = () =>
  Array.from({ length: 24 }, (_, i) => ({
    time: `${String(i).padStart(2, '0')}:00`,
    utilization: 60 + Math.random() * 30,
  }));

const generateTaskData = () =>
  Array.from({ length: 12 }, (_, i) => ({
    time: `${String(i * 2).padStart(2, '0')}:00`,
    completed: 80 + Math.floor(Math.random() * 40),
    failed: Math.floor(Math.random() * 8),
  }));

interface Alert {
  id: number;
  level: 'info' | 'warning' | 'critical';
  title: string;
  message: string;
}

const INITIAL_ALERTS: Alert[] = [
  { id: 1, level: 'critical', title: 'GPU Node gpu-rack-07 Offline', message: 'Node has been unresponsive for 3 minutes. Auto-failover initiated.' },
  { id: 2, level: 'warning', title: 'Memory Threshold Exceeded', message: 'Cluster memory usage at 88%. Consider scaling or evicting cold workloads.' },
  { id: 3, level: 'info', title: 'New GPU Pool Joined', message: 'gpu-farm-east-12 (8× A100) registered and passing health checks.' },
  { id: 4, level: 'warning', title: 'Task Queue Growing', message: 'Queue depth increased 34% in the last 10 minutes.' },
  { id: 5, level: 'info', title: 'Firmware Update Available', message: 'NVIDIA driver 550.127 available for 12 nodes in rack-03.' },
];

const HEALTH_BARS = [
  { label: 'Compute', value: 94, color: 'bg-green-500' },
  { label: 'Storage', value: 87, color: 'bg-blue-500' },
  { label: 'Network', value: 99, color: 'bg-emerald-500' },
  { label: 'Memory', value: 72, color: 'bg-yellow-500' },
];

const SwarmDashboardPanel: React.FC = () => {
  const [utilData, setUtilData] = useState(generateUtilData);
  const [taskData, setTaskData] = useState(generateTaskData);
  const [alerts, setAlerts] = useState<Alert[]>(INITIAL_ALERTS);
  const [lastRefresh, setLastRefresh] = useState(new Date());
  const [refreshing, setRefreshing] = useState(false);

  const refresh = useCallback(() => {
    setRefreshing(true);
    setTimeout(() => {
      setUtilData(generateUtilData());
      setTaskData(generateTaskData());
      setLastRefresh(new Date());
      setRefreshing(false);
    }, 400);
  }, []);

  useEffect(() => {
    const interval = setInterval(refresh, 30000);
    return () => clearInterval(interval);
  }, [refresh]);

  const dismissAlert = (id: number) => setAlerts((a) => a.filter((x) => x.id !== id));

  const alertIcon = (level: string) => {
    if (level === 'critical') return <AlertCircle size={14} className="text-red-400" />;
    if (level === 'warning') return <AlertTriangle size={14} className="text-yellow-400" />;
    return <Info size={14} className="text-blue-400" />;
  };

  const alertBorder = (level: string) => {
    if (level === 'critical') return 'border-l-red-500';
    if (level === 'warning') return 'border-l-yellow-500';
    return 'border-l-blue-500';
  };

  return (
    <div className="h-full flex flex-col bg-[#0a0a0f] text-white overflow-auto">
      {/* Header */}
      <div className="flex items-center justify-between px-5 py-4 border-b border-[#1a1a1a]">
        <div className="flex items-center gap-3">
          <Cpu size={18} className="text-green-400" />
          <h1 className="text-lg font-bold">GPU Swarm Dashboard</h1>
        </div>
        <div className="flex items-center gap-3">
          <span className="text-[10px] text-gray-500">
            Last refresh: {lastRefresh.toLocaleTimeString()}
          </span>
          <span className="flex items-center gap-1 text-[10px] text-green-400">
            <span className="w-1.5 h-1.5 rounded-full bg-green-400 animate-pulse" /> Auto-refresh
          </span>
          <button
            onClick={refresh}
            className="p-2 rounded-lg hover:bg-[#111111] transition-colors text-gray-500 hover:text-white"
          >
            <RefreshCw size={14} className={refreshing ? 'animate-spin' : ''} />
          </button>
        </div>
      </div>

      <div className="flex-1 p-5 space-y-5 overflow-auto">
        {/* Stat cards */}
        <div className="grid grid-cols-4 gap-3">
          {[
            { icon: Cpu, label: 'Active GPUs', value: '1,247', change: '+3.2%', up: true },
            { icon: Activity, label: 'Total TFLOPS', value: '892.4', change: '+1.8%', up: true },
            { icon: Layers, label: 'Queue Depth', value: '34', change: '-12.5%', up: false },
            { icon: Timer, label: 'Avg Latency', value: '23ms', change: '-5.1%', up: false },
          ].map((s) => (
            <div key={s.label} className="bg-[#111111] rounded-xl p-4 border border-[#1a1a1a]">
              <div className="flex items-center gap-2 mb-2">
                <s.icon size={14} className="text-gray-500" />
                <span className="text-xs text-gray-500">{s.label}</span>
              </div>
              <div className="text-2xl font-bold">{s.value}</div>
              <div
                className={clsx(
                  'text-xs mt-1',
                  s.label === 'Queue Depth' || s.label === 'Avg Latency'
                    ? 'text-green-400'
                    : s.up
                      ? 'text-green-400'
                      : 'text-red-400',
                )}
              >
                {s.change}
              </div>
            </div>
          ))}
        </div>

        {/* Charts */}
        <div className="grid grid-cols-2 gap-4">
          {/* Utilization */}
          <div className="bg-[#111111] rounded-xl p-4 border border-[#1a1a1a]">
            <h3 className="text-sm font-semibold mb-3">GPU Utilization (%)</h3>
            <ResponsiveContainer width="100%" height={180}>
              <AreaChart data={utilData}>
                <defs>
                  <linearGradient id="utilGrad" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="0%" stopColor="#22c55e" stopOpacity={0.3} />
                    <stop offset="100%" stopColor="#22c55e" stopOpacity={0} />
                  </linearGradient>
                </defs>
                <CartesianGrid strokeDasharray="3 3" stroke="#1a1a1a" />
                <XAxis dataKey="time" tick={{ fontSize: 10, fill: '#666' }} />
                <YAxis domain={[0, 100]} tick={{ fontSize: 10, fill: '#666' }} />
                <Tooltip
                  contentStyle={{ background: '#111', border: '1px solid #1a1a1a', borderRadius: 8, fontSize: 12 }}
                />
                <Area type="monotone" dataKey="utilization" stroke="#22c55e" fill="url(#utilGrad)" strokeWidth={2} />
              </AreaChart>
            </ResponsiveContainer>
          </div>

          {/* Task completion */}
          <div className="bg-[#111111] rounded-xl p-4 border border-[#1a1a1a]">
            <h3 className="text-sm font-semibold mb-3">Task Completion</h3>
            <ResponsiveContainer width="100%" height={180}>
              <LineChart data={taskData}>
                <CartesianGrid strokeDasharray="3 3" stroke="#1a1a1a" />
                <XAxis dataKey="time" tick={{ fontSize: 10, fill: '#666' }} />
                <YAxis tick={{ fontSize: 10, fill: '#666' }} />
                <Tooltip
                  contentStyle={{ background: '#111', border: '1px solid #1a1a1a', borderRadius: 8, fontSize: 12 }}
                />
                <Line type="monotone" dataKey="completed" stroke="#22c55e" strokeWidth={2} dot={false} />
                <Line type="monotone" dataKey="failed" stroke="#ef4444" strokeWidth={2} dot={false} />
              </LineChart>
            </ResponsiveContainer>
          </div>
        </div>

        {/* Bottom row */}
        <div className="grid grid-cols-2 gap-4">
          {/* Alerts */}
          <div className="bg-[#111111] rounded-xl p-4 border border-[#1a1a1a]">
            <h3 className="text-sm font-semibold mb-3">Recent Alerts</h3>
            <div className="space-y-2 max-h-48 overflow-auto">
              {alerts.length === 0 && (
                <div className="text-xs text-gray-500 text-center py-4">No active alerts</div>
              )}
              {alerts.map((a) => (
                <div
                  key={a.id}
                  className={clsx(
                    'flex items-start gap-2 p-2.5 rounded-lg bg-[#0a0a0f] border-l-2',
                    alertBorder(a.level),
                  )}
                >
                  <div className="mt-0.5">{alertIcon(a.level)}</div>
                  <div className="flex-1 min-w-0">
                    <div className="text-xs font-medium text-white">{a.title}</div>
                    <div className="text-[10px] text-gray-500 mt-0.5 leading-relaxed">{a.message}</div>
                  </div>
                  <button
                    onClick={() => dismissAlert(a.id)}
                    className="text-gray-600 hover:text-white transition-colors mt-0.5"
                  >
                    <X size={12} />
                  </button>
                </div>
              ))}
            </div>
          </div>

          {/* Health Status */}
          <div className="bg-[#111111] rounded-xl p-4 border border-[#1a1a1a]">
            <h3 className="text-sm font-semibold mb-3">Health Status</h3>
            <div className="space-y-4">
              {HEALTH_BARS.map((h) => (
                <div key={h.label}>
                  <div className="flex justify-between text-xs mb-1.5">
                    <span className="text-gray-400">{h.label}</span>
                    <span className={clsx('font-medium', h.value >= 90 ? 'text-green-400' : h.value >= 80 ? 'text-blue-400' : 'text-yellow-400')}>
                      {h.value}%
                    </span>
                  </div>
                  <div className="h-2 rounded-full bg-[#0a0a0f] overflow-hidden">
                    <div
                      className={clsx('h-full rounded-full transition-all', h.color)}
                      style={{ width: `${h.value}%` }}
                    />
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default SwarmDashboardPanel;
