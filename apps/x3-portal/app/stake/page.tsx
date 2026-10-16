'use client';

import React, { useState } from 'react';
import Link from 'next/link';
import {
  Coins,
  Lock,
  Unlock,
  TrendingUp,
  Gift,
  Shield,
  Clock,
  Zap,
  Wallet,
  CheckCircle,
  AlertCircle,
  ChevronDown,
  ArrowRight,
  Star,
  Trophy,
  Info,
  Calculator,
} from 'lucide-react';

// Staking tiers
const stakingTiers = [
  {
    name: 'Flexible',
    duration: '0 days',
    apy: '5%',
    multiplier: '1x',
    minStake: '100',
    features: ['Withdraw anytime', 'Base points rate', 'Basic rewards'],
    color: 'from-gray-500 to-gray-600',
    popular: false,
  },
  {
    name: 'Standard',
    duration: '30 days',
    apy: '12%',
    multiplier: '2x',
    minStake: '500',
    features: ['Higher APY', '2x point multiplier', 'Priority support'],
    color: 'from-blue-500 to-cyan-500',
    popular: false,
  },
  {
    name: 'Pro',
    duration: '90 days',
    apy: '18%',
    multiplier: '3x',
    minStake: '1,000',
    features: ['Maximum APY', '3x point multiplier', 'Governance rights', 'Exclusive airdrops'],
    color: 'from-orange-500 to-red-500',
    popular: true,
  },
  {
    name: 'Diamond',
    duration: '180 days',
    apy: '25%',
    multiplier: '5x',
    minStake: '10,000',
    features: ['Premium APY', '5x point multiplier', 'VIP governance', 'All airdrops', 'Private events'],
    color: 'from-purple-500 to-pink-500',
    popular: false,
  },
];

// User's positions
const userPositions = [
  { tier: 'Pro', amount: '5,000', startDate: 'Dec 5, 2025', unlockDate: 'Mar 5, 2026', earned: '150', points: '12,450' },
  { tier: 'Flexible', amount: '1,200', startDate: 'Dec 1, 2025', unlockDate: 'Anytime', earned: '8.5', points: '2,880' },
];

// Stats
const globalStats = [
  { label: 'Total Staked', value: '$124.5M', change: '+12.5%' },
  { label: 'Stakers', value: '45,892', change: '+5.2%' },
  { label: 'Average APY', value: '15.2%', change: '+2.1%' },
  { label: 'Total Rewards Paid', value: '$8.7M', change: '+18.4%' },
];

