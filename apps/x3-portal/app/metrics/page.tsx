'use client';

import React, { useState, useEffect } from 'react';
import { 
  BarChart3,
  TrendingUp,
  TrendingDown,
  Activity,
  Zap,
  DollarSign,
  Users,
  Globe,
  Target,
  ArrowUp,
  ArrowDown,
  Minus,
  RefreshCw,
  Clock,
  Shield,
  Coins,
  PieChart,
  LineChart,
  Gauge,
  CheckCircle,
  AlertTriangle,
  Play,
  Pause,
  Settings
} from 'lucide-react';

// Live DeFi metrics
const liveMetrics = [
  { label: 'Total Volume (24h)', value: '$847.3M', change: '+23.7%', trend: 'up', icon: DollarSign },
  { label: 'Active Protocols', value: '47', change: '+3', trend: 'up', icon: Activity },
  { label: 'Cross-chain Swaps', value: '12,847', change: '+18.2%', trend: 'up', icon: Globe },
  { label: 'AI Optimizations', value: '1,247', change: '+31.5%', trend: 'up', icon: Zap },
  { label: 'MEV Protected', value: '$5.7B', change: '+8.9%', trend: 'up', icon: Shield },
  { label: 'Success Rate', value: '99.97%', change: '+0.02%', trend: 'up', icon: Target }
];

// Protocol performance data
const protocolPerformance = [
  { name: 'Cross-Chain Bridge', volume: '$247M', apy: '0.1%', users: '34.2K', growth: '+15.3%', status: 'excellent' },
  { name: 'Lending Protocol', volume: '$189M', apy: '8.4%', users: '28.7K', growth: '+8.9%', status: 'good' },
  { name: 'AI Swarm', volume: '$156M', apy: '12.3%', users: '18.9K', growth: '+22.1%', status: 'excellent' },
  { name: 'Launchpad', volume: '$89M', apy: '15.7%', users: '12.4K', growth: '+5.2%', status: 'good' },
  { name: 'Staking Engine', volume: '$134M', apy: '11.2%', users: '21.8K', growth: '+12.4%', status: 'excellent' },
  { name: 'Yield Optimizer', volume: '$67M', apy: '9.8%', users: '15.6K', growth: '+7.1%', status: 'good' }
];

// Market trends data
const marketTrends = [
  { metric: 'DeFi TVL', current: '$2.8B', change: '+12.4%', icon: TrendingUp },
  { metric: 'Active Users', current: '127K', change: '+15.2%', icon: Users },
  { metric: 'Daily Transactions', current: '47.2K', change: '+8.7%', icon: Activity },
  { metric: 'Average APY', current: '11.3%', change: '+0.8%', icon: Target },
  { metric: 'Security Score', current: '99.97%', change: '+0.02%', icon: Shield },
  { metric: 'Uptime', current: '99.99%', change: 'Perfect', icon: CheckCircle }
];

// Chain distribution
const chainDistribution = [
  { chain: 'Ethereum', volume: '$234M', percentage: 27.6, color: 'bg-blue-500' },
  { chain: 'Arbitrum', volume: '$189M', percentage: 22.3, color: 'bg-blue-400' },
  { chain: 'Polygon', volume: '$156M', percentage: 18.4, color: 'bg-purple-500' },
  { chain: 'Optimism', volume: '$134M', percentage: 15.8, color: 'bg-red-500' },
  { chain: 'Base', volume: '$89M', percentage: 10.5, color: 'bg-blue-600' },
  { chain: 'Others', volume: '$45M', percentage: 5.4, color: 'bg-gray-500' }
];

