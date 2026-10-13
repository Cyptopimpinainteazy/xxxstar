'use client';

import { useState, useEffect } from 'react';
import Link from 'next/link';
import dynamic from 'next/dynamic';

// Dynamic import for the map component (SSR-safe)
const SwarmMap = dynamic(() => import('@/components/gpu-swarm/SwarmMap'), { 
  ssr: false,
  loading: () => (
    <div className="h-[400px] bg-black/40 rounded-xl flex items-center justify-center">
      <div className="text-gray-500 font-mono">Loading Swarm Map...</div>
    </div>
  )
});

interface SwarmNode {
  id: string;
  address: string;
  status: 'online' | 'offline' | 'syncing';
  jobs_completed: number;
  success_rate: number;
  last_seen: string;
  stake: string;
  region: string;
}

interface SwarmStats {
  total_nodes: number;
  online_nodes: number;
  total_jobs: number;
  pending_jobs: number;
  avg_execution_time_ms: number;
  total_gas_consumed: string;
}

interface AIAgent {
  id: string;
  type: 'PREDICTION' | 'TRADING' | 'ORACLE' | 'COMPUTE';
  status: 'ACTIVE' | 'IDLE' | 'TRAINING';
  earnings: string;
  tasksCompleted: number;
  reputation: number;
  lastAction: string;
}

interface TaskLog {
  id: number;
  timestamp: Date;
  agentId: string;
  action: string;
  target: string;
  result: 'SUCCESS' | 'PENDING' | 'FAILED';
  gasUsed: number;
  reward: string;
}

// Mock AI Agents
const MOCK_AGENTS: AIAgent[] = [
  { id: 'agent-pred-001', type: 'PREDICTION', status: 'ACTIVE', earnings: '12,450 ATLAS', tasksCompleted: 847, reputation: 98, lastAction: 'Submitted market signal for ETH/USD' },
  { id: 'agent-trade-002', type: 'TRADING', status: 'ACTIVE', earnings: '34,123 ATLAS', tasksCompleted: 2341, reputation: 96, lastAction: 'Executed arbitrage on DEX pool' },
  { id: 'agent-oracle-003', type: 'ORACLE', status: 'IDLE', earnings: '8,901 ATLAS', tasksCompleted: 1234, reputation: 99, lastAction: 'Price feed update BTC/ATLAS' },
  { id: 'agent-gpu-004', type: 'COMPUTE', status: 'ACTIVE', earnings: '56,789 ATLAS', tasksCompleted: 456, reputation: 97, lastAction: 'LLM inference batch #4521' },
  { id: 'agent-pred-005', type: 'PREDICTION', status: 'TRAINING', earnings: '3,456 ATLAS', tasksCompleted: 123, reputation: 92, lastAction: 'Model retraining in progress' },
];

// Mock Task Ledger
const MOCK_TASKS: TaskLog[] = [
  { id: 1, timestamp: new Date(Date.now() - 5000), agentId: 'agent-pred-001', action: 'SUBMIT_SIGNAL', target: 'Market #1 (ETH $5k)', result: 'SUCCESS', gasUsed: 45000, reward: '2.5 ATLAS' },
  { id: 2, timestamp: new Date(Date.now() - 15000), agentId: 'agent-trade-002', action: 'EXECUTE_SWAP', target: 'ATLAS/ETH Pool', result: 'SUCCESS', gasUsed: 120000, reward: '8.3 ATLAS' },
  { id: 3, timestamp: new Date(Date.now() - 30000), agentId: 'agent-gpu-004', action: 'COMPUTE_JOB', target: 'Inference Request #8921', result: 'SUCCESS', gasUsed: 250000, reward: '15.0 ATLAS' },
  { id: 4, timestamp: new Date(Date.now() - 45000), agentId: 'agent-oracle-003', action: 'PRICE_UPDATE', target: 'BTC/ATLAS Oracle', result: 'SUCCESS', gasUsed: 35000, reward: '1.2 ATLAS' },
  { id: 5, timestamp: new Date(Date.now() - 60000), agentId: 'agent-pred-001', action: 'CLAIM_REWARD', target: 'Prediction Market #2', result: 'SUCCESS', gasUsed: 65000, reward: '45.0 ATLAS' },
  { id: 6, timestamp: new Date(Date.now() - 90000), agentId: 'agent-trade-002', action: 'PROVIDE_LIQUIDITY', target: 'CCPM Pool #7', result: 'PENDING', gasUsed: 180000, reward: '0.0 ATLAS' },
  { id: 7, timestamp: new Date(Date.now() - 120000), agentId: 'agent-gpu-004', action: 'BID_JOB', target: 'GPU Marketplace', result: 'SUCCESS', gasUsed: 55000, reward: '0.0 ATLAS' },
];

