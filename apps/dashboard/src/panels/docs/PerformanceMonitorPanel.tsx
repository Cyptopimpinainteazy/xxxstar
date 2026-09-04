import React, { useState, useEffect } from 'react';
import { BarChart3, TrendingUp, Clock, Zap } from 'lucide-react';

interface PerformanceMetric {
  timestamp: string;
  cpuUsage: number;
  memoryUsage: number;
  diskUsage: number;
  networkLatency: number;
  throughput: number;
}

export const PerformanceMonitorPanel: React.FC = () => {
  const [metrics, setMetrics] = useState<PerformanceMetric[]>(
    Array.from({ length: 24 }, (_, i) => ({
      timestamp: `${i}:00`,
      cpuUsage: Math.random() * 100,
      memoryUsage: Math.random() * 100,
      diskUsage: Math.random() * 100,
      networkLatency: Math.random() * 200,
      throughput: Math.random() * 1000,
    }))
  );

  const [selectedMetric, setSelectedMetric] = useState<'cpu' | 'memory' | 'disk' | 'latency' | 'throughput'>('cpu');

  useEffect(() => {
    const interval = setInterval(() => {
      setMetrics((prev) => [
        ...prev.slice(1),
        {
          timestamp: new Date().getHours() + ':00',
          cpuUsage: Math.random() * 100,
          memoryUsage: Math.random() * 100,
          diskUsage: Math.random() * 100,
          networkLatency: Math.random() * 200,
          throughput: Math.random() * 1000,
        },
      ]);
    }, 5000);
    return () => clearInterval(interval);
  }, []);

  const getMetricData = () => {
    switch (selectedMetric) {
      case 'cpu':
        return metrics.map((m) => m.cpuUsage);
      case 'memory':
        return metrics.map((m) => m.memoryUsage);
      case 'disk':
        return metrics.map((m) => m.diskUsage);
      case 'latency':
        return metrics.map((m) => m.networkLatency);
      case 'throughput':
        return metrics.map((m) => m.throughput);
    }
  };

  const getMetricLabel = () => {
    switch (selectedMetric) {
      case 'cpu':
        return 'CPU Usage (%)';
      case 'memory':
        return 'Memory Usage (%)';
      case 'disk':
        return 'Disk Usage (%)';
      case 'latency':
        return 'Network Latency (ms)';
      case 'throughput':
        return 'Throughput (Mbps)';
    }
  };

  const getCurrentValue = () => {
    const lastMetric = metrics[metrics.length - 1];
    switch (selectedMetric) {
      case 'cpu':
        return lastMetric.cpuUsage.toFixed(1);
      case 'memory':
        return lastMetric.memoryUsage.toFixed(1);
      case 'disk':
        return lastMetric.diskUsage.toFixed(1);
      case 'latency':
        return lastMetric.networkLatency.toFixed(1);
      case 'throughput':
        return lastMetric.throughput.toFixed(1);
    }
  };

  const getUnit = () => {
    switch (selectedMetric) {
      case 'cpu':
      case 'memory':
      case 'disk':
        return '%';
      case 'latency':
        return 'ms';
      case 'throughput':
        return 'Mbps';
    }
  };

  const data = getMetricData();
  const maxValue = Math.max(...data);
  const minValue = Math.min(...data);
  const avgValue = (data.reduce((a, b) => a + b, 0) / data.length).toFixed(1);

  return (
    <div className="min-h-screen bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f] p-6">
      <div className="max-w-6xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between mb-8">
          <div>
            <h1 className="text-4xl font-bold text-transparent bg-clip-text bg-gradient-to-r from-cyan-400 to-blue-500 mb-2">
              Performance Monitor
            </h1>
            <p className="text-gray-400">Real-time system resource utilization and metrics</p>
          </div>
          <BarChart3 className="w-12 h-12 text-cyan-400" />
        </div>

        {/* Overview Cards */}
        <div className="grid grid-cols-5 gap-3 mb-8">
          {[
            { label: 'CPU', value: metrics[metrics.length - 1].cpuUsage.toFixed(1), unit: '%', color: 'text-cyan-400' },
            { label: 'Memory', value: metrics[metrics.length - 1].memoryUsage.toFixed(1), unit: '%', color: 'text-blue-400' },
            { label: 'Disk', value: metrics[metrics.length - 1].diskUsage.toFixed(1), unit: '%', color: 'text-teal-400' },
            { label: 'Latency', value: metrics[metrics.length - 1].networkLatency.toFixed(1), unit: 'ms', color: 'text-purple-400' },
            { label: 'Throughput', value: metrics[metrics.length - 1].throughput.toFixed(1), unit: 'Mbps', color: 'text-pink-400' },
          ].map((metric, idx) => (
            <div key={idx} className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-3">
              <div className="text-gray-400 text-xs mb-1">{metric.label}</div>
              <div className={`text-2xl font-bold ${metric.color}`}>
                {metric.value} {metric.unit}
              </div>
            </div>
          ))}
        </div>

        {/* Main Chart Area */}
        <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6 mb-6">
          <div className="flex items-center justify-between mb-6">
            <h2 className="text-xl font-bold text-white">{getMetricLabel()}</h2>
            <div className="text-right">
              <div className="text-3xl font-bold text-cyan-400">
                {getCurrentValue()} {getUnit()}
              </div>
              <p className="text-gray-400 text-sm">Current value</p>
            </div>
          </div>

          {/* Chart Placeholder */}
          <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg h-64 p-4 mb-4">
            <svg className="w-full h-full" viewBox="0 0 800 250">
              {/* Grid lines */}
              {Array.from({ length: 5 }).map((_, i) => (
                <line
                  key={`h-${i}`}
                  x1="0"
                  y1={(i * 250) / 5}
                  x2="800"
                  y2={(i * 250) / 5}
                  stroke="#2a2a35"
                  strokeWidth="1"
                />
              ))}

              {/* Data line chart */}
              <polyline
                points={data
                  .map((value, i) => {
                    const x = (i / (data.length - 1)) * 800;
                    const y = ((1 - value / Math.max(...data)) * 240) + 5;
                    return `${x},${y}`;
                  })
                  .join(' ')}
                fill="none"
                stroke="#06b6d4"
                strokeWidth="2"
              />

              {/* Data points */}
              {data.map((value, i) => {
                const x = (i / (data.length - 1)) * 800;
                const y = ((1 - value / Math.max(...data)) * 240) + 5;
                return (
                  <circle
                    key={`point-${i}`}
                    cx={x}
                    cy={y}
                    r="3"
                    fill="#0a0a0f"
                    stroke="#06b6d4"
                    strokeWidth="2"
                  />
                );
              })}

              {/* Axes */}
              <line x1="0" y1="245" x2="800" y2="245" stroke="#2a2a35" strokeWidth="1" />
              <line x1="0" y1="0" x2="0" y2="250" stroke="#2a2a35" strokeWidth="1" />
            </svg>
          </div>

          {/* Stats */}
          <div className="grid grid-cols-3 gap-4 text-sm">
            <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-3">
              <div className="text-gray-400 mb-1">Average</div>
              <div className="text-blue-400 font-bold">{avgValue} {getUnit()}</div>
            </div>
            <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-3">
              <div className="text-gray-400 mb-1">Max</div>
              <div className="text-red-400 font-bold">{maxValue.toFixed(1)} {getUnit()}</div>
            </div>
            <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-3">
              <div className="text-gray-400 mb-1">Min</div>
              <div className="text-green-400 font-bold">{minValue.toFixed(1)} {getUnit()}</div>
            </div>
          </div>
        </div>

        {/* Metric Selector */}
        <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
          <h2 className="text-white font-bold mb-4">Select Metric</h2>
          <div className="grid grid-cols-5 gap-2">
            {([
              { id: 'cpu', label: 'CPU', icon: Zap },
              { id: 'memory', label: 'Memory', icon: BarChart3 },
              { id: 'disk', label: 'Disk', icon: Clock },
              { id: 'latency', label: 'Latency', icon: TrendingUp },
              { id: 'throughput', label: 'Throughput', icon: Zap },
            ] as const).map((metric) => (
              <button
                key={metric.id}
                onClick={() => setSelectedMetric(metric.id)}
                className={`px-4 py-2 rounded-lg font-semibold transition ${
                  selectedMetric === metric.id
                    ? 'bg-cyan-600 text-white'
                    : 'bg-[#0a0a0f] border border-[#2a2a35] text-gray-400 hover:border-cyan-400'
                }`}
              >
                {metric.label}
              </button>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
};

export default PerformanceMonitorPanel;
