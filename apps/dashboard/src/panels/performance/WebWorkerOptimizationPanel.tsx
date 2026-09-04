import React, { useState } from 'react';
import { Zap, Cpu, Loader, TrendingUp, AlertCircle, CheckCircle } from 'lucide-react';

interface WorkerThread {
  workerId: number;
  assignedTask: string;
  cpuUtilization: number;
  memoryMb: number;
  tasksCompleted: number;
  status: 'active' | 'idle' | 'paused';
  lastActivity: number;
}

interface GpuCompositing {
  renderingBackend: string;
  enabled: boolean;
  framesPerSecond: number;
  compositeTime: number;
  gpuMemoryUsed: number;
  compositeTasksQueued: number;
  discardRate: number;
}

interface StartupPreload {
  moduleName: string;
  size: number;
  loadTime: number;
  priority: 'critical' | 'high' | 'normal';
  preloadStatus: 'loaded' | 'loading' | 'pending';
  hitRate: number;
}

interface PerformanceMetric {
  label: string;
  before: number;
  after: number;
  improvement: number;
}

export const WebWorkerOptimizationPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'workers' | 'compositing' | 'preload' | 'performance'>(
    'workers'
  );

  const [workerThreads] = useState<WorkerThread[]>([
    {
      workerId: 0,
      assignedTask: 'Image Processing Pipeline',
      cpuUtilization: 94.2,
      memoryMb: 245,
      tasksCompleted: 12450,
      status: 'active',
      lastActivity: Date.now() - 45,
    },
    {
      workerId: 1,
      assignedTask: 'Data Aggregation & Analytics',
      cpuUtilization: 87.5,
      memoryMb: 314,
      tasksCompleted: 8934,
      status: 'active',
      lastActivity: Date.now() - 120,
    },
    {
      workerId: 2,
      assignedTask: 'Crypto Operations (WASM)',
      cpuUtilization: 76.3,
      memoryMb: 428,
      tasksCompleted: 45230,
      status: 'active',
      lastActivity: Date.now() - 80,
    },
    {
      workerId: 3,
      assignedTask: 'JSON Parsing & Validation',
      cpuUtilization: 52.1,
      memoryMb: 128,
      tasksCompleted: 23450,
      status: 'idle',
      lastActivity: Date.now() - 2800,
    },
  ]);

  const [gpuCompositing] = useState<GpuCompositing>({
    renderingBackend: 'WebGL 2.0 + WGPU',
    enabled: true,
    framesPerSecond: 144,
    compositeTime: 6.8,
    gpuMemoryUsed: 512,
    compositeTasksQueued: 245,
    discardRate: 0.3,
  });

  const [startupPreloads] = useState<StartupPreload[]>([
    {
      moduleName: 'Core UI Framework',
      size: 2450,
      loadTime: 120,
      priority: 'critical',
      preloadStatus: 'loaded',
      hitRate: 98.5,
    },
    {
      moduleName: 'Crypto WASM Bundle',
      size: 8900,
      loadTime: 340,
      priority: 'critical',
      preloadStatus: 'loaded',
      hitRate: 96.2,
    },
    {
      moduleName: 'Database Layer',
      size: 1240,
      loadTime: 85,
      priority: 'high',
      preloadStatus: 'loaded',
      hitRate: 94.1,
    },
    {
      moduleName: 'Chart & Analytics',
      size: 1680,
      loadTime: 210,
      priority: 'high',
      preloadStatus: 'loading',
      hitRate: 87.3,
    },
    {
      moduleName: 'API Client Library',
      size: 450,
      loadTime: 45,
      priority: 'normal',
      preloadStatus: 'pending',
      hitRate: 0,
    },
  ]);

  const [performanceMetrics] = useState<PerformanceMetric[]>([
    {
      label: 'Initial Page Load',
      before: 2840,
      after: 840,
      improvement: 70.4,
    },
    {
      label: 'Time to Interactive',
      before: 3200,
      after: 1050,
      improvement: 67.2,
    },
    {
      label: 'Long Transaction Blocking',
      before: 240,
      after: 35,
      improvement: 85.4,
    },
    {
      label: 'Average Response Time',
      before: 450,
      after: 145,
      improvement: 67.8,
    },
    {
      label: 'GPU Composite Wait',
      before: 125,
      after: 6.8,
      improvement: 94.6,
    },
    {
      label: 'Memory Peak Usage',
      before: 2400,
      after: 1680,
      improvement: 30.0,
    },
  ]);

  const avgWorkerUtilization =
    (
      workerThreads.reduce((sum, w) => sum + w.cpuUtilization, 0) / workerThreads.length
    ).toFixed(1);
  const totalTasksCompleted = workerThreads.reduce((sum, w) => sum + w.tasksCompleted, 0);
  const totalWorkerMemory = workerThreads.reduce((sum, w) => sum + w.memoryMb, 0);
  const activeWorkers = workerThreads.filter((w) => w.status === 'active').length;

  return (
    <div className="min-h-screen bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f] p-6">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between mb-8">
          <div>
            <h1 className="text-4xl font-bold text-transparent bg-clip-text bg-gradient-to-r from-cyan-400 to-blue-500 mb-2">
              Web Workers & GPU Compositing
            </h1>
            <p className="text-gray-400">Worker Threads • WebGL 2.0 • WGPU • Startup Preload</p>
          </div>
          <Zap className="w-12 h-12 text-cyan-400" />
        </div>

        {/* KPI Grid */}
        <div className="grid grid-cols-4 gap-4 mb-8">
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Active Workers</div>
            <div className="text-2xl font-bold text-cyan-400">
              {activeWorkers}/{workerThreads.length}
            </div>
            <div className="text-xs text-gray-500 mt-2">CPU threads pooled</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Avg Utilization</div>
            <div className="text-2xl font-bold text-blue-400">{avgWorkerUtilization}%</div>
            <div className="text-xs text-gray-500 mt-2">Worker thread load</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">GPU Framerate</div>
            <div className="text-2xl font-bold text-purple-400">{gpuCompositing.framesPerSecond} FPS</div>
            <div className="text-xs text-gray-500 mt-2">Composite operations</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Page Load Time</div>
            <div className="text-2xl font-bold text-teal-400">840ms</div>
            <div className="text-xs text-gray-500 mt-2">-70.4% post-optimization</div>
          </div>
        </div>

        {/* Tabs */}
        <div className="flex gap-2 mb-6 border-b border-[#2a2a35]">
          {(['workers', 'compositing', 'preload', 'performance'] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={`px-4 py-3 font-semibold transition-colors ${
                activeTab === tab
                  ? 'text-cyan-400 border-b-2 border-cyan-400'
                  : 'text-gray-400 hover:text-gray-300'
              }`}
            >
              {tab === 'workers' && 'Worker Threads'}
              {tab === 'compositing' && 'GPU Compositing'}
              {tab === 'preload' && 'Startup Preload'}
              {tab === 'performance' && 'Performance Gains'}
            </button>
          ))}
        </div>

        {/* Content */}
        <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
          {activeTab === 'workers' && (
            <div className="space-y-4">
              <h3 className="text-lg font-semibold text-white mb-4">Worker Thread Pool</h3>
              <div className="grid grid-cols-2 gap-4 mb-6">
                <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                  <div className="text-gray-400 text-sm mb-1">Total Tasks Completed</div>
                  <div className="text-3xl font-bold text-cyan-400">{totalTasksCompleted.toLocaleString()}</div>
                </div>
                <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                  <div className="text-gray-400 text-sm mb-1">Worker Memory Pool</div>
                  <div className="text-3xl font-bold text-blue-400">{totalWorkerMemory} MB</div>
                </div>
              </div>
              <div className="space-y-3">
                {workerThreads.map((worker) => (
                  <div key={worker.workerId} className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                    <div className="flex items-start justify-between mb-3">
                      <div>
                        <h4 className="text-white font-semibold">Worker #{worker.workerId}</h4>
                        <p className="text-sm text-gray-400">{worker.assignedTask}</p>
                      </div>
                      <div
                        className={`px-3 py-1 rounded-full text-xs font-semibold ${
                          worker.status === 'active'
                            ? 'bg-green-500/20 text-green-400'
                            : worker.status === 'idle'
                            ? 'bg-gray-500/20 text-gray-400'
                            : 'bg-yellow-500/20 text-yellow-400'
                        }`}
                      >
                        {worker.status.toUpperCase()}
                      </div>
                    </div>
                    <div className="grid grid-cols-5 gap-3 text-sm">
                      <div>
                        <div className="text-gray-400 text-xs">CPU Utilization</div>
                        <div className="font-semibold text-white">{worker.cpuUtilization}%</div>
                        <div className="w-full bg-[#2a2a35] rounded-full h-1 mt-1">
                          <div
                            className="bg-cyan-500 h-1 rounded-full"
                            style={{ width: `${worker.cpuUtilization}%` }}
                          />
                        </div>
                      </div>
                      <div>
                        <div className="text-gray-400 text-xs">Memory</div>
                        <div className="text-white font-semibold">{worker.memoryMb} MB</div>
                      </div>
                      <div>
                        <div className="text-gray-400 text-xs">Tasks Completed</div>
                        <div className="text-white font-semibold">{worker.tasksCompleted.toLocaleString()}</div>
                      </div>
                      <div>
                        <div className="text-gray-400 text-xs">Last Activity</div>
                        <div className="text-white font-semibold">{worker.lastActivity}ms</div>
                      </div>
                      <div>
                        {worker.status === 'active' ? (
                          <CheckCircle className="w-5 h-5 text-green-400" />
                        ) : (
                          <AlertCircle className="w-5 h-5 text-gray-500" />
                        )}
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {activeTab === 'compositing' && (
            <div className="space-y-4">
              <h3 className="text-lg font-semibold text-white mb-6">GPU Compositing Engine</h3>
              <div className="grid grid-cols-2 gap-4 mb-6">
                <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                  <div className="text-gray-400 mb-2">Rendering Backend</div>
                  <div className="text-white font-semibold">{gpuCompositing.renderingBackend}</div>
                </div>
                <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                  <div className="text-gray-400 mb-2">Compositor Status</div>
                  <div className="flex items-center gap-2">
                    <div className="w-2 h-2 rounded-full bg-green-400"></div>
                    <span className="text-green-400 font-semibold">
                      {gpuCompositing.enabled ? 'Enabled' : 'Disabled'}
                    </span>
                  </div>
                </div>
              </div>

              <div className="space-y-4">
                <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                  <div className="flex items-baseline justify-between mb-3">
                    <h4 className="text-white font-semibold">Framerate</h4>
                    <div className="text-2xl font-bold text-cyan-400">{gpuCompositing.framesPerSecond} FPS</div>
                  </div>
                  <div className="w-full bg-[#2a2a35] rounded-full h-2">
                    <div
                      className="bg-gradient-to-r from-cyan-500 to-blue-500 h-2 rounded-full"
                      style={{ width: '95%' }}
                    />
                  </div>
                  <p className="text-gray-400 text-sm mt-2">Composite frame timing optimized for 144Hz display</p>
                </div>

                <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                  <div className="flex items-baseline justify-between mb-2">
                    <div>
                      <h4 className="text-white font-semibold">Composite Time Per Frame</h4>
                      <p className="text-gray-400 text-sm">Time to blend layers & render</p>
                    </div>
                    <div className="text-2xl font-bold text-blue-400">{gpuCompositing.compositeTime} ms</div>
                  </div>
                </div>

                <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                  <div className="flex items-baseline justify-between mb-2">
                    <div>
                      <h4 className="text-white font-semibold">GPU Memory Used</h4>
                      <p className="text-gray-400 text-sm">Texture cache + composition buffers</p>
                    </div>
                    <div className="text-2xl font-bold text-purple-400">{gpuCompositing.gpuMemoryUsed} MB</div>
                  </div>
                </div>

                <div className="grid grid-cols-2 gap-4">
                  <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                    <div className="text-gray-400 text-sm mb-1">Tasks Queued</div>
                    <div className="text-2xl font-bold text-teal-400">
                      {gpuCompositing.compositeTasksQueued.toLocaleString()}
                    </div>
                  </div>
                  <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                    <div className="text-gray-400 text-sm mb-1">Discard Rate</div>
                    <div className={`text-2xl font-bold ${
                      gpuCompositing.discardRate < 1 ? 'text-green-400' : 'text-yellow-400'
                    }`}>
                      {gpuCompositing.discardRate}%
                    </div>
                  </div>
                </div>
              </div>
            </div>
          )}

          {activeTab === 'preload' && (
            <div>
              <h3 className="text-lg font-semibold text-white mb-6">Startup Preload Modules</h3>
              <div className="space-y-3">
                {startupPreloads.map((preload) => (
                  <div key={preload.moduleName} className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                    <div className="flex items-start justify-between mb-3">
                      <div>
                        <h4 className="text-white font-semibold">{preload.moduleName}</h4>
                        <p className="text-sm text-gray-400">{preload.size} KB • Load time: {preload.loadTime}ms</p>
                      </div>
                      <div className="flex items-center gap-2">
                        <span
                          className={`px-2 py-1 rounded text-xs font-semibold ${
                            preload.priority === 'critical'
                              ? 'bg-red-500/20 text-red-400'
                              : preload.priority === 'high'
                              ? 'bg-orange-500/20 text-orange-400'
                              : 'bg-blue-500/20 text-blue-400'
                          }`}
                        >
                          {preload.priority.toUpperCase()}
                        </span>
                        <span
                          className={`px-2 py-1 rounded text-xs font-semibold ${
                            preload.preloadStatus === 'loaded'
                              ? 'bg-green-500/20 text-green-400'
                              : preload.preloadStatus === 'loading'
                              ? 'bg-yellow-500/20 text-yellow-400'
                              : 'bg-gray-500/20 text-gray-400'
                          }`}
                        >
                          {preload.preloadStatus.toUpperCase()}
                        </span>
                      </div>
                    </div>
                    {preload.hitRate > 0 && (
                      <div>
                        <div className="text-gray-400 text-xs mb-1">Cache Hit Rate: {preload.hitRate}%</div>
                        <div className="w-full bg-[#2a2a35] rounded-full h-2">
                          <div
                            className="bg-green-500 h-2 rounded-full"
                            style={{ width: `${preload.hitRate}%` }}
                          />
                        </div>
                      </div>
                    )}
                  </div>
                ))}
              </div>
            </div>
          )}

          {activeTab === 'performance' && (
            <div>
              <h3 className="text-lg font-semibold text-white mb-6">Performance Improvements</h3>
              <div className="space-y-4">
                {performanceMetrics.map((metric) => (
                  <div key={metric.label} className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                    <div className="flex items-baseline justify-between mb-3">
                      <h4 className="text-white font-semibold">{metric.label}</h4>
                      <div className="text-right">
                        <div className="text-green-400 font-bold text-lg">+{metric.improvement.toFixed(1)}%</div>
                      </div>
                    </div>
                    <div className="flex items-end gap-4">
                      <div className="flex-1">
                        <div className="text-gray-400 text-xs mb-1">Before: {metric.before}ms</div>
                        <div className="w-full bg-[#2a2a35] rounded-full h-3">
                          <div
                            className="bg-red-500 h-3 rounded-full"
                            style={{ width: '100%' }}
                          />
                        </div>
                      </div>
                      <div className="flex-1">
                        <div className="text-gray-400 text-xs mb-1">After: {metric.after}ms</div>
                        <div className="w-full bg-[#2a2a35] rounded-full h-3">
                          <div
                            className="bg-green-500 h-3 rounded-full"
                            style={{
                              width: `${(metric.after / metric.before) * 100}%`,
                            }}
                          />
                        </div>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export default WebWorkerOptimizationPanel;