export default function SwarmDashboard() {
  const [stats, setStats] = useState<SwarmStats | null>(null);
  const [nodes, setNodes] = useState<SwarmNode[]>([]);
  const [agents, setAgents] = useState<AIAgent[]>(MOCK_AGENTS);
  const [taskLog, setTaskLog] = useState<TaskLog[]>(MOCK_TASKS);
  const [loading, setLoading] = useState(true);
  const [activeTab, setActiveTab] = useState<'overview' | 'agents' | 'ledger' | 'map'>('overview');

  useEffect(() => {
    // Mock data
    setStats({
      total_nodes: 247,
      online_nodes: 231,
      total_jobs: 15847,
      pending_jobs: 23,
      avg_execution_time_ms: 145,
      total_gas_consumed: '1,234,567',
    });

    setNodes([
      {
        id: 'node-1',
        address: '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY',
        status: 'online',
        jobs_completed: 1234,
        success_rate: 99.2,
        last_seen: '2 seconds ago',
        stake: '10,000 ATLAS',
        region: 'US-East',
      },
      {
        id: 'node-2',
        address: '5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty',
        status: 'online',
        jobs_completed: 987,
        success_rate: 98.7,
        last_seen: '5 seconds ago',
        stake: '5,000 ATLAS',
        region: 'EU-West',
      },
      {
        id: 'node-3',
        address: '5DAAnrj7VHTznn2AWBemMuyBwZWs6FNFjdyVXUeYum3PTXFy',
        status: 'syncing',
        jobs_completed: 456,
        success_rate: 97.5,
        last_seen: '1 minute ago',
        stake: '7,500 ATLAS',
        region: 'Asia-Pacific',
      },
      {
        id: 'node-4',
        address: '5HGjWAeFDfFCWPsjFQdVV2Msvz2XtMktvgocEZcCj68kUMaw',
        status: 'offline',
        jobs_completed: 234,
        success_rate: 95.0,
        last_seen: '10 minutes ago',
        stake: '3,000 ATLAS',
        region: 'US-West',
      },
    ]);

    setLoading(false);
  }, []);

  // Simulate real-time task updates
  useEffect(() => {
    const interval = setInterval(() => {
      const actions = ['SUBMIT_SIGNAL', 'EXECUTE_SWAP', 'COMPUTE_JOB', 'PRICE_UPDATE', 'CLAIM_REWARD'];
      const agentIds = MOCK_AGENTS.map(a => a.id);
      const newTask: TaskLog = {
        id: Date.now(),
        timestamp: new Date(),
        agentId: agentIds[Math.floor(Math.random() * agentIds.length)],
        action: actions[Math.floor(Math.random() * actions.length)],
        target: `Task #${Math.floor(Math.random() * 10000)}`,
        result: Math.random() > 0.1 ? 'SUCCESS' : 'PENDING',
        gasUsed: Math.floor(Math.random() * 200000) + 30000,
        reward: (Math.random() * 20).toFixed(1) + ' ATLAS',
      };
      setTaskLog(prev => [newTask, ...prev.slice(0, 19)]);
    }, 8000);
    return () => clearInterval(interval);
  }, []);

  if (loading) {
    return (
      <div className="min-h-screen bg-gradient-to-br from-slate-950 via-blue-950/20 to-slate-950 flex items-center justify-center">
        <div className="animate-spin rounded-full h-32 w-32 border-t-2 border-b-2 border-cyan-500"></div>
      </div>
    );
  }

  const agentTypeColors: Record<string, string> = {
    PREDICTION: 'text-yellow-400 bg-yellow-500/20 border-yellow-500/30',
    TRADING: 'text-green-400 bg-green-500/20 border-green-500/30',
    ORACLE: 'text-purple-400 bg-purple-500/20 border-purple-500/30',
    COMPUTE: 'text-cyan-400 bg-cyan-500/20 border-cyan-500/30',
  };

  const resultColors: Record<string, string> = {
    SUCCESS: 'text-green-400 bg-green-500/20',
    PENDING: 'text-yellow-400 bg-yellow-500/20',
    FAILED: 'text-red-400 bg-red-500/20',
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-slate-950 via-blue-950/20 to-slate-950">
      {/* Header */}
      <header className="border-b border-cyan-800/30 backdrop-blur-xl bg-black/40 sticky top-0 z-50">
        <div className="container mx-auto px-6 py-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center space-x-4">
              <Link href="/x3" className="text-2xl font-bold bg-gradient-to-r from-cyan-400 to-blue-400 bg-clip-text text-transparent">
                X3 Sphere
              </Link>
              <span className="text-cyan-400/60">/</span>
              <span className="text-white font-medium">AI Swarm Network</span>
            </div>
            <nav className="flex items-center space-x-4">
              <Link href="/x3/evolution" className="text-gray-400 hover:text-white transition text-sm">Evolution</Link>
              <Link href="/x3/verifier" className="text-gray-400 hover:text-white transition text-sm">Verifier</Link>
              <Link href="/x3/scripts" className="text-gray-400 hover:text-white transition text-sm">Scripts</Link>
            </nav>
          </div>
        </div>
      </header>

      {/* Feature Links Bar */}
      <div className="border-b border-gray-800/50 bg-black/20">
        <div className="container mx-auto px-6 py-3">
          <div className="flex items-center gap-4 overflow-x-auto">
            <span className="text-gray-500 text-sm font-mono shrink-0">ECOSYSTEM:</span>
            <Link href="/x3/swarm/predictions" className="flex items-center gap-2 px-4 py-2 bg-yellow-500/10 border border-yellow-500/30 rounded-lg hover:bg-yellow-500/20 transition shrink-0">
              <span>🔮</span>
              <span className="text-yellow-400 text-sm font-mono">Predictions</span>
            </Link>
            <Link href="/x3/swarm/auctions" className="flex items-center gap-2 px-4 py-2 bg-purple-500/10 border border-purple-500/30 rounded-lg hover:bg-purple-500/20 transition shrink-0">
              <span>🔨</span>
              <span className="text-purple-400 text-sm font-mono">Auctions</span>
            </Link>
            <Link href="/x3/swarm/gpu" className="flex items-center gap-2 px-4 py-2 bg-cyan-500/10 border border-cyan-500/30 rounded-lg hover:bg-cyan-500/20 transition shrink-0">
              <span>🖥️</span>
              <span className="text-cyan-400 text-sm font-mono">GPU Market</span>
            </Link>
            <Link href="/swap" className="flex items-center gap-2 px-4 py-2 bg-green-500/10 border border-green-500/30 rounded-lg hover:bg-green-500/20 transition shrink-0">
              <span>💱</span>
              <span className="text-green-400 text-sm font-mono">DEX</span>
            </Link>
            <Link href="/earn" className="flex items-center gap-2 px-4 py-2 bg-orange-500/10 border border-orange-500/30 rounded-lg hover:bg-orange-500/20 transition shrink-0">
              <span>💰</span>
              <span className="text-orange-400 text-sm font-mono">Earn</span>
            </Link>
          </div>
        </div>
      </div>

      {/* Tabs */}
      <div className="container mx-auto px-6 pt-6">
        <div className="flex items-center gap-2 border-b border-gray-800">
          {[
            { id: 'overview', label: '📊 Overview' },
            { id: 'agents', label: '🤖 AI Agents' },
            { id: 'ledger', label: '📜 Task Ledger' },
            { id: 'map', label: '🗺️ Swarm Map' },
          ].map(tab => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id as any)}
              className={`px-4 py-3 font-mono text-sm transition border-b-2 -mb-px ${
                activeTab === tab.id
                  ? 'border-cyan-500 text-cyan-400'
                  : 'border-transparent text-gray-500 hover:text-gray-300'
              }`}
            >
              {tab.label}
            </button>
          ))}
        </div>
      </div>

      <main className="container mx-auto px-6 py-6">
        {/* Overview Tab */}
        {activeTab === 'overview' && (
          <>
            {/* Network Stats */}
            <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-4 mb-8">
              <StatCard title="Total Nodes" value={stats?.total_nodes.toString() || '0'} icon="🖥️" />
              <StatCard title="Online" value={stats?.online_nodes.toString() || '0'} icon="🟢" highlight />
              <StatCard title="Total Jobs" value={stats?.total_jobs.toLocaleString() || '0'} icon="⚡" />
              <StatCard title="AI Agents" value={agents.length.toString()} icon="🤖" />
              <StatCard title="Avg Time" value={`${stats?.avg_execution_time_ms}ms`} icon="⏱️" />
              <StatCard title="Gas Used" value={stats?.total_gas_consumed || '0'} icon="⛽" />
            </div>

            {/* Network Health */}
            <div className="bg-black/40 rounded-2xl border border-cyan-800/30 p-6 mb-8">
              <div className="flex items-center justify-between mb-6">
                <h2 className="text-xl font-semibold text-white flex items-center gap-2">
                  <span className="text-2xl">🌐</span> Network Health
                </h2>
                <div className="flex items-center gap-2">
                  <span className="w-3 h-3 bg-green-400 rounded-full animate-pulse"></span>
                  <span className="text-green-400 font-medium">Healthy</span>
                </div>
              </div>
              
              <div className="grid grid-cols-1 md:grid-cols-4 gap-6">
                <HealthMetric label="Node Uptime" value={93.5} suffix="%" color="green" />
                <HealthMetric label="Job Success Rate" value={98.7} suffix="%" color="green" />
                <HealthMetric label="Network Latency" value={45} suffix="ms" color="blue" />
                <HealthMetric label="AI Agent Activity" value={87.2} suffix="%" color="cyan" />
              </div>
            </div>

            {/* Quick Stats Grid */}
            <div className="grid grid-cols-1 lg:grid-cols-3 gap-6 mb-8">
              {/* Recent Activity */}
              <div className="bg-black/40 rounded-2xl border border-gray-800/50 p-6">
                <h3 className="text-lg font-semibold text-white mb-4 flex items-center gap-2">
                  <span>⚡</span> Live Activity
                </h3>
                <div className="space-y-3 max-h-[200px] overflow-y-auto">
                  {taskLog.slice(0, 5).map(task => (
                    <div key={task.id} className="flex items-center justify-between py-2 border-b border-gray-800/50 last:border-0">
                      <div className="flex items-center gap-2">
                        <span className={`w-2 h-2 rounded-full ${task.result === 'SUCCESS' ? 'bg-green-400' : 'bg-yellow-400'}`}></span>
                        <span className="text-gray-400 text-xs font-mono">{task.agentId.slice(0, 15)}</span>
                      </div>
                      <span className="text-cyan-400 text-xs">{task.reward}</span>
                    </div>
                  ))}
                </div>
              </div>

              {/* Top Agents */}
              <div className="bg-black/40 rounded-2xl border border-gray-800/50 p-6">
                <h3 className="text-lg font-semibold text-white mb-4 flex items-center gap-2">
                  <span>🏆</span> Top Agents
                </h3>
                <div className="space-y-3">
                  {agents.slice(0, 4).sort((a, b) => parseInt(b.earnings.replace(/,/g, '')) - parseInt(a.earnings.replace(/,/g, ''))).map((agent, idx) => (
                    <div key={agent.id} className="flex items-center justify-between py-2">
                      <div className="flex items-center gap-2">
                        <span className="text-gray-500 text-xs">#{idx + 1}</span>
                        <span className={`px-2 py-0.5 rounded text-xs font-mono border ${agentTypeColors[agent.type]}`}>
                          {agent.type}
                        </span>
                      </div>
                      <span className="text-yellow-400 text-sm font-mono">{agent.earnings}</span>
                    </div>
                  ))}
                </div>
              </div>

              {/* Quick Links */}
              <div className="bg-black/40 rounded-2xl border border-gray-800/50 p-6">
                <h3 className="text-lg font-semibold text-white mb-4 flex items-center gap-2">
                  <span>🚀</span> Quick Actions
                </h3>
                <div className="space-y-3">
                  <Link href="/x3/swarm/predictions" className="block p-3 bg-yellow-500/10 border border-yellow-500/20 rounded-lg hover:bg-yellow-500/20 transition">
                    <div className="text-yellow-400 font-mono text-sm">🔮 Trade Predictions</div>
                    <div className="text-gray-500 text-xs">AI-powered market signals</div>
                  </Link>
                  <Link href="/x3/swarm/auctions" className="block p-3 bg-purple-500/10 border border-purple-500/20 rounded-lg hover:bg-purple-500/20 transition">
                    <div className="text-purple-400 font-mono text-sm">🔨 Bid on Blockspace</div>
                    <div className="text-gray-500 text-xs">Dutch auctions for validators</div>
                  </Link>
                  <Link href="/x3/swarm/gpu" className="block p-3 bg-cyan-500/10 border border-cyan-500/20 rounded-lg hover:bg-cyan-500/20 transition">
                    <div className="text-cyan-400 font-mono text-sm">🖥️ Rent GPU Power</div>
                    <div className="text-gray-500 text-xs">Decentralized compute market</div>
                  </Link>
                </div>
              </div>
            </div>

            {/* Node List */}
            <div className="bg-black/40 rounded-2xl border border-gray-800/50 p-6">
              <div className="flex items-center justify-between mb-6">
                <h2 className="text-xl font-semibold text-white flex items-center gap-2">
                  <span className="text-2xl">🖥️</span> Active Nodes
                </h2>
                <button className="px-4 py-2 bg-cyan-600 hover:bg-cyan-500 text-white rounded-lg transition text-sm">
                  Register Node
                </button>
              </div>

              <div className="overflow-x-auto">
                <table className="w-full">
                  <thead>
                    <tr className="text-left text-gray-400 text-sm border-b border-gray-800/50">
                      <th className="pb-4">Node</th>
                      <th className="pb-4">Status</th>
                      <th className="pb-4">Jobs</th>
                      <th className="pb-4">Success</th>
                      <th className="pb-4">Stake</th>
                      <th className="pb-4">Region</th>
                    </tr>
                  </thead>
                  <tbody>
                    {nodes.map((node) => (
                      <tr key={node.id} className="border-b border-gray-800/30 last:border-0">
                        <td className="py-4">
                          <div className="font-mono text-sm text-white">
                            {node.address.slice(0, 8)}...{node.address.slice(-6)}
                          </div>
                        </td>
                        <td className="py-4"><StatusBadge status={node.status} /></td>
                        <td className="py-4 text-white">{node.jobs_completed.toLocaleString()}</td>
                        <td className="py-4">
                          <span className={node.success_rate >= 98 ? 'text-green-400' : 'text-yellow-400'}>
                            {node.success_rate}%
                          </span>
                        </td>
                        <td className="py-4 text-cyan-400">{node.stake}</td>
                        <td className="py-4 text-gray-400">{node.region}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          </>
        )}

        {/* AI Agents Tab */}
        {activeTab === 'agents' && (
          <div className="space-y-6">
            <div className="flex items-center justify-between">
              <h2 className="text-xl font-bold text-white">🤖 AI Agent Registry</h2>
              <button className="px-4 py-2 bg-gradient-to-r from-cyan-600 to-blue-600 text-white rounded-lg text-sm">
                Register Agent
              </button>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
              {agents.map(agent => (
                <div key={agent.id} className="bg-black/60 border border-gray-800/50 rounded-xl p-6 hover:border-cyan-500/30 transition">
                  <div className="flex items-center justify-between mb-4">
                    <span className={`px-3 py-1 rounded-full text-xs font-mono border ${agentTypeColors[agent.type]}`}>
                      {agent.type}
                    </span>
                    <span className={`px-2 py-1 rounded text-xs ${
                      agent.status === 'ACTIVE' ? 'text-green-400 bg-green-500/20' :
                      agent.status === 'IDLE' ? 'text-gray-400 bg-gray-500/20' :
                      'text-purple-400 bg-purple-500/20'
                    }`}>
                      {agent.status}
                    </span>
                  </div>
                  <div className="text-white font-mono text-sm mb-4">{agent.id}</div>
                  <div className="grid grid-cols-2 gap-4 text-sm mb-4">
                    <div>
                      <div className="text-gray-500 text-xs">Earnings</div>
                      <div className="text-yellow-400 font-mono">{agent.earnings}</div>
                    </div>
                    <div>
                      <div className="text-gray-500 text-xs">Tasks</div>
                      <div className="text-white font-mono">{agent.tasksCompleted}</div>
                    </div>
                  </div>
                  <div className="mb-4">
                    <div className="text-gray-500 text-xs mb-1">Reputation</div>
                    <div className="h-2 bg-gray-800 rounded-full overflow-hidden">
                      <div className="h-full bg-gradient-to-r from-cyan-500 to-blue-500 rounded-full" style={{ width: `${agent.reputation}%` }} />
                    </div>
                    <div className="text-right text-xs text-cyan-400 mt-1">{agent.reputation}%</div>
                  </div>
                  <div className="text-gray-400 text-xs border-t border-gray-800/50 pt-4">
                    <span className="text-gray-500">Last:</span> {agent.lastAction}
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Task Ledger Tab */}
        {activeTab === 'ledger' && (
          <div className="space-y-6">
            <div className="flex items-center justify-between">
              <h2 className="text-xl font-bold text-white">📜 Task Activity Ledger</h2>
              <div className="flex items-center gap-2">
                <span className="w-2 h-2 bg-green-400 rounded-full animate-pulse"></span>
                <span className="text-green-400 text-sm font-mono">Live</span>
              </div>
            </div>

            <div className="bg-black/60 border border-gray-800/50 rounded-xl overflow-hidden">
              <div className="overflow-x-auto">
                <table className="w-full">
                  <thead>
                    <tr className="text-left text-gray-400 text-xs border-b border-gray-800/50 bg-black/40">
                      <th className="px-4 py-3">Time</th>
                      <th className="px-4 py-3">Agent</th>
                      <th className="px-4 py-3">Action</th>
                      <th className="px-4 py-3">Target</th>
                      <th className="px-4 py-3">Result</th>
                      <th className="px-4 py-3">Gas</th>
                      <th className="px-4 py-3">Reward</th>
                    </tr>
                  </thead>
                  <tbody>
                    {taskLog.map(task => (
                      <tr key={task.id} className="border-b border-gray-800/30 last:border-0 hover:bg-white/5">
                        <td className="px-4 py-3 text-gray-500 text-xs font-mono">
                          {Math.floor((Date.now() - task.timestamp.getTime()) / 1000)}s ago
                        </td>
                        <td className="px-4 py-3">
                          <span className="text-cyan-400 text-xs font-mono">{task.agentId}</span>
                        </td>
                        <td className="px-4 py-3">
                          <span className="text-white text-xs font-mono">{task.action}</span>
                        </td>
                        <td className="px-4 py-3 text-gray-400 text-xs">{task.target}</td>
                        <td className="px-4 py-3">
                          <span className={`px-2 py-1 rounded text-xs font-mono ${resultColors[task.result]}`}>
                            {task.result}
                          </span>
                        </td>
                        <td className="px-4 py-3 text-gray-400 text-xs font-mono">{task.gasUsed.toLocaleString()}</td>
                        <td className="px-4 py-3 text-yellow-400 text-xs font-mono">{task.reward}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          </div>
        )}

        {/* Swarm Map Tab */}
        {activeTab === 'map' && (
          <div className="space-y-6">
            <div className="flex items-center justify-between">
              <h2 className="text-xl font-bold text-white">🗺️ Global Swarm Network</h2>
              <div className="text-sm text-gray-500">
                <span className="text-green-400">{stats?.online_nodes}</span> / {stats?.total_nodes} nodes online
              </div>
            </div>
            <div className="bg-black/60 border border-cyan-500/20 rounded-xl overflow-hidden">
              <SwarmMap />
            </div>
          </div>
        )}
      </main>
    </div>
  );
}

function StatCard({ title, value, icon, highlight }: {
  title: string;
  value: string;
  icon: string;
  highlight?: boolean;
}) {
  return (
    <div className={`bg-black/40 rounded-xl border ${highlight ? 'border-green-500/50' : 'border-gray-800/50'} p-4`}>
      <div className="flex items-center justify-between mb-1">
        <span className="text-gray-400 text-xs">{title}</span>
        <span className="text-lg">{icon}</span>
      </div>
      <div className={`text-2xl font-bold ${highlight ? 'text-green-400' : 'text-white'}`}>
        {value}
      </div>
    </div>
  );
}

function HealthMetric({ label, value, suffix, color }: {
  label: string;
  value: number;
  suffix: string;
  color: 'green' | 'blue' | 'cyan' | 'yellow';
}) {
  const colors = {
    green: 'from-green-500 to-green-400',
    blue: 'from-blue-500 to-blue-400',
    cyan: 'from-cyan-500 to-cyan-400',
    yellow: 'from-yellow-500 to-yellow-400',
  };

  return (
    <div>
      <div className="flex items-center justify-between mb-2">
        <span className="text-gray-400 text-sm">{label}</span>
        <span className="text-white font-medium">{value}{suffix}</span>
      </div>
      <div className="h-2 bg-gray-800 rounded-full overflow-hidden">
        <div 
          className={`h-full bg-gradient-to-r ${colors[color]} rounded-full transition-all duration-500`}
          style={{ width: `${Math.min(value, 100)}%` }}
        />
      </div>
    </div>
  );
}

function StatusBadge({ status }: { status: 'online' | 'offline' | 'syncing' }) {
  const styles = {
    online: 'bg-green-500/20 text-green-400 border-green-500/30',
    offline: 'bg-red-500/20 text-red-400 border-red-500/30',
    syncing: 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30',
  };

  return (
    <span className={`px-2 py-1 rounded-full text-xs border ${styles[status]}`}>
      {status.toUpperCase()}
    </span>
  );
}

