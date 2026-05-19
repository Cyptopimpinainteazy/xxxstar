import React, { useState, useEffect } from 'react';
import { Activity, Globe, Zap, Clock, Users, TrendingUp } from 'lucide-react';

const bids = [
  { price: 48.52, amount: 1240, total: 60134 },
  { price: 48.50, amount: 3580, total: 173630 },
  { price: 48.48, amount: 890, total: 43147 },
  { price: 48.45, amount: 5200, total: 251940 },
  { price: 48.42, amount: 2100, total: 101682 },
  { price: 48.40, amount: 4300, total: 208120 },
  { price: 48.38, amount: 1750, total: 84665 },
  { price: 48.35, amount: 6100, total: 294935 },
];

const asks = [
  { price: 48.55, amount: 980, total: 47579 },
  { price: 48.58, amount: 2400, total: 116592 },
  { price: 48.60, amount: 1550, total: 75330 },
  { price: 48.63, amount: 4100, total: 199383 },
  { price: 48.65, amount: 3200, total: 155680 },
  { price: 48.68, amount: 1800, total: 87624 },
  { price: 48.70, amount: 5500, total: 267850 },
  { price: 48.75, amount: 2900, total: 141375 },
];

const maxBidTotal = Math.max(...bids.map(b => b.total));
const maxAskTotal = Math.max(...asks.map(a => a.total));

const chainHashes = [
  '0xa4f8e2d901bc34567890abcdef1234567890abcdef1234567890abcdef123456',
  '0x3b5c91e7f02ad8901234abcdef567890abcdef1234567890abcdef1234567890',
  '0xd7e2f8a4b916c3450789abcdef012345abcdef6789012345abcdef6789012345',
  '0x8c1a3f5e9d04b67823456789abcdef0123456789abcdef0123456789abcdef01',
  '0xf2b8e4c7a510d93467890abcdef123456789abcdef0123456789abcdef012345',
  '0x5e9a1c3f7b28d604890abcdef01234567890abcdef01234567890abcdef012345',
];

