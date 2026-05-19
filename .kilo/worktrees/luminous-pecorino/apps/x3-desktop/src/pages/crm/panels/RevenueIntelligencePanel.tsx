/**
 * Revenue Intelligence Panel
 * Track: Deal pipeline value, revenue by validator tier, epoch earnings,
 * slashing events, fee market trends, and payout forecasting.
 */

import React, { useState, useMemo } from 'react';

interface Deal {
  id: string;
  client: string;
  tier: 'basic' | 'standard' | 'enterprise';
  annualValue: number;
  status: 'prospecting' | 'negotiating' | 'contracted' | 'active' | 'churned';
  probability: number;
  closeDate: string;
  validatorCount: number;
  gpuHours: number;
}

interface EpochRevenue {
  epoch: number;
  validatorRewards: number;
  protocolFees: number;
  slashingRevenue: number;
  bridgeFees: number;
  total: number;
}

interface SlashEvent {
  validatorId: string;
  epoch: number;
  amount: number;
  reason: 'downtime' | 'equivocation' | 'fraud_proof' | 'gpu_misbehaviour';
  recovered: boolean;
}

const TIER_COLORS: Record<string, string> = {
  basic: '#6366f1',
  standard: '#8b5cf6',
  enterprise: '#a855f7',
};

const STATUS_BADGES: Record<string, { label: string; color: string }> = {
  prospecting: { label: 'Prospecting', color: '#64748b' },
  negotiating:  { label: 'Negotiating',  color: '#f59e0b' },
  contracted:   { label: 'Contracted',   color: '#3b82f6' },
  active:       { label: 'Active',       color: '#22c55e' },
  churned:      { label: 'Churned',      color: '#ef4444' },
};

const SEED_DEALS: Deal[] = [
  {
    id: 'deal-001',
    client: 'NeuraScale AI',
    tier: 'enterprise',
    annualValue: 1_200_000,
    status: 'active',
    probability: 100,
    closeDate: '2026-01-15',
    validatorCount: 12,
    gpuHours: 87_600,
  },
  {
    id: 'deal-002',
    client: 'Quant Capital Group',
    tier: 'enterprise',
    annualValue: 840_000,
    status: 'contracted',
    probability: 95,
    closeDate: '2026-05-01',
    validatorCount: 8,
    gpuHours: 52_800,
  },
  {
    id: 'deal-003',
    client: 'Diffusion Labs',
    tier: 'standard',
    annualValue: 240_000,
    status: 'negotiating',
    probability: 70,
    closeDate: '2026-06-15',
    validatorCount: 3,
    gpuHours: 17_520,
  },
  {
    id: 'deal-004',
    client: 'Proof Protocol',
    tier: 'standard',
    annualValue: 180_000,
    status: 'prospecting',
    probability: 40,
    closeDate: '2026-07-30',
    validatorCount: 2,
    gpuHours: 12_000,
  },
  {
    id: 'deal-005',
    client: 'EdgeNode Co.',
    tier: 'basic',
    annualValue: 60_000,
    status: 'active',
    probability: 100,
    closeDate: '2025-11-01',
    validatorCount: 1,
    gpuHours: 8_760,
  },
];

const SEED_EPOCHS: EpochRevenue[] = Array.from({ length: 12 }, (_, i) => ({
  epoch: 1_284_380 + i,
  validatorRewards: 4_200 + Math.round(Math.sin(i) * 300),
  protocolFees: 820 + i * 12,
  slashingRevenue: i === 3 ? 15_000 : 0,
  bridgeFees: 380 + Math.round(Math.cos(i) * 80),
  total: 0,
})).map((e) => ({
  ...e,
  total: e.validatorRewards + e.protocolFees + e.slashingRevenue + e.bridgeFees,
}));

const SEED_SLASHES: SlashEvent[] = [
  {
    validatorId: 'val-007',
    epoch: 1_284_383,
    amount: 15_000,
    reason: 'equivocation',
    recovered: true,
  },
  {
    validatorId: 'val-012',
    epoch: 1_284_374,
    amount: 3_200,
    reason: 'downtime',
    recovered: false,
  },
];

const fmt = (n: number) =>
  new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD', maximumFractionDigits: 0 }).format(n);

