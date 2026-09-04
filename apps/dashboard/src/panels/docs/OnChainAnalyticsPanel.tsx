import React, { useState } from 'react';
import { BarChart3, TrendingUp, Zap, Users, Coins, Activity, Download, Filter } from 'lucide-react';

interface ChainMetric {
  label: string;
  value: string | number;
  change: number;
  timeframe: string;
}

interface SmartContractActivity {
  id: string;
  name: string;
  calls: number;
  gasUsed: number;
  volume: number;
  topMethod: string;
}

interface TokenHolder {
  address: string;
  balance: number;
  percentage: number;
}

export const OnChainAnalyticsPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'overview' | 'smart-contracts' | 'token-distribution' | 'gas'>('overview');
  const [timeframe, setTimeframe] = useState<'24h' | '7d' | '30d' | 'all'>('24h');

  const metrics: ChainMetric[] = [
    { label: 'TVL', value: '$2.84B', change: 5.2, timeframe: '24h' },
    { label: 'Daily Volume', value: '$542M', change: 12.3, timeframe: '24h' },
    { label: 'Transactions', value: '2.4M', change: -1.2, timeframe: '24h' },
    { label: 'Active Addresses', value: '128.5K', change: 8.7, timeframe: '7d' },
    { label: 'Avg Gas Price', value: '12.5 Gwei', change: -2.1, timeframe: '24h' },
    { label: 'Unique Swaps', value: '45.3K', change: 15.4, timeframe: '24h' },
  ];

  const smartContracts: SmartContractActivity[] = [
    {
      id: 'dex-pool',
      name: 'AMM Liquidity Pool',
      calls: 12450,
      gasUsed: 2840000,
      volume: 542000000,
      topMethod: 'swap()',
    },
    {
      id: 'staking-contract',
      name: 'Validator Staking',
      calls: 8320,
      gasUsed: 1562000,
      volume: 85000000,
      topMethod: 'stake()',
    },
    {
      id: 'nft-marketplace',
      name: 'NFT Marketplace',
      calls: 3240,
      gasUsed: 765000,
      volume: 125000,
      topMethod: 'transferFrom()',
    },
    {
      id: 'lending-protocol',
      name: 'Lending Protocol',
      calls: 6780,
      gasUsed: 1204000,
      volume: 280000000,
      topMethod: 'deposit()',
    },
  ];

  const tokenHolders: TokenHolder[] = [
    { address: '0x742d35Cc6634C0532925a3b844Bc500e74d...', balance: 12500000, percentage: 12.5 },
    { address: '0x8ba1f109551bD432803012645Ac136ddd64...', balance: 8750000, percentage: 8.8 },
    { address: '0x9f4e2E4D8C1A6F8E2b9D5C3A7E1F4D9...', balance: 7250000, percentage: 7.3 },
    { address: '0x2cD05e48c0dA47F8f2D5E0B8c1F7A9E...', balance: 6100000, percentage: 6.1 },
    { address: '0x5aB7E1F4D9C2E0B8f3A5C7d1e9F2B4...', balance: 5320000, percentage: 5.3 },
  ];

  return (
    <div className="h-full flex flex-col bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f]">
      {/* Header */}
      <div className="border-b border-[#2a2a35] p-4">
        <div className="flex items-center gap-3 mb-4">
          <div className="p-2 bg-gradient-to-br from-cyan-500 to-blue-500 rounded-lg">
            <BarChart3 className="w-5 h-5 text-white" />
          </div>
          <div>
            <h1 className="text-lg font-semibold text-white">On-Chain Analytics</h1>
            <p className="text-xs text-gray-400">Real-time TVL, volume, gas fees, smart contract activity</p>
          </div>
        </div>

        {/* Timeframe selector */}
        <div className="flex gap-2">
          {(['24h', '7d', '30d', 'all'] as const).map(tf => (
            <button
              key={tf}
              onClick={() => setTimeframe(tf)}
              className={`px-3 py-1 text-xs font-medium rounded transition ${
                timeframe === tf
                  ? 'bg-cyan-600 text-white'
                  : 'bg-[#1a1a2e] text-gray-400 hover:text-gray-200'
              }`}
            >
              {tf === 'all' ? 'All Time' : tf.toUpperCase()}
            </button>
          ))}
        </div>
      </div>

      {/* Tabs */}
      <div className="flex gap-2 px-4 pt-4 border-b border-[#2a2a35]">
        {(['overview', 'smart-contracts', 'token-distribution', 'gas'] as const).map(tab => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab)}
            className={`px-4 py-2 rounded-lg font-medium text-sm transition ${
              activeTab === tab
                ? 'bg-cyan-600 text-white'
                : 'text-gray-400 hover:text-gray-200'
            }`}
          >
            {tab === 'overview' && '📊 Overview'}
            {tab === 'smart-contracts' && '⚙️ Smart Contracts'}
            {tab === 'token-distribution' && '🪙 Token Distribution'}
            {tab === 'gas' && '⛽ Gas Fees'}
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        {activeTab === 'overview' && (
          <div className="p-4 space-y-3">
            <div className="grid grid-cols-2 gap-3">
              {metrics.map((metric, idx) => (
                <div key={idx} className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4 hover:border-cyan-500/50 transition">
                  <p className="text-xs text-gray-500 mb-2">{metric.label}</p>
                  <p className="text-xl font-bold text-white mb-1">{metric.value}</p>
                  <div className="flex items-center gap-1">
                    {metric.change > 0 ? (
                      <>
                        <TrendingUp size={12} className="text-green-400" />
                        <span className="text-xs text-green-400">{metric.change > 0 ? '+' : ''}{metric.change}%</span>
                      </>
                    ) : (
                      <>
                        <TrendingUp size={12} className="text-red-400 rotate-180" />
                        <span className="text-xs text-red-400">{metric.change}%</span>
                      </>
                    )}
                    <span className="text-xs text-gray-600 ml-auto">{metric.timeframe}</span>
                  </div>
                </div>
              ))}
            </div>

            <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
              <h3 className="font-semibold text-white mb-3">Network Health</h3>
              <div className="space-y-2">
                <div>
                  <div className="flex justify-between mb-1 text-xs">
                    <span className="text-gray-400">Block Time Consistency</span>
                    <span className="text-white font-semibold">99.8%</span>
                  </div>
                  <div className="w-full h-2 bg-[#0a0a0f] rounded-full overflow-hidden">
                    <div className="h-2 bg-green-500" style={{ width: '99.8%' }} />
                  </div>
                </div>
                <div>
                  <div className="flex justify-between mb-1 text-xs">
                    <span className="text-gray-400">Validator Uptime</span>
                    <span className="text-white font-semibold">98.2%</span>
                  </div>
                  <div className="w-full h-2 bg-[#0a0a0f] rounded-full overflow-hidden">
                    <div className="h-2 bg-yellow-500" style={{ width: '98.2%' }} />
                  </div>
                </div>
                <div>
                  <div className="flex justify-between mb-1 text-xs">
                    <span className="text-gray-400">Block Finality</span>
                    <span className="text-white font-semibold">100%</span>
                  </div>
                  <div className="w-full h-2 bg-[#0a0a0f] rounded-full overflow-hidden">
                    <div className="h-2 bg-green-500" style={{ width: '100%' }} />
                  </div>
                </div>
              </div>
            </div>
          </div>
        )}

        {activeTab === 'smart-contracts' && (
          <div className="p-4 space-y-3">
            {smartContracts.map(contract => (
              <div key={contract.id} className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4 hover:border-cyan-500/50 transition">
                <div className="flex items-start justify-between mb-2">
                  <div>
                    <h3 className="font-semibold text-white">{contract.name}</h3>
                    <p className="text-xs text-gray-500 mt-1">Top method: <code className="text-cyan-400">{contract.topMethod}</code></p>
                  </div>
                </div>

                <div className="grid grid-cols-3 gap-2 mb-2 text-xs">
                  <div className="bg-[#0a0a0f] rounded p-2 border border-[#2a2a35]">
                    <p className="text-gray-500 mb-1">Calls</p>
                    <p className="font-semibold text-white">{contract.calls.toLocaleString()}</p>
                  </div>
                  <div className="bg-[#0a0a0f] rounded p-2 border border-[#2a2a35]">
                    <p className="text-gray-500 mb-1">Gas Used</p>
                    <p className="font-semibold text-white">{(contract.gasUsed / 1000000).toFixed(2)}M</p>
                  </div>
                  <div className="bg-[#0a0a0f] rounded p-2 border border-[#2a2a35]">
                    <p className="text-gray-500 mb-1">Volume</p>
                    <p className="font-semibold text-cyan-400">${(contract.volume / 1000000).toFixed(1)}M</p>
                  </div>
                </div>

                <button className="w-full px-3 py-1.5 text-xs bg-cyan-600 hover:bg-cyan-700 text-white rounded transition font-medium">
                  View Contract Details
                </button>
              </div>
            ))}
          </div>
        )}

        {activeTab === 'token-distribution' && (
          <div className="p-4 space-y-3">
            <div className="flex items-center justify-between mb-4">
              <h3 className="font-semibold text-white">Top Token Holders</h3>
              <button className="text-xs px-2 py-1 bg-[#1a1a2e] border border-[#2a2a35] rounded text-gray-400 hover:text-gray-200 transition">
                <Download size={14} className="inline mr-1" />
                Export
              </button>
            </div>

            {tokenHolders.map((holder, idx) => (
              <div key={idx} className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4 hover:border-cyan-500/50 transition">
                <div className="flex items-start justify-between mb-2">
                  <div className="flex-1">
                    <p className="text-xs text-gray-400 font-mono mb-1">{holder.address}</p>
                    <p className="text-sm font-semibold text-white">{(holder.balance / 1000000).toFixed(2)}M X3</p>
                  </div>
                  <span className="text-sm font-bold text-cyan-400">{holder.percentage.toFixed(1)}%</span>
                </div>
                <div className="w-full h-2 bg-[#0a0a0f] rounded-full overflow-hidden">
                  <div
                    className="h-2 bg-gradient-to-r from-cyan-500 to-blue-500"
                    style={{ width: `${holder.percentage}%` }}
                  />
                </div>
              </div>
            ))}
          </div>
        )}

        {activeTab === 'gas' && (
          <div className="p-4 space-y-3">
            <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
              <h3 className="font-semibold text-white mb-3">Gas Fee Trends</h3>
              <div className="space-y-3">
                <div>
                  <div className="flex justify-between mb-1 text-xs">
                    <span className="text-gray-400">Current Avg</span>
                    <span className="text-white font-semibold">12.5 Gwei</span>
                  </div>
                  <div className="w-full h-2 bg-[#0a0a0f] rounded-full overflow-hidden">
                    <div className="h-2 bg-blue-500" style={{ width: '35%' }} />
                  </div>
                </div>
                <div>
                  <div className="flex justify-between mb-1 text-xs">
                    <span className="text-gray-400">Peak (24h)</span>
                    <span className="text-white font-semibold">24.2 Gwei</span>
                  </div>
                  <div className="w-full h-2 bg-[#0a0a0f] rounded-full overflow-hidden">
                    <div className="h-2 bg-red-500" style={{ width: '68%' }} />
                  </div>
                </div>
                <div>
                  <div className="flex justify-between mb-1 text-xs">
                    <span className="text-gray-400">Low (24h)</span>
                    <span className="text-white font-semibold">8.1 Gwei</span>
                  </div>
                  <div className="w-full h-2 bg-[#0a0a0f] rounded-full overflow-hidden">
                    <div className="h-2 bg-green-500" style={{ width: '23%' }} />
                  </div>
                </div>
              </div>
            </div>

            <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
              <h3 className="font-semibold text-white mb-3">Gas Used by Category</h3>
              <div className="space-y-2 text-xs">
                <div className="flex items-center justify-between">
                  <span className="text-gray-400">DEX Swaps</span>
                  <div className="flex items-center gap-2">
                    <div className="w-24 h-2 bg-[#0a0a0f] rounded-full"><div className="h-2 bg-blue-500" style={{ width: '45%' }} /></div>
                    <span className="w-20 text-right text-gray-300">45%</span>
                  </div>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-gray-400">Lending</span>
                  <div className="flex items-center gap-2">
                    <div className="w-24 h-2 bg-[#0a0a0f] rounded-full"><div className="h-2 bg-purple-500" style={{ width: '28%' }} /></div>
                    <span className="w-20 text-right text-gray-300">28%</span>
                  </div>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-gray-400">NFT Transfers</span>
                  <div className="flex items-center gap-2">
                    <div className="w-24 h-2 bg-[#0a0a0f] rounded-full"><div className="h-2 bg-pink-500" style={{ width: '15%' }} /></div>
                    <span className="w-20 text-right text-gray-300">15%</span>
                  </div>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-gray-400">Staking</span>
                  <div className="flex items-center gap-2">
                    <div className="w-24 h-2 bg-[#0a0a0f] rounded-full"><div className="h-2 bg-yellow-500" style={{ width: '12%' }} /></div>
                    <span className="w-20 text-right text-gray-300">12%</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default OnChainAnalyticsPanel;
