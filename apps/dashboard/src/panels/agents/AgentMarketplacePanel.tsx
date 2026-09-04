import React, { useState } from 'react';
import { Bot, ShoppingCart, Lock, TrendingUp, Zap, CheckCircle } from 'lucide-react';

interface Agent {
  id: string;
  name: string;
  description: string;
  creator: string;
  category: 'trading' | 'analysis' | 'automation' | 'governance' | 'custom';
  price: number;
  rating: number;
  downloads: number;
  sandbox: 'enabled' | 'restricted' | 'disabled';
  status: 'active' | 'inactive' | 'flagged';
  securityAudit: boolean;
}

interface MarketplaceMetrics {
  totalAgents: number;
  activeAgents: number;
  totalTransactions: number;
  totalVolume: number;
  avgAgentPrice: number;
  topCreators: number;
}

interface SecurityReview {
  agentId: string;
  agentName: string;
  auditor: string;
  timestamp: number;
  securityScore: number;
  vulnerabilities: number;
  status: 'passed' | 'warning' | 'failed';
}

interface MultiAgentCoordination {
  id: string;
  name: string;
  agents: number;
  coordinationType: 'sequential' | 'parallel' | 'hierarchical';
  status: 'active' | 'paused' | 'error';
  tasksCompleted: number;
  efficiency: number;
}

