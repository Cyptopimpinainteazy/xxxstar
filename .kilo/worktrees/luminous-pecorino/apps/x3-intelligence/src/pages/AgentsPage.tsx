// Agents Page — registry, reputation, and history with visual analytics

import { useState, useMemo } from "react";
import { AgentStatus } from "../types";
import type { Agent } from "../types";
import { Button, ProgressBar, Badge, Metric } from "../components/UIComponents";
import { AgentReputationChart, BondDistributionChart } from "../components/Charts";

const DEMO_AGENTS: Agent[] = [
  {
    id: "agent-alpha",
    status: AgentStatus.Active,
    bondAmount: 50_000,
    reputation: 97.2,
    successRate: 98.1,
    totalExecutions: 4_821,
    totalSlashes: 1,
    registeredAt: Date.now() - 86_400_000 * 30,
  },
  {
    id: "agent-bravo",
    status: AgentStatus.Active,
    bondAmount: 25_000,
    reputation: 88.4,
    successRate: 94.3,
    totalExecutions: 2_150,
    totalSlashes: 3,
    registeredAt: Date.now() - 86_400_000 * 15,
  },
  {
    id: "agent-charlie",
    status: AgentStatus.Active,
    bondAmount: 100_000,
    reputation: 99.1,
    successRate: 99.4,
    totalExecutions: 8_392,
    totalSlashes: 0,
    registeredAt: Date.now() - 86_400_000 * 60,
  },
  {
    id: "agent-delta",
    status: AgentStatus.Suspended,
    bondAmount: 10_000,
    reputation: 42.1,
    successRate: 67.2,
    totalExecutions: 312,
    totalSlashes: 8,
    registeredAt: Date.now() - 86_400_000 * 7,
  },
  {
    id: "agent-echo",
    status: AgentStatus.Deactivated,
    bondAmount: 0,
    reputation: 0.0,
    successRate: 0.0,
    totalExecutions: 14,
    totalSlashes: 14,
    registeredAt: Date.now() - 86_400_000 * 3,
  },
];

function statusColor(status: AgentStatus): string {
  switch (status) {
    case AgentStatus.Active:
      return "badge-green";
    case AgentStatus.Suspended:
      return "badge-amber";
    case AgentStatus.Deactivated:
      return "badge-red";
    case AgentStatus.Deregistered:
      return "badge-muted";
  }
}

function repColor(rep: number): string {
  if (rep >= 90) return "green";
  if (rep >= 70) return "amber";
  return "red";
}

function daysSince(ts: number): string {
  const days = Math.floor((Date.now() - ts) / 86_400_000);
  return `${days}d`;
}