export default function MetricsPage() {
  const [autoRefresh, setAutoRefresh] = useState(true);
  const [selectedTimeframe, setSelectedTimeframe] = useState('24h');
  const [lastUpdate, setLastUpdate] = useState(new Date());

  // Auto-refresh every 10 seconds
  useEffect(() => {
    if (!autoRefresh) return;
    
    const interval = setInterval(() => {
      setLastUpdate(new Date());
    }, 10000);

    return () => clearInterval(interval);
  }, [autoRefresh]);

  const getTrendIcon = (trend: string) => {
    switch (trend) {
      case 'up': return <ArrowUp className="w-4 h-4 text-green-400" />;
      case 'down': return <ArrowDown className="w-4 h-4 text-red-400" />;
      default: return <Minus className="w-4 h-4 text-gray-400" />;
    }
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'excellent': return 'text-green-400';
      case 'good': return 'text-blue-400';
      case 'warning': return 'text-yellow-400';
      case 'poor': return 'text-red-400';
      default: return 'text-gray-400';
    }
  };

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'excellent': return <CheckCircle className="w-4 h-4 text-green-400" />;
      case 'good': return <CheckCircle className="w-4 h-4 text-blue-400" />;
      case 'warning': return <AlertTriangle className="w-4 h-4 text-yellow-400" />;
      case 'poor': return <AlertTriangle className="w-4 h-4 text-red-400" />;
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
                Real-time Metrics
              </h1>
              <p className="text-xl text-gray-300 mb-6">
                Live DeFi ecosystem performance and analytics
              </p>
              <div className="flex items-center gap-4">
                <div className="flex items-center gap-2 text-green-400">
                  <div className="w-2 h-2 bg-green-400 rounded-full animate-pulse"></div>
                  <span className="text-sm font-medium">Live Data</span>
                </div>
                <div className="text-gray-400 text-sm">
                  Last updated: {lastUpdate.toLocaleTimeString()}
                </div>
              </div>
            </div>
            <div className="flex items-center gap-4">
              <div className="flex items-center gap-2">
                {['1h', '24h', '7d', '30d'].map((timeframe) => (
                  <button
                    key={timeframe}
                    onClick={() => setSelectedTimeframe(timeframe)}
                    className={`px-4 py-2 rounded-lg font-semibold transition-all ${
                      selectedTimeframe === timeframe
                        ? 'bg-purple-600 text-white'
                        : 'text-gray-400 hover:text-white hover:bg-slate-700/50'
                    }`}
                  >
                    {timeframe}
                  </button>
                ))}
              </div>
              <button
                onClick={() => setAutoRefresh(!autoRefresh)}
                className={`px-4 py-2 rounded-xl font-semibold transition-all flex items-center gap-2 ${
                  autoRefresh 
                    ? 'bg-green-600 text-white' 
                    : 'bg-slate-700 text-gray-300 hover:bg-slate-600'
                }`}
              >
                {autoRefresh ? <Pause className="w-4 h-4" /> : <Play className="w-4 h-4" />}
                {autoRefresh ? 'Auto ON' : 'Auto OFF'}
              </button>
            </div>
          </div>
        </div>
      </div>

      {/* Live Metrics */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12">
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-6 gap-6">
          {liveMetrics.map((metric, index) => (
            <div 
              key={index}
              className="bg-gradient-to-br from-slate-800/50 to-slate-700/50 backdrop-blur-xl rounded-2xl p-6 border border-slate-600/30 text-center hover:border-purple-500/50 transition-all duration-300 hover:transform hover:scale-105"
            >
              <metric.icon className="w-12 h-12 text-purple-400 mx-auto mb-4" />
              <div className="text-2xl md:text-3xl font-bold text-white mb-2">{metric.value}</div>
              <div className="text-gray-400 text-sm mb-2">{metric.label}</div>
              <div className="flex items-center justify-center gap-1">
                {getTrendIcon(metric.trend)}
                <span className="text-green-400 text-xs font-semibold">{metric.change}</span>
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Protocol Performance */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12">
        <h2 className="text-3xl font-bold text-center text-white mb-8">Protocol Performance</h2>
        <div className="bg-gradient-to-br from-slate-800/50 to-slate-700/50 backdrop-blur-xl rounded-2xl p-8 border border-slate-600/30">
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
            {protocolPerformance.map((protocol, index) => (
              <div 
                key={index}
                className="p-6 bg-slate-800/30 rounded-xl border border-slate-600/20 hover:border-purple-500/30 transition-all"
              >
                <div className="flex items-center justify-between mb-4">
                  <h3 className="text-xl font-bold text-white">{protocol.name}</h3>
                  {getStatusIcon(protocol.status)}
                </div>
                
                <div className="space-y-3">
                  <div className="flex justify-between">
                    <span className="text-gray-400">Volume</span>
                    <span className="text-white font-bold">{protocol.volume}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-400">APY</span>
                    <span className="text-purple-400 font-semibold">{protocol.apy}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-400">Users</span>
                    <span className="text-white font-bold">{protocol.users}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-400">Growth</span>
                    <div className="flex items-center gap-1">
                      <ArrowUp className="w-4 h-4 text-green-400" />
                      <span className="text-green-400 font-semibold">{protocol.growth}</span>
                    </div>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* Market Trends & Chain Distribution */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12">
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-8">
          
          {/* Market Trends */}
          <div className="bg-gradient-to-br from-slate-800/50 to-slate-700/50 backdrop-blur-xl rounded-2xl p-8 border border-slate-600/30">
            <div className="flex items-center gap-3 mb-6">
              <BarChart3 className="w-8 h-8 text-purple-400" />
              <h3 className="text-2xl font-bold text-white">Market Trends</h3>
            </div>
            
            <div className="space-y-6">
              {marketTrends.map((trend, index) => (
                <div key={index} className="flex items-center justify-between">
                  <div className="flex items-center gap-3">
                    <trend.icon className="w-6 h-6 text-purple-400" />
                    <span className="text-white font-medium">{trend.metric}</span>
                  </div>
                  <div className="text-right">
                    <div className="text-white font-bold">{trend.current}</div>
                    <div className="text-green-400 text-sm">{trend.change}</div>
                  </div>
                </div>
              ))}
            </div>
          </div>

          {/* Chain Distribution */}
          <div className="bg-gradient-to-br from-slate-800/50 to-slate-700/50 backdrop-blur-xl rounded-2xl p-8 border border-slate-600/30">
            <div className="flex items-center gap-3 mb-6">
              <Globe className="w-8 h-8 text-purple-400" />
              <h3 className="text-2xl font-bold text-white">Chain Distribution</h3>
            </div>
            
            <div className="space-y-4">
              {chainDistribution.map((chain, index) => (
                <div key={index} className="space-y-2">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-3">
                      <div className={`w-4 h-4 ${chain.color} rounded-full`}></div>
                      <span className="text-white font-medium">{chain.chain}</span>
                    </div>
                    <div className="text-right">
                      <div className="text-white font-bold">{chain.volume}</div>
                      <div className="text-gray-400 text-sm">{chain.percentage}%</div>
                    </div>
                  </div>
                  <div className="w-full bg-slate-700 rounded-full h-2">
                    <div 
                      className={`${chain.color} h-2 rounded-full transition-all duration-500`}
                      style={{ width: `${chain.percentage}%` }}
                    ></div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>

      {/* Quick Actions */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12">
        <h2 className="text-3xl font-bold text-center text-white mb-8">Analytics Actions</h2>
        <div className="grid grid-cols-1 md:grid-cols-4 gap-6">
          
          <div className="bg-gradient-to-br from-slate-800/50 to-slate-700/50 backdrop-blur-xl rounded-2xl p-8 border border-slate-600/30 text-center hover:border-purple-500/50 transition-all hover:transform hover:scale-105 cursor-pointer">
            <div className="w-16 h-16 bg-gradient-to-r from-blue-500 to-cyan-500 rounded-xl flex items-center justify-center mb-6 mx-auto">
              <LineChart className="w-8 h-8 text-white" />
            </div>
            <h3 className="text-xl font-bold text-white mb-4">View Reports</h3>
            <p className="text-gray-400 mb-6">Detailed analytics and reports</p>
            <div className="text-purple-400 font-semibold">Interactive Charts</div>
          </div>
          
          <div className="bg-gradient-to-br from-slate-800/50 to-slate-700/50 backdrop-blur-xl rounded-2xl p-8 border border-slate-600/30 text-center hover:border-purple-500/50 transition-all hover:transform hover:scale-105 cursor-pointer">
            <div className="w-16 h-16 bg-gradient-to-r from-green-500 to-emerald-500 rounded-xl flex items-center justify-center mb-6 mx-auto">
              <RefreshCw className="w-8 h-8 text-white" />
            </div>
            <h3 className="text-xl font-bold text-white mb-4">Refresh Data</h3>
            <p className="text-gray-400 mb-6">Update all metrics now</p>
            <div className="text-green-400 font-semibold">Live Updates</div>
          </div>

          <div className="bg-gradient-to-br from-slate-800/50 to-slate-700/50 backdrop-blur-xl rounded-2xl p-8 border border-slate-600/30 text-center hover:border-purple-500/50 transition-all hover:transform hover:scale-105 cursor-pointer">
            <div className="w-16 h-16 bg-gradient-to-r from-purple-500 to-pink-500 rounded-xl flex items-center justify-center mb-6 mx-auto">
              <Gauge className="w-8 h-8 text-white" />
            </div>
            <h3 className="text-xl font-bold text-white mb-4">Monitor Performance</h3>
            <p className="text-gray-400 mb-6">Real-time system monitoring</p>
            <div className="text-purple-400 font-semibold">System Health</div>
          </div>
          
          <div className="bg-gradient-to-br from-slate-800/50 to-slate-700/50 backdrop-blur-xl rounded-2xl p-8 border border-slate-600/30 text-center hover:border-purple-500/50 transition-all hover:transform hover:scale-105 cursor-pointer">
            <div className="w-16 h-16 bg-gradient-to-r from-orange-500 to-red-500 rounded-xl flex items-center justify-center mb-6 mx-auto">
              <Settings className="w-8 h-8 text-white" />
            </div>
            <h3 className="text-xl font-bold text-white mb-4">Configure Alerts</h3>
            <p className="text-gray-400 mb-6">Set up custom alerts</p>
            <div className="text-orange-400 font-semibold">Notifications</div>
          </div>
        </div>
      </div>
    </div>
  );
}
