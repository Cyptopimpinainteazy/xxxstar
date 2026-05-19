import React, { useState, useEffect } from 'react';
import { Map, Navigation, AlertCircle, CheckCircle, Activity } from 'lucide-react';

interface GeoLocation {
  latitude: number;
  longitude: number;
  address: string;
  city: string;
  country: string;
  isp: string;
  lastSeen: string;
  risk: 'low' | 'medium' | 'high';
}

export const GeoLocationPanel: React.FC = () => {
  const [locations, setLocations] = useState<GeoLocation[]>([
    {
      latitude: 37.7749,
      longitude: -122.4194,
      address: '123 Market St, San Francisco',
      city: 'San Francisco',
      country: 'United States',
      isp: 'Premium Fiber Networks',
      lastSeen: '2 hours ago',
      risk: 'low',
    },
    {
      latitude: 51.5074,
      longitude: -0.1278,
      address: '10 Downing Street, London',
      city: 'London',
      country: 'United Kingdom',
      isp: 'International Cloud Services Ltd',
      lastSeen: '5 days ago',
      risk: 'low',
    },
    {
      latitude: 35.6762,
      longitude: 139.6503,
      address: 'Shibuya Ward, Tokyo',
      city: 'Tokyo',
      country: 'Japan',
      isp: 'Tokyo Data Centers Inc',
      lastSeen: '1 week ago',
      risk: 'medium',
    },
  ]);

  const getRiskColor = (risk: string) => {
    switch (risk) {
      case 'low':
        return 'bg-green-500/10 text-green-400 border-green-500/20';
      case 'medium':
        return 'bg-yellow-500/10 text-yellow-400 border-yellow-500/20';
      case 'high':
        return 'bg-red-500/10 text-red-400 border-red-500/20';
      default:
        return 'bg-gray-500/10 text-gray-400 border-gray-500/20';
    }
  };

  const getRiskIcon = (risk: string) => {
    switch (risk) {
      case 'low':
        return <CheckCircle className="w-4 h-4" />;
      case 'medium':
        return <AlertCircle className="w-4 h-4" />;
      case 'high':
        return <Activity className="w-4 h-4 animate-pulse" />;
      default:
        return null;
    }
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f] p-6">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between mb-8">
          <div>
            <h1 className="text-4xl font-bold text-transparent bg-clip-text bg-gradient-to-r from-cyan-400 to-blue-500 mb-2">
              Geolocation Tracking
            </h1>
            <p className="text-gray-400">Monitor validator node locations and connection origins</p>
          </div>
          <Map className="w-12 h-12 text-cyan-400" />
        </div>

        {/* Stats */}
        <div className="grid grid-cols-3 gap-4 mb-8">
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-xs mb-2">ACTIVE LOCATIONS</div>
            <div className="text-3xl font-bold text-cyan-400 mb-1">{locations.length}</div>
            <div className="text-xs text-gray-500">Primary in San Francisco</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-xs mb-2">COVERAGE</div>
            <div className="text-3xl font-bold text-blue-400 mb-1">3</div>
            <div className="text-xs text-gray-500">Regions: North America, Europe, Asia</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-xs mb-2">SECURITY STATUS</div>
            <div className="text-3xl font-bold text-green-400 mb-1">Safe</div>
            <div className="text-xs text-gray-500">All locations verified</div>
          </div>
        </div>

        {/* Locations Grid */}
        <div className="grid grid-cols-1 gap-4 mb-8">
          {locations.map((location, idx) => (
            <div key={idx} className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg overflow-hidden">
              <div className="grid grid-cols-3 gap-4 items-start p-6">
                {/* Location Info */}
                <div>
                  <h3 className="text-white font-bold text-lg mb-1">{location.city}</h3>
                  <p className="text-gray-400 text-sm mb-3">{location.address}</p>
                  <p className="text-gray-500 text-xs mb-2">
                    <span className="text-cyan-400 font-semibold">{location.country}</span>
                  </p>
                  <p className="text-gray-500 text-xs">ISP: {location.isp}</p>
                </div>

                {/* Coordinates */}
                <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                  <div className="flex items-start gap-2 mb-3">
                    <Navigation className="w-4 h-4 text-cyan-400 mt-0.5 flex-shrink-0" />
                    <div>
                      <p className="text-gray-400 text-xs">Latitude</p>
                      <p className="text-white font-mono text-sm">{location.latitude.toFixed(4)}°</p>
                    </div>
                  </div>
                  <div className="flex items-start gap-2">
                    <Navigation className="w-4 h-4 text-blue-400 mt-0.5 flex-shrink-0" />
                    <div>
                      <p className="text-gray-400 text-xs">Longitude</p>
                      <p className="text-white font-mono text-sm">{location.longitude.toFixed(4)}°</p>
                    </div>
                  </div>
                </div>

                {/* Activity & Risk */}
                <div className="flex flex-col items-end justify-between">
                  <div className={`flex items-center gap-2 px-3 py-2 rounded border text-sm font-semibold ${getRiskColor(location.risk)}`}>
                    {getRiskIcon(location.risk)}
                    {location.risk === 'low' ? 'Safe' : location.risk === 'medium' ? 'Monitor' : 'Alert'}
                  </div>
                  <div className="text-right mt-4">
                    <p className="text-gray-500 text-xs">Last Activity</p>
                    <p className="text-white font-semibold">{location.lastSeen}</p>
                  </div>
                </div>
              </div>
            </div>
          ))}
        </div>

        {/* World Map Placeholder */}
        <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
          <h2 className="text-white font-bold mb-4 flex items-center gap-2">
            <Map className="w-5 h-5" /> World Map
          </h2>
          <div className="aspect-video bg-[#0a0a0f] border border-[#2a2a35] rounded-lg flex items-center justify-center text-gray-500">
            <div className="text-center">
              <Map className="w-12 h-12 mx-auto mb-2 opacity-50" />
              <p>Interactive map view (requires API integration)</p>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default GeoLocationPanel;
