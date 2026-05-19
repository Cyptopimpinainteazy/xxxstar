import React, { useState } from 'react';
import { Globe, MapPin, Users, Activity, TrendingUp } from 'lucide-react';

interface Validator {
  id: string;
  name: string;
  region: string;
  status: 'online' | 'offline' | 'warning';
  stake: number;
  blocks: number;
  gpuCount: number;
  uptime: number;
  coordinates: { lat: number; lng: number };
}

export const GeoDistributionPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'map' | 'list' | 'analytics'>('map');
  const [selectedValidator, setSelectedValidator] = useState<string | null>(null);

  const validators: Validator[] = [
    {
      id: '1',
      name: 'Validator-US-East',
      region: 'us-east-1 (Virginia)',
      status: 'online',
      stake: 1200000,
      blocks: 4520,
      gpuCount: 3,
      uptime: 99.8,
      coordinates: { lat: 38.8, lng: -77.8 },
    },
    {
      id: '2',
      name: 'Validator-EU-West',
      region: 'eu-west-1 (Ireland)',
      status: 'online',
      stake: 950000,
      blocks: 4215,
      gpuCount: 2,
      uptime: 99.5,
      coordinates: { lat: 53.4, lng: -8.2 },
    },
    {
      id: '3',
      name: 'Validator-AP-Singapore',
      region: 'ap-southeast-1 (Singapore)',
      status: 'online',
      stake: 800000,
      blocks: 3890,
      gpuCount: 2,
      uptime: 98.9,
      coordinates: { lat: 1.3, lng: 103.8 },
    },
    {
      id: '4',
      name: 'Validator-JP-Tokyo',
      region: 'ap-northeast-1 (Tokyo)',
      status: 'warning',
      stake: 650000,
      blocks: 3100,
      gpuCount: 1,
      uptime: 97.2,
      coordinates: { lat: 35.7, lng: 139.7 },
    },
    {
      id: '5',
      name: 'Validator-AU-Sydney',
      region: 'ap-southeast-2 (Sydney)',
      status: 'online',
      stake: 720000,
      blocks: 3650,
      gpuCount: 1,
      uptime: 99.1,
      coordinates: { lat: -33.9, lng: 151.2 },
    },
  ];

  const regionStats = [
    { region: 'North America', validators: 2, stake: '1.85M X3', coverage: 'East Coast' },
    { region: 'Europe', validators: 1, stake: '950K X3', coverage: 'Western EU' },
    { region: 'Asia-Pacific', validators: 2, stake: '1.52M X3', coverage: 'Singapore, Tokyo, Sydney' },
  ];

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'online':
        return 'bg-emerald-500/30 text-emerald-400 border-emerald-500/30';
      case 'warning':
        return 'bg-yellow-500/30 text-yellow-400 border-yellow-500/30';
      default:
        return 'bg-red-500/30 text-red-400 border-red-500/30';
    }
  };

  return (
    <div className="w-full h-full flex flex-col bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f] border-l border-[#2a2a35]">
      {/* Header */}
      <div className="px-6 py-4 border-b border-[#2a2a35] bg-gradient-to-r from-cyan-500/20 to-emerald-500/20">
        <div className="flex items-center gap-3 mb-2">
          <Globe className="w-5 h-5 text-cyan-400" />
          <h1 className="text-lg font-bold text-white">Validator Geographic Distribution</h1>
        </div>
        <p className="text-sm text-gray-400">5 validators across 4 regions, 5.29M X3 total stake</p>
      </div>

      {/* Tab Navigation */}
      <div className="flex gap-6 px-6 py-3 border-b border-[#2a2a35] bg-[#0f0f15]/50">
        {(['map', 'list', 'analytics'] as const).map((tab) => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab)}
            className={`px-3 py-2 text-sm font-medium transition ${
              activeTab === tab
                ? 'text-cyan-400 border-b-2 border-cyan-400'
                : 'text-gray-400 hover:text-gray-300'
            }`}
          >
            {tab === 'map' && 'World Map'}
            {tab === 'list' && 'Validator List'}
            {tab === 'analytics' && 'Regional Analytics'}
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        {activeTab === 'map' && (
          <div className="p-6">
            <div className="w-full aspect-video bg-[#0f0f15] border border-[#2a2a35] rounded-lg flex items-center justify-center relative overflow-hidden">
              {/* Simplified world map visualization */}
              <svg className="w-full h-full" viewBox="0 0 1000 600" preserveAspectRatio="xMidYMid meet">
                {/* World map regions (simplified) */}
                <g opacity="0.1" fill="none" stroke="currentColor" strokeWidth="1">
                  <line x1="100" y1="200" x2="200" y2="250" />
                  <line x1="200" y1="150" x2="300" y2="180" />
                  <line x1="600" y1="200" x2="700" y2="220" />
                  <line x1="700" y1="300" x2="800" y2="350" />
                </g>
                
                {/* Validator markers */}
                {validators.map((v) => {
                  const x = ((v.coordinates.lng + 180) / 360) * 1000;
                  const y = ((90 - v.coordinates.lat) / 180) * 600;
                  return (
                    <g key={v.id} onClick={() => setSelectedValidator(v.id)} style={{ cursor: 'pointer' }}>
                      <circle cx={x} cy={y} r="12" fill={v.status === 'online' ? '#10b981' : v.status === 'warning' ? '#eab308' : '#ef4444'} opacity="0.8" />
                      <circle cx={x} cy={y} r="20" fill="none" stroke={v.status === 'online' ? '#10b981' : v.status === 'warning' ? '#eab308' : '#ef4444'} strokeWidth="2" opacity="0.3" />
                    </g>
                  );
                })}
              </svg>
              
              {/* Legend */}
              <div className="absolute bottom-4 left-4 bg-[#0a0a0f]/80 border border-[#2a2a35] rounded p-3 text-xs space-y-2">
                <div className="flex items-center gap-2">
                  <div className="w-2 h-2 rounded-full bg-emerald-500" />
                  <span className="text-emerald-400">Online</span>
                </div>
                <div className="flex items-center gap-2">
                  <div className="w-2 h-2 rounded-full bg-yellow-500" />
                  <span className="text-yellow-400">Warning</span>
                </div>
                <div className="flex items-center gap-2">
                  <div className="w-2 h-2 rounded-full bg-red-500" />
                  <span className="text-red-400">Offline</span>
                </div>
              </div>
            </div>
            
            {selectedValidator && (
              <div className="mt-4 p-4 border border-[#2a2a35] rounded-lg bg-[#0f0f15]">
                {validators
                  .filter((v) => v.id === selectedValidator)
                  .map((v) => (
                    <div key={v.id}>
                      <h3 className="font-semibold text-white mb-2">{v.name}</h3>
                      <div className="grid grid-cols-2 gap-3 text-sm">
                        <div><span className="text-gray-500">Region:</span> {v.region}</div>
                        <div><span className="text-gray-500">Stake:</span> {(v.stake / 1000000).toFixed(2)}M X3</div>
                        <div><span className="text-gray-500">GPU Count:</span> {v.gpuCount}x</div>
                        <div><span className="text-gray-500">Uptime:</span> {v.uptime}%</div>
                      </div>
                    </div>
                  ))}
              </div>
            )}
          </div>
        )}

        {activeTab === 'list' && (
          <div className="p-6 space-y-3">
            {validators.map((v) => (
              <div
                key={v.id}
                onClick={() => setSelectedValidator(v.id)}
                className={`p-4 border rounded-lg cursor-pointer transition ${
                  selectedValidator === v.id
                    ? 'border-cyan-500 bg-cyan-500/10'
                    : 'border-[#2a2a35] hover:border-cyan-500/50'
                }`}
              >
                <div className="flex justify-between items-start mb-2">
                  <div>
                    <h3 className="font-semibold text-white">{v.name}</h3>
                    <p className="text-xs text-gray-500 mt-1">{v.region}</p>
                  </div>
                  <span className={`px-2 py-1 text-xs rounded font-semibold ${getStatusColor(v.status)}`}>
                    {v.status.toUpperCase()}
                  </span>
                </div>
                <div className="grid grid-cols-4 gap-2 text-sm">
                  <div>
                    <span className="text-gray-500 text-xs">Stake</span>
                    <p className="text-cyan-400 font-semibold">{(v.stake / 1000000).toFixed(2)}M</p>
                  </div>
                  <div>
                    <span className="text-gray-500 text-xs">Blocks</span>
                    <p className="text-emerald-400 font-semibold">{v.blocks}</p>
                  </div>
                  <div>
                    <span className="text-gray-500 text-xs">GPU</span>
                    <p className="text-purple-400 font-semibold">{v.gpuCount}x</p>
                  </div>
                  <div>
                    <span className="text-gray-500 text-xs">Uptime</span>
                    <p className="text-yellow-400 font-semibold">{v.uptime}%</p>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}

        {activeTab === 'analytics' && (
          <div className="p-6 space-y-4">
            {regionStats.map((stat) => (
              <div key={stat.region} className="p-4 border border-[#2a2a35] rounded-lg hover:border-cyan-500/30 transition">
                <div className="flex items-center gap-3 mb-3">
                  <MapPin className="w-4 h-4 text-cyan-400" />
                  <h3 className="font-semibold text-white">{stat.region}</h3>
                </div>
                <div className="grid grid-cols-3 gap-3 text-sm">
                  <div>
                    <span className="text-gray-500 text-xs">Validators</span>
                    <p className="text-cyan-400 font-semibold text-lg">{stat.validators}</p>
                  </div>
                  <div>
                    <span className="text-gray-500 text-xs">Total Stake</span>
                    <p className="text-emerald-400 font-semibold">{stat.stake}</p>
                  </div>
                  <div>
                    <span className="text-gray-500 text-xs">Coverage</span>
                    <p className="text-purple-400 font-semibold text-xs">{stat.coverage}</p>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};

export default GeoDistributionPanel;
