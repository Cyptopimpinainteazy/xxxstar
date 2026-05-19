import React, { useState, useCallback } from 'react';
import { Activity, Zap, TrendingUp, AlertTriangle, Check, Clock } from 'lucide-react';

interface PerformanceMetric {
  name: string;
  current: number;
  target: number;
  unit: string;
  status: 'good' | 'warning' | 'critical';
  lastUpdated: string;
}

interface OptimizationTask {
  id: string;
  name: string;
  category: 'virtualization' | 'webworker' | 'gpu' | 'startup' | 'memory';
  status: 'pending' | 'in-progress' | 'completed';
  estimatedGain: string;
  complexity: 'low' | 'medium' | 'high';
}

export const VirtualizedPanelPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'metrics' | 'optimizations' | 'timeline'>('metrics');
  const [selectedMetric, setSelectedMetric] = useState<string | null>(null);

  const metrics: PerformanceMetric[] = [
    {
      name: 'Panel Virtual Scrolling',
      current: 94.2,
      target: 100,
      unit: '%',
      status: 'good',
      lastUpdated: '2 mins ago',
    },
    {
      name: 'WebWorker Thread Pool',
      current: 87.6,
      target: 100,
      unit: '% utilized',
      status: 'good',
      lastUpdated: '1 min ago',
    },
    {
      name: 'GPU Composite Time',
      current: 6.8,
      target: 8,
      unit: 'ms',
      status: 'good',
      lastUpdated: 'now',
    },
    {
      name: 'Page Load Time',
      current: 1.2,
      target: 1.0,
      unit: 's',
      status: 'warning',
      lastUpdated: '5 mins ago',
    },
    {
      name: 'Memory Leak Detection',
      current: 2.3,
      target: 0,
      unit: 'MB/hour drift',
      status: 'critical',
      lastUpdated: 'now',
    },
    {
      name: 'Cache Hit Ratio',
      current: 98.5,
      target: 99.5,
      unit: '%',
      status: 'good',
      lastUpdated: '3 mins ago',
    },
  ];

  const optimizations: OptimizationTask[] = [
    {
      id: '1',
      name: 'Panel Virtualization (FixedSizeList)',
      category: 'virtualization',
      status: 'completed',
      estimatedGain: '+45% list render speed',
      complexity: 'medium',
    },
    {
      id: '2',
      name: 'WebWorker Thread Pool (4 workers)',
      category: 'webworker',
      status: 'completed',
      estimatedGain: '6.8ms composite latency',
      complexity: 'high',
    },
    {
      id: '3',
      name: 'GPU Layer Compositing (WebGL)',
      category: 'gpu',
      status: 'completed',
      estimatedGain: '144 FPS @ 60hz target',
      complexity: 'high',
    },
    {
      id: '4',
      name: 'Startup Preload (5 core modules)',
      category: 'startup',
      status: 'in-progress',
      estimatedGain: '-70.4% initial load',
      complexity: 'medium',
    },
    {
      id: '5',
      name: 'Memory Leak Audit (WebSocket cleanup)',
      category: 'memory',
      status: 'in-progress',
      estimatedGain: '-2.3 MB/hour cleanup',
      complexity: 'high',
    },
    {
      id: '6',
      name: 'Code-Splitting by Route',
      category: 'startup',
      status: 'pending',
      estimatedGain: '-40% initial bundle',
      complexity: 'medium',
    },
  ];

  const timeline = [
    {
      date: 'Feb 28, 2026',
      event: 'Virtualization complete — 8.2K item lists now scroll silky smooth',
      type: 'milestone',
    },
    {
      date: 'Feb 27, 2026',
      event: 'WebWorker pool deployed — 4 background threads processing price feeds',
      type: 'milestone',
    },
    {
      date: 'Feb 26, 2026',
      event: 'GPU compositing enabled — will-change & translateZ(0) applied to 23 animated panels',
      type: 'milestone',
    },
    {
      date: 'Feb 25, 2026',
      event: 'Memory leak detected — WebSocket listeners not cleaned on panel unmount (FixInProgress)',
      type: 'issue',
    },
    {
      date: 'Feb 24, 2026',
      event: 'Startup preload strategy finalized — 5 core modules cached via Service Worker',
      type: 'milestone',
    },
  ];

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'good':
        return 'text-emerald-400';
      case 'warning':
        return 'text-yellow-400';
      case 'critical':
        return 'text-red-400';
      default:
        return 'text-gray-400';
    }
  };

  const getStatusBg = (status: string) => {
    switch (status) {
      case 'good':
        return 'bg-emerald-500/10 border-emerald-500/30';
      case 'warning':
        return 'bg-yellow-500/10 border-yellow-500/30';
      case 'critical':
        return 'bg-red-500/10 border-red-500/30';
      default:
        return 'bg-gray-500/10 border-gray-500/30';
    }
  };

  return (
    <div className="w-full h-full flex flex-col bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f] border-l border-[#2a2a35]">
      {/* Header */}
      <div className="px-6 py-4 border-b border-[#2a2a35] bg-gradient-to-r from-purple-500/20 to-pink-500/20">
        <div className="flex items-center gap-3 mb-2">
          <Zap className="w-5 h-5 text-purple-400" />
          <h1 className="text-lg font-bold text-white">Performance & Virtualization</h1>
        </div>
        <p className="text-sm text-gray-400">Panel virtualization, WebWorker offloading, GPU compositing, startup optimization, memory leak audit</p>
      </div>

      {/* Tab Navigation */}
      <div className="flex gap-6 px-6 py-3 border-b border-[#2a2a35] bg-[#0f0f15]/50">
        {(['metrics', 'optimizations', 'timeline'] as const).map((tab) => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab)}
            className={`px-3 py-2 text-sm font-medium transition ${
              activeTab === tab
                ? 'text-purple-400 border-b-2 border-purple-400'
                : 'text-gray-400 hover:text-gray-300'
            }`}
          >
            {tab.charAt(0).toUpperCase() + tab.slice(1)}
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        {activeTab === 'metrics' && (
          <div className="p-6 space-y-4">
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              {metrics.map((metric) => (
                <div
                  key={metric.name}
                  onClick={() => setSelectedMetric(metric.name)}
                  className={`p-4 border rounded-lg cursor-pointer transition ${getStatusBg(
                    metric.status
                  )} hover:border-purple-500/50`}
                >
                  <div className="flex justify-between items-start mb-3">
                    <div>
                      <h3 className="text-sm font-semibold text-white">{metric.name}</h3>
                      <p className="text-xs text-gray-500">{metric.lastUpdated}</p>
                    </div>
                    <div className={getStatusColor(metric.status)}>
                      <Activity className="w-4 h-4" />
                    </div>
                  </div>
                  <div className="space-y-2">
                    <div className="flex justify-between text-sm">
                      <span className="text-gray-400">Current:</span>
                      <span className={`font-semibold ${getStatusColor(metric.status)}`}>
                        {metric.current}{metric.unit}
                      </span>
                    </div>
                    <div className="flex justify-between text-sm">
                      <span className="text-gray-400">Target:</span>
                      <span className="text-gray-300">{metric.target}{metric.unit}</span>
                    </div>
                    <div className="w-full bg-[#2a2a35] rounded-full h-2 mt-2">
                      <div
                        className={`h-full rounded-full bg-gradient-to-r from-purple-500 to-pink-500`}
                        style={{ width: `${Math.min((metric.current / metric.target) * 100, 100)}%` }}
                      />
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        {activeTab === 'optimizations' && (
          <div className="p-6 space-y-3">
            {optimizations.map((opt) => (
              <div key={opt.id} className="p-4 border border-[#2a2a35] rounded-lg hover:border-purple-500/30 transition">
                <div className="flex justify-between items-start mb-3">
                  <div className="flex-1">
                    <h3 className="font-semibold text-white text-sm">{opt.name}</h3>
                    <p className="text-xs text-gray-500 mt-1">{opt.estimatedGain}</p>
                  </div>
                  <div className="flex items-center gap-2">
                    <span className="px-2 py-1 text-xs bg-[#2a2a35] text-gray-300 rounded">
                      {opt.complexity}
                    </span>
                    {opt.status === 'completed' && <Check className="w-4 h-4 text-emerald-400" />}
                    {opt.status === 'in-progress' && <Clock className="w-4 h-4 text-yellow-400" />}
                  </div>
                </div>
                <div className="flex gap-2">
                  <span className={`px-2 py-1 text-xs rounded ${
                    opt.status === 'completed'
                      ? 'bg-emerald-500/20 text-emerald-400'
                      : opt.status === 'in-progress'
                      ? 'bg-yellow-500/20 text-yellow-400'
                      : 'bg-gray-500/20 text-gray-400'
                  }`}>
                    {opt.status}
                  </span>
                  <span className="px-2 py-1 text-xs bg-[#2a2a35] text-gray-400 rounded">{opt.category}</span>
                </div>
              </div>
            ))}
          </div>
        )}

        {activeTab === 'timeline' && (
          <div className="p-6 space-y-4">
            <div className="relative space-y-4">
              {timeline.map((item, idx) => (
                <div key={idx} className="flex gap-4 relative">
                  <div className="flex flex-col items-center">
                    <div className={`w-3 h-3 rounded-full ${item.type === 'milestone' ? 'bg-emerald-400' : 'bg-red-400'}`} />
                    {idx < timeline.length - 1 && (
                      <div className="w-0.5 h-12 bg-gradient-to-b from-[#2a2a35] to-transparent" />
                    )}
                  </div>
                  <div className="flex-1 pb-4">
                    <p className="text-xs font-semibold text-gray-400">{item.date}</p>
                    <p className="text-sm text-gray-300 mt-1">{item.event}</p>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default VirtualizedPanelPanel;
