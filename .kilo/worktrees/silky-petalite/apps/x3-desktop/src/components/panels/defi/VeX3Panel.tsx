import React, { useState, useMemo } from 'react';
import { Lock, TrendingUp, Zap, Gift, BarChart3 } from 'lucide-react';
import clsx from 'clsx';

interface LockPosition {
  id: string;
  amount: number;
  lockDuration: number; // in years
  unlocksAt: number;
  veX3Earned: number;
  votingPower: number;
}

interface LiquidityMiningPool {
  id: string;
  pair: string;
  tvl: number;
  apr: number;
  myShare: number;
  veX3Boost: number; // e.g., 1.5x for 4-year lock
}

const MOCK_LOCK_POSITIONS: LockPosition[] = [
  {
    id: '1',
    amount: 10000,
    lockDuration: 2,
    unlocksAt: Date.now() + 63072000000, // 2 years
    veX3Earned: 10000,
    votingPower: 8500,
  },
  {
    id: '2',
    amount: 5000,
    lockDuration: 1,
    unlocksAt: Date.now() + 31536000000, // 1 year
    veX3Earned: 5000,
    votingPower: 4000,
  },
];

const MOCK_POOLS: LiquidityMiningPool[] = [
  { id: '1', pair: 'X3/USDC', tvl: 32100000, apr: 24.5, myShare: 12450, veX3Boost: 1.8 },
  { id: '2', pair: 'ETH/X3', tvl: 18400000, apr: 18.2, myShare: 5200, veX3Boost: 1.5 },
  { id: '3', pair: 'SOL/X3', tvl: 12700000, apr: 31.8, myShare: 820, veX3Boost: 1.2 },
];

