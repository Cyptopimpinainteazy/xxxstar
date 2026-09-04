import React, { useState } from 'react';
import { TrendingUp, BarChart3, Link2, Lock, AlertCircle, CheckCircle } from 'lucide-react';

interface OracleProvider {
  id: string;
  name: string;
  networkCoverage: number;
  dataPoints: number;
  latency: number;
  reliability: number;
  status: 'active' | 'degraded' | 'offline';
  priceFeeds: number;
  volumeProcessed: number;
}

interface PriceFeed {
  id: string;
  asset: string;
  currentPrice: number;
  change24h: number;
  source: string;
  updateFrequency: string;
  dataPoints: number;
  lastUpdate: number;
  confidence: number;
}

interface TwapAggregation {
  asset: string;
  windowSize: string;
  twapPrice: number;
  volatility: number;
  dataPoints: number;
  status: 'active' | 'stale';
}

interface AmmLiquidity {
  dex: string;
  pair: string;
  liquidityUsd: number;
  volume24h: number;
  priceImpact: number;
  slippage: number;
}

export const PriceOraclePanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'providers' | 'feeds' | 'twap' | 'liquidity'>('providers');

  const [oracleProviders] = useState<OracleProvider[]>([
    {
      id: 'pyth',
      name: 'Pyth Network Oracle',
      networkCoverage: 98.5,
      dataPoints: 450,
      latency: 380,
      reliability: 99.8,
      status: 'active',
      priceFeeds: 250,
      volumeProcessed: 12450000,
    },
    {
      id: 'chainlink',
      name: 'Chainlink VRF',
      networkCoverage: 97.2,
      dataPoints: 320,
      latency: 420,
      reliability: 99.9,
      status: 'active',
      priceFeeds: 200,
      volumeProcessed: 8940000,
    },
    {
      id: 'band',
      name: 'Band Protocol',
      networkCoverage: 85.3,
      dataPoints: 180,
      latency: 550,
      reliability: 99.1,
      status: 'active',
      priceFeeds: 100,
      volumeProcessed: 3210000,
    },
  ]);

  const [priceFeeds] = useState<PriceFeed[]>([
    {
      id: 'btc-usd',
      asset: 'Bitcoin',
      currentPrice: 42850.50,
      change24h: 3.5,
      source: 'Pyth + Chainlink Consensus',
      updateFrequency: '500ms',
      dataPoints: 45,
      lastUpdate: Date.now() - 250,
      confidence: 99.7,
    },
    {
      id: 'eth-usd',
      asset: 'Ethereum',
      currentPrice: 2450.75,
      change24h: 2.8,
      source: 'Pyth + Band Protocol',
      updateFrequency: '480ms',
      dataPoints: 38,
      lastUpdate: Date.now() - 320,
      confidence: 99.5,
    },
    {
      id: 'x3-usd',
      asset: 'X3 Network Token',
      currentPrice: 12.45,
      change24h: 5.2,
      source: 'Pyth Primary',
      updateFrequency: '600ms',
      dataPoints: 28,
      lastUpdate: Date.now() - 150,
      confidence: 98.9,
    },
    {
      id: 'sol-usd',
      asset: 'Solana',
      currentPrice: 98.30,
      change24h: 4.1,
      source: 'Chainlink + Band',
      updateFrequency: '540ms',
      dataPoints: 32,
      lastUpdate: Date.now() - 410,
      confidence: 99.3,
    },
  ]);

  const [twapData] = useState<TwapAggregation[]>([
    {
      asset: 'BTC/USD',
      windowSize: '1h',
      twapPrice: 42750.25,
      volatility: 2.3,
      dataPoints: 7200,
      status: 'active',
    },
    {
      asset: 'ETH/USD',
      windowSize: '1h',
      twapPrice: 2425.50,
      volatility: 1.8,
      dataPoints: 7200,
      status: 'active',
    },
    {
      asset: 'X3/USD',
      windowSize: '15m',
      twapPrice: 12.42,
      volatility: 3.5,
      dataPoints: 1800,
      status: 'active',
    },
  ]);

  const [ammLiquidity] = useState<AmmLiquidity[]>([
    {
      dex: 'Uniswap V3',
      pair: 'ETH/USDC',
      liquidityUsd: 8450000,
      volume24h: 3240000,
      priceImpact: 0.15,
      slippage: 0.08,
    },
    {
      dex: 'Curve Finance',
      pair: 'USDC/USDT',
      liquidityUsd: 12500000,
      volume24h: 5680000,
      priceImpact: 0.04,
      slippage: 0.02,
    },
    {
      dex: 'Balancer',
      pair: 'X3/USDC',
      liquidityUsd: 2840000,
      volume24h: 1250000,
      priceImpact: 0.32,
      slippage: 0.18,
    },
  ]);

  const avgReliability =
    (oracleProviders.reduce((sum, p) => sum + p.reliability, 0) / oracleProviders.length).toFixed(1);
  const totalPriceFeeds = oracleProviders.reduce((sum, p) => sum + p.priceFeeds, 0);
  const totalVolume = oracleProviders.reduce((sum, p) => sum + p.volumeProcessed, 0);

  return (
    <div className="min-h-screen bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f] p-6">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between mb-8">
          <div>
            <h1 className="text-4xl font-bold text-transparent bg-clip-text bg-gradient-to-r from-teal-400 to-cyan-500 mb-2">
              Price Oracle
            </h1>
            <p className="text-gray-400">Pyth • Chainlink • TWAP • AMM Liquidity Providers</p>
          </div>
          <TrendingUp className="w-12 h-12 text-teal-400" />
        </div>

        {/* KPI Grid */}
        <div className="grid grid-cols-4 gap-4 mb-8">
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Price Feeds</div>
            <div className="text-2xl font-bold text-teal-400">{totalPriceFeeds}</div>
            <div className="text-xs text-gray-500 mt-2">Active across all providers</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Network Reliability</div>
            <div className="text-2xl font-bold text-green-400">{avgReliability}%</div>
            <div className="text-xs text-gray-500 mt-2">Consensus threshold: 99.1%</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Volume Processed</div>
            <div className="text-2xl font-bold text-blue-400">${(totalVolume / 1000000).toFixed(1)}M</div>
            <div className="text-xs text-gray-500 mt-2">Price updates/day</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Data Latency</div>
            <div className="text-2xl font-bold text-purple-400">380ms</div>
            <div className="text-xs text-gray-500 mt-2">Average publish delay</div>
          </div>
        </div>

        {/* Tabs */}
        <div className="flex gap-2 mb-6 border-b border-[#2a2a35]">
          {(['providers', 'feeds', 'twap', 'liquidity'] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={`px-4 py-3 font-semibold transition-colors ${
                activeTab === tab
                  ? 'text-teal-400 border-b-2 border-teal-400'
                  : 'text-gray-400 hover:text-gray-300'
              }`}
            >
              {tab === 'providers' && 'Oracle Providers'}
              {tab === 'feeds' && 'Price Feeds'}
              {tab === 'twap' && 'TWAP Aggregation'}
              {tab === 'liquidity' && 'AMM Liquidity'}
            </button>
          ))}
        </div>

        {/* Content */}
        <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
          {activeTab === 'providers' && (
            <div className="space-y-4">
              <h3 className="text-lg font-semibold text-white mb-4">Oracle Providers</h3>
              {oracleProviders.map((provider) => (
                <div key={provider.id} className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                  <div className="flex items-start justify-between mb-3">
                    <div>
                      <h4 className="text-white font-semibold">{provider.name}</h4>
                      <p className="text-xs text-gray-500">Oracle ID: {provider.id.toUpperCase()}</p>
                    </div>
                    <div
                      className={`px-3 py-1 rounded-full text-xs font-semibold ${
                        provider.status === 'active'
                          ? 'bg-green-500/20 text-green-400'
                          : 'bg-yellow-500/20 text-yellow-400'
                      }`}
                    >
                      {provider.status.toUpperCase()}
                    </div>
                  </div>
                  <div className="grid grid-cols-6 gap-4 text-sm">
                    <div>
                      <div className="text-gray-400">Network Coverage</div>
                      <div className="text-white font-semibold">{provider.networkCoverage}%</div>
                    </div>
                    <div>
                      <div className="text-gray-400">Data Points</div>
                      <div className="text-white font-semibold">{provider.dataPoints}</div>
                    </div>
                    <div>
                      <div className="text-gray-400">Latency</div>
                      <div className="text-cyan-400 font-semibold">{provider.latency}ms</div>
                    </div>
                    <div>
                      <div className="text-gray-400">Reliability</div>
                      <div className="text-green-400 font-semibold">{provider.reliability}%</div>
                    </div>
                    <div>
                      <div className="text-gray-400">Price Feeds</div>
                      <div className="text-blue-400 font-semibold">{provider.priceFeeds}</div>
                    </div>
                    <div>
                      <div className="text-gray-400">Volume</div>
                      <div className="text-purple-400 font-semibold">
                        ${(provider.volumeProcessed / 1000000).toFixed(1)}M
                      </div>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}

          {activeTab === 'feeds' && (
            <div>
              <h3 className="text-lg font-semibold text-white mb-6">Active Price Feeds</h3>
              <div className="space-y-4">
                {priceFeeds.map((feed) => (
                  <div key={feed.id} className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                    <div className="flex items-start justify-between mb-3">
                      <div>
                        <h4 className="text-white font-semibold">{feed.asset}</h4>
                        <p className="text-sm text-gray-400">{feed.source}</p>
                      </div>
                      <div className="text-right">
                        <div className="text-2xl font-bold text-cyan-400">${feed.currentPrice.toLocaleString()}</div>
                        <div
                          className={`text-sm font-semibold ${
                            feed.change24h > 0 ? 'text-green-400' : 'text-red-400'
                          }`}
                        >
                          {feed.change24h > 0 ? '+' : ''}{feed.change24h}% (24h)
                        </div>
                      </div>
                    </div>
                    <div className="grid grid-cols-5 gap-4 text-sm">
                      <div>
                        <div className="text-gray-400">Update Frequency</div>
                        <div className="text-white font-semibold">{feed.updateFrequency}</div>
                      </div>
                      <div>
                        <div className="text-gray-400">Data Points</div>
                        <div className="text-white font-semibold">{feed.dataPoints}</div>
                      </div>
                      <div>
                        <div className="text-gray-400">Last Update</div>
                        <div className="text-white font-semibold">{feed.lastUpdate}ms ago</div>
                      </div>
                      <div>
                        <div className="text-gray-400">Confidence</div>
                        <div className="text-green-400 font-semibold">{feed.confidence}%</div>
                      </div>
                      <div>
                        <CheckCircle className="w-4 h-4 text-green-400" />
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {activeTab === 'twap' && (
            <div>
              <h3 className="text-lg font-semibold text-white mb-6">Time-Weighted Average Price (TWAP)</h3>
              <div className="space-y-4">
                {twapData.map((twap) => (
                  <div key={twap.asset} className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                    <div className="flex items-start justify-between mb-3">
                      <div>
                        <h4 className="text-white font-semibold">{twap.asset}</h4>
                        <p className="text-sm text-gray-400">Window: {twap.windowSize}</p>
                      </div>
                      <div
                        className={`px-3 py-1 rounded-full text-xs font-semibold ${
                          twap.status === 'active'
                            ? 'bg-green-500/20 text-green-400'
                            : 'bg-yellow-500/20 text-yellow-400'
                        }`}
                      >
                        {twap.status.toUpperCase()}
                      </div>
                    </div>
                    <div className="grid grid-cols-4 gap-4 text-sm">
                      <div>
                        <div className="text-gray-400">TWAP Price</div>
                        <div className="text-cyan-400 font-semibold">${twap.twapPrice.toLocaleString()}</div>
                      </div>
                      <div>
                        <div className="text-gray-400">Volatility</div>
                        <div className="text-white font-semibold">{twap.volatility}%</div>
                      </div>
                      <div>
                        <div className="text-gray-400">Data Points</div>
                        <div className="text-white font-semibold">{twap.dataPoints.toLocaleString()}</div>
                      </div>
                      <div>
                        <div className="text-gray-400">Confidence</div>
                        <div className="text-green-400 font-semibold">99.4%</div>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {activeTab === 'liquidity' && (
            <div>
              <h3 className="text-lg font-semibold text-white mb-6">AMM Liquidity Providers</h3>
              <div className="space-y-4">
                {ammLiquidity.map((amm) => (
                  <div key={amm.pair} className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                    <div className="flex items-start justify-between mb-3">
                      <div>
                        <h4 className="text-white font-semibold">{amm.dex}</h4>
                        <p className="text-sm text-gray-400">{amm.pair}</p>
                      </div>
                    </div>
                    <div className="grid grid-cols-5 gap-4 text-sm mb-3">
                      <div>
                        <div className="text-gray-400">Liquidity</div>
                        <div className="text-cyan-400 font-semibold">${(amm.liquidityUsd / 1000000).toFixed(1)}M</div>
                      </div>
                      <div>
                        <div className="text-gray-400">Volume (24h)</div>
                        <div className="text-white font-semibold">${(amm.volume24h / 1000000).toFixed(1)}M</div>
                      </div>
                      <div>
                        <div className="text-gray-400">Price Impact</div>
                        <div className={amm.priceImpact < 0.2 ? 'text-green-400 font-semibold' : 'text-yellow-400 font-semibold'}>
                          {amm.priceImpact}%
                        </div>
                      </div>
                      <div>
                        <div className="text-gray-400">Slippage</div>
                        <div className={amm.slippage < 0.1 ? 'text-green-400 font-semibold' : 'text-yellow-400 font-semibold'}>
                          {amm.slippage}%
                        </div>
                      </div>
                      <div>
                        <div className="text-gray-400">L/V Ratio</div>
                        <div className="text-blue-400 font-semibold">
                          {((amm.liquidityUsd / amm.volume24h) * 100).toFixed(1)}%
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

export default PriceOraclePanel;
