// Slashing Page — slash history and constitution reference with analytics

import { useState, useEffect, useMemo } from "react";
import { SlashSeverity } from "../types";
import type { SlashEvent } from "../types";
import { getSlashEvents } from "../services/api";
import { SlashSeverityChart, type SlashCounts } from "../components/Charts";
import { dataIntegrity } from "../services/dataIntegrity";

const DEMO_SLASHES: SlashEvent[] = [
  {
    id: "slash-001",
    agentId: "agent-delta",
    severity: SlashSeverity.Critical,
    reason: "Double execution of intent 0xd4c3..9f87",
    amountSlashed: 10_000,
    proofHash: "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2",
    timestamp: Date.now() - 3_600_000,
  },
  {
    id: "slash-002",
    agentId: "agent-delta",
    severity: SlashSeverity.Major,
    reason: "State divergence during execution replay",
    amountSlashed: 5_000,
    proofHash: "b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3",
    timestamp: Date.now() - 86_400_000,
  },
  {
    id: "slash-003",
    agentId: "agent-bravo",
    severity: SlashSeverity.Moderate,
    reason: "Failed to repay flashloan within deadline",
    amountSlashed: 2_500,
    proofHash: "c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4",
    timestamp: Date.now() - 172_800_000,
  },
  {
    id: "slash-004",
    agentId: "agent-delta",
    severity: SlashSeverity.Minor,
    reason: "Exceeded fee cap by 2.1%",
    amountSlashed: 1_000,
    proofHash: "d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5",
    timestamp: Date.now() - 259_200_000,
  },
  {
    id: "slash-005",
    agentId: "agent-bravo",
    severity: SlashSeverity.Minor,
    reason: "Submitted intent without sufficient bond",
    amountSlashed: 500,
    proofHash: "e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6",
    timestamp: Date.now() - 345_600_000,
  },
];

function severityColor(severity: SlashSeverity): "red" | "amber" | "muted" {
  switch (severity) {
    case SlashSeverity.Critical:
    case SlashSeverity.Major:
      return "red";
    case SlashSeverity.Moderate:
      return "amber";
    case SlashSeverity.Minor:
      return "muted";
  }
}

function relativeTime(ts: number): string {
  const hours = Math.floor((Date.now() - ts) / 3_600_000);
  if (hours < 1) return "< 1h ago";
  if (hours < 24) return `${hours}h ago`;
  return `${Math.floor(hours / 24)}d ago`;
}

