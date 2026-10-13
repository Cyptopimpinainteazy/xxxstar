'use client';

import { useState, useEffect } from 'react';
import Link from 'next/link';
import dynamic from 'next/dynamic';

// Dynamic import for the map component (SSR-safe)
const SwarmMap = dynamic(() => import('@/components/gpu-swarm/SwarmMap'), { 
  ssr: false,
  loading: () => (
    <div className="h-[500px] bg-black/40 rounded-xl flex items-center justify-center">
      <div className="text-gray-500 font-mono">Loading GPU Network Map...</div>
    </div>
  )
});

interface GPUProvider {
  id: number;
  address: string;
  name: string;
  gpuModel: string;
  vram: number;
  tflops: number;
  pricePerHour: number;
  status: 'AVAILABLE' | 'BUSY' | 'OFFLINE' | 'RESERVED';
  uptime: number;
  jobsCompleted: number;
  reputation: number;
  location: string;
  specializations: string[];
}

interface ComputeJob {
  id: number;
  title: string;
  requester: string;
  budget: number;
  gpuRequirements: { minVram: number; minTflops: number };
  deadline: Date;
  bids: number;
  status: 'OPEN' | 'IN_PROGRESS' | 'COMPLETED' | 'CANCELLED';
  type: 'INFERENCE' | 'TRAINING' | 'FINE_TUNING' | 'RENDERING';
}

const MOCK_PROVIDERS: GPUProvider[] = [
  { id: 1, address: '0x7a23...f891', name: 'AlphaGPU Cluster', gpuModel: 'H100 SXM', vram: 80, tflops: 1978, pricePerHour: 4.50, status: 'AVAILABLE', uptime: 99.7, jobsCompleted: 1247, reputation: 98, location: 'US-West', specializations: ['ML Training', 'LLM Inference'] },
  { id: 2, address: '0x8b34...a567', name: 'NeuralForge', gpuModel: 'A100 80GB', vram: 80, tflops: 312, pricePerHour: 2.10, status: 'BUSY', uptime: 99.2, jobsCompleted: 892, reputation: 96, location: 'EU-Central', specializations: ['Fine-Tuning', 'RAG'] },
  { id: 3, address: '0x9c45...b234', name: 'DeepMind Pod', gpuModel: 'RTX 4090', vram: 24, tflops: 82.6, pricePerHour: 0.85, status: 'AVAILABLE', uptime: 98.5, jobsCompleted: 2341, reputation: 94, location: 'Asia-East', specializations: ['Inference', 'Image Gen'] },
  { id: 4, address: '0x3d56...c891', name: 'Quantum Render', gpuModel: 'H100 PCIe', vram: 80, tflops: 1513, pricePerHour: 3.80, status: 'RESERVED', uptime: 99.9, jobsCompleted: 567, reputation: 99, location: 'US-East', specializations: ['Video Rendering', 'Simulation'] },
  { id: 5, address: '0xf678...d234', name: 'Edge AI Node', gpuModel: 'A6000', vram: 48, tflops: 38.7, pricePerHour: 1.20, status: 'AVAILABLE', uptime: 97.8, jobsCompleted: 1823, reputation: 92, location: 'Asia-South', specializations: ['Edge Inference', 'Embedding'] },
  { id: 6, address: '0x2e89...e567', name: 'CloudBurst', gpuModel: 'L40S', vram: 48, tflops: 91.6, pricePerHour: 1.50, status: 'BUSY', uptime: 98.9, jobsCompleted: 445, reputation: 95, location: 'EU-North', specializations: ['Generative AI', 'NLP'] },
];

const MOCK_JOBS: ComputeJob[] = [
  { id: 1, title: 'LLM Fine-tuning - GPT4-style 7B', requester: '0xaa12...3456', budget: 2500, gpuRequirements: { minVram: 80, minTflops: 300 }, deadline: new Date(Date.now() + 86400000), bids: 3, status: 'OPEN', type: 'FINE_TUNING' },
  { id: 2, title: 'Stable Diffusion Batch Inference', requester: '0xbb34...7890', budget: 450, gpuRequirements: { minVram: 24, minTflops: 50 }, deadline: new Date(Date.now() + 43200000), bids: 7, status: 'OPEN', type: 'INFERENCE' },
  { id: 3, title: 'RAG Pipeline Training', requester: '0xcc56...1234', budget: 1200, gpuRequirements: { minVram: 48, minTflops: 200 }, deadline: new Date(Date.now() + 172800000), bids: 2, status: 'OPEN', type: 'TRAINING' },
  { id: 4, title: 'Video Rendering - 4K Animation', requester: '0xdd78...5678', budget: 800, gpuRequirements: { minVram: 48, minTflops: 80 }, deadline: new Date(Date.now() + 28800000), bids: 5, status: 'IN_PROGRESS', type: 'RENDERING' },
];

