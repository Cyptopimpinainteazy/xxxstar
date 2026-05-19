import React, { useState } from 'react';
import { Cpu, Zap, TrendingUp, Activity, AlertCircle, CheckCircle } from 'lucide-react';

interface GpuDevice {
  id: string;
  name: string;
  memory: number;
  memoryUsed: number;
  status: 'active' | 'idle' | 'offline';
  kernelVersion: string;
  temp: number;
  power: number;
  benchmark: number;
}

interface MemoryPool {
  poolId: string;
  devices: string[];
  totalMemory: number;
  usedMemory: number;
  allocations: { taskId: string; size: number }[];
  fragmentation: number;
}

interface FallbackChain {
  id: string;
  stage: number;
  type: 'CUDA' | 'OpenCL' | 'CPU';
  status: 'active' | 'standby' | 'disabled';
  executionTime: number;
  reliability: number;
}

export const ChainCoreOptimizationPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'devices' | 'memory' | 'fallback' | 'benchmarks'>('devices');
  const [gpuDevices] = useState<GpuDevice[]>([
    {
      id: 'gpu-0',
      name: 'NVIDIA A100 (Primary)',
      memory: 40960,
      memoryUsed: 28672,
      status: 'active',
      kernelVersion: '12.2',
      temp: 58,
      power: 250,
      benchmark: 9850,
    },
    {
      id: 'gpu-1',
      name: 'NVIDIA RTX 4090',
      memory: 24576,
      memoryUsed: 15360,
      status: 'active',
      kernelVersion: '12.2',
      temp: 45,
      power: 180,
      benchmark: 7200,
    },
    {
      id: 'gpu-2',
      name: 'NVIDIA RTX 3090',
      memory: 24576,
      memoryUsed: 8192,
      status: 'idle',
      kernelVersion: '12.1',
      temp: 32,
      power: 45,
      benchmark: 5400,
    },
  ]);

  const [memoryPools] = useState<MemoryPool[]>([
    {
      poolId: 'pool-unified',
      devices: ['gpu-0', 'gpu-1', 'gpu-2'],
      totalMemory: 89088,
      usedMemory: 52224,
      allocations: [
        { taskId: 'tx-verification', size: 18432 },
        { taskId: 'state-proof', size: 12288 },
        { taskId: 'consensus-engine', size: 21504 },
      ],
      fragmentation: 12.4,
    },
    {
      poolId: 'pool-inference',
      devices: ['gpu-0', 'gpu-1'],
      totalMemory: 65536,
      usedMemory: 44032,
      allocations: [
        { taskId: 'ml-validator', size: 28672 },
        { taskId: 'anomaly-detection', size: 15360 },
      ],
      fragmentation: 8.2,
    },
  ]);

  const [fallbackChain] = useState<FallbackChain[]>([
    {
      id: 'stage-1',
      stage: 1,
      type: 'CUDA',
      status: 'active',
      executionTime: 42,
      reliability: 99.7,
    },
    {
      id: 'stage-2',
      stage: 2,
      type: 'OpenCL',
      status: 'standby',
      executionTime: 156,
      reliability: 98.2,
    },
    {
      id: 'stage-3',
      stage: 3,
      type: 'CPU',
      status: 'standby',
      executionTime: 890,
      reliability: 100.0,
    },
  ]);

  const [benchmarks] = useState([
    { date: 'Mar 1', throughput: 8420, latency: 42, efficiency: 94.2 },
    { date: 'Mar 2', throughput: 8650, latency: 39, efficiency: 95.1 },
    { date: 'Mar 3', throughput: 9100, latency: 36, efficiency: 96.3 },
    { date: 'Mar 4', throughput: 9850, latency: 32, efficiency: 97.8 },
  ]);

  const totalGpuMemory = gpuDevices.reduce((sum, gpu) => sum + gpu.memory, 0);
  const totalMemoryUsed = gpuDevices.reduce((sum, gpu) => sum + gpu.memoryUsed, 0);
  const memoryUtilization = (totalMemoryUsed / totalGpuMemory) * 100;
  const activeDevices = gpuDevices.filter((g) => g.status === 'active').length;

  return (
    <div className="min-h-screen bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f] p-6">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between mb-8">
          <div>
            <h1 className="text-4xl font-bold text-transparent bg-clip-text bg-gradient-to-r from-cyan-400 to-blue-500 mb-2">
              Chain Core Optimization
            </h1>
            <p className="text-gray-400">GPU Pooling • Memory Management • Fallback Chains</p>
          </div>
          <Cpu className="w-12 h-12 text-cyan-400" />
        </div>

        {/* KPI Grid */}
        <div className="grid grid-cols-4 gap-4 mb-8">
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">GPU Memory</div>
            <div className="text-2xl font-bold text-cyan-400">{(totalGpuMemory / 1024).toFixed(1)}GB</div>
            <div className="text-xs text-gray-500 mt-2">Used: {memoryUtilization.toFixed(1)}%</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Active Devices</div>
            <div className="text-2xl font-bold text-green-400">{activeDevices}/3</div>
            <div className="text-xs text-gray-500 mt-2">All operational</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Memory Fragmentation</div>
            <div className="text-2xl font-bold text-yellow-400">10.3%</div>
            <div className="text-xs text-gray-500 mt-2">Optimal range detected</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Throughput</div>
            <div className="text-2xl font-bold text-blue-400">9.85K tx/s</div>
            <div className="text-xs text-gray-500 mt-2">+16% vs baseline</div>
          </div>
        </div>

        {/* Tabs */}
        <div className="flex gap-2 mb-6 border-b border-[#2a2a35]">
          {(['devices', 'memory', 'fallback', 'benchmarks'] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={`px-4 py-3 font-semibold transition-colors ${
                activeTab === tab
                  ? 'text-cyan-400 border-b-2 border-cyan-400'
                  : 'text-gray-400 hover:text-gray-300'
              }`}
            >
              {tab === 'devices' && 'GPU Devices'}
              {tab === 'memory' && 'Memory Pools'}
              {tab === 'fallback' && 'Fallback Chain'}
              {tab === 'benchmarks' && 'Benchmarks'}
            </button>
          ))}
        </div>

        {/* Content */}
        <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
          {activeTab === 'devices' && (
            <div className="space-y-4">
              <h3 className="text-lg font-semibold text-white mb-4">GPU Devices</h3>
              {gpuDevices.map((gpu) => (
                <div key={gpu.id} className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                  <div className="flex items-start justify-between mb-4">
                    <div>
                      <h4 className="text-white font-semibold">{gpu.name}</h4>
                      <p className="text-sm text-gray-400">Kernel: {gpu.kernelVersion}</p>
                    </div>
                    <div
                      className={`px-3 py-1 rounded-full text-xs font-semibold ${
                        gpu.status === 'active'
                          ? 'bg-green-500/20 text-green-400'
                          : gpu.status === 'idle'
                            ? 'bg-yellow-500/20 text-yellow-400'
                            : 'bg-red-500/20 text-red-400'
                      }`}
                    >
                      {gpu.status.toUpperCase()}
                    </div>
                  </div>
                  <div className="grid grid-cols-5 gap-4 text-sm">
                    <div>
                      <div className="text-gray-400">Memory</div>
                      <div className="text-white font-semibold">
                        {(gpu.memoryUsed / 1024).toFixed(1)}/{(gpu.memory / 1024).toFixed(1)}GB
                      </div>
                    </div>
                    <div>
                      <div className="text-gray-400">Temperature</div>
                      <div className="text-white font-semibold">{gpu.temp}°C</div>
                    </div>
                    <div>
                      <div className="text-gray-400">Power</div>
                      <div className="text-white font-semibold">{gpu.power}W</div>
                    </div>
                    <div>
                      <div className="text-gray-400">Benchmark</div>
                      <div className="text-white font-semibold">{gpu.benchmark}</div>
                    </div>
                    <div>
                      <div className="text-gray-400">Utilization</div>
                      <div className="text-white font-semibold">
                        {((gpu.memoryUsed / gpu.memory) * 100).toFixed(0)}%
                      </div>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}

          {activeTab === 'memory' && (
            <div className="space-y-4">
              <h3 className="text-lg font-semibold text-white mb-4">Memory Pools</h3>
              {memoryPools.map((pool) => (
                <div key={pool.poolId} className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                  <h4 className="text-white font-semibold mb-4">{pool.poolId.replace('-', ' ').toUpperCase()}</h4>
                  <div className="grid grid-cols-3 gap-4 mb-4">
                    <div>
                      <div className="text-sm text-gray-400">Total Memory</div>
                      <div className="text-lg text-white font-semibold">{(pool.totalMemory / 1024).toFixed(1)}GB</div>
                    </div>
                    <div>
                      <div className="text-sm text-gray-400">Utilization</div>
                      <div className="text-lg text-white font-semibold">
                        {((pool.usedMemory / pool.totalMemory) * 100).toFixed(1)}%
                      </div>
                    </div>
                    <div>
                      <div className="text-sm text-gray-400">Fragmentation</div>
                      <div className="text-lg text-white font-semibold">{pool.fragmentation}%</div>
                    </div>
                  </div>
                  <div className="space-y-2">
                    <p className="text-sm text-gray-400">Active Allocations:</p>
                    {pool.allocations.map((alloc) => (
                      <div key={alloc.taskId} className="flex justify-between text-sm">
                        <span className="text-gray-300">{alloc.taskId}</span>
                        <span className="text-cyan-400">{(alloc.size / 1024).toFixed(1)}GB</span>
                      </div>
                    ))}
                  </div>
                </div>
              ))}
            </div>
          )}

          {activeTab === 'fallback' && (
            <div className="space-y-4">
              <h3 className="text-lg font-semibold text-white mb-4">Fallback Chain</h3>
              <div className="space-y-3">
                {fallbackChain.map((stage) => (
                  <div key={stage.id} className="flex items-center gap-4 bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                    <div className="flex-shrink-0">
                      <div className="w-10 h-10 rounded-full bg-gradient-to-br from-cyan-500 to-blue-500 flex items-center justify-center text-white font-bold">
                        {stage.stage}
                      </div>
                    </div>
                    <div className="flex-1">
                      <p className="text-white font-semibold">{stage.type}</p>
                      <p className="text-sm text-gray-400">Execution: {stage.executionTime}ms</p>
                    </div>
                    <div>
                      <div
                        className={`px-3 py-1 rounded-full text-xs font-semibold ${
                          stage.status === 'active'
                            ? 'bg-green-500/20 text-green-400'
                            : 'bg-yellow-500/20 text-yellow-400'
                        }`}
                      >
                        {stage.status.toUpperCase()}
                      </div>
                    </div>
                    <div className="text-right">
                      <p className="text-white font-semibold">{stage.reliability}%</p>
                      <p className="text-xs text-gray-400">Reliability</p>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {activeTab === 'benchmarks' && (
            <div>
              <h3 className="text-lg font-semibold text-white mb-4">Performance Benchmarks</h3>
              <div className="grid grid-cols-4 gap-4 mb-6">
                {benchmarks.map((bench) => (
                  <div key={bench.date} className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-3">
                    <p className="text-sm text-gray-400 mb-2">{bench.date}</p>
                    <div className="space-y-2">
                      <div className="flex justify-between items-center">
                        <span className="text-xs text-gray-500">Throughput</span>
                        <span className="text-sm font-semibold text-cyan-400">{bench.throughput}</span>
                      </div>
                      <div className="flex justify-between items-center">
                        <span className="text-xs text-gray-500">Latency</span>
                        <span className="text-sm font-semibold text-green-400">{bench.latency}ms</span>
                      </div>
                      <div className="flex justify-between items-center">
                        <span className="text-xs text-gray-500">Efficiency</span>
                        <span className="text-sm font-semibold text-blue-400">{bench.efficiency}%</span>
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

export default ChainCoreOptimizationPanel;
