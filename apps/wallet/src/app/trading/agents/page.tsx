'use client';

import { useState, useMemo } from 'react';
import type { Agent, AgentStatus } from '@/lib/x3/types';
import { AgentStatus as AgentStatusEnum } from '@/lib/x3/types';
import { Badge } from '@/components/x3/UIComponents';
import { AgentReputationChart, BondDistributionChart } from '@/components/x3/Charts';

const DEMO_TS = 1_700_000_120_000;

const DEMO_AGENTS: Agent[] = [
  {
    id: 'agent-alpha',
    status: AgentStatusEnum.Active,
    bondAmount: 50_000,
    reputation: 97.2,
    successRate: 98.1,
    totalExecutions: 4_821,
    totalSlashes: 1,
    registeredAt: DEMO_TS - 86_400_000 * 30,
  },
  {
    id: 'agent-bravo',
    status: AgentStatusEnum.Active,
    bondAmount: 25_000,
    reputation: 88.4,
    successRate: 94.3,
    totalExecutions: 2_150,
    totalSlashes: 3,
    registeredAt: DEMO_TS - 86_400_000 * 15,
  },
  {
    id: 'agent-charlie',
    status: AgentStatusEnum.Active,
    bondAmount: 100_000,
    reputation: 99.1,
    successRate: 99.4,
    totalExecutions: 8_392,
    totalSlashes: 0,
    registeredAt: DEMO_TS - 86_400_000 * 60,
  },
];

function statusColor(status: AgentStatus): 'green' | 'amber' | 'red' | 'muted' {
  switch (status) {
    case AgentStatusEnum.Active:
      return 'green';
    case AgentStatusEnum.Suspended:
      return 'amber';
    default:
      return 'red';
  }
}

export default function AgentsPage() {
  const [agents] = useState<Agent[]>(DEMO_AGENTS);

  const activeAgents = useMemo(() => agents.filter((a) => a.status === AgentStatusEnum.Active), [agents]);
  const avgSuccessRate = useMemo(
    () => activeAgents.reduce((s, a) => s + a.successRate, 0) / Math.max(activeAgents.length, 1),
    [activeAgents]
  );
  const totalBond = useMemo(() => agents.reduce((s, a) => s + a.bondAmount, 0), [agents]);

  const reputationChartData = useMemo(
    () =>
      agents
        .filter((a) => a.status === AgentStatusEnum.Active)
        .map((a) => ({
          agent: a.id,
          reputation: a.reputation,
          successRate: a.successRate,
        })),
    [agents]
  );

  const bondChartData = useMemo(()=> agents.filter((a) => a.bondAmount > 0).map((a) => ({ agent: a.id, bond: a.bondAmount })), [agents]);

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-baseline gap-4">
        <h1 className="text-3xl font-bold">Agent Registry</h1>
        <span className="text-gray-400">{activeAgents.length} active agents</span>
      </div>

      <div className="grid grid-cols-4 gap-4">
        <div className="bg-x3-dark-gray p-4 rounded border border-x3-dark">
          <div className="text-xs text-gray-400 uppercase tracking-wide mb-2">Total Agents</div>
          <div className="text-2xl font-bold">{agents.length}</div>
        </div>
        <div className="bg-x3-dark-gray p-4 rounded border border-x3-dark">
          <div className="text-xs text-gray-400 uppercase tracking-wide mb-2">Total Bond</div>
          <div className="text-2xl font-bold font-mono">{totalBond.toLocaleString()}</div>
        </div>
        <div className="bg-x3-dark-gray p-4 rounded border border-x3-dark">
          <div className="text-xs text-gray-400 uppercase tracking-wide mb-2">Avg Success Rate</div>
          <div className="text-2xl font-bold text-green-400">{avgSuccessRate.toFixed(1)}%</div>
        </div>
        <div className="bg-x3-dark-gray p-4 rounded border border-x3-dark">
          <div className="text-xs text-gray-400 uppercase tracking-wide mb-2">Total Slashes</div>
          <div className="text-2xl font-bold text-red-400">{agents.reduce((s, a) => s + a.totalSlashes, 0)}</div>
        </div>
      </div>

      <div className="grid grid-cols-2 gap-4">
        <div className="bg-x3-dark p-4 rounded border border-x3-dark-gray">
          <h2 className="text-lg font-bold mb-4">Reputation & Success Rate</h2>
          <AgentReputationChart data={reputationChartData} />
        </div>
        <div className="bg-x3-dark p-4 rounded border border-x3-dark-gray">
          <h2 className="text-lg font-bold mb-4">Bond Distribution</h2>
          <BondDistributionChart data={bondChartData} />
        </div>
      </div>

      <div className="bg-x3-dark p-4 rounded border border-x3-dark-gray">
        <h2 className="text-lg font-bold mb-4">Full Registry</h2>
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead className="text-xs text-gray-400 uppercase tracking-wider border-b border-x3-dark-gray">
              <tr>
                <th className="text-left py-2">Agent</th>
                <th className="text-left py-2">Status</th>
                <th className="text-left py-2">Bond</th>
                <th className="text-left py-2">Reputation</th>
                <th className="text-left py-2">Success Rate</th>
                <th className="text-left py-2">Executions</th>
              </tr>
            </thead>
            <tbody>
              {agents.map((agent) => (
                <tr key={agent.id} className="border-b border-x3-dark hover:bg-x3-dark-gray">
                  <td className="py-2 font-mono text-xs">{agent.id}</td>
                  <td className="py-2">
                    <Badge variant={statusColor(agent.status)}>{agent.status}</Badge>
                  </td>
                  <td className="py-2 font-mono">{agent.bondAmount.toLocaleString()}</td>
                  <td className="py-2 font-mono text-green-400">{agent.reputation.toFixed(1)}</td>
                  <td className="py-2 font-mono">{agent.successRate.toFixed(1)}%</td>
                  <td className="py-2 font-mono">{agent.totalExecutions.toLocaleString()}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}
