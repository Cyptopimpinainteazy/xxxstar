"use client";

import { useState, useEffect } from 'react';
import { 
  Brain, 
  Zap, 
  Cpu, 
  TrendingUp, 
  Activity, 
  Target,
  Network,
  Database,
  DollarSign,
  BarChart3,
  Settings,
  PlayCircle,
  RefreshCw
} from 'lucide-react';

interface AISwarmNode {
  id: string;
  name: string;
  gpuType: string;
  status: 'active' | 'idle' | 'maintenance' | 'offline';
  utilization: number;
  currentTask: string;
  tasksCompleted: number;
  earnings: number;
  location: string;
}

const mockNodes: AISwarmNode[] = [
  { id: '1', name: 'Node Alpha', gpuType: 'RTX 4090', status: 'active', utilization: 87, currentTask: 'Arbitrage Detection', tasksCompleted: 1247, earnings: 2340.50, location: 'US-East' },
  { id: '2', name: 'Node Beta', gpuType: 'A100', status: 'active', utilization: 92, currentTask: 'Strategy Optimization', tasksCompleted: 3892, earnings: 8920.75, location: 'EU-West' },
  { id: '3', name: 'Node Gamma', gpuType: 'RTX 4080', status: 'idle', utilization: 15, currentTask: 'Idle', tasksCompleted: 892, earnings: 1560.25, location: 'Asia-Pacific' },
  { id: '4', name: 'Node Delta', gpuType: 'H100', status: 'active', utilization: 95, currentTask: 'MEV Extraction', tasksCompleted: 5621, earnings: 15780.00, location: 'US-West' },
];

const swarmStats = {
  totalNodes: 247,
  activeNodes: 198,
  totalGPUPower: '847 TFLOPS',
  tasksPerSecond: 12847,
  totalEarnings: '$2.4M',
  avgResponseTime: '12ms',
};

