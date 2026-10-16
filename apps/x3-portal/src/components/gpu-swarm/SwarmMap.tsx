'use client';

import React, { useState, useEffect, useMemo, useCallback } from 'react';
import dynamic from 'next/dynamic';

interface GpuNode {
  id: string;
  lat: number;
  lng: number;
  city: string;
  country: string;
  gpu: string;
  vram: number;
  status: 'active' | 'idle' | 'syncing' | 'offline';
  tasks: number;
  hashrate: number;
}

// Mock GPU node data - in production this would come from the swarm coordinator
const MOCK_GPU_NODES: GpuNode[] = [
  // North America
  { id: 'node-sf-01', lat: 37.7749, lng: -122.4194, city: 'San Francisco', country: 'USA', gpu: 'RTX 4090', vram: 24, status: 'active', tasks: 12, hashrate: 145.2 },
  { id: 'node-la-01', lat: 34.0522, lng: -118.2437, city: 'Los Angeles', country: 'USA', gpu: 'RTX 4080', vram: 16, status: 'active', tasks: 8, hashrate: 98.7 },
  { id: 'node-nyc-01', lat: 40.7128, lng: -74.0060, city: 'New York', country: 'USA', gpu: 'A100', vram: 80, status: 'active', tasks: 24, hashrate: 312.5 },
  { id: 'node-chi-01', lat: 41.8781, lng: -87.6298, city: 'Chicago', country: 'USA', gpu: 'RTX 3090', vram: 24, status: 'idle', tasks: 0, hashrate: 0 },
  { id: 'node-sea-01', lat: 47.6062, lng: -122.3321, city: 'Seattle', country: 'USA', gpu: 'RTX 4090', vram: 24, status: 'active', tasks: 15, hashrate: 167.3 },
  { id: 'node-tor-01', lat: 43.6532, lng: -79.3832, city: 'Toronto', country: 'Canada', gpu: 'RTX 4070', vram: 12, status: 'active', tasks: 6, hashrate: 72.1 },
  { id: 'node-van-01', lat: 49.2827, lng: -123.1207, city: 'Vancouver', country: 'Canada', gpu: 'RTX 3080', vram: 10, status: 'active', tasks: 4, hashrate: 54.8 },
  { id: 'node-mia-01', lat: 25.7617, lng: -80.1918, city: 'Miami', country: 'USA', gpu: 'H100', vram: 80, status: 'active', tasks: 32, hashrate: 456.2 },
  { id: 'node-den-01', lat: 39.7392, lng: -104.9903, city: 'Denver', country: 'USA', gpu: 'RTX 4080', vram: 16, status: 'syncing', tasks: 2, hashrate: 23.4 },
  { id: 'node-aus-01', lat: 30.2672, lng: -97.7431, city: 'Austin', country: 'USA', gpu: 'RTX 4090', vram: 24, status: 'active', tasks: 18, hashrate: 189.6 },
  
  // Europe
  { id: 'node-lon-01', lat: 51.5074, lng: -0.1278, city: 'London', country: 'UK', gpu: 'A100', vram: 80, status: 'active', tasks: 28, hashrate: 345.8 },
  { id: 'node-ams-01', lat: 52.3676, lng: 4.9041, city: 'Amsterdam', country: 'Netherlands', gpu: 'RTX 4090', vram: 24, status: 'active', tasks: 14, hashrate: 156.4 },
  { id: 'node-fra-01', lat: 50.1109, lng: 8.6821, city: 'Frankfurt', country: 'Germany', gpu: 'H100', vram: 80, status: 'active', tasks: 36, hashrate: 489.3 },
  { id: 'node-par-01', lat: 48.8566, lng: 2.3522, city: 'Paris', country: 'France', gpu: 'RTX 4080', vram: 16, status: 'active', tasks: 9, hashrate: 87.2 },
  { id: 'node-ber-01', lat: 52.5200, lng: 13.4050, city: 'Berlin', country: 'Germany', gpu: 'RTX 4090', vram: 24, status: 'idle', tasks: 0, hashrate: 0 },
  { id: 'node-zur-01', lat: 47.3769, lng: 8.5417, city: 'Zurich', country: 'Switzerland', gpu: 'A100', vram: 80, status: 'active', tasks: 22, hashrate: 287.6 },
  { id: 'node-sto-01', lat: 59.3293, lng: 18.0686, city: 'Stockholm', country: 'Sweden', gpu: 'RTX 4070', vram: 12, status: 'active', tasks: 5, hashrate: 61.3 },
  { id: 'node-dub-01', lat: 53.3498, lng: -6.2603, city: 'Dublin', country: 'Ireland', gpu: 'RTX 4090', vram: 24, status: 'active', tasks: 11, hashrate: 134.7 },
  { id: 'node-mad-01', lat: 40.4168, lng: -3.7038, city: 'Madrid', country: 'Spain', gpu: 'RTX 3090', vram: 24, status: 'syncing', tasks: 1, hashrate: 12.4 },
  { id: 'node-mil-01', lat: 45.4642, lng: 9.1900, city: 'Milan', country: 'Italy', gpu: 'RTX 4080', vram: 16, status: 'active', tasks: 7, hashrate: 78.9 },
  
  // Asia Pacific
  { id: 'node-tok-01', lat: 35.6762, lng: 139.6503, city: 'Tokyo', country: 'Japan', gpu: 'H100', vram: 80, status: 'active', tasks: 42, hashrate: 534.2 },
  { id: 'node-sin-01', lat: 1.3521, lng: 103.8198, city: 'Singapore', country: 'Singapore', gpu: 'A100', vram: 80, status: 'active', tasks: 31, hashrate: 398.7 },
  { id: 'node-hkg-01', lat: 22.3193, lng: 114.1694, city: 'Hong Kong', country: 'China', gpu: 'RTX 4090', vram: 24, status: 'active', tasks: 16, hashrate: 178.3 },
  { id: 'node-syd-01', lat: -33.8688, lng: 151.2093, city: 'Sydney', country: 'Australia', gpu: 'RTX 4090', vram: 24, status: 'active', tasks: 13, hashrate: 145.6 },
  { id: 'node-mel-01', lat: -37.8136, lng: 144.9631, city: 'Melbourne', country: 'Australia', gpu: 'RTX 4080', vram: 16, status: 'idle', tasks: 0, hashrate: 0 },
  { id: 'node-sel-01', lat: 37.5665, lng: 126.9780, city: 'Seoul', country: 'South Korea', gpu: 'H100', vram: 80, status: 'active', tasks: 38, hashrate: 478.9 },
  { id: 'node-mum-01', lat: 19.0760, lng: 72.8777, city: 'Mumbai', country: 'India', gpu: 'RTX 4090', vram: 24, status: 'active', tasks: 10, hashrate: 112.4 },
  { id: 'node-ban-01', lat: 12.9716, lng: 77.5946, city: 'Bangalore', country: 'India', gpu: 'A100', vram: 80, status: 'active', tasks: 25, hashrate: 321.8 },
  { id: 'node-jkt-01', lat: -6.2088, lng: 106.8456, city: 'Jakarta', country: 'Indonesia', gpu: 'RTX 3090', vram: 24, status: 'syncing', tasks: 3, hashrate: 34.2 },
  { id: 'node-bkk-01', lat: 13.7563, lng: 100.5018, city: 'Bangkok', country: 'Thailand', gpu: 'RTX 4070', vram: 12, status: 'active', tasks: 4, hashrate: 48.7 },
  
  // South America
  { id: 'node-sao-01', lat: -23.5505, lng: -46.6333, city: 'São Paulo', country: 'Brazil', gpu: 'RTX 4090', vram: 24, status: 'active', tasks: 9, hashrate: 98.3 },
  { id: 'node-bue-01', lat: -34.6037, lng: -58.3816, city: 'Buenos Aires', country: 'Argentina', gpu: 'RTX 4080', vram: 16, status: 'active', tasks: 6, hashrate: 67.4 },
  { id: 'node-bog-01', lat: 4.7110, lng: -74.0721, city: 'Bogotá', country: 'Colombia', gpu: 'RTX 3080', vram: 10, status: 'idle', tasks: 0, hashrate: 0 },
  { id: 'node-lim-01', lat: -12.0464, lng: -77.0428, city: 'Lima', country: 'Peru', gpu: 'RTX 4070', vram: 12, status: 'active', tasks: 3, hashrate: 36.1 },
  { id: 'node-scl-01', lat: -33.4489, lng: -70.6693, city: 'Santiago', country: 'Chile', gpu: 'RTX 4090', vram: 24, status: 'active', tasks: 7, hashrate: 82.5 },
  
  // Middle East & Africa
  { id: 'node-dxb-01', lat: 25.2048, lng: 55.2708, city: 'Dubai', country: 'UAE', gpu: 'H100', vram: 80, status: 'active', tasks: 29, hashrate: 367.8 },
  { id: 'node-tlv-01', lat: 32.0853, lng: 34.7818, city: 'Tel Aviv', country: 'Israel', gpu: 'A100', vram: 80, status: 'active', tasks: 21, hashrate: 276.4 },
  { id: 'node-cpt-01', lat: -33.9249, lng: 18.4241, city: 'Cape Town', country: 'South Africa', gpu: 'RTX 4080', vram: 16, status: 'active', tasks: 5, hashrate: 54.3 },
  { id: 'node-jnb-01', lat: -26.2041, lng: 28.0473, city: 'Johannesburg', country: 'South Africa', gpu: 'RTX 4090', vram: 24, status: 'syncing', tasks: 2, hashrate: 21.7 },
  { id: 'node-cai-01', lat: 30.0444, lng: 31.2357, city: 'Cairo', country: 'Egypt', gpu: 'RTX 3090', vram: 24, status: 'active', tasks: 4, hashrate: 43.2 },
];

