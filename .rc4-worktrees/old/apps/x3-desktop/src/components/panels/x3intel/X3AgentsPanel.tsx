import { useState } from 'react';
import {
  ResponsiveContainer, BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip,
} from 'recharts';

type AgentStatus = 'Active' | 'Suspended' | 'Deregistered';

interface Agent {
  id: string; status: AgentStatus; bond: string;
  reputation: number; successRate: number;
  executions: number; slashes: number;
}

const statusColor: Record<AgentStatus, string> = {
  Active: '#00d4aa',
  Suspended: '#ffaa00',
  Deregistered: '#ff4444',
};

const agents: Agent[] = [
  { id: 'agent-alpha',   status: 'Active',       bond: '10,000 USDC', reputation: 97, successRate: 98.2, executions: 4521, slashes: 0 },
  { id: 'agent-beta',    status: 'Active',       bond: '8,500 USDC',  reputation: 92, successRate: 95.1, executions: 3012, slashes: 2 },
  { id: 'agent-gamma',   status: 'Active',       bond: '12,000 USDC', reputation: 88, successRate: 93.7, executions: 2780, slashes: 4 },
  { id: 'agent-delta',   status: 'Suspended',    bond: '5,000 USDC',  reputation: 54, successRate: 78.4, executions: 1905, slashes: 12 },
  { id: 'agent-epsilon', status: 'Active',       bond: '7,200 USDC',  reputation: 85, successRate: 91.0, executions: 2100, slashes: 3 },
  { id: 'agent-zeta',    status: 'Active',       bond: '9,000 USDC',  reputation: 90, successRate: 94.5, executions: 3400, slashes: 1 },
  { id: 'agent-eta',     status: 'Deregistered', bond: '0 USDC',      reputation: 12, successRate: 45.2, executions: 320,  slashes: 18 },
  { id: 'agent-theta',   status: 'Active',       bond: '6,800 USDC',  reputation: 81, successRate: 89.3, executions: 1650, slashes: 5 },
];

const reputationChart = agents.slice(0, 6).map(a => ({ name: a.id.replace('agent-', ''), reputation: a.reputation }));

const tooltipStyle = { backgroundColor: '#1a1a1d', border: '1px solid #2a2a2e' };

export default function X3AgentsPanel() {
  const [search, setSearch] = useState('');
  const [filterStatus, setFilterStatus] = useState<AgentStatus | 'All'>('All');

  const filtered = agents
    .filter(a => filterStatus === 'All' || a.status === filterStatus)
    .filter(a => a.id.toLowerCase().includes(search.toLowerCase()));

  return (
    <div className="min-h-full bg-[#0a0a0f] text-gray-300 p-6 space-y-6 overflow-auto">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-semibold text-white">Agents <span className="text-gray-500 text-sm ml-2">({agents.length})</span></h1>
      </div>

      {/* Search + Filter */}
      <div className="flex gap-3 items-center">
        <input value={search} onChange={e => setSearch(e.target.value)} placeholder="Search agents…"
          className="bg-[#111116] border border-[#222] rounded px-3 py-1.5 text-sm text-gray-300 w-60 focus:outline-none focus:border-[#00d4aa]" />
        {(['All', 'Active', 'Suspended', 'Deregistered'] as const).map(s => (
          <button key={s} onClick={() => setFilterStatus(s)}
            className={`px-3 py-1 rounded text-xs border transition ${filterStatus === s ? 'border-[#00d4aa] text-[#00d4aa] bg-[#00d4aa11]' : 'border-[#222] text-gray-500 hover:text-gray-300'}`}>
            {s}
          </button>
        ))}
      </div>

      {/* Reputation Chart */}
      <div className="bg-[#111116] border border-[#1a1a1a] rounded-lg p-4">
        <p className="text-sm text-gray-400 mb-3">Agent Reputation</p>
        <ResponsiveContainer width="100%" height={180}>
          <BarChart data={reputationChart}>
            <CartesianGrid strokeDasharray="3 3" stroke="#2a2a2e" />
            <XAxis dataKey="name" stroke="#666" tick={{ fontSize: 11 }} />
            <YAxis stroke="#666" tick={{ fontSize: 11 }} domain={[0, 100]} />
            <Tooltip contentStyle={tooltipStyle} />
            <Bar dataKey="reputation" fill="#4488ff" radius={[4, 4, 0, 0]} />
          </BarChart>
        </ResponsiveContainer>
      </div>

      {/* Agents Table */}
      <div className="bg-[#111116] border border-[#1a1a1a] rounded-lg overflow-hidden">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-[#222] text-gray-500 text-xs uppercase">
              <th className="text-left p-3">ID</th><th className="text-left p-3">Status</th>
              <th className="text-right p-3">Bond</th><th className="text-right p-3">Reputation</th>
              <th className="text-right p-3">Success Rate</th><th className="text-right p-3">Executions</th>
              <th className="text-right p-3">Slashes</th>
            </tr>
          </thead>
          <tbody>
            {filtered.map(a => (
              <tr key={a.id} className="border-b border-[#1a1a1a] hover:bg-[#ffffff04]">
                <td className="p-3 font-mono text-xs">{a.id}</td>
                <td className="p-3">
                  <span className="px-2 py-0.5 rounded text-xs font-medium"
                    style={{ background: statusColor[a.status] + '22', color: statusColor[a.status] }}>
                    {a.status}
                  </span>
                </td>
                <td className="p-3 text-right font-mono">{a.bond}</td>
                <td className="p-3 text-right font-mono">{a.reputation}</td>
                <td className="p-3 text-right font-mono">{a.successRate}%</td>
                <td className="p-3 text-right font-mono">{a.executions.toLocaleString()}</td>
                <td className="p-3 text-right font-mono">{a.slashes}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
