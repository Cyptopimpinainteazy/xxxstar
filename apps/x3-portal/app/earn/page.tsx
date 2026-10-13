'use client';

import React, { useState } from 'react';
import Link from 'next/link';
import {
  ArrowRight,
  Zap,
  Gift,
  Trophy,
  Users,
  Rocket,
  Star,
  Coins,
  ArrowUpRight,
  ChevronDown,
  ChevronUp,
  Wallet,
  TrendingUp,
  Target,
  Award,
  Flame,
  Sparkles,
  Shield,
  Clock,
  CheckCircle,
  Layers,
  Globe,
  Copy,
  ExternalLink,
} from 'lucide-react';

// Points multipliers for different activities
const earnActivities = [
  {
    id: 'bridge',
    icon: <Layers className="w-8 h-8" />,
    title: 'Bridge Assets',
    description: 'Transfer assets between chains and earn X3 Points for every transaction',
    pointsPerAction: '10-100',
    multiplier: '2x',
    status: 'active',
    color: 'from-orange-500 to-red-500',
    details: [
      'Base: 10 points per $100 bridged',
      'Bonus: First bridge of the day = 2x points',
      'Premium chains (ETH, SOL): 1.5x multiplier',
    ],
  },
  {
    id: 'swap',
    icon: <TrendingUp className="w-8 h-8" />,
    title: 'Swap Tokens',
    description: 'Trade on X3 DEX and accumulate points with every swap',
    pointsPerAction: '5-50',
    multiplier: '1.5x',
    status: 'active',
    color: 'from-amber-500 to-orange-500',
    details: [
      'Base: 5 points per $100 swapped',
      'Cross-VM swaps: 2x points',
      'New pair discovery: +25 bonus points',
    ],
  },
  {
    id: 'stake',
    icon: <Coins className="w-8 h-8" />,
    title: 'Stake X3',
    description: 'Lock your X3 tokens and earn continuous points rewards',
    pointsPerAction: '1/hour',
    multiplier: '3x',
    status: 'active',
    color: 'from-emerald-500 to-teal-500',
    details: [
      '1 point per X3 staked per hour',
      '30-day lock: 2x multiplier',
      '90-day lock: 3x multiplier',
    ],
  },
  {
    id: 'lp',
    icon: <Target className="w-8 h-8" />,
    title: 'Provide Liquidity',
    description: 'Add liquidity to X3 pools and earn boosted points',
    pointsPerAction: '2/hour',
    multiplier: '4x',
    status: 'active',
    color: 'from-blue-500 to-cyan-500',
    details: [
      '2 points per $100 LP per hour',
      'Core pairs (X3/ETH, X3/USDC): 2x boost',
      'New pool pioneer: +100 bonus points',
    ],
  },
  {
    id: 'referral',
    icon: <Users className="w-8 h-8" />,
    title: 'Refer Friends',
    description: 'Share your referral code and earn from friend activities',
    pointsPerAction: '10%',
    multiplier: '∞',
    status: 'active',
    color: 'from-purple-500 to-pink-500',
    details: [
      'Earn 10% of referee\'s points forever',
      '25 bonus points per successful referral',
      'Top referrers: Monthly prize pool',
    ],
  },
  {
    id: 'quests',
    icon: <Rocket className="w-8 h-8" />,
    title: 'Complete Quests',
    description: 'Daily and weekly challenges with bonus point rewards',
    pointsPerAction: '50-500',
    multiplier: 'Varies',
    status: 'coming-soon',
    color: 'from-pink-500 to-rose-500',
    details: [
      'Daily login: 10 points',
      'Weekly volume milestone: Up to 500 points',
      'Special event quests: Limited-time bonuses',
    ],
  },
];

