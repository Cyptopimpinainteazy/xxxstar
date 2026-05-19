import React, { useState } from 'react';
import { Droplets, Plus, TrendingUp, DollarSign, Zap } from 'lucide-react';
import clsx from 'clsx';

interface LPPosition {
  id: string;
  pair: string;
  amount0: number;
  amount1: number;
  feeTier: number;
  lowerTick: number;
  upperTick: number;
  liquidity: number;
  uncollectedFees: number;
  isNFT: boolean;
  estimatedAPR: number;
}

const MOCK_POSITIONS: LPPosition[] = [
  {
    id: '1',
    pair: 'X3/USDC',
    amount0: 50,
    amount1: 62500,
    feeTier: 0.3,
    lowerTick: -1000,
    upperTick: 1000,
    liquidity: 125000,
    uncollectedFees: 234.50,
    isNFT: true,
    estimatedAPR: 28.5,
  },
  {
    id: '2',
    pair: 'ETH/USDC',
    amount0: 1.5,
    amount1: 4875,
    feeTier: 0.05,
    lowerTick: -500,
    upperTick: 500,
    liquidity: 87500,
    uncollectedFees: 89.25,
    isNFT: false,
    estimatedAPR: 15.3,
  },
];

const ConcentratedLiquidityPanel: React.FC = () => {
  const [positions, setPositions] = useState<LPPosition[]>(MOCK_POSITIONS);
  const [selectedPosition, setSelectedPosition] = useState<LPPosition | null>(null);
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [newPosition, setNewPosition] = useState({
    pair: 'X3/USDC',
    amount0: 0,
    amount1: 0,
    feeTier: 0.3,
    priceRange: 'medium', // tight, medium, wide
  });

  const totalLiquidity = positions.reduce((sum, p) => sum + p.liquidity, 0);
  const totalUncollectedFees = positions.reduce((sum, p) => sum + p.uncollectedFees, 0);
  const avgAPR = positions.length > 0 
    ? positions.reduce((sum, p) => sum + p.estimatedAPR, 0) / positions.length 
    : 0;

  const getPriceRangeDisplay = (pos: LPPosition) => {
    const range = pos.upperTick - pos.lowerTick;
    if (range < 500) return 'Tight (2.3% range)';
    if (range < 2000) return 'Medium (8.5% range)';
    return 'Wide (25% range)';
  };

  const handleCollectFees = (positionId: string) => {
    setPositions(
      positions.map(p =>
        p.id === positionId
          ? { ...p, uncollectedFees: 0 }
          : p
      )
    );
  };

  return (
    <div className="h-full flex flex-col bg-[#0a0a0f] text-white overflow-auto">
      {/* Header */}
      <div className="flex items-center justify-between px-5 py-4 border-b border-[#1a1a1a]">
        <div className="flex items-center gap-3">
          <Droplets size={18} className="text-blue-400" />
          <h1 className="text-lg font-bold">Concentrated Liquidity (V3)</h1>
          <span className="text-xs bg-blue-500/20 text-blue-400 px-2 py-0.5 rounded">Uniswap V3 Model</span>
        </div>
        <button
          onClick={() => setShowCreateModal(true)}
          className="flex items-center gap-2 bg-gradient-to-r from-blue-500 to-blue-600 hover:from-blue-400 hover:to-blue-500 text-white px-4 py-2 rounded-lg font-semibold text-sm transition-all"
        >
          <Plus size={14} /> New Position
        </button>
      </div>

      {/* Stats */}
      <div className="grid grid-cols-4 gap-3 px-5 py-4 border-b border-[#1a1a1a]">
        <div className="bg-[#111111] rounded-xl p-3 border border-[#1a1a1a]">
          <div className="text-xs text-gray-500">Active Positions</div>
          <div className="text-lg font-bold text-white">{positions.length}</div>
        </div>
        <div className="bg-[#111111] rounded-xl p-3 border border-[#1a1a1a]">
          <div className="text-xs text-gray-500">Total Liquidity</div>
          <div className="text-lg font-bold text-blue-400">${(totalLiquidity / 1000).toFixed(1)}K</div>
        </div>
        <div className="bg-[#111111] rounded-xl p-3 border border-[#1a1a1a]">
          <div className="text-xs text-gray-500">Uncollected Fees</div>
          <div className="text-lg font-bold text-green-400">${totalUncollectedFees.toFixed(2)}</div>
        </div>
        <div className="bg-[#111111] rounded-xl p-3 border border-[#1a1a1a]">
          <div className="text-xs text-gray-500">Avg. APR</div>
          <div className="text-lg font-bold text-yellow-400">{avgAPR.toFixed(1)}%</div>
        </div>
      </div>

      {/* Positions List */}
      <div className="flex-1 overflow-auto px-5 py-4 space-y-3">
        {positions.map((pos) => (
          <div
            key={pos.id}
            onClick={() => setSelectedPosition(pos)}
            className="bg-[#111111] border border-[#1a1a1a] rounded-lg p-4 hover:border-[#2a2a2a] transition-colors cursor-pointer"
          >
            <div className="flex items-center justify-between mb-3">
              <div className="flex items-center gap-3">
                <div className="bg-blue-500/20 rounded-lg p-2">
                  <Droplets size={16} className="text-blue-400" />
                </div>
                <div>
                  <div className="font-semibold text-white">{pos.pair}</div>
                  <div className="text-xs text-gray-500">
                    {getPriceRangeDisplay(pos)} • {pos.feeTier}% fee tier
                  </div>
                </div>
              </div>
              <div className="text-right">
                <div className="font-bold text-blue-400">${(pos.liquidity / 1000).toFixed(1)}K</div>
                <div className="text-xs text-green-400 font-semibold">{pos.estimatedAPR.toFixed(1)}% APR</div>
              </div>
            </div>

            {/* Position Details Grid */}
            <div className="grid grid-cols-4 gap-2 mb-3 text-xs">
              <div className="bg-[#0a0a0f] rounded p-2">
                <div className="text-gray-500 mb-1">Amount A</div>
                <div className="font-mono font-semibold text-white">{pos.amount0}</div>
              </div>
              <div className="bg-[#0a0a0f] rounded p-2">
                <div className="text-gray-500 mb-1">Amount B</div>
                <div className="font-mono font-semibold text-white">{pos.amount1}</div>
              </div>
              <div className="bg-[#0a0a0f] rounded p-2">
                <div className="text-gray-500 mb-1">Uncollected</div>
                <div className="font-mono font-semibold text-green-400">${pos.uncollectedFees.toFixed(2)}</div>
              </div>
              <div className="bg-[#0a0a0f] rounded p-2">
                <div className="text-gray-500 mb-1">Type</div>
                <div className="font-semibold">
                  {pos.isNFT ? <span className="text-purple-400">NFT 🎨</span> : <span className="text-gray-400">LP Token</span>}
                </div>
              </div>
            </div>

            {/* Price Range Visualization */}
            <div className="bg-[#0a0a0f] rounded-lg p-2 mb-3">
              <div className="text-xs text-gray-500 mb-1">Price Range</div>
              <div className="flex items-end gap-1 h-8">
                {Array.from({ length: 20 }).map((_, i) => (
                  <div
                    key={i}
                    className={clsx(
                      'flex-1 rounded-sm transition-colors',
                      i >= 6 && i <= 13 ? 'bg-blue-500 h-full' : 'bg-[#1a1a1a] h-2'
                    )}
                  />
                ))}
              </div>
              <div className="flex justify-between text-xs text-gray-600 mt-1">
                <span>${pos.lowerTick}</span>
                <span className="text-blue-400 font-semibold">Current</span>
                <span>${pos.upperTick}</span>
              </div>
            </div>

            {/* Actions */}
            {pos.uncollectedFees > 0 && (
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  handleCollectFees(pos.id);
                }}
                className="w-full flex items-center justify-center gap-2 py-2 rounded-lg bg-gradient-to-r from-green-500/20 to-green-600/20 hover:from-green-500/30 hover:to-green-600/30 text-green-400 text-xs font-semibold transition-all"
              >
                <Zap size={12} /> Collect Fees (${pos.uncollectedFees.toFixed(2)})
              </button>
            )}
          </div>
        ))}

        {positions.length === 0 && (
          <div className="text-center py-12 text-gray-500">
            <Droplets size={32} className="mx-auto mb-2 opacity-20" />
            <p>No active positions yet. Create one to start earning!</p>
          </div>
        )}
      </div>

      {/* Position Detail Modal */}
      {selectedPosition && (
        <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
          <div className="bg-[#111111] border border-[#2a2a2a] rounded-lg p-6 w-96 shadow-2xl max-h-96 overflow-auto">
            <h3 className="font-bold text-white text-lg mb-4">{selectedPosition.pair} Position</h3>

            <div className="space-y-3 mb-6">
              <div className="bg-[#0a0a0f] rounded-lg p-3">
                <div className="text-xs text-gray-500 mb-1">Total Liquidity</div>
                <div className="text-2xl font-bold text-blue-400">${(selectedPosition.liquidity / 1000).toFixed(1)}K</div>
              </div>

              <div className="grid grid-cols-2 gap-2">
                <div className="bg-[#0a0a0f] rounded-lg p-3">
                  <div className="text-xs text-gray-500 mb-1">Token A</div>
                  <div className="font-bold text-white">{selectedPosition.amount0}</div>
                </div>
                <div className="bg-[#0a0a0f] rounded-lg p-3">
                  <div className="text-xs text-gray-500 mb-1">Token B</div>
                  <div className="font-bold text-white">{selectedPosition.amount1}</div>
                </div>
              </div>

              <div className="bg-[#0a0a0f] rounded-lg p-3">
                <div className="text-xs text-gray-500 mb-2">Concentration</div>
                <div className="flex items-center justify-between text-sm">
                  <span className="text-white">{70}% in range</span>
                  <span className="text-green-400 font-semibold">{selectedPosition.estimatedAPR.toFixed(1)}% APR</span>
                </div>
                <div className="w-full bg-[#0a0a0f] rounded-full h-2 border border-[#1a1a1a] mt-2 overflow-hidden">
                  <div className="bg-gradient-to-r from-green-500 to-blue-500 h-full" style={{ width: '70%' }} />
                </div>
              </div>

              <div className="bg-[#0a0a0f] rounded-lg p-3">
                <div className="text-xs text-gray-500 mb-1">Uncollected Fees (24h)</div>
                <div className="text-lg font-bold text-green-400">${selectedPosition.uncollectedFees.toFixed(2)}</div>
              </div>
            </div>

            <div className="flex gap-2 justify-between">
              <button
                onClick={() => setSelectedPosition(null)}
                className="flex-1 px-4 py-2 rounded-lg bg-[#0a0a0f] border border-[#1a1a1a] text-gray-400 hover:text-white transition-colors"
              >
                Close
              </button>
              {selectedPosition.uncollectedFees > 0 && (
                <button
                  onClick={() => {
                    handleCollectFees(selectedPosition.id);
                    setSelectedPosition(null);
                  }}
                  className="flex items-center gap-2 flex-1 px-4 py-2 rounded-lg bg-gradient-to-r from-green-500 to-green-600 text-white font-semibold hover:from-green-400 hover:to-green-500 transition-all"
                >
                  <Zap size={14} /> Collect Fees
                </button>
              )}
            </div>
          </div>
        </div>
      )}

      {/* Create Position Modal */}
      {showCreateModal && (
        <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
          <div className="bg-[#111111] border border-[#2a2a2a] rounded-lg p-6 w-96 shadow-2xl">
            <h3 className="font-bold text-white mb-4 flex items-center gap-2">
              <Plus size={18} className="text-blue-400" /> Create Concentrated Position
            </h3>

            <div className="space-y-4 mb-6">
              <div>
                <label className="block text-xs text-gray-500 mb-2">Pair</label>
                <select
                  value={newPosition.pair}
                  onChange={(e) => setNewPosition({ ...newPosition, pair: e.target.value })}
                  className="w-full bg-[#0a0a0f] border border-[#1a1a1a] rounded-lg p-3 text-white text-sm outline-none focus:border-blue-500/40"
                >
                  <option>X3/USDC</option>
                  <option>ETH/USDC</option>
                </select>
              </div>

              <div>
                <label className="block text-xs text-gray-500 mb-2">Fee Tier</label>
                <div className="grid grid-cols-3 gap-2">
                  {[0.01, 0.05, 1.0].map((fee) => (
                    <button
                      key={fee}
                      onClick={() => setNewPosition({ ...newPosition, feeTier: fee })}
                      className={clsx(
                        'py-2 px-3 rounded-lg text-xs font-semibold transition-all',
                        newPosition.feeTier === fee
                          ? 'bg-blue-500/30 border border-blue-500/60 text-blue-400'
                          : 'bg-[#0a0a0f] border border-[#1a1a1a] text-gray-400 hover:text-white'
                      )}
                    >
                      {fee}%
                    </button>
                  ))}
                </div>
              </div>

              <div>
                <label className="block text-xs text-gray-500 mb-2">Price Range</label>
                <div className="grid grid-cols-3 gap-2">
                  {['tight', 'medium', 'wide'].map((range) => (
                    <button
                      key={range}
                      onClick={() => setNewPosition({ ...newPosition, priceRange: range as any })}
                      className={clsx(
                        'py-2 px-3 rounded-lg text-xs font-semibold transition-all capitalize',
                        newPosition.priceRange === range
                          ? 'bg-blue-500/30 border border-blue-500/60 text-blue-400'
                          : 'bg-[#0a0a0f] border border-[#1a1a1a] text-gray-400 hover:text-white'
                      )}
                    >
                      {range}
                    </button>
                  ))}
                </div>
              </div>
            </div>

            <div className="flex gap-2 justify-end">
              <button
                onClick={() => setShowCreateModal(false)}
                className="px-4 py-2 rounded-lg bg-[#0a0a0f] border border-[#1a1a1a] text-gray-400 hover:text-white transition-colors"
              >
                Cancel
              </button>
              <button
                onClick={() => setShowCreateModal(false)}
                className="px-4 py-2 rounded-lg bg-gradient-to-r from-blue-500 to-blue-600 text-white font-semibold hover:from-blue-400 hover:to-blue-500 transition-all"
              >
                Create Position
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default ConcentratedLiquidityPanel;