export const AgentMarketplacePanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'agents' | 'marketplace' | 'security' | 'coordination'>('agents');

  const [agents] = useState<Agent[]>([
    {
      id: 'a1',
      name: 'Arbitrage Hunter Pro',
      description: 'Advanced DEX arbitrage detection and execution',
      creator: 'QuantumTrading Labs',
      category: 'trading',
      price: 50,
      rating: 4.8,
      downloads: 3240,
      sandbox: 'enabled',
      status: 'active',
      securityAudit: true,
    },
    {
      id: 'a2',
      name: 'Portfolio Rebalancer',
      description: 'Automated portfolio optimization and rebalancing',
      creator: 'TradeFi Collective',
      category: 'automation',
      price: 35,
      rating: 4.6,
      downloads: 2150,
      sandbox: 'enabled',
      status: 'active',
      securityAudit: true,
    },
    {
      id: 'a3',
      name: 'Governance Analyzer',
      description: 'Analyze and recommend governance proposals',
      creator: 'DAO Research Institute',
      category: 'governance',
      price: 25,
      rating: 4.7,
      downloads: 1840,
      sandbox: 'enabled',
      status: 'active',
      securityAudit: true,
    },
    {
      id: 'a4',
      name: 'Smart Contract Auditor',
      description: 'Automated security analysis for smart contracts',
      creator: 'SecurityFirst AI',
      category: 'analysis',
      price: 75,
      rating: 4.9,
      downloads: 5230,
      sandbox: 'restricted',
      status: 'active',
      securityAudit: true,
    },
  ]);

  const [securityReviews] = useState<SecurityReview[]>([
    {
      agentId: 'a1',
      agentName: 'Arbitrage Hunter Pro',
      auditor: 'OpenZeppelin Security',
      timestamp: Date.now() - 86400000 * 15,
      securityScore: 92,
      vulnerabilities: 0,
      status: 'passed',
    },
    {
      agentId: 'a2',
      agentName: 'Portfolio Rebalancer',
      auditor: 'Trail of Bits',
      timestamp: Date.now() - 86400000 * 30,
      securityScore: 85,
      vulnerabilities: 2,
      status: 'warning',
    },
    {
      agentId: 'a4',
      agentName: 'Smart Contract Auditor',
      auditor: 'Certora',
      timestamp: Date.now() - 86400000 * 7,
      securityScore: 97,
      vulnerabilities: 0,
      status: 'passed',
    },
  ]);

  const [multiAgentCoordinations] = useState<MultiAgentCoordination[]>([
    {
      id: 'mac1',
      name: 'Automated Trading Ensemble',
      agents: 5,
      coordinationType: 'hierarchical',
      status: 'active',
      tasksCompleted: 4240,
      efficiency: 94.5,
    },
    {
      id: 'mac2',
      name: 'Portfolio Optimization Pipeline',
      agents: 3,
      coordinationType: 'sequential',
      status: 'active',
      tasksCompleted: 1820,
      efficiency: 88.2,
    },
    {
      id: 'mac3',
      name: 'Governance Risk Analysis',
      agents: 4,
      coordinationType: 'parallel',
      status: 'paused',
      tasksCompleted: 520,
      efficiency: 76.3,
    },
  ]);

  const [metrics] = useState<MarketplaceMetrics>({
    totalAgents: 48,
    activeAgents: 42,
    totalTransactions: 12450,
    totalVolume: 2840000,
    avgAgentPrice: 42.5,
    topCreators: 12,
  });

  return (
    <div className="min-h-screen bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f] p-6">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between mb-8">
          <div>
            <h1 className="text-4xl font-bold text-transparent bg-clip-text bg-gradient-to-r from-indigo-400 to-violet-500 mb-2">
              Agent Marketplace
            </h1>
            <p className="text-gray-400">Buy/Sell Agents • Sandboxing • Multi-Agent Coordination • Security Audits</p>
          </div>
          <Bot className="w-12 h-12 text-indigo-400" />
        </div>

        {/* KPI Grid */}
        <div className="grid grid-cols-4 gap-4 mb-8">
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Active Agents</div>
            <div className="text-2xl font-bold text-indigo-400">{metrics.activeAgents}/{metrics.totalAgents}</div>
            <div className="text-xs text-gray-500 mt-2">Available for deployment</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Total Volume</div>
            <div className="text-2xl font-bold text-green-400">${(metrics.totalVolume / 1000000).toFixed(1)}M</div>
            <div className="text-xs text-gray-500 mt-2">{metrics.totalTransactions.toLocaleString()} txs</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Avg Agent Price</div>
            <div className="text-2xl font-bold text-purple-400">${metrics.avgAgentPrice}</div>
            <div className="text-xs text-gray-500 mt-2">X3 per license</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Top Creators</div>
            <div className="text-2xl font-bold text-blue-400">{metrics.topCreators}</div>
            <div className="text-xs text-gray-500 mt-2">Verified developers</div>
          </div>
        </div>

        {/* Tabs */}
        <div className="flex gap-2 mb-6 border-b border-[#2a2a35]">
          {(['agents', 'marketplace', 'security', 'coordination'] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={`px-4 py-3 font-semibold transition-colors ${
                activeTab === tab
                  ? 'text-indigo-400 border-b-2 border-indigo-400'
                  : 'text-gray-400 hover:text-gray-300'
              }`}
            >
              {tab === 'agents' && 'Available Agents'}
              {tab === 'marketplace' && 'Marketplace Stats'}
              {tab === 'security' && 'Security Reviews'}
              {tab === 'coordination' && 'Multi-Agent'}
            </button>
          ))}
        </div>

        {/* Content */}
        <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
          {activeTab === 'agents' && (
            <div>
              <h3 className="text-lg font-semibold text-white mb-6">Available Agents for Purchase</h3>
              <div className="space-y-4">
                {agents.map((agent) => (
                  <div key={agent.id} className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                    <div className="flex items-start justify-between mb-3">
                      <div className="flex-1">
                        <h4 className="text-white font-semibold flex items-center gap-2">
                          {agent.name}
                          {agent.securityAudit && <CheckCircle className="w-4 h-4 text-green-400" />}
                        </h4>
                        <p className="text-sm text-gray-400">{agent.description}</p>
                      </div>
                      <div className="text-right">
                        <div className="text-2xl font-bold text-cyan-400 mb-1">${agent.price}</div>
                        <button className="flex items-center gap-2 bg-green-500/20 text-green-400 px-3 py-1 rounded text-xs font-semibold hover:bg-green-500/30">
                          <ShoppingCart className="w-3 h-3" /> Buy
                        </button>
                      </div>
                    </div>
                    <div className="grid grid-cols-6 gap-4 text-sm mb-3 pb-3 border-b border-[#2a2a35]">
                      <div>
                        <div className="text-gray-400">Creator</div>
                        <div className="text-white font-semibold text-xs">{agent.creator}</div>
                      </div>
                      <div>
                        <div className="text-gray-400">Category</div>
                        <div className="text-white font-semibold capitalize text-xs">{agent.category}</div>
                      </div>
                      <div>
                        <div className="text-gray-400">Rating</div>
                        <div className="text-yellow-400 font-semibold">⭐ {agent.rating}</div>
                      </div>
                      <div>
                        <div className="text-gray-400">Downloads</div>
                        <div className="text-white font-semibold">{(agent.downloads / 1000).toFixed(1)}K</div>
                      </div>
                      <div>
                        <div className="text-gray-400">Sandbox</div>
                        <div className={`text-xs font-semibold ${
                          agent.sandbox === 'enabled' ? 'text-green-400' : 'text-yellow-400'
                        }`}>
                          {agent.sandbox.toUpperCase()}
                        </div>
                      </div>
                      <div>
                        <div className="text-gray-400">Status</div>
                        <div className={`text-xs font-semibold ${
                          agent.status === 'active' ? 'text-green-400' : 'text-red-400'
                        }`}>
                          {agent.status.toUpperCase()}
                        </div>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {activeTab === 'marketplace' && (
            <div>
              <h3 className="text-lg font-semibold text-white mb-6">Marketplace Statistics</h3>
              <div className="grid grid-cols-3 gap-6">
                <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                  <h4 className="text-white font-semibold mb-4">Agent Distribution</h4>
                  <div className="space-y-3">
                    <div>
                      <div className="flex justify-between mb-2">
                        <span className="text-sm text-gray-400">Trading Agents</span>
                        <span className="text-sm text-white font-semibold">15</span>
                      </div>
                      <div className="w-full bg-[#2a2a35] rounded-full h-2">
                        <div className="bg-gradient-to-r from-cyan-500 to-blue-500 h-2 rounded-full" style={{ width: '31%' }} />
                      </div>
                    </div>
                    <div>
                      <div className="flex justify-between mb-2">
                        <span className="text-sm text-gray-400">Analysis Agents</span>
                        <span className="text-sm text-white font-semibold">12</span>
                      </div>
                      <div className="w-full bg-[#2a2a35] rounded-full h-2">
                        <div className="bg-gradient-to-r from-purple-500 to-pink-500 h-2 rounded-full" style={{ width: '25%' }} />
                      </div>
                    </div>
                    <div>
                      <div className="flex justify-between mb-2">
                        <span className="text-sm text-gray-400">Automation Agents</span>
                        <span className="text-sm text-white font-semibold">18</span>
                      </div>
                      <div className="w-full bg-[#2a2a35] rounded-full h-2">
                        <div className="bg-gradient-to-r from-green-500 to-emerald-500 h-2 rounded-full" style={{ width: '37%' }} />
                      </div>
                    </div>
                    <div>
                      <div className="flex justify-between mb-2">
                        <span className="text-sm text-gray-400">Governance Agents</span>
                        <span className="text-sm text-white font-semibold">3</span>
                      </div>
                      <div className="w-full bg-[#2a2a35] rounded-full h-2">
                        <div className="bg-gradient-to-r from-yellow-500 to-orange-500 h-2 rounded-full" style={{ width: '6%' }} />
                      </div>
                    </div>
                  </div>
                </div>
                <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                  <h4 className="text-white font-semibold mb-4">Revenue Breakdown</h4>
                  <div className="space-y-3">
                    <div className="flex justify-between">
                      <span className="text-gray-400">Total Revenue</span>
                      <span className="text-green-400 font-semibold">${(metrics.totalVolume / 1000000).toFixed(1)}M</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-400">Creator Payouts</span>
                      <span className="text-green-400 font-semibold">${(metrics.totalVolume * 0.8 / 1000000).toFixed(1)}M</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-400">Protocol Fee (20%)</span>
                      <span className="text-blue-400 font-semibold">${(metrics.totalVolume * 0.2 / 1000000).toFixed(1)}M</span>
                    </div>
                    <div className="flex justify-between border-t border-[#2a2a35] pt-3 mt-3">
                      <span className="text-gray-300">Avg per Creator</span>
                      <span className="text-purple-400 font-semibold">${((metrics.totalVolume * 0.8 / metrics.topCreators) / 1000).toFixed(0)}K</span>
                    </div>
                  </div>
                </div>
                <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                  <h4 className="text-white font-semibold mb-4">Market Health</h4>
                  <div className="space-y-3">
                    <div className="flex justify-between">
                      <span className="text-gray-400">Uptime</span>
                      <span className="text-green-400 font-semibold">99.8%</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-400">Avg Resolution</span>
                      <span className="text-white font-semibold">1.2s</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-400">Error Rate</span>
                      <span className="text-green-400 font-semibold">0.02%</span>
                    </div>
                    <div className="flex justify-between border-t border-[#2a2a35] pt-3 mt-3">
                      <span className="text-gray-300">Disputes</span>
                      <span className="text-yellow-400 font-semibold">3 (of 12.4K)</span>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          )}

          {activeTab === 'security' && (
            <div>
              <h3 className="text-lg font-semibold text-white mb-6">Security Audit Reports</h3>
              <div className="space-y-4">
                {securityReviews.map((review) => (
                  <div key={review.agentId} className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                    <div className="flex items-start justify-between mb-3">
                      <div>
                        <h4 className="text-white font-semibold">{review.agentName}</h4>
                        <p className="text-sm text-gray-400">Auditor: {review.auditor}</p>
                      </div>
                      <div
                        className={`px-3 py-1 rounded-full text-xs font-semibold ${
                          review.status === 'passed'
                            ? 'bg-green-500/20 text-green-400'
                            : 'bg-yellow-500/20 text-yellow-400'
                        }`}
                      >
                        {review.status.toUpperCase()}
                      </div>
                    </div>
                    <div className="grid grid-cols-4 gap-4 text-sm">
                      <div>
                        <div className="text-gray-400">Security Score</div>
                        <div className="text-white font-semibold text-lg">{review.securityScore}/100</div>
                      </div>
                      <div>
                        <div className="text-gray-400">Vulnerabilities</div>
                        <div className={review.vulnerabilities === 0 ? 'text-green-400 font-semibold text-lg' : 'text-red-400 font-semibold text-lg'}>
                          {review.vulnerabilities}
                        </div>
                      </div>
                      <div>
                        <div className="text-gray-400">Audit Date</div>
                        <div className="text-white font-semibold">
                          {Math.round((Date.now() - review.timestamp) / 86400000)}d ago
                        </div>
                      </div>
                      <div>
                        <div className="text-gray-400">Status</div>
                        <div className={review.vulnerabilities === 0 ? 'text-green-400 font-semibold' : 'text-yellow-400 font-semibold'}>
                          {review.vulnerabilities === 0 ? 'Clear' : 'Review'}
                        </div>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {activeTab === 'coordination' && (
            <div>
              <h3 className="text-lg font-semibold text-white mb-6">Multi-Agent Coordination</h3>
              <div className="space-y-4">
                {multiAgentCoordinations.map((coord) => (
                  <div key={coord.id} className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                    <div className="flex items-start justify-between mb-3">
                      <div>
                        <h4 className="text-white font-semibold">{coord.name}</h4>
                        <p className="text-sm text-gray-400">
                          {coord.agents} agents • {coord.coordinationType.replace('-', ' ').toUpperCase()}
                        </p>
                      </div>
                      <div
                        className={`px-3 py-1 rounded-full text-xs font-semibold ${
                          coord.status === 'active'
                            ? 'bg-green-500/20 text-green-400'
                            : 'bg-yellow-500/20 text-yellow-400'
                        }`}
                      >
                        {coord.status.toUpperCase()}
                      </div>
                    </div>
                    <div className="grid grid-cols-4 gap-4 text-sm mb-3">
                      <div>
                        <div className="text-gray-400">Tasks Completed</div>
                        <div className="text-white font-semibold">{coord.tasksCompleted.toLocaleString()}</div>
                      </div>
                      <div>
                        <div className="text-gray-400">Efficiency</div>
                        <div className="text-blue-400 font-semibold">{coord.efficiency.toFixed(1)}%</div>
                      </div>
                      <div>
                        <div className="text-gray-400">Agents</div>
                        <div className="text-white font-semibold">{coord.agents}</div>
                      </div>
                      <div>
                        <div className="text-gray-400">Type</div>
                        <div className="text-white font-semibold text-xs capitalize">{coord.coordinationType}</div>
                      </div>
                    </div>
                    <div className="w-full bg-[#2a2a35] rounded-full h-2">
                      <div
                        className="bg-gradient-to-r from-indigo-500 to-purple-500 h-2 rounded-full"
                        style={{ width: `${coord.efficiency}%` }}
                      />
                    </div>
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

export default AgentMarketplacePanel;