// Leaderboard mock data
const leaderboardData = [
  { rank: 1, address: '0x7a16...3f4d', points: 1847293, badge: '🔥', change: '+5.2%' },
  { rank: 2, address: '0x3b89...a2c1', points: 1523847, badge: '⚡', change: '+3.8%' },
  { rank: 3, address: '0x9e4f...d8b2', points: 1298456, badge: '💎', change: '+2.1%' },
  { rank: 4, address: '0x5c2a...f9e3', points: 987234, badge: '🚀', change: '+4.5%' },
  { rank: 5, address: '0x8d71...c4a6', points: 876543, badge: '⭐', change: '+1.9%' },
  { rank: 6, address: '0x2f6e...b1d8', points: 765432, badge: '', change: '+2.7%' },
  { rank: 7, address: '0x4a93...e7f5', points: 654321, badge: '', change: '+1.2%' },
  { rank: 8, address: '0x6b84...d2c9', points: 543210, badge: '', change: '+3.1%' },
];

// Stats
const globalStats = [
  { label: 'Total Points Distributed', value: '2.4B', icon: <Star className="w-5 h-5" /> },
  { label: 'Active Participants', value: '127K', icon: <Users className="w-5 h-5" /> },
  { label: 'Total Value Bridged', value: '$847M', icon: <Layers className="w-5 h-5" /> },
  { label: 'Average Daily Points', value: '12.4K', icon: <TrendingUp className="w-5 h-5" /> },
];

// FAQ data
const faqItems = [
  {
    question: 'What are X3 Points?',
    answer: 'X3 Points are loyalty rewards you earn by participating in the Atlas Sphere ecosystem. They track your engagement across bridging, swapping, staking, and other activities. Points may be used for future rewards, governance participation, and exclusive benefits within the ecosystem.',
  },
  {
    question: 'How do I start earning points?',
    answer: 'Connect your wallet to start earning! Every interaction with X3 Atlas Sphere earns you points. Bridge assets between chains, swap tokens on our DEX, stake X3, or provide liquidity to earn continuous rewards. The more active you are, the more points you accumulate.',
  },
  {
    question: 'Do points expire?',
    answer: 'No, your earned points never expire. They accumulate in your account permanently. However, multiplier bonuses and special event rewards may be time-limited, so stay active to maximize your earnings.',
  },
  {
    question: 'What can I do with my points?',
    answer: 'Points serve as a measure of your contribution to the X3 ecosystem. While specific utilities will be announced over time, points may provide access to airdrops, governance rights, exclusive features, NFT mints, and other ecosystem benefits.',
  },
  {
    question: 'How does the referral program work?',
    answer: 'Share your unique referral link with friends. When they join and start earning points, you automatically receive 10% of their earned points forever. Plus, you get a 25-point bonus for each successful referral. There\'s no limit to how many people you can refer!',
  },
  {
    question: 'Are there any risks?',
    answer: 'X3 Points do not entitle users to any guaranteed rewards, rights, or compensation. Any future distributions are discretionary and not guaranteed. Always DYOR and understand that participating in DeFi carries inherent risks.',
  },
];

// Season info
const currentSeason = {
  name: 'Season 1: Genesis',
  startDate: 'Dec 1, 2025',
  endDate: 'Feb 28, 2026',
  totalPool: '50,000,000 X3',
  daysRemaining: 79,
  progress: 23,
};