const RevenueIntelligencePanel: React.FC = () => {
  const [deals] = useState<Deal[]>(SEED_DEALS);
  const [epochs] = useState<EpochRevenue[]>(SEED_EPOCHS);
  const [slashes] = useState<SlashEvent[]>(SEED_SLASHES);

  const pipelineStats = useMemo(() => {
    const total = deals.reduce((s, d) => s + d.annualValue * (d.probability / 100), 0);
    const contracted = deals
      .filter((d) => d.status === 'active' || d.status === 'contracted')
      .reduce((s, d) => s + d.annualValue, 0);
    const atRisk = deals
      .filter((d) => d.status === 'negotiating' || d.status === 'prospecting')
      .reduce((s, d) => s + d.annualValue * (d.probability / 100), 0);
    return { total, contracted, atRisk };
  }, [deals]);

  const epochTotals = useMemo(
    () => epochs.reduce((s, e) => s + e.total, 0),
    [epochs],
  );

  return (
    <div className="panel revenue-panel">
      <h2 className="panel-title">💰 Revenue Intelligence</h2>

      {/* KPI Row */}
      <div className="kpi-row">
        <div className="kpi-card">
          <span className="kpi-label">Weighted Pipeline</span>
          <span className="kpi-value">{fmt(pipelineStats.total)}</span>
        </div>
        <div className="kpi-card">
          <span className="kpi-label">Contracted ARR</span>
          <span className="kpi-value green">{fmt(pipelineStats.contracted)}</span>
        </div>
        <div className="kpi-card">
          <span className="kpi-label">12-Epoch Protocol Revenue</span>
          <span className="kpi-value">{fmt(epochTotals)}</span>
        </div>
        <div className="kpi-card">
          <span className="kpi-label">At-Risk Revenue</span>
          <span className="kpi-value amber">{fmt(pipelineStats.atRisk)}</span>
        </div>
      </div>

      {/* Deal Table */}
      <section className="panel-section">
        <h3>Deal Pipeline</h3>
        <table className="data-table">
          <thead>
            <tr>
              <th>Client</th>
              <th>Tier</th>
              <th>ARR</th>
              <th>Probability</th>
              <th>Validators</th>
              <th>Status</th>
              <th>Close Date</th>
            </tr>
          </thead>
          <tbody>
            {deals.map((d) => (
              <tr key={d.id}>
                <td>{d.client}</td>
                <td>
                  <span
                    className="badge"
                    style={{ backgroundColor: TIER_COLORS[d.tier] }}
                  >
                    {d.tier}
                  </span>
                </td>
                <td>{fmt(d.annualValue)}</td>
                <td>
                  <div className="prob-bar">
                    <div
                      className="prob-fill"
                      style={{ width: `${d.probability}%` }}
                    />
                    <span>{d.probability}%</span>
                  </div>
                </td>
                <td>{d.validatorCount}</td>
                <td>
                  <span
                    className="badge"
                    style={{ backgroundColor: STATUS_BADGES[d.status].color }}
                  >
                    {STATUS_BADGES[d.status].label}
                  </span>
                </td>
                <td>{d.closeDate}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>

      {/* Epoch Revenue Sparkline */}
      <section className="panel-section">
        <h3>Epoch Revenue (last 12)</h3>
        <div className="epoch-chart">
          {epochs.map((e) => (
            <div key={e.epoch} className="epoch-bar-group" title={`Epoch ${e.epoch}\n${fmt(e.total)}`}>
              <div
                className="epoch-bar"
                style={{ height: `${Math.round((e.total / (epochTotals / epochs.length)) * 40)}px` }}
              />
              <span className="epoch-label">{e.epoch % 100}</span>
            </div>
          ))}
        </div>
        <div className="epoch-legend">
          <span className="legend-item validator">Validator Rewards</span>
          <span className="legend-item fees">Protocol Fees</span>
          <span className="legend-item bridge">Bridge Fees</span>
          <span className="legend-item slash">Slashing Revenue</span>
        </div>
      </section>

      {/* Slash Events */}
      {slashes.length > 0 && (
        <section className="panel-section">
          <h3>Recent Slash Events</h3>
          <table className="data-table">
            <thead>
              <tr>
                <th>Validator</th>
                <th>Epoch</th>
                <th>Amount Slashed</th>
                <th>Reason</th>
                <th>Recovered</th>
              </tr>
            </thead>
            <tbody>
              {slashes.map((s) => (
                <tr key={`${s.validatorId}-${s.epoch}`} className={s.recovered ? '' : 'row-warn'}>
                  <td>{s.validatorId}</td>
                  <td>{s.epoch}</td>
                  <td>{fmt(s.amount)}</td>
                  <td>
                    <span className="badge badge-red">{s.reason.replace(/_/g, ' ')}</span>
                  </td>
                  <td>{s.recovered ? '✅' : '⚠️ Pending'}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
      )}
    </div>
  );
};

export default RevenueIntelligencePanel;
