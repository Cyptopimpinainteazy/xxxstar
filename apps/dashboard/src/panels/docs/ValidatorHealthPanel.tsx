import React, { useState, useEffect } from 'react';
import { TrendingUp, TrendingDown, AlertTriangle, CheckCircle } from 'lucide-react';

interface ValidatorMetrics {
  uptime: number;
  signedBlocks: number;
  missedBlocks: number;
  totalEligible: number;
  slashingRisk: number;
  lastVoteTime: string;
}

export const ValidatorHealthPanelComponent: React.FC = () => {
  const [metrics, setMetrics] = useState<ValidatorMetrics>({
    uptime: 99.87,
    signedBlocks: 45230,
    missedBlocks: 23,
    totalEligible: 45253,
    slashingRisk: 0.12,
    lastVoteTime: '2 seconds ago',
  });

  const signRate = (metrics.signedBlocks / metrics.totalEligible) * 100;

  useEffect(() => {
    const interval = setInterval(() => {
      setMetrics((prev) => ({
        ...prev,
        uptime: Math.min(99.95, prev.uptime + (Math.random() - 0.4) * 0.05),
        signedBlocks: prev.signedBlocks + (Math.random() > 0.1 ? 1 : 0),
        lastVoteTime: 'Just now',
      }));
    }, 3000);
    return () => clearInterval(interval);
  }, []);

  return (
    <div className="min-h-screen bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f] p-6">
      <div className="max-w-6xl mx-auto">
        {/* Header */}
        <div className="mb-8">
          <h1 className="text-4xl font-bold text-transparent bg-clip-text bg-gradient-to-r from-cyan-400 to-blue-500 mb-2">
            Validator Health
          </h1>
          <p className="text-gray-400">Real-time performance monitoring and slashing risk analysis</p>
        </div>

        {/* Primary Metrics Grid */}
        <div className="grid grid-cols-4 gap-4 mb-8">
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-xs font-semibold mb-2">UPTIME</div>
            <div className="text-3xl font-bold text-green-400 mb-1">{metrics.uptime.toFixed(2)}%</div>
            <div className="text-xs text-gray-500">Last 30 days</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-xs font-semibold mb-2">SIGNED BLOCKS</div>
            <div className="text-3xl font-bold text-blue-400 mb-1">{metrics.signedBlocks.toLocaleString()}</div>
            <div className="text-xs text-gray-500">Sign rate: {signRate.toFixed(2)}%</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-xs font-semibold mb-2">MISSED BLOCKS</div>
            <div className="text-3xl font-bold text-orange-400 mb-1">{metrics.missedBlocks}</div>
            <div className="text-xs text-gray-500">Out of {metrics.totalEligible.toLocaleString()}</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-xs font-semibold mb-2">SLASHING RISK</div>
            <div className="text-3xl font-bold text-red-400 mb-1">{metrics.slashingRisk.toFixed(2)}%</div>
            <div className="text-xs text-gray-500">Low risk</div>
          </div>
        </div>

        {/* Health Status Indicator */}
        <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6 mb-6">
          <div className="flex items-start justify-between">
            <div>
              <h2 className="text-xl font-bold text-white mb-3">System Status</h2>
              <div className="space-y-3">
                <div className="flex items-center gap-3">
                  <CheckCircle className="w-5 h-5 text-green-400" />
                  <div>
                    <p className="text-white font-semibold">All Systems Operational</p>
                    <p className="text-gray-400 text-sm">Last check: {metrics.lastVoteTime}</p>
                  </div>
                </div>
                <div className="flex items-center gap-3">
                  <CheckCircle className="w-5 h-5 text-green-400" />
                  <div>
                    <p className="text-white font-semibold">Network Connectivity</p>
                    <p className="text-gray-400 text-sm">2/2 peers active</p>
                  </div>
                </div>
                <div className="flex items-center gap-3">
                  <CheckCircle className="w-5 h-5 text-green-400" />
                  <div>
                    <p className="text-white font-semibold">Memory & Storage</p>
                    <p className="text-gray-400 text-sm">84GB / 256GB available</p>
                  </div>
                </div>
              </div>
            </div>
            <div className="text-right">
              <div className="inline-flex items-center gap-2 bg-green-500/10 border border-green-500/20 rounded-full px-4 py-2">
                <div className="w-2 h-2 bg-green-500 rounded-full animate-pulse" />
                <span className="text-green-400 font-semibold text-sm">Online</span>
              </div>
            </div>
          </div>
        </div>

        {/* Charts Grid */}
        <div className="grid grid-cols-2 gap-6">
          {/* Block Performance Chart */}
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
            <h3 className="text-white font-semibold mb-4">Block Performance (24h)</h3>
            <div className="space-y-3">
              {[...Array(4)].map((_, i) => (
                <div key={i} className="flex items-center gap-3">
                  <div className="w-16 text-xs text-gray-400">
                    {6 - i * 2}:00 UTC
                  </div>
                  <div className="flex-1 bg-[#0a0a0f] rounded-full h-2 overflow-hidden">
                    <div
                      className="h-full bg-gradient-to-r from-blue-500 to-cyan-400"
                      style={{ width: `${85 + Math.random() * 15}%` }}
                    />
                  </div>
                  <div className="w-10 text-right text-xs text-gray-400">
                    {(98 + Math.random() * 2).toFixed(1)}%
                  </div>
                </div>
              ))}
            </div>
          </div>

          {/* Slashing Events */}
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
            <h3 className="text-white font-semibold mb-4">Recent Activity</h3>
            <div className="space-y-3">
              <div className="flex items-start gap-3 pb-3 border-b border-[#2a2a35]">
                <TrendingUp className="w-4 h-4 text-green-400 mt-1 flex-shrink-0" />
                <div className="flex-1">
                  <p className="text-gray-300 text-sm">Earned 2.5 ETH in rewards</p>
                  <p className="text-gray-500 text-xs">2 hours ago</p>
                </div>
              </div>
              <div className="flex items-start gap-3 pb-3 border-b border-[#2a2a35]">
                <CheckCircle className="w-4 h-4 text-blue-400 mt-1 flex-shrink-0" />
                <div className="flex-1">
                  <p className="text-gray-300 text-sm">Voted on block 15432891</p>
                  <p className="text-gray-500 text-xs">Just now</p>
                </div>
              </div>
              <div className="flex items-start gap-3">
                <AlertTriangle className="w-4 h-4 text-yellow-400 mt-1 flex-shrink-0" />
                <div className="flex-1">
                  <p className="text-gray-300 text-sm">1 missed attestation (network delay)</p>
                  <p className="text-gray-500 text-xs">15 minutes ago</p>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default ValidatorHealthPanelComponent;