export function AgentsPage() {
  const [agents] = useState<Agent[]>(DEMO_AGENTS);
  const [selectedAgent, setSelectedAgent] = useState<Agent | null>(null);

  const activeAgents = useMemo(() => agents.filter((a) => a.status === AgentStatus.Active), [agents]);
  const avgSuccessRate = useMemo(
    () => activeAgents.reduce((s, a) => s + a.successRate, 0) / Math.max(activeAgents.length, 1),
    [activeAgents]
  );
  const totalBond = useMemo(() => agents.reduce((s, a) => s + a.bondAmount, 0), [agents]);
  const totalSlashes = useMemo(() => agents.reduce((s, a) => s + a.totalSlashes, 0), [agents]);

  const reputationChartData = useMemo(
    () =>
      agents
        .filter((a) => a.status === AgentStatus.Active)
        .map((a) => ({
          agent: a.id,
          reputation: a.reputation,
          successRate: a.successRate,
        })),
    [agents]
  );

  const bondChartData = useMemo(
    () =>
      agents
        .filter((a) => a.bondAmount > 0)
        .map((a) => ({
          agent: a.id,
          bond: a.bondAmount,
        })),
    [agents]
  );

  return (
    <div className="page">
      <div className="page-header">
        <h1>Agent Registry</h1>
        <span className="subtitle">{activeAgents.length} active agents</span>
      </div>

      {/* Summary stats */}
      <div className="stats-grid">
        <div className="stat-card">
          <div className="stat-label">Total Agents</div>
          <div className="stat-value">{agents.length}</div>
        </div>
        <div className="stat-card">
          <div className="stat-label">Total Bond Locked</div>
          <div className="stat-value mono">{totalBond.toLocaleString()}</div>
        </div>
        <div className="stat-card">
          <div className="stat-label">Avg Success Rate</div>
          <div className="stat-value green">{avgSuccessRate.toFixed(1)}%</div>
        </div>
        <div className="stat-card">
          <div className="stat-label">Total Slashes</div>
          <div className="stat-value red">{totalSlashes}</div>
        </div>
      </div>

      {/* Analytics Charts */}
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16, marginBottom: 24 }}>
        <div className="card">
          <div className="card-header">
            <h2>Agent Reputation & Success Rate</h2>
          </div>
          <AgentReputationChart data={reputationChartData} />
        </div>

        <div className="card">
          <div className="card-header">
            <h2>Bond Distribution</h2>
          </div>
          <BondDistributionChart data={bondChartData} />
        </div>
      </div>

      {/* Top Performers */}
      <div className="card">
        <div className="card-header">
          <h2>Top Performers</h2>
        </div>
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "repeat(auto-fit, minmax(250px, 1fr))",
            gap: 16,
          }}
        >
          {agents
            .filter((a) => a.status === AgentStatus.Active)
            .sort((a, b) => b.reputation - a.reputation)
            .slice(0, 3)
            .map((agent) => (
              <div
                key={agent.id}
                style={{
                  padding: 16,
                  background: "var(--bg-tertiary)",
                  borderRadius: "var(--radius-md)",
                  border: "1px solid var(--border)",
                }}
              >
                <div style={{ marginBottom: 12 }}>
                  <div className="metric-label">{agent.id}</div>
                  <div style={{ marginTop: 4 }}>
                    <Badge variant="green">Active</Badge>
                  </div>
                </div>
                <Metric label="Reputation" value={agent.reputation.toFixed(1)} highlight />
                <Metric label="Success Rate" value={`${agent.successRate.toFixed(1)}%`} />
                <Metric label="Executions" value={agent.totalExecutions.toLocaleString()} />
                <Metric label="Bond" value={`${(agent.bondAmount / 1000).toFixed(0)}K`} unit="USDC" />
              </div>
            ))}
        </div>
      </div>

      {/* Full Registry Table */}
      <div className="card" style={{ marginTop: 24 }}>
        <div className="card-header">
          <h2>Full Agent Registry</h2>
          <span className="secondary mono" style={{ fontSize: 11 }}>
            {agents.length} total
          </span>
        </div>
        <div className="table-wrapper">
          <table>
            <thead>
              <tr>
                <th>Agent</th>
                <th>Status</th>
                <th>Bond</th>
                <th>Reputation</th>
                <th>Success Rate</th>
                <th>Executions</th>
                <th>Slashes</th>
                <th>Registered</th>
                <th>Action</th>
              </tr>
            </thead>
            <tbody>
              {agents.map((agent) => (
                <tr key={agent.id}>
                  <td className="mono" style={{ fontSize: 12, fontWeight: 600 }}>
                    {agent.id}
                  </td>
                  <td>
                    <Badge variant={statusColor(agent.status) as any}>
                      {agent.status}
                    </Badge>
                  </td>
                  <td className="mono">{agent.bondAmount.toLocaleString()}</td>
                  <td>
                    <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                      <span className={`mono ${repColor(agent.reputation)}`}>
                        {agent.reputation.toFixed(1)}
                      </span>
                      <ProgressBar value={agent.reputation} max={100} color="green" />
                    </div>
                  </td>
                  <td>
                    <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                      <span className={`mono ${repColor(agent.successRate)}`}>
                        {agent.successRate.toFixed(1)}%
                      </span>
                      <ProgressBar value={agent.successRate} max={100} color="blue" />
                    </div>
                  </td>
                  <td className="mono">{agent.totalExecutions.toLocaleString()}</td>
                  <td className="mono">
                    {agent.totalSlashes > 0 ? (
                      <span className="red">{agent.totalSlashes}</span>
                    ) : (
                      <span className="muted">0</span>
                    )}
                  </td>
                  <td className="secondary" style={{ fontSize: 12 }}>
                    {daysSince(agent.registeredAt)}
                  </td>
                  <td>
                    <Button
                      size="sm"
                      variant="secondary"
                      onClick={() => setSelectedAgent(agent)}
                    >
                      View
                    </Button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}