export default function GPUMarketplace() {
  const [providers, setProviders] = useState(MOCK_PROVIDERS);
  const [jobs, setJobs] = useState(MOCK_JOBS);
  const [activeTab, setActiveTab] = useState<'providers' | 'jobs' | 'map'>('providers');
  const [selectedProvider, setSelectedProvider] = useState<number | null>(null);
  const [filterStatus, setFilterStatus] = useState<string>('ALL');

  // Simulate real-time status updates
  useEffect(() => {
    const interval = setInterval(() => {
      setProviders(prev => prev.map(p => ({
        ...p,
        uptime: Math.max(95, Math.min(100, p.uptime + (Math.random() - 0.5) * 0.2)),
        jobsCompleted: Math.random() > 0.95 ? p.jobsCompleted + 1 : p.jobsCompleted,
      })));
    }, 5000);
    return () => clearInterval(interval);
  }, []);

  const statusColors: Record<string, string> = {
    AVAILABLE: 'text-green-400 bg-green-500/20 border-green-500/30',
    BUSY: 'text-yellow-400 bg-yellow-500/20 border-yellow-500/30',
    OFFLINE: 'text-red-400 bg-red-500/20 border-red-500/30',
    RESERVED: 'text-purple-400 bg-purple-500/20 border-purple-500/30',
  };

  const jobTypeColors: Record<string, string> = {
    INFERENCE: 'text-cyan-400 bg-cyan-500/20 border-cyan-500/30',
    TRAINING: 'text-orange-400 bg-orange-500/20 border-orange-500/30',
    FINE_TUNING: 'text-purple-400 bg-purple-500/20 border-purple-500/30',
    RENDERING: 'text-pink-400 bg-pink-500/20 border-pink-500/30',
  };

  const filteredProviders = filterStatus === 'ALL' 
    ? providers 
    : providers.filter(p => p.status === filterStatus);

  const totalTflops = providers.reduce((acc, p) => acc + p.tflops, 0);
  const totalVram = providers.reduce((acc, p) => acc + p.vram, 0);
  const availableProviders = providers.filter(p => p.status === 'AVAILABLE').length;

  return (
    <div className="min-h-screen bg-gradient-to-br from-slate-950 via-cyan-950/10 to-slate-950">
      {/* Header */}
      <div className="border-b border-cyan-500/20 bg-black/40 backdrop-blur-xl sticky top-0 z-50">
        <div className="container mx-auto px-6 py-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-4">
              <Link href="/x3/swarm" className="text-gray-400 hover:text-white transition">
                ← Back to Swarm
              </Link>
              <span className="text-2xl">🖥️</span>
              <h1 className="text-2xl font-bold bg-gradient-to-r from-cyan-400 to-blue-400 bg-clip-text text-transparent">
                GPU Marketplace
              </h1>
            </div>
            <div className="flex items-center gap-4">
              <button className="px-4 py-2 bg-black/40 border border-cyan-500/30 text-cyan-400 rounded-lg font-mono text-sm hover:bg-cyan-500/10 transition">
                Register GPU
              </button>
              <button className="px-4 py-2 bg-gradient-to-r from-cyan-600 to-blue-600 text-white rounded-lg font-mono text-sm hover:opacity-90 transition">
                Post Job
              </button>
            </div>
          </div>
        </div>
      </div>

      {/* Stats */}
      <div className="container mx-auto px-6 py-6">
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          <div className="bg-black/60 border border-cyan-500/20 rounded-xl p-4">
            <div className="text-xs text-gray-500 font-mono uppercase">Total Providers</div>
            <div className="text-2xl font-bold text-cyan-400 font-mono">{providers.length}</div>
            <div className="text-xs text-green-400 mt-1">{availableProviders} available</div>
          </div>
          <div className="bg-black/60 border border-cyan-500/20 rounded-xl p-4">
            <div className="text-xs text-gray-500 font-mono uppercase">Network TFLOPS</div>
            <div className="text-2xl font-bold text-white font-mono">{totalTflops.toFixed(0)}</div>
          </div>
          <div className="bg-black/60 border border-cyan-500/20 rounded-xl p-4">
            <div className="text-xs text-gray-500 font-mono uppercase">Total VRAM</div>
            <div className="text-2xl font-bold text-purple-400 font-mono">{totalVram} GB</div>
          </div>
          <div className="bg-black/60 border border-cyan-500/20 rounded-xl p-4">
            <div className="text-xs text-gray-500 font-mono uppercase">Open Jobs</div>
            <div className="text-2xl font-bold text-yellow-400 font-mono">{jobs.filter(j => j.status === 'OPEN').length}</div>
          </div>
        </div>
      </div>

      {/* Tabs */}
      <div className="container mx-auto px-6 mb-6">
        <div className="flex items-center gap-2 border-b border-gray-800">
          {[
            { id: 'providers', label: '🖥️ GPU Providers', count: providers.length },
            { id: 'jobs', label: '📋 Compute Jobs', count: jobs.length },
            { id: 'map', label: '🗺️ Network Map', count: null },
          ].map(tab => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id as any)}
              className={`px-4 py-3 font-mono text-sm transition border-b-2 -mb-px ${
                activeTab === tab.id
                  ? 'border-cyan-500 text-cyan-400'
                  : 'border-transparent text-gray-500 hover:text-gray-300'
              }`}
            >
              {tab.label} {tab.count !== null && <span className="text-gray-600">({tab.count})</span>}
            </button>
          ))}
        </div>
      </div>

      <div className="container mx-auto px-6 pb-12">
        {/* Providers Tab */}
        {activeTab === 'providers' && (
          <div>
            {/* Filter */}
            <div className="flex items-center gap-2 mb-6">
              {['ALL', 'AVAILABLE', 'BUSY', 'RESERVED', 'OFFLINE'].map(status => (
                <button
                  key={status}
                  onClick={() => setFilterStatus(status)}
                  className={`px-4 py-2 rounded-lg font-mono text-sm transition ${
                    filterStatus === status
                      ? 'bg-cyan-500/30 text-cyan-400 border border-cyan-500/50'
                      : 'bg-black/40 text-gray-500 border border-gray-700 hover:border-cyan-500/30'
                  }`}
                >
                  {status}
                </button>
              ))}
            </div>

            {/* Providers Grid */}
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
              {filteredProviders.map(provider => (
                <div 
                  key={provider.id}
                  className={`p-6 bg-black/60 border rounded-xl cursor-pointer transition-all ${
                    selectedProvider === provider.id 
                      ? 'border-cyan-500/50 ring-2 ring-cyan-500/20' 
                      : 'border-gray-800/50 hover:border-cyan-500/30'
                  }`}
                  onClick={() => setSelectedProvider(selectedProvider === provider.id ? null : provider.id)}
                >
                  <div className="flex items-start justify-between mb-4">
                    <div>
                      <h3 className="text-lg font-semibold text-white">{provider.name}</h3>
                      <span className="text-xs text-gray-500 font-mono">{provider.address}</span>
                    </div>
                    <span className={`px-3 py-1 rounded-full text-xs font-mono border ${statusColors[provider.status]}`}>
                      {provider.status}
                    </span>
                  </div>

                  {/* GPU Info */}
                  <div className="bg-black/40 rounded-lg p-4 mb-4">
                    <div className="flex items-center justify-between mb-2">
                      <span className="text-cyan-400 font-mono">{provider.gpuModel}</span>
                      <span className="text-yellow-400 font-mono text-sm">${provider.pricePerHour}/hr</span>
                    </div>
                    <div className="grid grid-cols-2 gap-4 text-sm">
                      <div>
                        <span className="text-gray-500">VRAM</span>
                        <span className="text-white ml-2">{provider.vram}GB</span>
                      </div>
                      <div>
                        <span className="text-gray-500">TFLOPS</span>
                        <span className="text-white ml-2">{provider.tflops}</span>
                      </div>
                    </div>
                  </div>

                  {/* Stats */}
                  <div className="flex items-center justify-between text-sm mb-4">
                    <div>
                      <span className="text-gray-500">Uptime</span>
                      <span className="text-green-400 ml-2">{provider.uptime.toFixed(1)}%</span>
                    </div>
                    <div>
                      <span className="text-gray-500">Jobs</span>
                      <span className="text-white ml-2">{provider.jobsCompleted}</span>
                    </div>
                    <div>
                      <span className="text-gray-500">Rep</span>
                      <span className="text-purple-400 ml-2">{provider.reputation}%</span>
                    </div>
                  </div>

                  {/* Specializations */}
                  <div className="flex flex-wrap gap-2">
                    {provider.specializations.map((spec, idx) => (
                      <span key={idx} className="px-2 py-1 bg-gray-800/50 text-gray-400 rounded text-xs">
                        {spec}
                      </span>
                    ))}
                  </div>

                  {/* Rent Button (expanded) */}
                  {selectedProvider === provider.id && provider.status === 'AVAILABLE' && (
                    <div className="mt-4 pt-4 border-t border-gray-800">
                      <button className="w-full py-3 bg-gradient-to-r from-cyan-600 to-blue-600 text-white rounded-lg font-mono hover:opacity-90 transition">
                        Rent GPU
                      </button>
                    </div>
                  )}
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Jobs Tab */}
        {activeTab === 'jobs' && (
          <div className="space-y-4">
            {jobs.map(job => (
              <div key={job.id} className="p-6 bg-black/60 border border-gray-800/50 rounded-xl hover:border-cyan-500/30 transition">
                <div className="flex items-start justify-between mb-4">
                  <div>
                    <span className={`px-3 py-1 rounded-full text-xs font-mono border ${jobTypeColors[job.type]}`}>
                      {job.type.replace('_', ' ')}
                    </span>
                    <h3 className="text-lg font-semibold text-white mt-2">{job.title}</h3>
                    <span className="text-xs text-gray-500 font-mono">by {job.requester}</span>
                  </div>
                  <div className="text-right">
                    <div className="text-2xl font-bold text-yellow-400 font-mono">{job.budget} ATLAS</div>
                    <div className="text-xs text-gray-500">Budget</div>
                  </div>
                </div>

                <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-4">
                  <div className="bg-black/40 rounded-lg p-3">
                    <div className="text-xs text-gray-500">Min VRAM</div>
                    <div className="text-white font-mono">{job.gpuRequirements.minVram} GB</div>
                  </div>
                  <div className="bg-black/40 rounded-lg p-3">
                    <div className="text-xs text-gray-500">Min TFLOPS</div>
                    <div className="text-white font-mono">{job.gpuRequirements.minTflops}</div>
                  </div>
                  <div className="bg-black/40 rounded-lg p-3">
                    <div className="text-xs text-gray-500">Deadline</div>
                    <div className="text-cyan-400 font-mono text-sm">
                      {Math.floor((job.deadline.getTime() - Date.now()) / 3600000)}h
                    </div>
                  </div>
                  <div className="bg-black/40 rounded-lg p-3">
                    <div className="text-xs text-gray-500">Bids</div>
                    <div className="text-purple-400 font-mono">{job.bids}</div>
                  </div>
                </div>

                <div className="flex items-center justify-between">
                  <span className={`px-3 py-1 rounded text-xs font-mono ${
                    job.status === 'OPEN' ? 'bg-green-500/20 text-green-400' :
                    job.status === 'IN_PROGRESS' ? 'bg-yellow-500/20 text-yellow-400' :
                    'bg-gray-500/20 text-gray-400'
                  }`}>
                    {job.status.replace('_', ' ')}
                  </span>
                  {job.status === 'OPEN' && (
                    <button className="px-6 py-2 bg-gradient-to-r from-cyan-600 to-blue-600 text-white rounded-lg font-mono text-sm hover:opacity-90 transition">
                      Submit Bid
                    </button>
                  )}
                </div>
              </div>
            ))}
          </div>
        )}

        {/* Map Tab */}
        {activeTab === 'map' && (
          <div className="space-y-6">
            <div className="bg-black/60 border border-cyan-500/20 rounded-xl overflow-hidden">
              <SwarmMap />
            </div>
            <p className="text-gray-500 text-sm text-center">
              Interactive map showing global GPU node distribution. Click on nodes for details.
            </p>
          </div>
        )}
      </div>
    </div>
  );
}