export function SlashingPage() {
  const [slashes, setSlashes] = useState<SlashEvent[]>(DEMO_SLASHES);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    setLoading(true);
    getSlashEvents(undefined, 1, 100)
      .then((res) => {
        if (res && res.items) {
          setSlashes(res.items);
        }
      })
      .catch((e: unknown) => {
        // Use demo data — raise integrity flag so banner alerts the user.
        dataIntegrity.reportDemoFallback(
          "SlashingPage",
          e instanceof Error ? e.message : String(e),
        );
      })
      .finally(() => setLoading(false));
  }, []);

  const totalSlashed = useMemo(() => slashes.reduce((s, e) => s + e.amountSlashed, 0), [slashes]);

  const severityDistribution: SlashCounts[] = useMemo(() => {
    const counts: Record<SlashSeverity, number> = {
      [SlashSeverity.Critical]: 0,
      [SlashSeverity.Major]: 0,
      [SlashSeverity.Moderate]: 0,
      [SlashSeverity.Minor]: 0,
    };
    slashes.forEach((s) => {
      counts[s.severity]++;
    });
    return Object.entries(counts).map(([severity, count]) => ({
      severity: severity as SlashSeverity,
      count,
    }));
  }, [slashes]);

  const topSlashedAgents = useMemo(() => {
    const agentTotals: Record<string, { count: number; amount: number }> = {};
    slashes.forEach((s) => {
      if (!agentTotals[s.agentId]) {
        agentTotals[s.agentId] = { count: 0, amount: 0 };
      }
      agentTotals[s.agentId].count++;
      agentTotals[s.agentId].amount += s.amountSlashed;
    });
    return Object.entries(agentTotals)
      .sort((a, b) => b[1].amount - a[1].amount)
      .slice(0, 5);
  }, [slashes]);

  return (
    <div className="page">
      <div className="page-header">
        <h1>Slashing Ledger</h1>
        <span className="subtitle">Immutable. Deterministic. Automatic {loading ? "(Loading...)" : ""}</span>
      </div>

      <div className="stats-grid">
        <div className="stat-card">
          <div className="stat-label">Total Slashes</div>
          <div className="stat-value red">{slashes.length}</div>
        </div>
        <div className="stat-card">
          <div className="stat-label">Total Amount Slashed</div>
          <div className="stat-value red mono">{totalSlashed.toLocaleString()}</div>
        </div>
        <div className="stat-card">
          <div className="stat-label">Critical Events</div>
          <div className="stat-value red">
            {slashes.filter((s) => s.severity === SlashSeverity.Critical).length}
          </div>
        </div>
        <div className="stat-card">
          <div className="stat-label">Unique Agents Slashed</div>
          <div className="stat-value amber">
            {new Set(slashes.map((s) => s.agentId)).size}
          </div>
        </div>
      </div>

      {/* Analytics Charts */}
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16, marginBottom: 24 }}>
        <div className="card">
          <div className="card-header">
            <h2>Slash Severity Distribution</h2>
          </div>
          {severityDistribution.length > 0 && <SlashSeverityChart data={severityDistribution} />}
        </div>

        <div className="card">
          <div className="card-header">
            <h2>Top Slashed Agents</h2>
          </div>
          <div style={{ padding: 16 }}>
            {topSlashedAgents.length > 0 ? (
              topSlashedAgents.map(([agentId, { count, amount }], idx) => (
                <div key={agentId} style={{ marginBottom: 16 }}>
                  <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 6 }}>
                    {idx + 1}. {agentId}
                  </div>
                  <div style={{ display: "flex", justifyContent: "space-between", fontSize: 11, marginBottom: 4 }}>
                    <span className="secondary">Events: {count}</span>
                    <span className="red mono">Amount: {amount.toLocaleString()}</span>
                  </div>
                  <div
                    style={{
                      height: 6,
                      background: "var(--bg-tertiary)",
                      borderRadius: 3,
                      overflow: "hidden",
                    }}
                  >
                    <div
                      style={{
                        height: "100%",
                        width: `${Math.min((amount / (totalSlashed || 1)) * 100, 100)}%`,
                        background: "var(--accent-red)",
                      }}
                    />
                  </div>
                </div>
              ))
            ) : (
              <p className="muted">No slash data available.</p>
            )}
          </div>
        </div>
      </div>

      <div className="card">
        <div className="card-header">
          <h2>Slash History</h2>
          <span className="secondary mono" style={{ fontSize: 11 }}>{slashes.length} events</span>
        </div>
        <div className="table-wrapper">
          <table>
            <thead>
              <tr>
                <th>ID</th>
                <th>Agent</th>
                <th>Severity</th>
                <th>Reason</th>
                <th>Amount</th>
                <th>Proof</th>
                <th>When</th>
              </tr>
            </thead>
            <tbody>
              {slashes.map((slash) => (
                <tr key={slash.id}>
                  <td className="mono" style={{ fontSize: 12 }}>{slash.id}</td>
                  <td className="mono" style={{ fontSize: 12 }}>{slash.agentId}</td>
                  <td>
                    <span className={`badge badge-${severityColor(slash.severity)}`}>
                      {slash.severity}
                    </span>
                  </td>
                  <td style={{ fontSize: 12, maxWidth: 300 }}>{slash.reason}</td>
                  <td className="mono red">{slash.amountSlashed.toLocaleString()}</td>
                  <td className="hash">{slash.proofHash.slice(0, 16)}...</td>
                  <td className="secondary" style={{ fontSize: 12 }}>
                    {relativeTime(slash.timestamp)}
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
