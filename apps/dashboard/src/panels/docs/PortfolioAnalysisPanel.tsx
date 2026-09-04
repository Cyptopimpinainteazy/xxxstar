import React, { useState } from 'react';
import { TrendingUp, PieChart, Activity, ArrowUpRight, ArrowDownLeft } from 'lucide-react';

interface PortfolioAsset {
  symbol: string;
  name: string;
  amount: number;
  price: number;
  value: number;
  change24h: number;
  allocation: number;
}

export const PortfolioAnalysisPanel: React.FC = () => {
  const [assets] = useState<PortfolioAsset[]>([
    {
      symbol: 'ETH',
      name: 'Ethereum',
      amount: 15.5,
      price: 2850.25,
      value: 44177.88,
      change24h: 3.45,
      allocation: 35,
    },
    {
      symbol: 'BTC',
      name: 'Bitcoin',
      amount: 0.85,
      price: 42500.0,
      value: 36125.0,
      change24h: -2.15,
      allocation: 29,
    },
    {
      symbol: 'SOL',
      name: 'Solana',
      amount: 250,
      price: 95.50,
      value: 23875.0,
      change24h: 8.2,
      allocation: 19,
    },
    {
      symbol: 'USDC',
      name: 'USDC Stablecoin',
      amount: 20000,
      price: 1.0,
      value: 20000.0,
      change24h: 0.0,
      allocation: 16,
    },
  ]);

  const totalValue = assets.reduce((sum, asset) => sum + asset.value, 0);
  const totalChange = assets.reduce((sum, asset) => sum + asset.value * (asset.change24h / 100), 0);
  const totalChangePercent = (totalChange / totalValue) * 100;

  return (
    <div className="min-h-screen bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f] p-6">
      <div className="max-w-6xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between mb-8">
          <div>
            <h1 className="text-4xl font-bold text-transparent bg-clip-text bg-gradient-to-r from-cyan-400 to-blue-500 mb-2">
              Portfolio Analysis
            </h1>
            <p className="text-gray-400">Real-time asset allocation and performance tracking</p>
          </div>
          <TrendingUp className="w-12 h-12 text-cyan-400" />
        </div>

        {/* Portfolio Summary */}
        <div className="grid grid-cols-3 gap-4 mb-8">
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
            <div className="text-gray-400 text-sm font-semibold mb-2">TOTAL VALUE</div>
            <div className="text-4xl font-bold text-cyan-400 mb-2">
              ${totalValue.toLocaleString('en-US', { maximumFractionDigits: 2 })}
            </div>
            <div className={`text-sm font-semibold flex items-center gap-1 ${totalChangePercent >= 0 ? 'text-green-400' : 'text-red-400'}`}>
              {totalChangePercent >= 0 ? <ArrowUpRight className="w-4 h-4" /> : <ArrowDownLeft className="w-4 h-4" />}
              {Math.abs(totalChangePercent).toFixed(2)}% (${Math.abs(totalChange).toLocaleString('en-US', { maximumFractionDigits: 2 })})
            </div>
          </div>

          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
            <div className="text-gray-400 text-sm font-semibold mb-2">ASSETS</div>
            <div className="text-4xl font-bold text-blue-400 mb-2">{assets.length}</div>
            <div className="text-sm text-gray-500">Diversified across chains</div>
          </div>

          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
            <div className="text-gray-400 text-sm font-semibold mb-2">24H ACTIVITY</div>
            <div className="text-4xl font-bold text-teal-400 mb-2">
              {assets.filter((a) => Math.abs(a.change24h) > 0).length}/{assets.length}
            </div>
            <div className="text-sm text-gray-500">Assets with movement</div>
          </div>
        </div>

        <div className="grid grid-cols-3 gap-6">
          {/* Asset List */}
          <div className="col-span-2 bg-[#1a1a2e] border border-[#2a2a35] rounded-lg overflow-hidden">
            <div className="bg-[#0a0a0f] border-b border-[#2a2a35] px-6 py-4">
              <h2 className="text-white font-bold">Holdings</h2>
            </div>
            <div className="divide-y divide-[#2a2a35]">
              {assets.map((asset) => (
                <div key={asset.symbol} className="p-4 hover:bg-[#0a0a0f]/50 transition">
                  <div className="flex items-start justify-between mb-3">
                    <div>
                      <h3 className="text-white font-bold">{asset.symbol}</h3>
                      <p className="text-gray-400 text-sm">{asset.name}</p>
                    </div>
                    <div className="text-right">
                      <div className="text-white font-bold">
                        ${asset.value.toLocaleString('en-US', { maximumFractionDigits: 2 })}
                      </div>
                      <div className={`text-sm font-semibold flex items-center justify-end gap-1 ${
                        asset.change24h >= 0 ? 'text-green-400' : 'text-red-400'
                      }`}>
                        {asset.change24h >= 0 ? <ArrowUpRight className="w-3 h-3" /> : <ArrowDownLeft className="w-3 h-3" />}
                        {Math.abs(asset.change24h).toFixed(2)}%
                      </div>
                    </div>
                  </div>

                  <div className="grid grid-cols-3 gap-4 text-xs">
                    <div>
                      <span className="text-gray-500">Amount</span>
                      <p className="text-cyan-400 font-semibold">{asset.amount} {asset.symbol}</p>
                    </div>
                    <div>
                      <span className="text-gray-500">Price</span>
                      <p className="text-blue-400 font-semibold">${asset.price.toLocaleString()}</p>
                    </div>
                    <div>
                      <span className="text-gray-500">Allocation</span>
                      <p className="text-teal-400 font-semibold">{asset.allocation}%</p>
                    </div>
                  </div>

                  <div className="mt-3 bg-[#0a0a0f] rounded-full h-2 overflow-hidden">
                    <div
                      className="h-full bg-gradient-to-r from-cyan-500 to-blue-500"
                      style={{ width: `${asset.allocation}%` }}
                    />
                  </div>
                </div>
              ))}
            </div>
          </div>

          {/* Allocation Pie Chart */}
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
            <h2 className="text-white font-bold mb-6 flex items-center gap-2">
              <PieChart className="w-5 h-5" /> Asset Allocation
            </h2>

            <div className="space-y-3">
              {assets.map((asset) => (
                <div key={asset.symbol}>
                  <div className="flex justify-between items-center mb-1">
                    <span className="text-gray-400 text-sm">{asset.symbol}</span>
                    <span className="text-white font-semibold">{asset.allocation}%</span>
                  </div>
                  <div className="bg-[#0a0a0f] rounded-full h-2 overflow-hidden">
                    <div
                      className="h-full bg-gradient-to-r from-cyan-500 to-blue-500"
                      style={{ width: `${asset.allocation}%` }}
                    />
                  </div>
                </div>
              ))}
            </div>

            <div className="mt-6 pt-6 border-t border-[#2a2a35]">
              <div className="text-center">
                <div className="text-gray-400 text-xs mb-2">Rebalance Status</div>
                <div className="inline-flex items-center gap-2 bg-green-500/10 border border-green-500/20 rounded-full px-3 py-1">
                  <Activity className="w-3 h-3 text-green-400" />
                  <span className="text-green-400 text-xs font-semibold">Balanced</span>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default PortfolioAnalysisPanel;