export default function EarnPage() {
  const [expandedActivity, setExpandedActivity] = useState<string | null>(null);
  const [expandedFaq, setExpandedFaq] = useState<number | null>(null);
  const [isConnected, setIsConnected] = useState(false);
  const [referralCode] = useState('X3-ATLAS-' + Math.random().toString(36).substring(2, 8).toUpperCase());

  const toggleActivity = (id: string) => {
    setExpandedActivity(expandedActivity === id ? null : id);
  };

  const toggleFaq = (index: number) => {
    setExpandedFaq(expandedFaq === index ? null : index);
  };

  const copyReferralCode = () => {
    navigator.clipboard.writeText(`https://atlas-sphere.io/earn?ref=${referralCode}`);
  };

  return (
    <div className="relative bg-black min-h-screen">
      {/* Hero Section */}
      <section className="relative pt-24 pb-16 overflow-hidden">
        {/* Background effects */}
        <div className="absolute inset-0 bg-black" />
        <div className="absolute inset-0 mesh-gradient opacity-50" />
        <div className="absolute inset-0 grid-pattern opacity-20" />
        
        {/* Animated orbs */}
        <div className="absolute top-1/4 left-1/4 w-96 h-96 bg-orange-600/15 rounded-full blur-3xl animate-float" />
        <div className="absolute bottom-1/3 right-1/4 w-80 h-80 bg-amber-500/10 rounded-full blur-3xl animate-float" style={{ animationDelay: '2s' }} />
        <div className="absolute top-1/2 right-1/3 w-64 h-64 bg-red-600/10 rounded-full blur-3xl animate-float" style={{ animationDelay: '4s' }} />

        <div className="relative z-10 container-wide">
          {/* Season Banner */}
          <div className="mb-8 p-4 rounded-2xl bg-gradient-to-r from-orange-500/10 via-red-500/10 to-amber-500/10 border border-orange-500/20">
            <div className="flex flex-col md:flex-row md:items-center md:justify-between gap-4">
              <div className="flex items-center gap-3">
                <div className="w-12 h-12 rounded-xl bg-gradient-to-br from-orange-500 to-red-600 flex items-center justify-center">
                  <Flame className="w-6 h-6 text-white" />
                </div>
                <div>
                  <h3 className="font-bold text-white">{currentSeason.name}</h3>
                  <p className="text-sm text-gray-400">{currentSeason.startDate} - {currentSeason.endDate}</p>
                </div>
              </div>
              <div className="flex items-center gap-6">
                <div className="text-center">
                  <p className="text-sm text-gray-500">Prize Pool</p>
                  <p className="font-bold text-orange-400">{currentSeason.totalPool}</p>
                </div>
                <div className="text-center">
                  <p className="text-sm text-gray-500">Days Left</p>
                  <p className="font-bold text-white">{currentSeason.daysRemaining}</p>
                </div>
                <div className="hidden sm:block w-32">
                  <div className="h-2 rounded-full bg-[#1a1a1a] overflow-hidden">
                    <div 
                      className="h-full bg-gradient-to-r from-orange-500 to-red-500 rounded-full"
                      style={{ width: `${currentSeason.progress}%` }}
                    />
                  </div>
                  <p className="text-xs text-gray-500 mt-1 text-center">{currentSeason.progress}% Complete</p>
                </div>
              </div>
            </div>
          </div>

          <div className="grid lg:grid-cols-2 gap-12 items-center">
            {/* Left Content */}
            <div>
              <div className="inline-flex items-center px-4 py-2 rounded-full bg-orange-500/10 border border-orange-500/30 mb-6">
                <Sparkles className="w-4 h-4 text-orange-400 mr-2" />
                <span className="text-sm text-orange-300">Earn Rewards • Build Your Future</span>
              </div>

              <h1 className="text-4xl md:text-6xl lg:text-7xl font-bold mb-6 tracking-tight">
                <span className="text-white">Earn </span>
                <span className="gradient-text">X3 Points</span>
              </h1>

              <p className="text-xl text-gray-400 mb-8 max-w-xl">
                Participate in the X3 Atlas Sphere ecosystem and accumulate points. 
                Bridge, swap, stake, and refer friends to maximize your rewards.
              </p>

              {/* Quick Stats */}
              <div className="grid grid-cols-2 gap-4 mb-8">
                {globalStats.map((stat, i) => (
                  <div key={i} className="p-4 rounded-xl bg-[#111111] border border-[#1a1a1a]">
                    <div className="flex items-center gap-2 text-gray-500 mb-1">
                      {stat.icon}
                      <span className="text-xs">{stat.label}</span>
                    </div>
                    <p className="text-2xl font-bold gradient-text">{stat.value}</p>
                  </div>
                ))}
              </div>

              {/* CTA Buttons */}
              <div className="flex flex-wrap gap-4">
                <button 
                  onClick={() => setIsConnected(!isConnected)}
                  className="btn-primary flex items-center text-lg"
                >
                  <Wallet className="mr-2 w-5 h-5" />
                  {isConnected ? 'Connected' : 'Connect Wallet'}
                </button>
                <Link href="/swap" className="btn-secondary flex items-center text-lg">
                  Start Earning
                  <ArrowRight className="ml-2 w-5 h-5" />
                </Link>
              </div>
            </div>

            {/* Right Content - Points Dashboard */}
            <div className="glass-card p-6 md:p-8">
              <div className="text-center mb-6">
                <div className="inline-flex items-center justify-center w-20 h-20 rounded-2xl bg-gradient-to-br from-orange-500 to-red-600 mb-4 shadow-lg shadow-orange-500/25 animate-glow">
                  <Trophy className="w-10 h-10 text-white" />
                </div>
                <h2 className="text-2xl font-bold text-white mb-1">Your Points</h2>
                <p className="text-gray-500 text-sm">Connect wallet to see your balance</p>
              </div>

              {isConnected ? (
                <>
                  <div className="text-center mb-6">
                    <p className="text-5xl font-bold gradient-text mb-2">12,458</p>
                    <p className="text-sm text-gray-400">
                      Rank #847 of 127,000 participants
                    </p>
                  </div>

                  <div className="grid grid-cols-3 gap-3 mb-6">
                    <div className="p-3 rounded-xl bg-[#0a0a0a] border border-[#1a1a1a] text-center">
                      <p className="text-xs text-gray-500 mb-1">Today</p>
                      <p className="font-bold text-emerald-400">+245</p>
                    </div>
                    <div className="p-3 rounded-xl bg-[#0a0a0a] border border-[#1a1a1a] text-center">
                      <p className="text-xs text-gray-500 mb-1">This Week</p>
                      <p className="font-bold text-orange-400">+1,847</p>
                    </div>
                    <div className="p-3 rounded-xl bg-[#0a0a0a] border border-[#1a1a1a] text-center">
                      <p className="text-xs text-gray-500 mb-1">Multiplier</p>
                      <p className="font-bold text-purple-400">2.5x</p>
                    </div>
                  </div>

                  {/* Referral Code */}
                  <div className="p-4 rounded-xl bg-[#0a0a0a] border border-orange-500/20">
                    <p className="text-xs text-gray-500 mb-2">Your Referral Code</p>
                    <div className="flex items-center gap-2">
                      <code className="flex-1 text-orange-400 font-mono text-sm">{referralCode}</code>
                      <button 
                        onClick={copyReferralCode}
                        className="p-2 rounded-lg bg-[#1a1a1a] hover:bg-[#252525] transition-colors"
                      >
                        <Copy className="w-4 h-4 text-gray-400" />
                      </button>
                    </div>
                    <p className="text-xs text-gray-600 mt-2">Share to earn 10% of friend's points</p>
                  </div>
                </>
              ) : (
                <div className="space-y-4">
                  <div className="p-4 rounded-xl bg-[#0a0a0a] border border-dashed border-[#333333] text-center">
                    <p className="text-gray-500 text-sm">Connect your wallet to view your points balance and start earning</p>
                  </div>
                  <div className="grid grid-cols-3 gap-3">
                    <div className="p-3 rounded-xl bg-[#0a0a0a] border border-[#1a1a1a] text-center">
                      <p className="text-xs text-gray-500 mb-1">Bridge</p>
                      <p className="font-bold text-gray-600">--</p>
                    </div>
                    <div className="p-3 rounded-xl bg-[#0a0a0a] border border-[#1a1a1a] text-center">
                      <p className="text-xs text-gray-500 mb-1">Swap</p>
                      <p className="font-bold text-gray-600">--</p>
                    </div>
                    <div className="p-3 rounded-xl bg-[#0a0a0a] border border-[#1a1a1a] text-center">
                      <p className="text-xs text-gray-500 mb-1">Stake</p>
                      <p className="font-bold text-gray-600">--</p>
                    </div>
                  </div>
                </div>
              )}
            </div>
          </div>
        </div>
      </section>

      {/* Ways to Earn Section */}
      <section className="py-20 relative">
        <div className="absolute inset-0 bg-gradient-to-b from-black via-[#050505] to-black" />
        
        <div className="relative z-10 container-wide">
          <div className="text-center mb-12">
            <h2 className="text-3xl md:text-4xl font-bold mb-4">
              <span className="text-white">Ways to </span>
              <span className="gradient-text">Earn Points</span>
            </h2>
            <p className="text-lg text-gray-500 max-w-2xl mx-auto">
              Multiple pathways to accumulate X3 Points. Stack multipliers and maximize your rewards.
            </p>
          </div>

          <div className="grid md:grid-cols-2 lg:grid-cols-3 gap-6">
            {earnActivities.map((activity) => (
              <div
                key={activity.id}
                className={`glass-card overflow-hidden transition-all duration-300 ${
                  expandedActivity === activity.id ? 'ring-2 ring-orange-500/50' : ''
                } ${activity.status === 'coming-soon' ? 'opacity-60' : ''}`}
              >
                <div 
                  className="p-6 cursor-pointer"
                  onClick={() => toggleActivity(activity.id)}
                >
                  <div className="flex items-start justify-between mb-4">
                    <div className={`p-3 rounded-xl bg-gradient-to-br ${activity.color} bg-opacity-20`}>
                      <span className="text-white">{activity.icon}</span>
                    </div>
                    <div className="flex items-center gap-2">
                      {activity.status === 'coming-soon' ? (
                        <span className="badge bg-gray-500/20 text-gray-400 border-gray-500/30">Soon</span>
                      ) : (
                        <span className="badge badge-fire">{activity.multiplier}</span>
                      )}
                    </div>
                  </div>

                  <h3 className="text-xl font-bold text-white mb-2">{activity.title}</h3>
                  <p className="text-gray-500 text-sm mb-4">{activity.description}</p>

                  <div className="flex items-center justify-between">
                    <div>
                      <p className="text-xs text-gray-600">Points per action</p>
                      <p className="font-semibold text-orange-400">{activity.pointsPerAction}</p>
                    </div>
                    <button className="text-gray-400 hover:text-white transition-colors">
                      {expandedActivity === activity.id ? (
                        <ChevronUp className="w-5 h-5" />
                      ) : (
                        <ChevronDown className="w-5 h-5" />
                      )}
                    </button>
                  </div>
                </div>

                {/* Expanded Details */}
                {expandedActivity === activity.id && (
                  <div className="px-6 pb-6 border-t border-[#1a1a1a] pt-4">
                    <ul className="space-y-2">
                      {activity.details.map((detail, i) => (
                        <li key={i} className="flex items-start gap-2">
                          <CheckCircle className="w-4 h-4 text-orange-500 flex-shrink-0 mt-0.5" />
                          <span className="text-sm text-gray-400">{detail}</span>
                        </li>
                      ))}
                    </ul>
                    {activity.status === 'active' && (
                      <Link
                        href={activity.id === 'bridge' ? '/bridge' : activity.id === 'swap' ? '/swap' : `/${activity.id}`}
                        className="mt-4 w-full btn-secondary flex items-center justify-center text-sm"
                      >
                        Start Earning
                        <ArrowRight className="ml-2 w-4 h-4" />
                      </Link>
                    )}
                  </div>
                )}
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Leaderboard Section */}
      <section className="py-20 relative bg-black">
        <div className="container-wide">
          <div className="grid lg:grid-cols-3 gap-8">
            {/* Leaderboard Table */}
            <div className="lg:col-span-2">
              <div className="flex items-center justify-between mb-6">
                <div>
                  <h2 className="text-2xl md:text-3xl font-bold text-white">Leaderboard</h2>
                  <p className="text-gray-500">Top earners this season</p>
                </div>
                <Link href="/leaderboard" className="text-orange-400 hover:text-orange-300 flex items-center text-sm">
                  View All <ArrowUpRight className="ml-1 w-4 h-4" />
                </Link>
              </div>

              <div className="glass-card overflow-hidden">
                <div className="overflow-x-auto">
                  <table className="w-full">
                    <thead>
                      <tr className="border-b border-[#1a1a1a]">
                        <th className="table-header">Rank</th>
                        <th className="table-header">Address</th>
                        <th className="table-header text-right">Points</th>
                        <th className="table-header text-right">24h Change</th>
                      </tr>
                    </thead>
                    <tbody>
                      {leaderboardData.map((entry) => (
                        <tr key={entry.rank} className="border-b border-[#1a1a1a]/50 hover:bg-[#111111] transition-colors">
                          <td className="table-cell">
                            <div className="flex items-center gap-2">
                              {entry.rank <= 3 ? (
                                <span className={`w-8 h-8 rounded-lg flex items-center justify-center font-bold ${
                                  entry.rank === 1 ? 'bg-gradient-to-br from-amber-400 to-orange-500 text-white' :
                                  entry.rank === 2 ? 'bg-gradient-to-br from-gray-300 to-gray-400 text-gray-800' :
                                  'bg-gradient-to-br from-amber-600 to-amber-700 text-white'
                                }`}>
                                  {entry.rank}
                                </span>
                              ) : (
                                <span className="w-8 h-8 rounded-lg bg-[#1a1a1a] flex items-center justify-center text-gray-400">
                                  {entry.rank}
                                </span>
                              )}
                              {entry.badge && <span className="text-lg">{entry.badge}</span>}
                            </div>
                          </td>
                          <td className="table-cell">
                            <span className="font-mono text-gray-300">{entry.address}</span>
                          </td>
                          <td className="table-cell text-right">
                            <span className="font-semibold text-white">{entry.points.toLocaleString()}</span>
                          </td>
                          <td className="table-cell text-right">
                            <span className="text-emerald-400">{entry.change}</span>
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>
            </div>

            {/* Side Stats */}
            <div className="space-y-6">
              {/* Your Rank Card */}
              <div className="glass-card p-6">
                <h3 className="font-semibold text-white mb-4 flex items-center gap-2">
                  <Award className="w-5 h-5 text-orange-400" />
                  Your Ranking
                </h3>
                {isConnected ? (
                  <div className="text-center py-4">
                    <p className="text-4xl font-bold text-white mb-1">#847</p>
                    <p className="text-sm text-gray-500">of 127,000 participants</p>
                    <div className="mt-4 p-3 rounded-xl bg-[#0a0a0a] border border-emerald-500/20">
                      <p className="text-xs text-gray-500">Points to #846</p>
                      <p className="font-semibold text-emerald-400">+124 points</p>
                    </div>
                  </div>
                ) : (
                  <div className="text-center py-6">
                    <p className="text-gray-500 text-sm">Connect wallet to see your rank</p>
                  </div>
                )}
              </div>

              {/* Multiplier Boosts */}
              <div className="glass-card p-6">
                <h3 className="font-semibold text-white mb-4 flex items-center gap-2">
                  <Zap className="w-5 h-5 text-amber-400" />
                  Active Boosts
                </h3>
                <div className="space-y-3">
                  <div className="flex items-center justify-between p-3 rounded-xl bg-[#0a0a0a] border border-[#1a1a1a]">
                    <span className="text-sm text-gray-400">Early Adopter</span>
                    <span className="badge badge-fire">1.5x</span>
                  </div>
                  <div className="flex items-center justify-between p-3 rounded-xl bg-[#0a0a0a] border border-[#1a1a1a]">
                    <span className="text-sm text-gray-400">7-Day Streak</span>
                    <span className="badge badge-success">+10%</span>
                  </div>
                  <div className="flex items-center justify-between p-3 rounded-xl bg-[#0a0a0a] border border-dashed border-[#333333]">
                    <span className="text-sm text-gray-600">Weekend Bonus</span>
                    <span className="text-xs text-gray-600">Sat-Sun</span>
                  </div>
                </div>
              </div>

              {/* Quick Actions */}
              <div className="glass-card p-6">
                <h3 className="font-semibold text-white mb-4 flex items-center gap-2">
                  <Rocket className="w-5 h-5 text-purple-400" />
                  Quick Actions
                </h3>
                <div className="space-y-2">
                  <Link href="/swap" className="w-full btn-secondary flex items-center justify-center text-sm py-2">
                    <TrendingUp className="mr-2 w-4 h-4" />
                    Swap Now
                  </Link>
                  <Link href="/stake" className="w-full btn-secondary flex items-center justify-center text-sm py-2">
                    <Coins className="mr-2 w-4 h-4" />
                    Stake X3
                  </Link>
                  <Link href="/bridge" className="w-full btn-secondary flex items-center justify-center text-sm py-2">
                    <Layers className="mr-2 w-4 h-4" />
                    Bridge Assets
                  </Link>
                </div>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* How It Works */}
      <section className="py-20 relative overflow-hidden">
        <div className="absolute inset-0 bg-gradient-to-b from-black via-[#080505] to-black" />
        
        <div className="relative z-10 container-wide">
          <div className="text-center mb-12">
            <h2 className="text-3xl md:text-4xl font-bold mb-4">
              <span className="text-white">How It </span>
              <span className="gradient-text">Works</span>
            </h2>
            <p className="text-lg text-gray-500 max-w-2xl mx-auto">
              Three simple steps to start earning X3 Points
            </p>
          </div>

          <div className="grid md:grid-cols-3 gap-8">
            {[
              {
                step: 1,
                icon: <Wallet className="w-8 h-8" />,
                title: 'Connect Wallet',
                description: 'Link your Web3 wallet to start tracking your activity and accumulating points automatically.',
              },
              {
                step: 2,
                icon: <Globe className="w-8 h-8" />,
                title: 'Use the Platform',
                description: 'Bridge assets, swap tokens, stake X3, or provide liquidity. Every action earns you points.',
              },
              {
                step: 3,
                icon: <Gift className="w-8 h-8" />,
                title: 'Claim Rewards',
                description: 'Watch your points grow and unlock exclusive benefits, airdrops, and ecosystem privileges.',
              },
            ].map((item) => (
              <div key={item.step} className="relative">
                <div className="glass-card p-8 text-center h-full">
                  <div className="absolute -top-4 left-1/2 -translate-x-1/2 w-8 h-8 rounded-full bg-gradient-to-br from-orange-500 to-red-600 flex items-center justify-center font-bold text-white text-sm">
                    {item.step}
                  </div>
                  <div className="w-16 h-16 rounded-2xl bg-gradient-to-br from-orange-500/20 to-red-500/20 flex items-center justify-center mx-auto mb-4 mt-2">
                    <span className="text-orange-400">{item.icon}</span>
                  </div>
                  <h3 className="text-xl font-bold text-white mb-2">{item.title}</h3>
                  <p className="text-gray-500">{item.description}</p>
                </div>
                {item.step < 3 && (
                  <div className="hidden md:block absolute top-1/2 -right-4 w-8 text-orange-500/30">
                    <ArrowRight className="w-8 h-8" />
                  </div>
                )}
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* FAQ Section */}
      <section className="py-20 relative bg-black">
        <div className="container-wide">
          <div className="max-w-3xl mx-auto">
            <div className="text-center mb-12">
              <h2 className="text-3xl md:text-4xl font-bold mb-4">
                <span className="text-white">Frequently Asked </span>
                <span className="gradient-text">Questions</span>
              </h2>
              <p className="text-lg text-gray-500">
                Everything you need to know about X3 Points
              </p>
            </div>

            <div className="space-y-4">
              {faqItems.map((item, index) => (
                <div key={index} className="glass-card overflow-hidden">
                  <button
                    onClick={() => toggleFaq(index)}
                    className="w-full p-6 flex items-center justify-between text-left"
                  >
                    <span className="font-semibold text-white pr-4">{item.question}</span>
                    {expandedFaq === index ? (
                      <ChevronUp className="w-5 h-5 text-orange-400 flex-shrink-0" />
                    ) : (
                      <ChevronDown className="w-5 h-5 text-gray-400 flex-shrink-0" />
                    )}
                  </button>
                  {expandedFaq === index && (
                    <div className="px-6 pb-6 border-t border-[#1a1a1a] pt-4">
                      <p className="text-gray-400">{item.answer}</p>
                    </div>
                  )}
                </div>
              ))}
            </div>

            <div className="text-center mt-8">
              <Link href="/faq" className="text-orange-400 hover:text-orange-300 flex items-center justify-center">
                View all FAQs <ArrowUpRight className="ml-1 w-4 h-4" />
              </Link>
            </div>
          </div>
        </div>
      </section>

      {/* CTA Section */}
      <section className="py-20 relative overflow-hidden">
        <div className="absolute inset-0 bg-gradient-to-b from-black via-[#0a0505] to-black" />
        <div className="absolute inset-0 mesh-gradient opacity-30" />
        
        <div className="relative z-10 max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 text-center">
          <div className="glass-card p-8 md:p-12">
            <div className="w-20 h-20 rounded-2xl bg-gradient-to-br from-orange-500 to-red-600 mx-auto mb-6 flex items-center justify-center shadow-lg shadow-orange-500/25 animate-glow">
              <Rocket className="w-10 h-10 text-white" />
            </div>
            <h2 className="text-3xl md:text-4xl font-bold mb-4">
              <span className="text-white">Ready to </span>
              <span className="gradient-text">Start Earning?</span>
            </h2>
            <p className="text-xl text-gray-400 mb-8 max-w-2xl mx-auto">
              Join thousands of users earning X3 Points every day. 
              Connect your wallet and start accumulating rewards now.
            </p>
            <div className="flex flex-col sm:flex-row items-center justify-center gap-4">
              <button 
                onClick={() => setIsConnected(!isConnected)}
                className="btn-primary text-lg px-8 py-4 flex items-center"
              >
                <Wallet className="mr-2 w-5 h-5" />
                {isConnected ? 'Wallet Connected' : 'Connect Wallet'}
              </button>
              <Link href="/developers/docs" className="btn-secondary text-lg px-8 py-4">
                Learn More
              </Link>
            </div>
          </div>
        </div>
      </section>

      {/* Disclaimer Footer */}
      <section className="py-8 border-t border-[#1a1a1a]">
        <div className="container-wide">
          <div className="flex flex-col md:flex-row items-center justify-between gap-4 text-xs text-gray-600">
            <p className="text-center md:text-left">
              <Shield className="w-4 h-4 inline mr-1" />
              X3 Points do not entitle users to any rewards, rights, or compensation. Any future distributions are discretionary and not guaranteed.
            </p>
            <div className="flex items-center gap-4">
              <Link href="/privacy" className="hover:text-gray-400 transition-colors">Privacy Policy</Link>
              <Link href="/terms" className="hover:text-gray-400 transition-colors">Terms of Service</Link>
              <Link href="/faq" className="hover:text-gray-400 transition-colors">FAQs</Link>
            </div>
          </div>
        </div>
      </section>
    </div>
  );
}
