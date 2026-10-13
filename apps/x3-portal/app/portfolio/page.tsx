"use client";

import { useState, useEffect } from 'react';
import { motion } from 'framer-motion';
import { TrendingUp, TrendingDown, Zap, Shield, DollarSign, BarChart3 } from 'lucide-react';

interface PortfolioPosition {
  id: string;
  chain: string;
  protocol: string;
  token: string;
  balance: string;
  valueUSD: number;
  healthFactor: number;
  yield: number;
  risk: 'low' | 'medium' | 'high';
}

interface CrossChainMetrics {
  totalValueUSD: number;
  totalYield: number;
  totalPositions: number;
  chainsConnected: number;
  healthScore: number;
}

export default function CrossChainPortfolio() {
  const [metrics, setMetrics] = useState<CrossChainMetrics>({
    totalValueUSD: 0,
    totalYield: 0,
    totalPositions: 0,
    chainsConnected: 0,
    healthScore: 0
  });

  const [positions, setPositions] = useState<PortfolioPosition[]>([]);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    // Simulate real-time data fetching
    const fetchPortfolioData = async () => {
      setIsLoading(true);
      
      // Mock data for 103 chains
      const mockPositions: PortfolioPosition[] = [
        {
          id: '1',
          chain: 'Ethereum',
          protocol: 'Aave V3',
          token: 'USDC',
          balance: '50,000',
          valueUSD: 50000,
          healthFactor: 2.5,
          yield: 4.2,
          risk: 'low'
        },
        {
          id: '2',
          chain: 'Arbitrum',
          protocol: 'Compound',
          token: 'ETH',
          balance: '25.5',
          valueUSD: 75000,
          healthFactor: 1.8,
          yield: 6.1,
          risk: 'medium'
        },
        {
          id: '3',
          chain: 'Polygon',
          protocol: 'QuickSwap',
          token: 'MATIC',
          balance: '100,000',
          valueUSD: 45000,
          healthFactor: 3.2,
          yield: 8.5,
          risk: 'low'
        },
        {
          id: '4',
          chain: 'Base',
          protocol: 'Aerodrome',
          token: 'USDT',
          balance: '75,000',
          valueUSD: 75000,
          healthFactor: 2.1,
          yield: 5.8,
          risk: 'medium'
        },
        {
          id: '5',
          chain: 'Avalanche',
          protocol: 'Trader Joe',
          token: 'AVAX',
          balance: '1,200',
          valueUSD: 42000,
          healthFactor: 2.8,
          yield: 7.2,
          risk: 'low'
        }
      ];

      const totalValue = mockPositions.reduce((sum, pos) => sum + pos.valueUSD, 0);
      const totalYield = mockPositions.reduce((sum, pos) => sum + pos.yield, 0) / mockPositions.length;
      const avgHealth = mockPositions.reduce((sum, pos) => sum + pos.healthFactor, 0) / mockPositions.length;

      setMetrics({
        totalValueUSD: totalValue,
        totalYield: totalYield,
        totalPositions: mockPositions.length,
        chainsConnected: 5,
        healthScore: avgHealth
      });

      setPositions(mockPositions);
      setIsLoading(false);
    };

    fetchPortfolioData();
    const interval = setInterval(fetchPortfolioData, 3000); // Update every 3 seconds

    return () => clearInterval(interval);
  }, []);

  const getRiskColor = (risk: string) => {
    switch (risk) {
      case 'low': return 'text-green-400';
      case 'medium': return 'text-yellow-400';
      case 'high': return 'text-red-400';
      default: return 'text-gray-400';
    }
  };

  const getHealthColor = (health: number) => {
    if (health >= 2.5) return 'text-green-400';
    if (health >= 1.5) return 'text-yellow-400';
    return 'text-red-400';
  };

  if (isLoading) {
    return (
      <div className="min-h-screen bg-gradient-to-br from-slate-900 via-blue-900 to-slate-900 flex items-center justify-center">
        <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-400"></div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gradient-to-br from-slate-900 via-blue-900 to-slate-900 text-white">
      {/* Header */}
      <div className="border-b border-slate-700 bg-slate-900/50 backdrop-blur-sm sticky top-0 z-10">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-6">
          <motion.h1 
            initial={{ opacity: 0, y: -20 }}
            animate={{ opacity: 1, y: 0 }}
            className="text-3xl font-bold bg-gradient-to-r from-blue-400 to-cyan-300 bg-clip-text text-transparent"
          >
            Cross-Chain Portfolio
          </motion.h1>
          <p className="text-slate-400 mt-2">
            Manage your positions across 103+ blockchain networks
          </p>
        </div>
      </div>

      {/* Metrics Overview */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-5 gap-6 mb-8">
          <motion.div 
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.1 }}
            className="bg-slate-800/50 backdrop-blur-sm border border-slate-700 rounded-xl p-6"
          >
            <div className="flex items-center justify-between">
              <div>
                <p className="text-slate-400 text-sm">Total Value</p>
                <p className="text-2xl font-bold text-white">${metrics.totalValueUSD.toLocaleString()}</p>
              </div>
              <DollarSign className="h-8 w-8 text-green-400" />
            </div>
          </motion.div>

          <motion.div 
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.2 }}
            className="bg-slate-800/50 backdrop-blur-sm border border-slate-700 rounded-xl p-6"
          >
            <div className="flex items-center justify-between">
              <div>
                <p className="text-slate-400 text-sm">Avg Yield</p>
                <p className="text-2xl font-bold text-blue-400">{metrics.totalYield.toFixed(1)}%</p>
              </div>
              <TrendingUp className="h-8 w-8 text-blue-400" />
            </div>
          </motion.div>

          <motion.div 
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.3 }}
            className="bg-slate-800/50 backdrop-blur-sm border border-slate-700 rounded-xl p-6"
          >
            <div className="flex items-center justify-between">
              <div>
                <p className="text-slate-400 text-sm">Positions</p>
                <p className="text-2xl font-bold text-white">{metrics.totalPositions}</p>
              </div>
              <BarChart3 className="h-8 w-8 text-cyan-400" />
            </div>
          </motion.div>

          <motion.div 
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.4 }}
            className="bg-slate-800/50 backdrop-blur-sm border border-slate-700 rounded-xl p-6"
          >
            <div className="flex items-center justify-between">
              <div>
                <p className="text-slate-400 text-sm">Chains</p>
                <p className="text-2xl font-bold text-purple-400">{metrics.chainsConnected}</p>
              </div>
              <Zap className="h-8 w-8 text-purple-400" />
            </div>
          </motion.div>

          <motion.div 
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.5 }}
            className="bg-slate-800/50 backdrop-blur-sm border border-slate-700 rounded-xl p-6"
          >
            <div className="flex items-center justify-between">
              <div>
                <p className="text-slate-400 text-sm">Health Score</p>
                <p className={`text-2xl font-bold ${getHealthColor(metrics.healthScore)}`}>
                  {metrics.healthScore.toFixed(1)}
                </p>
              </div>
              <Shield className="h-8 w-8 text-green-400" />
            </div>
          </motion.div>
        </div>

        {/* Positions Table */}
        <motion.div 
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.6 }}
          className="bg-slate-800/50 backdrop-blur-sm border border-slate-700 rounded-xl overflow-hidden"
        >
          <div className="px-6 py-4 border-b border-slate-700">
            <h2 className="text-xl font-semibold text-white">Active Positions</h2>
          </div>
          
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead className="bg-slate-900/50">
                <tr>
                  <th className="px-6 py-3 text-left text-xs font-medium text-slate-400 uppercase tracking-wider">Chain</th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-slate-400 uppercase tracking-wider">Protocol</th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-slate-400 uppercase tracking-wider">Token</th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-slate-400 uppercase tracking-wider">Balance</th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-slate-400 uppercase tracking-wider">Value USD</th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-slate-400 uppercase tracking-wider">Yield</th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-slate-400 uppercase tracking-wider">Health Factor</th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-slate-400 uppercase tracking-wider">Risk</th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-slate-400 uppercase tracking-wider">Actions</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-700">
                {positions.map((position, index) => (
                  <motion.tr 
                    key={position.id}
                    initial={{ opacity: 0, x: -20 }}
                    animate={{ opacity: 1, x: 0 }}
                    transition={{ delay: 0.1 * index }}
                    className="hover:bg-slate-700/30 transition-colors"
                  >
                    <td className="px-6 py-4 whitespace-nowrap text-sm text-blue-400 font-medium">
                      {position.chain}
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-sm text-slate-300">
                      {position.protocol}
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-sm text-white font-medium">
                      {position.token}
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-sm text-slate-300">
                      {position.balance}
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-sm text-green-400 font-medium">
                      ${position.valueUSD.toLocaleString()}
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-sm text-blue-400 font-medium">
                      {position.yield.toFixed(1)}%
                    </td>
                    <td className={`px-6 py-4 whitespace-nowrap text-sm font-medium ${getHealthColor(position.healthFactor)}`}>
                      {position.healthFactor.toFixed(1)}
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap">
                      <span className={`px-2 py-1 text-xs rounded-full ${getRiskColor(position.risk)} bg-opacity-20`}>
                        {position.risk.toUpperCase()}
                      </span>
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-sm">
                      <div className="flex space-x-2">
                        <button className="text-blue-400 hover:text-blue-300 transition-colors">
                          Migrate
                        </button>
                        <button className="text-green-400 hover:text-green-300 transition-colors">
                          Rebalance
                        </button>
                      </div>
                    </td>
                  </motion.tr>
                ))}
              </tbody>
            </table>
          </div>
        </motion.div>

        {/* Quick Actions */}
        <motion.div 
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.7 }}
          className="mt-8 grid grid-cols-1 md:grid-cols-3 gap-6"
        >
          <button className="bg-gradient-to-r from-blue-600 to-blue-700 hover:from-blue-700 hover:to-blue-800 text-white font-semibold py-3 px-6 rounded-xl transition-all transform hover:scale-105">
            Optimize Portfolio
          </button>
          <button className="bg-gradient-to-r from-green-600 to-green-700 hover:from-green-700 hover:to-green-800 text-white font-semibold py-3 px-6 rounded-xl transition-all transform hover:scale-105">
            Rebalance All
          </button>
          <button className="bg-gradient-to-r from-purple-600 to-purple-700 hover:from-purple-700 hover:to-purple-800 text-white font-semibold py-3 px-6 rounded-xl transition-all transform hover:scale-105">
            Emergency Close
          </button>
        </motion.div>
      </div>
    </div>
  );
}