const VeX3Panel: React.FC = () => {
  const [positions, setPositions] = useState<LockPosition[]>(MOCK_LOCK_POSITIONS);
  const [pools, setPools] = useState<LiquidityMiningPool[]>(MOCK_POOLS);
  const [lockAmount, setLockAmount] = useState('');
  const [lockDuration, setLockDuration] = useState(1);
  const [showLockModal, setShowLockModal] = useState(false);

  const totalLockedX3 = useMemo(
    () => positions.reduce((sum, p) => sum + p.amount, 0),
    [positions]
  );

  const totalVeX3 = useMemo(
    () => positions.reduce((sum, p) => sum + p.veX3Earned, 0),
    [positions]
  );

  const totalVotingPower = useMemo(
    () => positions.reduce((sum, p) => sum + p.votingPower, 0),
    [positions]
  );

  const getTotalAprWithBoost = (pool: LiquidityMiningPool) => {
    const avgBoost = positions.length > 0
      ? positions.reduce((sum, p) => sum + (p.lockDuration ** 1.5), 0) / positions.length
      : 1;
    return (pool.apr * avgBoost).toFixed(1);
  };

  const handleCreateLock = () => {
    if (lockAmount && lockDuration) {
      const veX3 = Number(lockAmount) * (1 + lockDuration * 0.25); // Simple formula: 1 year = +25%
      const newPosition: LockPosition = {
        id: Math.random().toString(),
        amount: Number(lockAmount),
        lockDuration: Number(lockDuration),
        unlocksAt: Date.now() + lockDuration * 31536000000,
        veX3Earned: veX3,
        votingPower: veX3 * 0.85,
      };
      setPositions([...positions, newPosition]);
      setLockAmount('');
      setLockDuration(1);
      setShowLockModal(false);
    }
  };

  const formatDate = (timestamp: number) => {
    return new Date(timestamp).toLocaleDateString('en-US', {
      year: '2-digit',
      month: 'short',
      day: '2-digit',
    });
  };

  return (
    <div className="h-full flex flex-col bg-[#0a0a0f] text-white overflow-auto">
      {/* Header */}
      <div className="flex items-center justify-between px-5 py-4 border-b border-[#1a1a1a]">
        <div className="flex items-center gap-3">
          <Lock size={18} className="text-blue-400" />
          <h1 className="text-lg font-bold">veX3 (Vote-Escrow)</h1>
          <span className="text-xs bg-blue-500/20 text-blue-400 px-2 py-0.5 rounded">Curve Wars Model</span>
        </div>
        <button
          onClick={() => setShowLockModal(true)}
          className="flex items-center gap-2 bg-gradient-to-r from-blue-500 to-blue-600 hover:from-blue-400 hover:to-blue-500 text-white px-4 py-2 rounded-lg font-semibold text-sm transition-all shadow-lg shadow-blue-500/20"
        >
          <Lock size={14} /> New Lock
        </button>
      </div>

      {/* Stats */}
      <div className="grid grid-cols-4 gap-3 px-5 py-4 border-b border-[#1a1a1a]">
        <div className="bg-[#111111] rounded-xl p-3 border border-[#1a1a1a] hover:border-[#2a2a2a] transition">
          <div className="text-xs text-gray-500">Total Locked</div>
          <div className="text-lg font-bold text-white">{totalLockedX3.toLocaleString()} X3</div>
        </div>
        <div className="bg-[#111111] rounded-xl p-3 border border-[#1a1a1a] hover:border-[#2a2a2a] transition">
          <div className="text-xs text-gray-500">veX3 Balance</div>
          <div className="text-lg font-bold text-blue-400">{totalVeX3.toLocaleString()}</div>
        </div>
        <div className="bg-[#111111] rounded-xl p-3 border border-[#1a1a1a] hover:border-[#2a2a2a] transition">
          <div className="text-xs text-gray-500">Voting Power</div>
          <div className="text-lg font-bold text-purple-400">{totalVotingPower.toLocaleString()}</div>
        </div>
        <div className="bg-[#111111] rounded-xl p-3 border border-[#1a1a1a] hover:border-[#2a2a2a] transition">
          <div className="text-xs text-gray-500">LM Pools Voted</div>
          <div className="text-lg font-bold text-green-400">3</div>
        </div>
      </div>

      {/* Tabs */}
      <div className="flex gap-1 px-5 py-3 border-b border-[#1a1a1a] bg-[#0a0a0f]">
        <button className="px-4 py-2 rounded-lg text-sm font-medium text-blue-400 bg-blue-500/10 border border-blue-500/40">
          🔒 My Locks
        </button>
        <button className="px-4 py-2 rounded-lg text-sm font-medium text-gray-500 hover:text-white transition-colors">
          💰 LM Farming
        </button>
        <button className="px-4 py-2 rounded-lg text-sm font-medium text-gray-500 hover:text-white transition-colors">
          🗳️ Vote
        </button>
      </div>

      {/* Lock Positions */}
      <div className="flex-1 flex flex-col overflow-auto px-5 py-4">
        <h2 className="text-sm font-semibold text-gray-400 mb-3">Active Lock Positions</h2>
        <div className="space-y-3">
          {positions.map((pos) => (
            <div key={pos.id} className="bg-[#111111] rounded-lg border border-[#1a1a1a] p-4 hover:border-[#2a2a2a] transition">
              <div className="flex items-center justify-between mb-3">
                <div className="flex items-center gap-3">
                  <div className="bg-blue-500/20 rounded-lg p-2">
                    <Lock size={16} className="text-blue-400" />
                  </div>
                  <div>
                    <div className="font-semibold text-white">{pos.amount.toLocaleString()} X3</div>
                    <div className="text-xs text-gray-500">Locked for {pos.lockDuration} year{pos.lockDuration > 1 ? 's' : ''}</div>
                  </div>
                </div>
                <div className="text-right">
                  <div className="text-sm font-semibold text-green-400">{pos.veX3Earned.toLocaleString()} veX3</div>
                  <div className="text-xs text-gray-500">Unlocks {formatDate(pos.unlocksAt)}</div>
                </div>
              </div>
              <div className="w-full bg-[#0a0a0f] rounded-full h-1.5 border border-[#1a1a1a] overflow-hidden">
                <div
                  className="bg-gradient-to-r from-blue-500 to-blue-600 h-full"
                  style={{ width: `${(pos.veX3Earned / totalVeX3 || 0) * 100}%` }}
                />
              </div>
              <div className="mt-3 grid grid-cols-2 gap-2 text-xs">
                <div className="flex items-center gap-1 text-gray-500">
                  <Zap size={12} /> {pos.votingPower.toLocaleString()} voting power
                </div>
                <div className="text-right text-gray-500">
                  {((pos.lockDuration / 4) * 100).toFixed(0)}% of max boost
                </div>
              </div>
            </div>
          ))}
        </div>

        {/* Liquidity Mining Pools */}
        <h2 className="text-sm font-semibold text-gray-400 mt-6 mb-3">Liquidity Mining (LM) Pools</h2>
        <p className="text-xs text-gray-500 mb-3">Your veX3 boosts farming rewards in voted pools (up to {Math.max(...positions.map(p => p.lockDuration * 0.5))}x)</p>
        <div className="space-y-3">
          {pools.map((pool) => (
            <div key={pool.id} className="bg-[#111111] rounded-lg border border-[#1a1a1a] p-4 hover:border-[#2a2a2a] transition">
              <div className="flex items-center justify-between mb-3">
                <div className="flex items-center gap-3">
                  <div className="bg-purple-500/20 rounded-lg p-2">
                    <BarChart3 size={16} className="text-purple-400" />
                  </div>
                  <div>
                    <div className="font-semibold text-white">{pool.pair}</div>
                    <div className="text-xs text-gray-500">TVL: ${(pool.tvl / 1000000).toFixed(1)}M</div>
                  </div>
                </div>
                <div className="text-right">
                  <div className="text-sm font-semibold text-green-400">{getTotalAprWithBoost(pool)}% APR</div>
                  <div className="text-xs text-gray-500">(Base: {pool.apr}%)</div>
                </div>
              </div>
              <div className="grid grid-cols-2 gap-3 text-xs">
                <div>
                  <div className="text-gray-500">Your Stake</div>
                  <div className="font-semibold text-white">${pool.myShare.toLocaleString()}</div>
                </div>
                <div>
                  <div className="text-gray-500">Your Boost</div>
                  <div className="font-semibold text-yellow-400">{pool.veX3Boost}x</div>
                </div>
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Lock Modal */}
      {showLockModal && (
        <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
          <div className="bg-[#111111] border border-[#2a2a2a] rounded-lg p-6 w-96 shadow-2xl">
            <h3 className="font-bold text-white mb-4 flex items-center gap-2">
              <Lock size={18} className="text-blue-400" />
              Create New Lock
            </h3>

            <div className="space-y-4 mb-6">
              <div>
                <label className="block text-xs text-gray-500 mb-2">Amount (X3)</label>
                <input
                  type="number"
                  value={lockAmount}
                  onChange={(e) => setLockAmount(e.target.value)}
                  placeholder="0.00"
                  className="w-full bg-[#0a0a0f] border border-[#1a1a1a] rounded-lg p-3 text-white outline-none focus:border-blue-500/40"
                />
              </div>

              <div>
                <label className="block text-xs text-gray-500 mb-3">Lock Duration</label>
                <div className="grid grid-cols-4 gap-2">
                  {[1, 2, 3, 4].map((year) => (
                    <button
                      key={year}
                      onClick={() => setLockDuration(year)}
                      className={clsx(
                        'py-2 px-3 rounded-lg text-sm font-semibold transition-all',
                        lockDuration === year
                          ? 'bg-gradient-to-r from-blue-500 to-blue-600 text-white'
                          : 'bg-[#0a0a0f] border border-[#1a1a1a] text-gray-400 hover:text-white'
                      )}
                    >
                      {year}y
                    </button>
                  ))}
                </div>
              </div>

              <div className="bg-[#0a0a0f] rounded-lg p-3 space-y-2">
                <div className="flex justify-between text-xs">
                  <span className="text-gray-500">veX3 You'll Get</span>
                  <span className="text-green-400 font-semibold">
                    {lockAmount ? (Number(lockAmount) * (1 + lockDuration * 0.25)).toLocaleString() : '0'}
                  </span>
                </div>
                <div className="flex justify-between text-xs">
                  <span className="text-gray-500">Boost Factor</span>
                  <span className="text-yellow-400 font-semibold">{(1 + lockDuration * 0.25).toFixed(2)}x</span>
                </div>
              </div>
            </div>

            <div className="flex gap-2 justify-end">
              <button
                onClick={() => setShowLockModal(false)}
                className="px-4 py-2 rounded-lg bg-[#0a0a0f] border border-[#1a1a1a] text-gray-400 hover:text-white transition-colors"
              >
                Cancel
              </button>
              <button
                onClick={handleCreateLock}
                disabled={!lockAmount || Number(lockAmount) <= 0}
                className="px-4 py-2 rounded-lg bg-gradient-to-r from-blue-500 to-blue-600 text-white font-semibold disabled:from-gray-600 disabled:to-gray-600 transition-all"
              >
                Lock Now
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default VeX3Panel;

