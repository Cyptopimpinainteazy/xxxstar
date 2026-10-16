'use client';

import React, { useState, useEffect } from 'react';
import { 
  DollarSign,
  PieChart,
  TrendingUp,
  Users,
  Building,
  Shield,
  ArrowRight,
  PieChart as PieChartIcon,
  BarChart3,
  Target,
  Activity,
  Clock,
  CheckCircle,
  RefreshCw,
  ExternalLink,
  Wallet,
  Coins,
  Zap,
  Globe,
  Crown,
  Star,
  ArrowUpRight,
  Download,
  Upload,
  Settings
} from 'lucide-react';

// Treasury distribution data
const distributionData = [
  { category: 'DAO Treasury', percentage: 40, amount: '$35.6M', color: 'bg-blue-500', description: 'Community governance & development' },
  { category: 'Development Fund', percentage: 20, amount: '$17.8M', color: 'bg-green-500', description: 'Core development & innovation' },
  { category: 'Marketing', percentage: 10, amount: '$8.9M', color: 'bg-purple-500', description: 'Growth & adoption' },
  { category: 'Liquidity Pool', percentage: 15, amount: '$13.35M', color: 'bg-orange-500', description: 'Market stability' },
  { category: 'Buyback & Burn', percentage: 10, amount: '$8.9M', color: 'bg-pink-500', description: 'Token value enhancement' },
  { category: 'Insurance Fund', percentage: 5, amount: '$4.45M', color: 'bg-red-500', description: 'Risk mitigation' }
];

// Treasury metrics
const treasuryMetrics = [
  { label: 'Total Treasury Value', value: '$89.0M', change: '+12.4%', icon: DollarSign, trend: 'up' },
  { label: 'Daily Revenue', value: '$127K', change: '+8.7%', icon: TrendingUp, trend: 'up' },
  { label: 'Active Distributions', value: '6', change: '0', icon: Activity, trend: 'neutral' },
  { label: 'DAO Participation', value: '94.7%', change: '+2.1%', icon: Users, trend: 'up' },
  { label: 'Burn Rate', value: '$340K', change: '-5.2%', icon: Zap, trend: 'down' },
  { label: 'Auto Distribution', value: '100%', change: 'Automated', icon: Settings, trend: 'up' }
];

// Recent transactions
const recentTransactions = [
  { id: 1, type: 'distribution', description: 'Monthly DAO treasury allocation', amount: '$3.56M', recipient: 'DAO Treasury', time: '2 hours ago', status: 'completed' },
  { id: 2, type: 'buyback', description: 'Automated token buyback & burn', amount: '$89K', recipient: 'Burn Address', time: '4 hours ago', status: 'completed' },
  { id: 3, type: 'marketing', description: 'Marketing campaign funding', amount: '$890K', recipient: 'Marketing Dept', time: '6 hours ago', status: 'completed' },
  { id: 4, type: 'development', description: 'Core development fund', amount: '$1.78M', recipient: 'Dev Fund', time: '8 hours ago', status: 'pending' },
  { id: 5, type: 'insurance', description: 'Insurance fund top-up', amount: '$445K', recipient: 'Insurance Pool', time: '12 hours ago', status: 'completed' }
];

// Protocol revenue streams
const revenueStreams = [
  { protocol: 'Cross-Chain Bridge', revenue: '$45.2K', fee: '0.1%', volume: '$45.2M', growth: '+15.3%' },
  { protocol: 'Lending Protocol', revenue: '$38.7K', fee: '0.15%', volume: '$25.8M', growth: '+8.9%' },
  { protocol: 'AI Swarm', revenue: '$28.4K', fee: '0.05%', volume: '$56.8M', growth: '+22.1%' },
  { protocol: 'Launchpad', revenue: '$14.7K', fee: '2.5%', volume: '$588K', growth: '+5.2%' }
];