interface SwarmMapProps {
  className?: string;
}

// Dynamically import the map component to avoid SSR issues with Leaflet
const LeafletMap = dynamic(() => import('./LeafletMap'), {
  ssr: false,
  loading: () => (
    <div className="w-full h-[600px] bg-black/80 rounded-2xl flex items-center justify-center border border-cyan-500/20">
      <div className="text-cyan-400 font-mono animate-pulse">INITIALIZING NEURAL MAP...</div>
    </div>
  ),
});

export function SwarmMap({ className = '' }: SwarmMapProps) {
  const [selectedNode, setSelectedNode] = useState<GpuNode | null>(null);
  const [nodes, setNodes] = useState<GpuNode[]>(MOCK_GPU_NODES);
  const [filter, setFilter] = useState<'all' | 'active' | 'idle' | 'syncing'>('all');

  // Simulate real-time updates
  useEffect(() => {
    const interval = setInterval(() => {
      setNodes(prev => prev.map(node => ({
        ...node,
        tasks: node.status === 'active' ? Math.max(0, node.tasks + Math.floor(Math.random() * 3) - 1) : node.tasks,
        hashrate: node.status === 'active' ? Math.max(0, node.hashrate + (Math.random() * 10 - 5)) : node.hashrate,
      })));
    }, 2000);
    return () => clearInterval(interval);
  }, []);

  const filteredNodes = useMemo(() => {
    if (filter === 'all') return nodes;
    return nodes.filter(n => n.status === filter);
  }, [nodes, filter]);

  const stats = useMemo(() => ({
    total: nodes.length,
    active: nodes.filter(n => n.status === 'active').length,
    idle: nodes.filter(n => n.status === 'idle').length,
    syncing: nodes.filter(n => n.status === 'syncing').length,
    totalVram: nodes.reduce((acc, n) => acc + n.vram, 0),
    totalTasks: nodes.reduce((acc, n) => acc + n.tasks, 0),
    totalHashrate: nodes.reduce((acc, n) => acc + n.hashrate, 0),
  }), [nodes]);

  const handleNodeSelect = useCallback((node: GpuNode | null) => {
    setSelectedNode(node);
  }, []);

  const getNodeColor = (status: string) => {
    switch (status) {
      case 'active': return { main: '#00ff9d', glow: 'rgba(0, 255, 157, 0.8)' };
      case 'idle': return { main: '#fbbf24', glow: 'rgba(251, 191, 36, 0.6)' };
      case 'syncing': return { main: '#06b6d4', glow: 'rgba(6, 182, 212, 0.7)' };
      case 'offline': return { main: '#ef4444', glow: 'rgba(239, 68, 68, 0.6)' };
      default: return { main: '#6b7280', glow: 'rgba(107, 114, 128, 0.5)' };
    }
  };

  return (
    <div className={`relative ${className}`}>
      {/* Stats Bar */}
      <div className="flex flex-wrap gap-4 mb-6 p-4 bg-black/60 backdrop-blur-xl border border-cyan-500/20 rounded-xl">
        <div className="flex-1 min-w-[150px]">
          <div className="text-xs text-cyan-400 uppercase tracking-wider font-mono">Total Nodes</div>
          <div className="text-2xl font-bold text-white font-mono">{stats.total}</div>
        </div>
        <div className="flex-1 min-w-[150px]">
          <div className="text-xs text-green-400 uppercase tracking-wider font-mono">Active</div>
          <div className="text-2xl font-bold text-green-400 font-mono">{stats.active}</div>
        </div>
        <div className="flex-1 min-w-[150px]">
          <div className="text-xs text-cyan-400 uppercase tracking-wider font-mono">Total VRAM</div>
          <div className="text-2xl font-bold text-white font-mono">{stats.totalVram} GB</div>
        </div>
        <div className="flex-1 min-w-[150px]">
          <div className="text-xs text-cyan-400 uppercase tracking-wider font-mono">Active Tasks</div>
          <div className="text-2xl font-bold text-cyan-400 font-mono">{stats.totalTasks}</div>
        </div>
        <div className="flex-1 min-w-[150px]">
          <div className="text-xs text-pink-400 uppercase tracking-wider font-mono">Hashrate</div>
          <div className="text-2xl font-bold text-pink-400 font-mono">{stats.totalHashrate.toFixed(1)} TH/s</div>
        </div>
      </div>

      {/* Filter Buttons */}
      <div className="flex gap-2 mb-4">
        {(['all', 'active', 'idle', 'syncing'] as const).map(f => (
          <button
            key={f}
            onClick={() => setFilter(f)}
            className={`px-4 py-2 rounded-lg font-mono text-sm uppercase tracking-wider transition-all ${
              filter === f 
                ? 'bg-cyan-500/30 text-cyan-300 border border-cyan-500/50' 
                : 'bg-black/40 text-gray-500 border border-gray-800 hover:border-cyan-500/30 hover:text-cyan-400'
            }`}
          >
            {f}
          </button>
        ))}
      </div>

      {/* Real World Map with Leaflet */}
      <div className="relative rounded-2xl overflow-hidden border border-cyan-500/20">
        {/* Cyberpunk overlay effects */}
        <div className="absolute inset-0 pointer-events-none z-[1000] opacity-[0.03]"
          style={{
            backgroundImage: 'repeating-linear-gradient(0deg, transparent, transparent 2px, rgba(0, 255, 255, 0.1) 2px, rgba(0, 255, 255, 0.1) 4px)',
          }}
        />
        
        {/* Corner Decorations */}
        <div className="absolute top-0 left-0 w-20 h-20 border-l-2 border-t-2 border-cyan-500/50 z-[1000] pointer-events-none" />
        <div className="absolute top-0 right-0 w-20 h-20 border-r-2 border-t-2 border-cyan-500/50 z-[1000] pointer-events-none" />
        <div className="absolute bottom-0 left-0 w-20 h-20 border-l-2 border-b-2 border-cyan-500/50 z-[1000] pointer-events-none" />
        <div className="absolute bottom-0 right-0 w-20 h-20 border-r-2 border-b-2 border-cyan-500/50 z-[1000] pointer-events-none" />

        {/* Title Overlay */}
        <div className="absolute top-4 left-4 font-mono z-[1000] pointer-events-none">
          <div className="text-cyan-400 text-xs uppercase tracking-[0.3em] drop-shadow-[0_0_10px_rgba(0,255,255,0.8)]">Atlas Sphere</div>
          <div className="text-white text-2xl font-bold tracking-wider drop-shadow-[0_0_20px_rgba(255,255,255,0.5)]">GPU SWARM NETWORK</div>
        </div>

        {/* Legend */}
        <div className="absolute bottom-4 left-4 flex gap-4 font-mono text-xs z-[1000] pointer-events-none bg-black/60 backdrop-blur-sm px-4 py-2 rounded-lg border border-cyan-500/30">
          <div className="flex items-center gap-2">
            <span className="w-3 h-3 rounded-full bg-[#00ff9d] shadow-[0_0_10px_#00ff9d,0_0_20px_#00ff9d]" />
            <span className="text-gray-300">Active</span>
          </div>
          <div className="flex items-center gap-2">
            <span className="w-3 h-3 rounded-full bg-[#fbbf24] shadow-[0_0_10px_#fbbf24]" />
            <span className="text-gray-300">Idle</span>
          </div>
          <div className="flex items-center gap-2">
            <span className="w-3 h-3 rounded-full bg-[#06b6d4] shadow-[0_0_10px_#06b6d4]" />
            <span className="text-gray-300">Syncing</span>
          </div>
        </div>

        <LeafletMap 
          nodes={filteredNodes} 
          selectedNode={selectedNode}
          onNodeSelect={handleNodeSelect}
        />
      </div>

      {/* Selected Node Detail Panel */}
      {selectedNode && (
        <div className="mt-4 p-6 bg-black/80 backdrop-blur-xl border border-cyan-500/30 rounded-xl">
          <div className="flex justify-between items-start mb-4">
            <div>
              <div className="text-cyan-400 text-xs uppercase tracking-wider font-mono">Selected Node</div>
              <div className="text-2xl font-bold text-white">{selectedNode.city}, {selectedNode.country}</div>
              <div className="text-gray-500 font-mono text-sm">{selectedNode.id}</div>
            </div>
            <button 
              onClick={() => setSelectedNode(null)}
              className="text-gray-500 hover:text-white text-xl"
            >
              ×
            </button>
          </div>
          
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            <div className="p-4 bg-black/60 rounded-lg border border-gray-800">
              <div className="text-xs text-gray-500 uppercase tracking-wider font-mono">Status</div>
              <div className="flex items-center gap-2 mt-1">
                <span className="w-2 h-2 rounded-full" style={{ backgroundColor: getNodeColor(selectedNode.status).main }} />
                <span className="text-white capitalize font-mono">{selectedNode.status}</span>
              </div>
            </div>
            <div className="p-4 bg-black/60 rounded-lg border border-gray-800">
              <div className="text-xs text-gray-500 uppercase tracking-wider font-mono">GPU</div>
              <div className="text-white font-mono mt-1">{selectedNode.gpu}</div>
            </div>
            <div className="p-4 bg-black/60 rounded-lg border border-gray-800">
              <div className="text-xs text-gray-500 uppercase tracking-wider font-mono">VRAM</div>
              <div className="text-cyan-400 font-mono mt-1">{selectedNode.vram} GB</div>
            </div>
            <div className="p-4 bg-black/60 rounded-lg border border-gray-800">
              <div className="text-xs text-gray-500 uppercase tracking-wider font-mono">Active Tasks</div>
              <div className="text-green-400 font-mono mt-1">{selectedNode.tasks}</div>
            </div>
            <div className="p-4 bg-black/60 rounded-lg border border-gray-800">
              <div className="text-xs text-gray-500 uppercase tracking-wider font-mono">Hashrate</div>
              <div className="text-pink-400 font-mono mt-1">{selectedNode.hashrate.toFixed(1)} TH/s</div>
            </div>
            <div className="p-4 bg-black/60 rounded-lg border border-gray-800">
              <div className="text-xs text-gray-500 uppercase tracking-wider font-mono">Coordinates</div>
              <div className="text-gray-300 font-mono mt-1 text-sm">{selectedNode.lat.toFixed(2)}°, {selectedNode.lng.toFixed(2)}°</div>
            </div>
            <div className="col-span-2 p-4 bg-black/60 rounded-lg border border-gray-800">
              <div className="text-xs text-gray-500 uppercase tracking-wider font-mono">Network Connection</div>
              <div className="flex items-center gap-2 mt-2">
                <div className="flex-1 h-1 bg-gray-800 rounded-full overflow-hidden">
                  <div 
                    className="h-full bg-gradient-to-r from-cyan-500 to-green-500 rounded-full"
                    style={{ width: selectedNode.status === 'active' ? '95%' : selectedNode.status === 'syncing' ? '60%' : '0%' }}
                  />
                </div>
                <span className="text-gray-400 font-mono text-xs">
                  {selectedNode.status === 'active' ? '95%' : selectedNode.status === 'syncing' ? '60%' : '0%'}
                </span>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default SwarmMap;