const QuantumEnhancedPanel: React.FC = () => {
  const [tick, setTick] = useState(0);

  useEffect(() => {
    const interval = setInterval(() => setTick(t => t + 1), 2000);
    return () => clearInterval(interval);
  }, []);

  const stats = [
    { label: 'Block Time', value: `${(6.0 + (tick % 3) * 0.1).toFixed(1)}s`, icon: <Clock size={14} />, trend: '↓ 0.2s' },
    { label: 'Finality', value: `${12 + (tick % 4)}s`, icon: <Zap size={14} />, trend: '→ stable' },
    { label: 'TPS', value: `${1842 + (tick % 100)}`, icon: <TrendingUp size={14} />, trend: '↑ 3.2%' },
    { label: 'Peer Count', value: `${248 + (tick % 5)}`, icon: <Users size={14} />, trend: '↑ 2' },
  ];

  return (
    <div className="flex flex-col h-full bg-[#0a0a0f] text-gray-300">
      <div className="flex items-center justify-between px-5 py-3 border-b border-[#1a1a1a]">
        <div className="flex items-center gap-2">
          <Activity size={18} className="text-[#ff6b35]" />
          <h1 className="text-lg font-semibold text-white">Quantum Enhanced Terminal</h1>
        </div>
        <div className="flex items-center gap-2">
          <div className="w-2 h-2 rounded-full bg-green-400 animate-pulse" />
          <span className="text-xs text-green-400">LIVE</span>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto p-5 space-y-5">
        {/* Validator Globe Placeholder */}
        <div className="flex justify-center">
          <div className="relative w-40 h-40">
            {/* Globe sphere */}
            <div className="absolute inset-0 rounded-full"
              style={{
                background: 'radial-gradient(circle at 35% 35%, #1a1a3a, #0a0a1a 60%, #050510)',
                boxShadow: '0 0 40px rgba(255,107,53,0.1), inset 0 0 30px rgba(255,107,53,0.05)',
              }}
            />
            {/* Grid lines */}
            <div className="absolute inset-2 rounded-full border border-[#ff6b35]/10" />
            <div className="absolute inset-6 rounded-full border border-[#ff6b35]/10" />
            <div className="absolute inset-0 flex items-center justify-center">
              <div className="w-full h-px bg-[#ff6b35]/10" />
            </div>
            <div className="absolute inset-0 flex items-center justify-center">
              <div className="h-full w-px bg-[#ff6b35]/10" />
            </div>
            {/* Orbiting dots */}
            {[0, 1, 2, 3, 4].map(i => {
              const angle = ((tick * 15 + i * 72) % 360) * (Math.PI / 180);
              const rx = 55 + (i % 2) * 10;
              const ry = 45 + (i % 3) * 8;
              const x = 80 + Math.cos(angle) * rx;
              const y = 80 + Math.sin(angle) * ry;
              return (
                <div
                  key={i}
                  className="absolute w-2 h-2 rounded-full bg-[#ff6b35] transition-all duration-1000"
                  style={{
                    left: `${x}px`,
                    top: `${y}px`,
                    opacity: 0.4 + (i * 0.12),
                    boxShadow: '0 0 6px rgba(255,107,53,0.6)',
                  }}
                />
              );
            })}
            <div className="absolute inset-0 flex items-center justify-center">
              <Globe size={24} className="text-[#ff6b35]/30" />
            </div>
          </div>
        </div>

        {/* Network Telemetry */}
        <div className="grid grid-cols-4 gap-3">
          {stats.map((s, i) => (
            <div key={i} className="p-3 bg-[#111118] border border-[#1a1a1a] rounded-lg">
              <div className="flex items-center gap-1.5 mb-1.5">
                <span className="text-[#ff6b35]">{s.icon}</span>
                <span className="text-xs text-gray-500">{s.label}</span>
              </div>
              <p className="text-lg font-bold text-white">{s.value}</p>
              <p className="text-xs text-gray-500 mt-0.5">{s.trend}</p>
              {/* Sparkline */}
              <div className="flex items-end gap-px mt-2 h-4">
                {Array.from({ length: 12 }).map((_, j) => {
                  const h = 20 + Math.sin((tick + j + i * 3) * 0.8) * 60 + Math.random() * 20;
                  return <div key={j} className="flex-1 bg-[#ff6b35]/30 rounded-sm" style={{ height: `${Math.max(10, h)}%` }} />;
                })}
              </div>
            </div>
          ))}
        </div>

        {/* Quantum Orderbook */}
        <div>
          <h3 className="text-sm font-semibold text-white mb-3">Quantum Orderbook — X3/USDC</h3>
          <div className="grid grid-cols-2 gap-3">
            {/* Bids */}
            <div className="bg-[#111118] border border-[#1a1a1a] rounded-lg overflow-hidden">
              <div className="grid grid-cols-3 px-3 py-2 text-xs text-gray-500 border-b border-[#1a1a1a]">
                <span>Price</span><span className="text-right">Amount</span><span className="text-right">Total</span>
              </div>
              {bids.map((b, i) => (
                <div key={i} className="relative grid grid-cols-3 px-3 py-1.5 text-xs">
                  <div className="absolute inset-0 bg-green-500/5" style={{ width: `${(b.total / maxBidTotal) * 100}%` }} />
                  <span className="relative text-green-400">{b.price.toFixed(2)}</span>
                  <span className="relative text-right text-gray-300">{b.amount.toLocaleString()}</span>
                  <span className="relative text-right text-gray-500">{b.total.toLocaleString()}</span>
                </div>
              ))}
            </div>
            {/* Asks */}
            <div className="bg-[#111118] border border-[#1a1a1a] rounded-lg overflow-hidden">
              <div className="grid grid-cols-3 px-3 py-2 text-xs text-gray-500 border-b border-[#1a1a1a]">
                <span>Price</span><span className="text-right">Amount</span><span className="text-right">Total</span>
              </div>
              {asks.map((a, i) => (
                <div key={i} className="relative grid grid-cols-3 px-3 py-1.5 text-xs">
                  <div className="absolute right-0 inset-y-0 bg-red-500/5" style={{ width: `${(a.total / maxAskTotal) * 100}%` }} />
                  <span className="relative text-red-400">{a.price.toFixed(2)}</span>
                  <span className="relative text-right text-gray-300">{a.amount.toLocaleString()}</span>
                  <span className="relative text-right text-gray-500">{a.total.toLocaleString()}</span>
                </div>
              ))}
            </div>
          </div>
        </div>

        {/* Chain State Hash */}
        <div>
          <h3 className="text-sm font-semibold text-white mb-2">Chain State Hashes</h3>
          <div className="bg-[#050508] border border-[#1a1a1a] rounded-lg p-3 overflow-hidden max-h-24">
            {chainHashes.map((h, i) => (
              <p key={i} className="text-[10px] font-mono text-gray-500 leading-relaxed truncate">
                <span className="text-gray-600">#{(1284520 - i).toLocaleString()}</span> {h}
              </p>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
};

export default QuantumEnhancedPanel;
