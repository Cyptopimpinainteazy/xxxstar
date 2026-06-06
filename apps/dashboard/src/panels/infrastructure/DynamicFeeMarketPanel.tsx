import React, { useState } from 'react';
import { Zap, DollarSign, Shield, TrendingDown, AlertCircle, CheckCircle } from 'lucide-react';

interface FeeMetrics {
  baseFee: number;
  burneratePerBlock: number;
  priorityFee: number;
  gasPrice: number;
  mevProtectionFee: number;
}

interface MevProtection {
  id: string;
  strategy: 'commit-reveal' | 'threshold-encrypt' | 'dark-pool';
  status: 'active' | 'testing' | 'disabled';
  successRate: number;
  avgExtraction: number;
  costPerProtection: number;
}

interface SlashingInsurance {
  poolId: string;
  fundSize: number;
  claimsPaid: number;
  activeInsurances: number;
  coverage: number;
  premiumRate: number;
}

interface ValidatorCommission {
  validatorId: string;
  currentRate: number;
  maxCapRate: number;
  status: 'compliant' | 'warning' | 'violation';
  historicalRate: number[];
}

export const DynamicFeeMarketPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'fees' | 'mev' | 'insurance' | 'commission'>('fees');
  const [feeMetrics] = useState<FeeMetrics>({
    baseFee: 45.23,
    burneratePerBlock: 234.56,
    priorityFee: 12.5,
    gasPrice: 78.9,
    mevProtectionFee: 8.2,
  });

  const [mevStrategies] = useState<MevProtection[]>([
    {
      id: 'commit-reveal',
      strategy: 'commit-reveal',
      status: 'active',
      successRate: 99.2,
      avgExtraction: 0.0,
      costPerProtection: 2.3,
    },
    {
      id: 'threshold-encrypt',
      strategy: 'threshold-encrypt',
      status: 'active',
      successRate: 98.7,
      avgExtraction: 0.1,
      costPerProtection: 3.1,
    },
    {
      id: 'dark-pool',
      strategy: 'dark-pool',
      status: 'testing',
      successRate: 95.4,
      avgExtraction: 0.05,
      costPerProtection: 1.8,
    },
  ]);

  const [slashingInsurance] = useState<SlashingInsurance>({
    poolId: 'core-insurance-pool',
    fundSize: 2500000,
    claimsPaid: 45200,
    activeInsurances: 8423,
    coverage: 92.5,
    premiumRate: 0.5,
  });

  const [validatorCommissions] = useState<ValidatorCommission[]>([
    {
      validatorId: 'val-primary',
      currentRate: 8.5,
      maxCapRate: 12,
      status: 'compliant',
      historicalRate: [8.2, 8.3, 8.4, 8.5],
    },
    {
      validatorId: 'val-secondary',
      currentRate: 11.8,
      maxCapRate: 12,
      status: 'warning',
      historicalRate: [11.2, 11.5, 11.7, 11.8],
    },
    {
      validatorId: 'val-enterprise',
      currentRate: 9.2,
      maxCapRate: 12,
      status: 'compliant',
      historicalRate: [9.0, 9.1, 9.2, 9.2],
    },
  ]);

  const [feeHistory] = useState([
    { block: 12850000, baseFee: 42.1, burners: 210.5, mev: 0.2 },
    { block: 12850100, baseFee: 43.5, burners: 225.3, mev: 0.15 },
    { block: 12850200, baseFee: 45.2, burners: 234.5, mev: 0.18 },
  ]);

  return (
    <div className="min-h-screen bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f] p-6">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between mb-8">
          <div>
            <h1 className="text-4xl font-bold text-transparent bg-clip-text bg-gradient-to-r from-yellow-400 to-orange-500 mb-2">
              Dynamic Fee Market
            </h1>
            <p className="text-gray-400">EIP-1559 • MEV Protection • Slashing Insurance • Commission Caps</p>
          </div>
          <DollarSign className="w-12 h-12 text-yellow-400" />
        </div>

        {/* KPI Grid */}
        <div className="grid grid-cols-4 gap-4 mb-8">
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Base Fee</div>
            <div className="text-2xl font-bold text-yellow-400">{feeMetrics.baseFee} Gwei</div>
            <div className="text-xs text-gray-500 mt-2">Per block burn</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">MEV Protection Fee</div>
            <div className="text-2xl font-bold text-orange-400">{feeMetrics.mevProtectionFee} Gwei</div>
            <div className="text-xs text-gray-500 mt-2">99.2% success rate</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Insurance Fund</div>
            <div className="text-2xl font-bold text-green-400">$2.5M</div>
            <div className="text-xs text-gray-500 mt-2">{slashingInsurance.coverage}% coverage</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Burn Rate</div>
            <div className="text-2xl font-bold text-red-400">{feeMetrics.burneratePerBlock} X3/blk</div>
            <div className="text-xs text-gray-500 mt-2">Deflationary pressure</div>
          </div>
        </div>

        {/* Tabs */}
        <div className="flex gap-2 mb-6 border-b border-[#2a2a35]">
          {(['fees', 'mev', 'insurance', 'commission'] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={`px-4 py-3 font-semibold transition-colors ${
                activeTab === tab
                  ? 'text-yellow-400 border-b-2 border-yellow-400'
                  : 'text-gray-400 hover:text-gray-300'
              }`}
            >
              {tab === 'fees' && 'EIP-1559 Fees'}
              {tab === 'mev' && 'MEV Protection'}
              {tab === 'insurance' && 'Slashing Insurance'}
              {tab === 'commission' && 'Commission Caps'}
            </button>
          ))}
        </div>

        {/* Content */}
        <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
          {activeTab === 'fees' && (
            <div>
              <h3 className="text-lg font-semibold text-white mb-6">EIP-1559 Fee Structure</h3>
              <div className="grid grid-cols-2 gap-6 mb-8">
                <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                  <h4 className="text-white font-semibold mb-4">Current Metrics</h4>
                  <div className="space-y-3">
                    <div className="flex justify-between">
                      <span className="text-gray-400">Base Fee</span>
                      <span className="text-white font-semibold">{feeMetrics.baseFee} Gwei</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-400">Priority Fee</span>
                      <span className="text-white font-semibold">{feeMetrics.priorityFee} Gwei</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-400">Gas Price</span>
                      <span className="text-white font-semibold">{feeMetrics.gasPrice} Gwei</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-400">Block Burn Rate</span>
                      <span className="text-red-400 font-semibold">{feeMetrics.burneratePerBlock} X3</span>
                    </div>
                  </div>
                </div>
                <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                  <h4 className="text-white font-semibold mb-4">Fee Adjustments</h4>
                  <div className="space-y-3">
                    <div>
                      <div className="flex justify-between items-center mb-2">
                        <span className="text-sm text-gray-400">Base Fee Elasticity</span>
                        <span className="text-xs bg-blue-500/20 text-blue-400 px-2 py-1 rounded">12.5%</span>
                      </div>
                      <div className="w-full bg-[#2a2a35] rounded-full h-2">
                        <div className="bg-gradient-to-r from-blue-500 to-blue-400 h-2 rounded-full" style={{ width: '65%' }} />
                      </div>
                    </div>
                    <div>
                      <div className="flex justify-between items-center mb-2">
                        <span className="text-sm text-gray-400">Congestion Index</span>
                        <span className="text-xs bg-yellow-500/20 text-yellow-400 px-2 py-1 rounded">48%</span>
                      </div>
                      <div className="w-full bg-[#2a2a35] rounded-full h-2">
                        <div className="bg-gradient-to-r from-yellow-500 to-yellow-400 h-2 rounded-full" style={{ width: '48%' }} />
                      </div>
                    </div>
                  </div>
                </div>
              </div>
              <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                <h4 className="text-white font-semibold mb-4">Recent Blocks</h4>
                <div className="space-y-2">
                  {feeHistory.map((h) => (
                    <div key={h.block} className="flex justify-between items-center text-sm">
                      <span className="text-gray-400">Block {h.block.toLocaleString()}</span>
                      <div className="flex gap-4">
                        <span className="text-yellow-400">{h.baseFee} Gwei</span>
                        <span className="text-red-400">{h.burners} X3 burned</span>
                        <span className="text-orange-400">{h.mev} X3 protected</span>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            </div>
          )}

          {activeTab === 'mev' && (
            <div>
              <h3 className="text-lg font-semibold text-white mb-6">MEV Protection Strategies</h3>
              <div className="space-y-4">
                {mevStrategies.map((strat) => (
                  <div key={strat.id} className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                    <div className="flex items-start justify-between mb-4">
                      <div>
                        <h4 className="text-white font-semibold">
                          {strat.strategy === 'commit-reveal'
                            ? 'Commit-Reveal Scheme'
                            : strat.strategy === 'threshold-encrypt'
                              ? 'Threshold Encryption'
                              : 'Dark Pool'}
                        </h4>
                        <p className="text-sm text-gray-400">
                          Cost: {strat.costPerProtection} Gwei per protection
                        </p>
                      </div>
                      <div
                        className={`px-3 py-1 rounded-full text-xs font-semibold ${
                          strat.status === 'active'
                            ? 'bg-green-500/20 text-green-400'
                            : 'bg-yellow-500/20 text-yellow-400'
                        }`}
                      >
                        {strat.status.toUpperCase()}
                      </div>
                    </div>
                    <div className="grid grid-cols-3 gap-4">
                      <div>
                        <div className="text-xs text-gray-400 mb-2">Success Rate</div>
                        <div className="text-lg text-white font-semibold">{strat.successRate}%</div>
                      </div>
                      <div>
                        <div className="text-xs text-gray-400 mb-2">Avg MEV Extracted</div>
                        <div className="text-lg text-orange-400 font-semibold">{strat.avgExtraction} X3</div>
                      </div>
                      <div>
                        <div className="text-xs text-gray-400 mb-2">Status</div>
                        <div className="flex items-center gap-1">
                          {strat.successRate > 98 ? (
                            <CheckCircle className="w-4 h-4 text-green-400" />
                          ) : (
                            <AlertCircle className="w-4 h-4 text-yellow-400" />
                          )}
                          <span className="text-sm text-white">
                            {strat.successRate > 98 ? 'Optimal' : 'Monitor'}
                          </span>
                        </div>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {activeTab === 'insurance' && (
            <div>
              <h3 className="text-lg font-semibold text-white mb-6">Slashing Insurance Pool</h3>
              <div className="grid grid-cols-2 gap-6">
                <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                  <h4 className="text-white font-semibold mb-4">Pool Statistics</h4>
                  <div className="space-y-3">
                    <div className="flex justify-between">
                      <span className="text-gray-400">Total Fund Size</span>
                      <span className="text-green-400 font-semibold">${(slashingInsurance.fundSize / 1000000).toFixed(1)}M</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-400">Claims Paid (YTD)</span>
                      <span className="text-white font-semibold">${(slashingInsurance.claimsPaid / 1000).toFixed(1)}K</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-400">Active Insurances</span>
                      <span className="text-blue-400 font-semibold">{slashingInsurance.activeInsurances.toLocaleString()}</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-400">Premium Rate</span>
                      <span className="text-yellow-400 font-semibold">{slashingInsurance.premiumRate}% annually</span>
                    </div>
                  </div>
                </div>
                <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                  <h4 className="text-white font-semibold mb-4">Coverage Status</h4>
                  <div className="space-y-4">
                    <div>
                      <div className="flex justify-between mb-2">
                        <span className="text-sm text-gray-400">Coverage Ratio</span>
                        <span className="text-sm text-white font-semibold">{slashingInsurance.coverage}%</span>
                      </div>
                      <div className="w-full bg-[#2a2a35] rounded-full h-3">
                        <div
                          className="bg-gradient-to-r from-green-500 to-green-400 h-3 rounded-full"
                          style={{ width: `${slashingInsurance.coverage}%` }}
                        />
                      </div>
                    </div>
                    <div className="bg-green-500/10 border border-green-500/30 rounded-lg p-3">
                      <p className="text-sm text-green-400">✓ Fund is adequately capitalized</p>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          )}

          {activeTab === 'commission' && (
            <div>
              <h3 className="text-lg font-semibold text-white mb-6">Validator Commission Caps</h3>
              <div className="space-y-4">
                {validatorCommissions.map((val) => (
                  <div key={val.validatorId} className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                    <div className="flex items-start justify-between mb-4">
                      <div>
                        <h4 className="text-white font-semibold">{val.validatorId.toUpperCase()}</h4>
                        <p className="text-sm text-gray-400">Max Cap: {val.maxCapRate}%</p>
                      </div>
                      <div
                        className={`px-3 py-1 rounded-full text-xs font-semibold ${
                          val.status === 'compliant'
                            ? 'bg-green-500/20 text-green-400'
                            : 'bg-yellow-500/20 text-yellow-400'
                        }`}
                      >
                        {val.status.toUpperCase()}
                      </div>
                    </div>
                    <div className="mb-4">
                      <div className="flex justify-between mb-2">
                        <span className="text-sm text-gray-400">Current Rate</span>
                        <span className="text-white font-semibold">{val.currentRate}%</span>
                      </div>
                      <div className="w-full bg-[#2a2a35] rounded-full h-2">
                        <div
                          className={`h-2 rounded-full ${
                            val.status === 'compliant'
                              ? 'bg-gradient-to-r from-green-500 to-green-400'
                              : 'bg-gradient-to-r from-yellow-500 to-yellow-400'
                          }`}
                          style={{ width: `${(val.currentRate / val.maxCapRate) * 100}%` }}
                        />
                      </div>
                    </div>
                    <p className="text-xs text-gray-500">
                      Limit: {val.currentRate}/{val.maxCapRate}% | Margin: {(val.maxCapRate - val.currentRate).toFixed(1)}%
                    </p>
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

export default DynamicFeeMarketPanel;
