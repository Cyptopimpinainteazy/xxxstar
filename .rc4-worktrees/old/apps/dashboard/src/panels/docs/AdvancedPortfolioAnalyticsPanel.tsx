import React, { useState, useMemo } from 'react';
import { TrendingUp, TrendingDown, Activity, AlertTriangle, BarChart3, PieChart as PieChartIcon } from 'lucide-react';

interface Asset {
  symbol: string;
  name: string;
  amount: number;
  price: number;
  change24h: number;
  volatility: number;
  beta: number;
}

interface AnalyticsData {
  assets: Asset[];
  sharpeRatio: number;
  maxDrawdown: number;
  volatility: number;
  var95: number;
  var99: number;
  correlationMatrix: number[][];
  riskScore: number;
  totalValue: number;
  totalReturn: number;
}

export const AdvancedPortfolioAnalyticsPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'overview' | 'metrics' | 'correlation' | 'risks'>('overview');

  const analyticsData: AnalyticsData = useMemo(() => {
    const assets: Asset[] = [
      { symbol: 'ETH', name: 'Ethereum', amount: 12.5, price: 3245.32, change24h: 3.45, volatility: 2.8, beta: 1.2 },
      { symbol: 'BTC', name: 'Bitcoin', amount: 0.35, price: 92130.00, change24h: -2.15, volatility: 2.1, beta: 1.0 },
      { symbol: 'SOL', name: 'Solana', amount: 45.2, price: 184.50, change24h: 8.2, volatility: 3.5, beta: 1.45 },
      { symbol: 'USDC', name: 'USD Coin', amount: 25000, price: 1.00, change24h: 0.0, volatility: 0.05, beta: 0.0 },
    ];

    const totalValue = assets.reduce((sum, a) => sum + (a.amount * a.price), 0);
    const totalReturn = assets.reduce((sum, a) => sum + ((a.amount * a.price) * (a.change24h / 100)), 0);

    // Simplified calculations (in production, use real time-series data)
    const sharpeRatio = 2.34; // (return - risk_free) / std_dev
    const maxDrawdown = -18.5; // Max loss from peak
    const volatility = 2.15; // Annual volatility
    const var95 = totalValue * 0.084; // 8.4% loss at 95% confidence
    const var99 = totalValue * 0.142; // 14.2% loss at 99% confidence
    const riskScore = 6.8; // 1-10 scale

    // Correlation matrix (simplified)
    const correlationMatrix = [
      [1.0, 0.72, 0.65, 0.05],  // ETH
      [0.72, 1.0, 0.58, 0.03],  // BTC
      [0.65, 0.58, 1.0, 0.08],  // SOL
      [0.05, 0.03, 0.08, 1.0],  // USDC
    ];

    return {
      assets,
      sharpeRatio,
      maxDrawdown,
      volatility,
      var95,
      var99,
      correlationMatrix,
      riskScore,
      totalValue,
      totalReturn,
    };
  }, []);

  const getRiskColor = (score: number) => {
    if (score < 3) return 'text-green-400';
    if (score < 6) return 'text-yellow-400';
    if (score < 8) return 'text-orange-400';
    return 'text-red-400';
  };

  const getRiskBgColor = (score: number) => {
    if (score < 3) return 'bg-green-500/10 border-green-500/30';
    if (score < 6) return 'bg-yellow-500/10 border-yellow-500/30';
    if (score < 8) return 'bg-orange-500/10 border-orange-500/30';
    return 'bg-red-500/10 border-red-500/30';
  };

  return (
    <div className="h-full flex flex-col bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f]">
      {/* Header */}
      <div className="border-b border-[#2a2a35] p-4">
        <div className="flex items-center gap-3">
          <div className="p-2 bg-gradient-to-br from-cyan-500 to-blue-500 rounded-lg">
            <BarChart3 className="w-5 h-5 text-white" />
          </div>
          <div>
            <h1 className="text-lg font-semibold text-white">Advanced Portfolio Analytics</h1>
            <p className="text-xs text-gray-400">Sharpe ratio, VaR, drawdown, correlation analysis</p>
          </div>
        </div>
      </div>

      {/* Tabs */}
      <div className="flex gap-2 px-4 pt-4 border-b border-[#2a2a35]">
        {(['overview', 'metrics', 'correlation', 'risks'] as const).map(tab => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab)}
            className={`px-4 py-2 rounded-lg font-medium text-sm transition ${
              activeTab === tab
                ? 'bg-cyan-600 text-white'
                : 'text-gray-400 hover:text-gray-200'
            }`}
          >
            {tab === 'overview' && 'Overview'}
            {tab === 'metrics' && 'Metrics'}
            {tab === 'correlation' && 'Correlation'}
            {tab === 'risks' && 'Risks'}
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        {activeTab === 'overview' && (
          <div className="p-4 space-y-4">
            {/* Risk Score Card */}
            <div className={`border rounded-lg p-4 ${getRiskBgColor(analyticsData.riskScore)}`}>
              <div className="flex items-center justify-between mb-3">
                <h3 className="font-semibold text-white">Portfolio Risk Score</h3>
                <div className={`text-3xl font-bold ${getRiskColor(analyticsData.riskScore)}`}>
                  {analyticsData.riskScore.toFixed(1)}/10
                </div>
              </div>
              <div className="w-full bg-[#0a0a0f] rounded-full h-2">
                <div
                  className={`h-2 rounded-full transition-all ${
                    analyticsData.riskScore < 3 ? 'bg-green-500' :
                    analyticsData.riskScore < 6 ? 'bg-yellow-500' :
                    analyticsData.riskScore < 8 ? 'bg-orange-500' :
                    'bg-red-500'
                  }`}
                  style={{ width: `${(analyticsData.riskScore / 10) * 100}%` }}
                />
              </div>
              <p className="text-xs text-gray-400 mt-2">
                {analyticsData.riskScore < 3 ? 'Conservative - Low volatility portfolio' :
                 analyticsData.riskScore < 6 ? 'Moderate - Balanced risk exposure' :
                 analyticsData.riskScore < 8 ? 'Aggressive - High volatility assets' :
                 'Very Aggressive - Extreme risk exposure'}
              </p>
            </div>

            {/* Key Metrics Grid */}
            <div className="grid grid-cols-2 gap-3">
              <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-3">
                <p className="text-xs text-gray-400 mb-1">Sharpe Ratio</p>
                <p className="text-lg font-bold text-green-400">{analyticsData.sharpeRatio.toFixed(2)}</p>
                <p className="text-xs text-gray-500 mt-1">Risk-adjusted returns (>1.0 = good)</p>
              </div>

              <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-3">
                <p className="text-xs text-gray-400 mb-1">Max Drawdown</p>
                <p className="text-lg font-bold text-red-400">{analyticsData.maxDrawdown.toFixed(1)}%</p>
                <p className="text-xs text-gray-500 mt-1">Peak to trough decline</p>
              </div>

              <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-3">
                <p className="text-xs text-gray-400 mb-1">Annual Volatility</p>
                <p className="text-lg font-bold text-yellow-400">{analyticsData.volatility.toFixed(2)}%</p>
                <p className="text-xs text-gray-500 mt-1">Standard deviation</p>
              </div>

              <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-3">
                <p className="text-xs text-gray-400 mb-1">Portfolio Value</p>
                <p className="text-lg font-bold text-cyan-400">${(analyticsData.totalValue / 1000).toFixed(1)}K</p>
                <p className="text-xs text-gray-500 mt-1">Total worth</p>
              </div>
            </div>

            {/* Asset Breakdown */}
            <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
              <h3 className="font-semibold text-white mb-3">Asset Composition</h3>
              <div className="space-y-2">
                {analyticsData.assets.map(asset => {
                  const allocation = ((asset.amount * asset.price) / analyticsData.totalValue) * 100;
                  return (
                    <div key={asset.symbol}>
                      <div className="flex justify-between mb-1">
                        <span className="text-sm text-white">{asset.symbol}</span>
                        <span className="text-sm font-semibold text-cyan-400">{allocation.toFixed(1)}%</span>
                      </div>
                      <div className="w-full bg-[#0a0a0f] rounded-full h-2">
                        <div
                          className="h-2 rounded-full bg-gradient-to-r from-cyan-500 to-blue-500"
                          style={{ width: `${allocation}%` }}
                        />
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          </div>
        )}

        {activeTab === 'metrics' && (
          <div className="p-4 space-y-4">
            <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
              <h3 className="font-semibold text-white mb-4">Risk Metrics</h3>

              <div className="space-y-3">
                <div className="flex justify-between items-center pb-3 border-b border-[#2a2a35]">
                  <span className="text-gray-400 text-sm">Sharpe Ratio</span>
                  <span className="text-white font-semibold">{analyticsData.sharpeRatio.toFixed(2)}</span>
                </div>

                <div className="flex justify-between items-center pb-3 border-b border-[#2a2a35]">
                  <span className="text-gray-400 text-sm">Sortino Ratio</span>
                  <span className="text-white font-semibold">3.12</span>
                </div>

                <div className="flex justify-between items-center pb-3 border-b border-[#2a2a35]">
                  <span className="text-gray-400 text-sm">Calmar Ratio</span>
                  <span className="text-white font-semibold">1.84</span>
                </div>

                <div className="flex justify-between items-center pb-3 border-b border-[#2a2a35]">
                  <span className="text-gray-400 text-sm">Max Drawdown</span>
                  <span className="text-red-400 font-semibold">{analyticsData.maxDrawdown.toFixed(1)}%</span>
                </div>

                <div className="flex justify-between items-center pb-3 border-b border-[#2a2a35]">
                  <span className="text-gray-400 text-sm">Annual Volatility</span>
                  <span className="text-yellow-400 font-semibold">{analyticsData.volatility.toFixed(2)}%</span>
                </div>

                <div className="flex justify-between items-center pb-3 border-b border-[#2a2a35]">
                  <span className="text-gray-400 text-sm">Current Drawdown</span>
                  <span className="text-orange-400 font-semibold">-8.3%</span>
                </div>

                <div className="flex justify-between items-center">
                  <span className="text-gray-400 text-sm">Upside Capture</span>
                  <span className="text-green-400 font-semibold">124%</span>
                </div>
              </div>
            </div>

            <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
              <h3 className="font-semibold text-white mb-4">Value at Risk (VaR)</h3>

              <div className="space-y-3">
                <div>
                  <p className="text-sm text-gray-400 mb-1">95% Confidence Level</p>
                  <div className="flex items-baseline gap-2">
                    <p className="text-2xl font-bold text-orange-400">
                      ${analyticsData.var95.toFixed(0)}
                    </p>
                    <p className="text-xs text-gray-500">max daily loss</p>
                  </div>
                </div>

                <div className="border-t border-[#2a2a35] pt-3">
                  <p className="text-sm text-gray-400 mb-1">99% Confidence Level</p>
                  <div className="flex items-baseline gap-2">
                    <p className="text-2xl font-bold text-red-400">
                      ${analyticsData.var99.toFixed(0)}
                    </p>
                    <p className="text-xs text-gray-500">max daily loss</p>
                  </div>
                </div>

                <div className="bg-blue-500/10 border border-blue-500/30 rounded p-3 mt-3">
                  <p className="text-xs text-blue-300">
                    VaR estimates the maximum loss you could experience with 95-99% confidence over a 1-day horizon.
                  </p>
                </div>
              </div>
            </div>
          </div>
        )}

        {activeTab === 'correlation' && (
          <div className="p-4 space-y-4">
            <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
              <h3 className="font-semibold text-white mb-4">Asset Correlation Matrix</h3>

              <div className="overflow-x-auto">
                <table className="w-full text-sm">
                  <thead>
                    <tr>
                      <th className="text-left text-gray-400 font-semibold pb-2">Asset</th>
                      {analyticsData.assets.map(a => (
                        <th key={a.symbol} className="text-center text-gray-400 font-semibold pb-2 px-2">{a.symbol}</th>
                      ))}
                    </tr>
                  </thead>
                  <tbody>
                    {analyticsData.assets.map((asset, i) => (
                      <tr key={asset.symbol} className="border-t border-[#2a2a35]">
                        <td className="text-white font-semibold py-2">{asset.symbol}</td>
                        {analyticsData.correlationMatrix[i].map((corr, j) => (
                          <td key={j} className="text-center py-2 px-2">
                            <div className={`px-2 py-1 rounded text-xs font-semibold ${
                              corr > 0.7 ? 'bg-red-500/20 text-red-400' :
                              corr > 0.4 ? 'bg-yellow-500/20 text-yellow-400' :
                              'bg-green-500/20 text-green-400'
                            }`}>
                              {corr.toFixed(2)}
                            </div>
                          </td>
                        ))}
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>

              <p className="text-xs text-gray-400 mt-4">
                Higher correlation (1.0) = assets move together. Lower correlation (0.0) = independent movement.
              </p>
            </div>

            <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
              <h3 className="font-semibold text-white mb-4">Asset Beta</h3>

              <div className="space-y-3">
                {analyticsData.assets.map(asset => (
                  <div key={asset.symbol}>
                    <div className="flex justify-between mb-1">
                      <span className="text-white font-semibold">{asset.symbol} Beta</span>
                      <span className="text-cyan-400 font-semibold">{asset.beta.toFixed(2)}</span>
                    </div>
                    <div className="w-full bg-[#0a0a0f] rounded-full h-2">
                      <div
                        className="h-2 rounded-full bg-gradient-to-r from-cyan-500 to-blue-500"
                        style={{ width: `${(asset.beta / 2) * 100}%` }}
                      />
                    </div>
                    <p className="text-xs text-gray-500 mt-1">
                      {asset.beta < 1 ? 'Less volatile than market' : asset.beta > 1 ? 'More volatile than market' : 'Market-correlated'}
                    </p>
                  </div>
                ))}
              </div>
            </div>
          </div>
        )}

        {activeTab === 'risks' && (
          <div className="p-4 space-y-4">
            <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
              <h3 className="font-semibold text-white mb-4 flex items-center gap-2">
                <AlertTriangle className="w-4 h-4 text-yellow-400" />
                Risk Warnings & Alerts
              </h3>

              <div className="space-y-3">
                <div className="bg-yellow-500/10 border border-yellow-500/30 rounded p-3">
                  <p className="text-sm font-semibold text-yellow-400">High Concentration in Crypto</p>
                  <p className="text-xs text-yellow-300 mt-1">Your portfolio is heavily weighted towards volatile assets (92% crypto). Consider diversifying.</p>
                </div>

                <div className="bg-orange-500/10 border border-orange-500/30 rounded p-3">
                  <p className="text-sm font-semibold text-orange-400">Drawdown Risk Increased</p>
                  <p className="text-xs text-orange-300 mt-1">Current drawdown (-8.3%) approaching historical max (-18.5%). Monitor positions closely.</p>
                </div>

                <div className="bg-green-500/10 border border-green-500/30 rounded p-3">
                  <p className="text-sm font-semibold text-green-400">Diversification Score: 6.2/10</p>
                  <p className="text-xs text-green-300 mt-1">Your assets show moderate correlation. Adding stablecoins or uncorrelated assets could improve diversification.</p>
                </div>
              </div>
            </div>

            <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
              <h3 className="font-semibold text-white mb-4">Volatility Analysis</h3>

              <div className="space-y-3">
                {analyticsData.assets.map(asset => (
                  <div key={asset.symbol}>
                    <div className="flex justify-between mb-1">
                      <span className="text-white font-semibold">{asset.symbol} Volatility</span>
                      <span className={asset.volatility > 2 ? 'text-orange-400' : 'text-green-400'}>
                        {asset.volatility.toFixed(2)}%
                      </span>
                    </div>
                    <div className="w-full bg-[#0a0a0f] rounded-full h-2">
                      <div
                        className={`h-2 rounded-full ${asset.volatility > 2 ? 'bg-orange-500' : 'bg-green-500'}`}
                        style={{ width: `${(asset.volatility / 5) * 100}%` }}
                      />
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default AdvancedPortfolioAnalyticsPanel;