export default function StakePage() {
  const [selectedTier, setSelectedTier] = useState(stakingTiers[2]); // Pro tier default
  const [stakeAmount, setStakeAmount] = useState('');
  const [isConnected, setIsConnected] = useState(false);
  const [showCalculator, setShowCalculator] = useState(false);

  const x3Balance = '12,500.00';
  
  // Calculate estimated rewards
  const calculateRewards = () => {
    const amount = parseFloat(stakeAmount.replace(',', '')) || 0;
    const apyRate = parseFloat(selectedTier.apy) / 100;
    const durationDays = selectedTier.duration === '0 days' ? 365 : parseInt(selectedTier.duration);
    const yearlyReward = amount * apyRate;
    const periodReward = (yearlyReward * durationDays) / 365;
    const pointsPerHour = amount * parseInt(selectedTier.multiplier);
    return {
      periodReward: periodReward.toFixed(2),
      yearlyReward: yearlyReward.toFixed(2),
      pointsPerDay: (pointsPerHour * 24).toFixed(0),
    };
  };

  const rewards = calculateRewards();

  return (
    <div className="relative bg-black min-h-screen pt-24 pb-16">
      {/* Background effects */}
      <div className="absolute inset-0 bg-black" />
      <div className="absolute inset-0 mesh-gradient opacity-30" />
      <div className="absolute inset-0 grid-pattern opacity-20" />
      
      <div className="absolute top-1/4 left-1/4 w-96 h-96 bg-orange-600/10 rounded-full blur-3xl animate-float" />
      <div className="absolute bottom-1/4 right-1/4 w-80 h-80 bg-purple-600/10 rounded-full blur-3xl animate-float" style={{ animationDelay: '2s' }} />

      <div className="relative z-10 container-wide">
        {/* Header */}
        <div className="text-center mb-8">
          <div className="inline-flex items-center px-4 py-2 rounded-full bg-orange-500/10 border border-orange-500/30 mb-4">
            <Gift className="w-4 h-4 text-orange-400 mr-2" />
            <span className="text-sm text-orange-300">Earn up to 25% APY + Points</span>
          </div>
          <h1 className="text-3xl md:text-5xl font-bold mb-3">
            <span className="text-white">Stake </span>
            <span className="gradient-text">X3 Tokens</span>
          </h1>
          <p className="text-gray-400 max-w-xl mx-auto">
            Lock your X3 tokens to earn rewards and multiply your points. The longer you stake, the higher your benefits.
          </p>
        </div>

        {/* Global Stats */}
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-12">
          {globalStats.map((stat, i) => (
            <div key={i} className="glass-card p-4 text-center">
              <p className="text-2xl md:text-3xl font-bold gradient-text">{stat.value}</p>
              <p className="text-xs text-gray-500 mb-1">{stat.label}</p>
              <span className="text-xs text-emerald-400">{stat.change}</span>
            </div>
          ))}
        </div>

        <div className="grid lg:grid-cols-3 gap-8">
          {/* Staking Tiers */}
          <div className="lg:col-span-2 space-y-4">
            <h2 className="text-xl font-bold text-white mb-4">Choose Your Staking Tier</h2>
            
            <div className="grid md:grid-cols-2 gap-4">
              {stakingTiers.map((tier) => (
                <button
                  key={tier.name}
                  onClick={() => setSelectedTier(tier)}
                  className={`relative p-6 rounded-2xl border transition-all text-left ${
                    selectedTier.name === tier.name
                      ? 'bg-[#111111] border-orange-500/50 ring-2 ring-orange-500/20'
                      : 'bg-[#0a0a0a] border-[#1a1a1a] hover:border-[#333333]'
                  }`}
                >
                  {tier.popular && (
                    <span className="absolute -top-2 -right-2 px-2 py-1 text-xs font-bold bg-gradient-to-r from-orange-500 to-red-500 text-white rounded-full">
                      POPULAR
                    </span>
                  )}
                  
                  <div className="flex items-center justify-between mb-4">
                    <div className={`w-12 h-12 rounded-xl bg-gradient-to-br ${tier.color} flex items-center justify-center`}>
                      <Coins className="w-6 h-6 text-white" />
                    </div>
                    <div className="text-right">
                      <p className="text-2xl font-bold text-white">{tier.apy}</p>
                      <p className="text-xs text-gray-500">APY</p>
                    </div>
                  </div>
                  
                  <h3 className="text-lg font-bold text-white mb-1">{tier.name}</h3>
                  <p className="text-sm text-gray-500 mb-4">
                    {tier.duration === '0 days' ? 'No lock period' : `${tier.duration} lock`}
                  </p>
                  
                  <div className="flex items-center justify-between text-sm mb-4">
                    <span className="text-gray-500">Min stake</span>
                    <span className="text-white">{tier.minStake} X3</span>
                  </div>
                  
                  <div className="flex items-center justify-between text-sm mb-4">
                    <span className="text-gray-500">Points multiplier</span>
                    <span className="text-orange-400 font-semibold">{tier.multiplier}</span>
                  </div>
                  
                  <ul className="space-y-2">
                    {tier.features.map((feature, i) => (
                      <li key={i} className="flex items-center gap-2 text-xs text-gray-400">
                        <CheckCircle className="w-3 h-3 text-emerald-500" />
                        {feature}
                      </li>
                    ))}
                  </ul>
                </button>
              ))}
            </div>
          </div>

          {/* Staking Form */}
          <div className="space-y-6">
            {/* Stake Input */}
            <div className="glass-card p-6">
              <div className="flex items-center justify-between mb-4">
                <h3 className="font-semibold text-white">Stake X3</h3>
                <button 
                  onClick={() => setShowCalculator(!showCalculator)}
                  className="text-xs text-orange-400 hover:text-orange-300 flex items-center gap-1"
                >
                  <Calculator className="w-4 h-4" />
                  Calculator
                </button>
              </div>
              
              <div className="p-4 rounded-xl bg-[#0a0a0a] border border-[#1a1a1a] mb-4">
                <div className="flex items-center justify-between mb-2">
                  <span className="text-sm text-gray-500">Amount to stake</span>
                  <span className="text-xs text-gray-600">Balance: {x3Balance} X3</span>
                </div>
                <div className="flex items-center gap-3">
                  <input
                    type="number"
                    value={stakeAmount}
                    onChange={(e) => setStakeAmount(e.target.value)}
                    placeholder="0.0"
                    className="flex-1 bg-transparent text-2xl text-white outline-none placeholder-gray-600"
                  />
                  <div className="flex items-center gap-2">
                    <button 
                      onClick={() => setStakeAmount(x3Balance.replace(',', ''))}
                      className="px-2 py-1 text-xs bg-[#1a1a1a] rounded text-orange-400 hover:bg-[#252525]"
                    >
                      MAX
                    </button>
                    <span className="text-white font-medium">X3</span>
                  </div>
                </div>
              </div>

              {/* Selected Tier Info */}
              <div className="p-4 rounded-xl bg-gradient-to-r from-orange-500/10 to-red-500/10 border border-orange-500/20 mb-4">
                <div className="flex items-center justify-between mb-2">
                  <span className="text-sm text-gray-400">Selected tier</span>
                  <span className="text-white font-medium">{selectedTier.name}</span>
                </div>
                <div className="flex items-center justify-between mb-2">
                  <span className="text-sm text-gray-400">Lock period</span>
                  <span className="text-white">{selectedTier.duration === '0 days' ? 'None' : selectedTier.duration}</span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-sm text-gray-400">APY</span>
                  <span className="text-emerald-400 font-semibold">{selectedTier.apy}</span>
                </div>
              </div>

              {/* Rewards Estimate */}
              {stakeAmount && (
                <div className="p-4 rounded-xl bg-[#0a0a0a] border border-[#1a1a1a] mb-4 space-y-2">
                  <p className="text-xs text-gray-500 mb-2">Estimated rewards</p>
                  <div className="flex items-center justify-between text-sm">
                    <span className="text-gray-400">Period rewards</span>
                    <span className="text-emerald-400">+{rewards.periodReward} X3</span>
                  </div>
                  <div className="flex items-center justify-between text-sm">
                    <span className="text-gray-400">Points per day</span>
                    <span className="text-orange-400">+{rewards.pointsPerDay} pts</span>
                  </div>
                </div>
              )}

              {/* Action Button */}
              {isConnected ? (
                <button
                  disabled={!stakeAmount || parseFloat(stakeAmount) < parseFloat(selectedTier.minStake)}
                  className={`w-full py-4 rounded-xl font-semibold text-white transition-all flex items-center justify-center gap-2 ${
                    stakeAmount && parseFloat(stakeAmount) >= parseFloat(selectedTier.minStake)
                      ? 'bg-gradient-to-r from-orange-500 to-red-500 hover:from-orange-400 hover:to-red-400 shadow-lg shadow-orange-500/25'
                      : 'bg-[#1a1a1a] text-gray-500 cursor-not-allowed'
                  }`}
                >
                  <Lock className="w-5 h-5" />
                  Stake X3
                </button>
              ) : (
                <button
                  onClick={() => setIsConnected(true)}
                  className="w-full py-4 rounded-xl font-semibold text-white bg-gradient-to-r from-orange-500 to-red-500 hover:from-orange-400 hover:to-red-400 shadow-lg shadow-orange-500/25 flex items-center justify-center gap-2"
                >
                  <Wallet className="w-5 h-5" />
                  Connect Wallet
                </button>
              )}

              <p className="text-xs text-gray-600 text-center mt-3">
                Min stake: {selectedTier.minStake} X3
              </p>
            </div>

            {/* Your Positions */}
            {isConnected && userPositions.length > 0 && (
              <div className="glass-card p-6">
                <h3 className="font-semibold text-white mb-4 flex items-center gap-2">
                  <Trophy className="w-5 h-5 text-orange-400" />
                  Your Positions
                </h3>
                <div className="space-y-3">
                  {userPositions.map((pos, i) => (
                    <div key={i} className="p-4 rounded-xl bg-[#0a0a0a] border border-[#1a1a1a]">
                      <div className="flex items-center justify-between mb-2">
                        <span className="font-medium text-white">{pos.tier}</span>
                        <span className="text-lg font-bold text-white">{pos.amount} X3</span>
                      </div>
                      <div className="grid grid-cols-2 gap-2 text-xs">
                        <div>
                          <p className="text-gray-500">Unlock</p>
                          <p className="text-gray-300">{pos.unlockDate}</p>
                        </div>
                        <div>
                          <p className="text-gray-500">Earned</p>
                          <p className="text-emerald-400">+{pos.earned} X3</p>
                        </div>
                        <div>
                          <p className="text-gray-500">Points</p>
                          <p className="text-orange-400">+{pos.points}</p>
                        </div>
                        <div>
                          <button className="text-orange-400 hover:text-orange-300 flex items-center gap-1">
                            {pos.unlockDate === 'Anytime' ? (
                              <>
                                <Unlock className="w-3 h-3" />
                                Unstake
                              </>
                            ) : (
                              <>
                                <Clock className="w-3 h-3" />
                                Locked
                              </>
                            )}
                          </button>
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* Quick Links */}
            <div className="grid grid-cols-2 gap-3">
              <Link href="/earn" className="p-4 rounded-xl bg-[#111111] border border-[#1a1a1a] hover:border-orange-500/30 transition-colors text-center">
                <Gift className="w-6 h-6 text-orange-400 mx-auto mb-2" />
                <span className="text-sm text-gray-400">Earn Points</span>
              </Link>
              <Link href="/swap" className="p-4 rounded-xl bg-[#111111] border border-[#1a1a1a] hover:border-orange-500/30 transition-colors text-center">
                <TrendingUp className="w-6 h-6 text-orange-400 mx-auto mb-2" />
                <span className="text-sm text-gray-400">Get X3</span>
              </Link>
            </div>
          </div>
        </div>

        {/* Info Section */}
        <div className="mt-12 glass-card p-8">
          <h2 className="text-xl font-bold text-white mb-6 text-center">Why Stake X3?</h2>
          <div className="grid md:grid-cols-4 gap-6">
            {[
              { icon: <TrendingUp className="w-8 h-8" />, title: 'Earn APY', desc: 'Up to 25% annual yield on your tokens' },
              { icon: <Star className="w-8 h-8" />, title: 'Point Multiplier', desc: 'Boost your points earning rate up to 5x' },
              { icon: <Shield className="w-8 h-8" />, title: 'Governance', desc: 'Vote on protocol decisions' },
              { icon: <Gift className="w-8 h-8" />, title: 'Airdrops', desc: 'Qualify for exclusive token distributions' },
            ].map((item, i) => (
              <div key={i} className="text-center">
                <div className="w-16 h-16 rounded-2xl bg-gradient-to-br from-orange-500/20 to-red-500/20 flex items-center justify-center mx-auto mb-3">
                  <span className="text-orange-400">{item.icon}</span>
                </div>
                <h3 className="font-semibold text-white mb-1">{item.title}</h3>
                <p className="text-sm text-gray-500">{item.desc}</p>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