export default function TreasuryPage() {
  const [autoDistribution, setAutoDistribution] = useState(true);
  const [lastUpdate, setLastUpdate] = useState(new Date());

  // Auto-refresh every 30 seconds
  useEffect(() => {
    const interval = setInterval(() => {
      setLastUpdate(new Date());
    }, 30000);

    return () => clearInterval(interval);
  }, []);

  const getTransactionIcon = (type: string) => {
    switch (type) {
      case 'distribution': return <PieChartIcon className="w-5 h-5 text-blue-400" />;
      case 'buyback': return <Zap className="w-5 h-5 text-pink-400" />;
      case 'marketing': return <TrendingUp className="w-5 h-5 text-purple-400" />;
      case 'development': return <Building className="w-5 h-5 text-green-400" />;
      case 'insurance': return <Shield className="w-5 h-5 text-red-400" />;
      default: return <Activity className="w-5 h-5 text-gray-400" />;
    }
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'completed': return 'text-green-400';
      case 'pending': return 'text-yellow-400';
      case 'failed': return 'text-red-400';
      default: return 'text-gray-400';
    }
  };

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'completed': return <CheckCircle className="w-4 h-4 text-green-400" />;
      case 'pending': return <Clock className="w-4 h-4 text-yellow-400" />;
      case 'failed': return <RefreshCw className="w-4 h-4 text-red-400" />;
      default: return <Activity className="w-4 h-4 text-gray-400" />;
    }
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-slate-900 via-purple-900 to-slate-900">
      {/* Hero Section */}
      <div className="relative overflow-hidden">
        <div className="absolute inset-0 bg-gradient-to-r from-purple-600/20 to-blue-600/20 backdrop-blur-3xl"></div>
        <div className="relative max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 pt-20 pb-16">
          <div className="flex items-center justify-between">
            <div>
              <h1 className="text-4xl md:text-6xl font-bold bg-gradient-to-r from-white via-purple-200 to-blue-200 bg-clip-text text-transparent mb-4">
                Treasury Management
              </h1>
              <p className="text-xl text-gray-300 mb-6">
                Automated fee distribution across ecosystem participants
              </p>
              <div className="flex items-center gap-4">
                <div className="flex items-center gap-2 text-green-400">
                  <div className="w-2 h-2 bg-green-400 rounded-full animate-pulse"></div>
                  <span className="text-sm font-medium">Auto Distribution Active</span>
                </div>
                <div className="text-gray-400 text-sm">
                  Last updated: {lastUpdate.toLocaleTimeString()}
                </div>
              </div>
            </div>
            <div className="flex items-center gap-4">
              <button
                onClick={() => setAutoDistribution(!autoDistribution)}
                className={`px-4 py-2 rounded-xl font-semibold transition-all flex items-center gap-2 ${
                  autoDistribution 
                    ? 'bg-green-600 text-white' 
                    : 'bg-slate-700 text-gray-300 hover:bg-slate-600'
                }`}
              >
                {autoDistribution ? <Crown className="w-4 h-4" /> : <Settings className="w-4 h-4" />}
                {autoDistribution ? 'Auto ON' : 'Auto OFF'}
              </button>
              <button className="bg-gradient-to-r from-purple-600 to-blue-600 text-white px-6 py-3 rounded-xl font-semibold hover:from-purple-700 hover:to-blue-700 transition-all transform hover:scale-105 flex items-center gap-2">
                <Settings className="w-5 h-5" />
                Configure
              </button>
            </div>
          </div>
        </div>
      </div>

      {/* Treasury Metrics */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12">
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-6 gap-6">
          {treasuryMetrics.map((metric, index) => (
            <div 
              key={index}
              className="bg-gradient-to-br from-slate-800/50 to-slate-700/50 backdrop-blur-xl rounded-2xl p-6 border border-slate-600/30 text-center hover:border-purple-500/50 transition-all duration-300 hover:transform hover:scale-105"
            >
              <metric.icon className="w-12 h-12 text-purple-400 mx-auto mb-4" />
              <div className="text-2xl md:text-3xl font-bold text-white mb-2">{metric.value}</div>
              <div className="text-gray-400 text-sm mb-2">{metric.label}</div>
              <div className="text-green-400 text-xs font-semibold">{metric.change}</div>
            </div>
          ))}
        </div>
      </div>

      {/* Main Dashboard */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12">
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
          
          {/* Distribution Breakdown */}
          <div className="lg:col-span-2 bg-gradient-to-br from-slate-800/50 to-slate-700/50 backdrop-blur-xl rounded-2xl p-8 border border-slate-600/30">
            <div className="flex items-center gap-3 mb-6">
              <PieChartIcon className="w-8 h-8 text-purple-400" />
              <h3 className="text-2xl font-bold text-white">Fee Distribution</h3>
            </div>
            
            <div className="space-y-6">
              {distributionData.map((item, index) => (
                <div key={index} className="space-y-3">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-3">
                      <div className={`w-4 h-4 ${item.color} rounded-full`}></div>
                      <span className="text-white font-semibold">{item.category}</span>
                    </div>
                    <div className="text-right">
                      <div className="text-white font-bold">{item.amount}</div>
                      <div className="text-gray-400 text-sm">{item.percentage}%</div>
                    </div>
                  </div>
                  <div className="w-full bg-slate-700 rounded-full h-3">
                    <div 
                      className={`${item.color} h-3 rounded-full transition-all duration-1000`}
                      style={{ width: `${item.percentage}%` }}
                    ></div>
                  </div>
                  <div className="text-gray-400 text-sm">{item.description}</div>
                </div>
              ))}
            </div>

            <div className="mt-8 p-4 bg-slate-800/30 rounded-xl border border-slate-600/20">
              <div className="flex items-center justify-between">
                <div>
                  <h4 className="text-white font-semibold">Total Treasury Value</h4>
                  <p className="text-gray-400 text-sm">Across all distribution categories</p>
                </div>
                <div className="text-right">
                  <div className="text-3xl font-bold text-green-400">$89.0M</div>
                  <div className="text-green-400 text-sm">+12.4% this month</div>
                </div>
              </div>
            </div>
          </div>

          {/* Recent Transactions */}
          <div className="bg-gradient-to-br from-slate-800/50 to-slate-700/50 backdrop-blur-xl rounded-2xl p-8 border border-slate-600/30">
            <div className="flex items-center gap-3 mb-6">
              <Activity className="w-8 h-8 text-purple-400" />
              <h3 className="text-2xl font-bold text-white">Recent Activity</h3>
            </div>
            
            <div className="space-y-4 max-h-96 overflow-y-auto">
              {recentTransactions.map((tx) => (
                <div 
                  key={tx.id}
                  className="flex items-center justify-between p-4 bg-slate-800/30 rounded-xl border border-slate-600/20 hover:border-purple-500/30 transition-all"
                >
                  <div className="flex items-center gap-3">
                    {getTransactionIcon(tx.type)}
                    <div>
                      <div className="text-white font-medium text-sm">{tx.description}</div>
                      <div className="text-gray-400 text-xs">{tx.time}</div>
                    </div>
                  </div>
                  <div className="text-right">
                    <div className="text-white font-bold">{tx.amount}</div>
                    <div className="flex items-center gap-1">
                      {getStatusIcon(tx.status)}
                      <span className={`text-xs ${getStatusColor(tx.status)}`}>
                        {tx.status}
                      </span>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>

      {/* Revenue Streams */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12">
        <h2 className="text-3xl font-bold text-center text-white mb-8">Protocol Revenue Streams</h2>
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
          {revenueStreams.map((stream, index) => (
            <div 
              key={index}
              className="bg-gradient-to-br from-slate-800/50 to-slate-700/50 backdrop-blur-xl rounded-2xl p-6 border border-slate-600/30 hover:border-purple-500/50 transition-all duration-300 hover:transform hover:scale-105"
            >
              <div className="flex items-center justify-between mb-4">
                <Globe className="w-8 h-8 text-purple-400" />
                <div className="flex items-center gap-1 text-green-400">
                  <ArrowUpRight className="w-4 h-4" />
                  <span className="text-sm font-semibold">{stream.growth}</span>
                </div>
              </div>
              
              <h3 className="text-xl font-bold text-white mb-4">{stream.protocol}</h3>
              
              <div className="space-y-3">
                <div className="flex justify-between">
                  <span className="text-gray-400">Daily Revenue</span>
                  <span className="text-white font-bold">{stream.revenue}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Fee Rate</span>
                  <span className="text-purple-400 font-semibold">{stream.fee}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Volume</span>
                  <span className="text-white font-bold">{stream.volume}</span>
                </div>
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Quick Actions */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12">
        <h2 className="text-3xl font-bold text-center text-white mb-8">Treasury Actions</h2>
        <div className="grid grid-cols-1 md:grid-cols-4 gap-6">
          
          <div className="bg-gradient-to-br from-slate-800/50 to-slate-700/50 backdrop-blur-xl rounded-2xl p-8 border border-slate-600/30 text-center hover:border-purple-500/50 transition-all hover:transform hover:scale-105 cursor-pointer">
            <div className="w-16 h-16 bg-gradient-to-r from-blue-500 to-cyan-500 rounded-xl flex items-center justify-center mb-6 mx-auto">
              <Download className="w-8 h-8 text-white" />
            </div>
            <h3 className="text-xl font-bold text-white mb-4">Export Treasury Data</h3>
            <p className="text-gray-400 mb-6">Download financial reports</p>
            <div className="text-cyan-400 font-semibold">Data Export</div>
          </div>
        </div>
      </div>
    </div>
  );
}