export default function AISwarmPage() {
  const [nodes, setNodes] = useState<AISwarmNode[]>(mockNodes);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    const timer = setTimeout(() => setIsLoading(false), 1000);
    return () => clearTimeout(timer);
  }, []);

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'active': return 'text-green-400 bg-green-400/20';
      case 'idle': return 'text-yellow-400 bg-yellow-400/20';
      case 'maintenance': return 'text-blue-400 bg-blue-400/20';
      case 'offline': return 'text-red-400 bg-red-400/20';
      default: return 'text-gray-400 bg-gray-400/20';
    }
  };

  if (isLoading) {
    return (
      <div className="min-h-screen bg-gradient-to-br from-slate-900 via-purple-900 to-slate-900 flex items-center justify-center">
        <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-purple-400"></div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gradient-to-br from-slate-900 via-purple-900 to-slate-900 text-white">
      {/* Header */}
      <div className="border-b border-slate-700 bg-slate-900/50 backdrop-blur-sm sticky top-0 z-10">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-6">
          <div className="flex items-center justify-between">
            <div>
              <h1 className="text-3xl font-bold bg-gradient-to-r from-purple-400 to-pink-300 bg-clip-text text-transparent">
                AI Swarm Intelligence
              </h1>
              <p className="text-slate-400 mt-2">
                Distributed GPU-powered DeFi optimization across 103+ chains
              </p>
            </div>
            <div className="flex items-center space-x-4">
              <button className="bg-gradient-to-r from-purple-600 to-purple-700 hover:from-purple-700 hover:to-purple-800 text-white font-semibold py-2 px-4 rounded-lg transition-all transform hover:scale-105 flex items-center space-x-2">
                <PlayCircle className="h-5 w-5" />
                <span>Start Swarm</span>
              </button>
              <button className="bg-slate-700 hover:bg-slate-600 text-white font-semibold py-2 px-4 rounded-lg transition-all flex items-center space-x-2">
                <Settings className="h-5 w-5" />
                <span>Configure</span>
              </button>
            </div>
          </div>
        </div>
      </div>

      {/* Stats Grid */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-4 mb-8">
          {[
            { label: 'Total Nodes', value: swarmStats.totalNodes, icon: Network },
            { label: 'Active Nodes', value: swarmStats.activeNodes, icon: Activity },
            { label: 'GPU Power', value: swarmStats.totalGPUPower, icon: Cpu },
            { label: 'Tasks/sec', value: swarmStats.tasksPerSecond.toLocaleString(), icon: Zap },
            { label: 'Total Earnings', value: swarmStats.totalEarnings, icon: DollarSign },
            { label: 'Avg Response', value: swarmStats.avgResponseTime, icon: Target },
          ].map((stat, index) => (
            <div key={index} className="bg-slate-800/50 backdrop-blur-sm border border-slate-700 rounded-xl p-4">
              <div className="flex items-center space-x-2 mb-2">
                <stat.icon className="h-4 w-4 text-purple-400" />
                <span className="text-slate-400 text-sm">{stat.label}</span>
              </div>
              <p className="text-xl font-bold text-white">{stat.value}</p>
            </div>
          ))}
        </div>

        {/* Nodes Table */}
        <div className="bg-slate-800/50 backdrop-blur-sm border border-slate-700 rounded-xl overflow-hidden">
          <div className="p-4 border-b border-slate-700 flex items-center justify-between">
            <h2 className="text-lg font-semibold text-white flex items-center">
              <Brain className="h-5 w-5 mr-2 text-purple-400" />
              Swarm Nodes
            </h2>
            <button className="text-slate-400 hover:text-white transition-colors">
              <RefreshCw className="h-5 w-5" />
            </button>
          </div>
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead className="bg-slate-900/50">
                <tr>
                  <th className="px-4 py-3 text-left text-sm font-medium text-slate-400">Node</th>
                  <th className="px-4 py-3 text-left text-sm font-medium text-slate-400">GPU</th>
                  <th className="px-4 py-3 text-left text-sm font-medium text-slate-400">Status</th>
                  <th className="px-4 py-3 text-left text-sm font-medium text-slate-400">Utilization</th>
                  <th className="px-4 py-3 text-left text-sm font-medium text-slate-400">Current Task</th>
                  <th className="px-4 py-3 text-left text-sm font-medium text-slate-400">Completed</th>
                  <th className="px-4 py-3 text-left text-sm font-medium text-slate-400">Earnings</th>
                  <th className="px-4 py-3 text-left text-sm font-medium text-slate-400">Location</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-700">
                {nodes.map((node) => (
                  <tr key={node.id} className="hover:bg-slate-700/30 transition-colors">
                    <td className="px-4 py-4 text-white font-medium">{node.name}</td>
                    <td className="px-4 py-4 text-slate-300">{node.gpuType}</td>
                    <td className="px-4 py-4">
                      <span className={`px-2 py-1 rounded-full text-xs font-medium ${getStatusColor(node.status)}`}>
                        {node.status}
                      </span>
                    </td>
                    <td className="px-4 py-4">
                      <div className="flex items-center space-x-2">
                        <div className="w-24 h-2 bg-slate-700 rounded-full overflow-hidden">
                          <div 
                            className="h-full bg-gradient-to-r from-purple-500 to-pink-500 rounded-full"
                            style={{ width: `${node.utilization}%` }}
                          />
                        </div>
                        <span className="text-slate-300 text-sm">{node.utilization}%</span>
                      </div>
                    </td>
                    <td className="px-4 py-4 text-slate-300">{node.currentTask}</td>
                    <td className="px-4 py-4 text-slate-300">{node.tasksCompleted.toLocaleString()}</td>
                    <td className="px-4 py-4 text-green-400 font-medium">${node.earnings.toLocaleString()}</td>
                    <td className="px-4 py-4 text-slate-400">{node.location}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>

        {/* Bottom Section */}
        <div className="grid md:grid-cols-2 gap-6 mt-8">
          <div className="bg-slate-800/50 backdrop-blur-sm border border-slate-700 rounded-xl p-6">
            <h3 className="text-lg font-semibold text-white mb-4 flex items-center">
              <TrendingUp className="h-5 w-5 mr-2 text-green-400" />
              Recent Arbitrage Opportunities
            </h3>
            <div className="space-y-3">
              {[
                { pair: 'ETH/USDC', profit: '0.12%', chain: 'Ethereum → Arbitrum', time: '2 min ago' },
                { pair: 'USDT/USDC', profit: '0.08%', chain: 'Polygon → BSC', time: '5 min ago' },
                { pair: 'AVAX/ETH', profit: '0.15%', chain: 'Avalanche → Ethereum', time: '8 min ago' },
              ].map((opp, index) => (
                <div key={index} className="flex items-center justify-between p-3 bg-slate-700/30 rounded-lg">
                  <div>
                    <p className="text-white font-medium">{opp.pair}</p>
                    <p className="text-slate-400 text-sm">{opp.chain}</p>
                  </div>
                  <div className="text-right">
                    <p className="text-green-400 font-medium">{opp.profit}</p>
                    <p className="text-slate-500 text-sm">{opp.time}</p>
                  </div>
                </div>
              ))}
            </div>
          </div>

          <div className="bg-slate-800/50 backdrop-blur-sm border border-slate-700 rounded-xl p-6">
            <h3 className="text-lg font-semibold text-white mb-4 flex items-center">
              <BarChart3 className="h-5 w-5 mr-2 text-purple-400" />
              Strategy Performance
            </h3>
            <div className="space-y-3">
              {[
                { strategy: 'LP Optimization', performance: '+12.4%', confidence: '94%' },
                { strategy: 'Yield Farming', performance: '+8.7%', confidence: '89%' },
                { strategy: 'MEV Extraction', performance: '+15.2%', confidence: '96%' },
              ].map((strat, index) => (
                <div key={index} className="flex items-center justify-between p-3 bg-slate-700/30 rounded-lg">
                  <div>
                    <p className="text-white font-medium">{strat.strategy}</p>
                    <p className="text-slate-400 text-sm">Confidence: {strat.confidence}</p>
                  </div>
                  <p className="text-green-400 font-medium">{strat.performance}</p>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
