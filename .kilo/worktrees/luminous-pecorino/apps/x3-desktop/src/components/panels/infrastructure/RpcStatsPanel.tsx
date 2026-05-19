/**
 * RpcStatsPanel — Full-page RPC pool statistics dashboard.
 *
 * Deep-dive into: Provider breakdown, endpoint health, gas savings comparison,
 * latency distribution, chain coverage, tier breakdown, fastest/slowest endpoints.
 * Linked from InfrastructurePanel's RPC Pool / Gas Savings cards.
 */
import React, { useState, useEffect, useCallback } from 'react';
import { useRpcStats } from '@/hooks/useSubstrate';

/* ── Types ─────────────────────────────────────────── */
interface ProviderStat {
  provider: string;
  count: number;
  avg_latency: number;
  rps: number;
}

interface TierStat {
  tier: string;
  count: number;
}

interface FastEndpoint {
  chain_id: string;
  url: string;
  provider: string;
  latency_ms: number;
  rate_limit_rps: number;
}

interface GasSavings {
  infura_growth_equiv: number;
  alchemy_growth_equiv: number;
  quicknode_build_equiv: number;
  moralis_pro_equiv: number;
  total_monthly_saved: number;
  your_cost: number;
}

interface RpcPoolStats {
  total_endpoints: number;
  healthy_endpoints: number;
  chains_covered: number;
  combined_rps: number;
  avg_latency_ms: number;
  min_latency_ms: number;
  by_provider: ProviderStat[];
  by_tier: TierStat[];
  top_fastest: FastEndpoint[];
  gas_savings: GasSavings;
}

/* ── Constants ─────────────────────────────────────── */
const CHAIN_DB_URL = 'http://localhost:7070';

async function fetchJSON<T>(url: string): Promise<T | null> {
  try {
    const res = await fetch(url, { signal: AbortSignal.timeout(5000) });
    if (!res.ok) return null;
    return await res.json();
  } catch { return null; }
}

/* ── Component ─────────────────────────────────────── */
const RpcStatsPanel: React.FC = () => {
  const [data, setData] = useState<RpcPoolStats | null>(null);
  const [loading, setLoading] = useState(true);
  const [lastUpdate, setLastUpdate] = useState('');

  const { data: realStats } = useRpcStats();

  const load = useCallback(async () => {
    const d = await fetchJSON<RpcPoolStats>(`${CHAIN_DB_URL}/api/rpc/stats`);
    if (d) setData(d);
    setLastUpdate(new Date().toLocaleTimeString());
    setLoading(false);
  }, []);

  useEffect(() => {
    load();
    const iv = setInterval(load, 10000);
    return () => clearInterval(iv);
  }, [load]);

  /* ── Styles ── */
  const s = {
    root: { display: 'flex', flexDirection: 'column' as const, height: '100%', background: '#0a0e17', color: '#e0e0e0', fontFamily: '-apple-system, BlinkMacSystemFont, monospace', fontSize: '0.8rem', overflow: 'auto' },
    header: { display: 'flex', alignItems: 'center', gap: 8, padding: '12px 16px', borderBottom: '1px solid #1a1f2e', flexShrink: 0 },
    grid: { display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(180px, 1fr))', gap: 10, padding: 16 },
    card: { background: '#111827', border: '1px solid #1f2937', borderRadius: 10, padding: '14px 16px' },
    title: { fontSize: '0.68rem', color: '#9ca3af', textTransform: 'uppercase' as const, letterSpacing: 1, marginBottom: 8 },
    big: { fontSize: '1.4rem', fontWeight: 700 },
    sub: { fontSize: '0.7rem', color: '#6b7280', marginTop: 2 },
    section: { padding: '0 16px 16px' },
    table: { width: '100%', borderCollapse: 'collapse' as const, fontSize: '0.72rem' },
    th: { textAlign: 'left' as const, padding: '6px 8px', color: '#555', fontWeight: 600, borderBottom: '1px solid #1f2937', fontSize: '0.68rem' },
    td: { padding: '5px 8px', borderBottom: '1px solid #1a1f2e' },
  };

  const pct = (v: number, max: number) => Math.max(1, (v / Math.max(max, 1)) * 100);

  if (loading) {
    return (
      <div style={s.root}>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', flex: 1, color: '#666' }}>Loading RPC stats…</div>
      </div>
    );
  }

  if (!data && !realStats) {
    return (
      <div style={s.root}>
        <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', flex: 1, gap: 8 }}>
          <span style={{ fontSize: '2rem' }}>🔌</span>
          <span style={{ color: '#ef4444', fontWeight: 700 }}>Node RPC Offlinel</span>
          <span style={{ color: '#6b7280', fontSize: '0.72rem' }}>Waiting for websocket connection...</span>
          <button onClick={load} style={{ marginTop: 8, background: '#1f2937', border: '1px solid #374151', borderRadius: 6, padding: '6px 14px', color: '#e0e0e0', cursor: 'pointer', fontSize: '0.72rem' }}>↻ Retry</button>
        </div>
      </div>
    );
  }

  const safeData: RpcPoolStats = data || {
    total_endpoints: 1,
    healthy_endpoints: 1,
    chains_covered: 1,
    combined_rps: 0,
    avg_latency_ms: 0,
    min_latency_ms: 0,
    by_provider: [],
    by_tier: [],
    top_fastest: [],
    gas_savings: { infura_growth_equiv: 225, alchemy_growth_equiv: 199, quicknode_build_equiv: 299, moralis_pro_equiv: 299, total_monthly_saved: 1250, your_cost: 0 },
  };

  const deadCount = safeData.total_endpoints - safeData.healthy_endpoints;
  const healthPct = safeData.total_endpoints > 0 ? ((safeData.healthy_endpoints / safeData.total_endpoints) * 100).toFixed(1) : '100.0';
  const maxProviderCount = Math.max(...safeData.by_provider.map(p => p.count), 1);
  const maxRps = Math.max(...safeData.by_provider.map(p => p.rps || 0), 1);

  const calculatedCost = safeData.gas_savings;

  const plans = [
    { name: 'Infura Growth', cost: calculatedCost.infura_growth_equiv, color: '#f59e0b', desc: '$225/mo per 50 RPS' },
    { name: 'Alchemy Growth', cost: calculatedCost.alchemy_growth_equiv, color: '#8b5cf6', desc: '$199/mo per 660 RPS' },
    { name: 'QuickNode Build', cost: calculatedCost.quicknode_build_equiv, color: '#3b82f6', desc: '$299/mo per 300 RPS' },
    { name: 'Moralis Pro', cost: calculatedCost.moralis_pro_equiv, color: '#ec4899', desc: '$299/mo per 500 RPS' },
    { name: 'X3 Chain', cost: 0, color: '#10b981', desc: 'Free forever 🎯' },
  ];
  const maxPlanCost = Math.max(...plans.map(p => p.cost), 1);

  return (
    <div style={s.root}>
      {/* Header */}
      <div style={s.header}>
        <span style={{ fontSize: '1.1rem' }}>🔗</span>
        <span style={{ fontWeight: 700, fontSize: '0.95rem' }}>RPC Pool Statistics</span>
        <div style={{ flex: 1 }} />
        <span style={{ fontSize: '0.65rem', color: '#555' }}>{lastUpdate}</span>
        <button onClick={load} style={{ background: 'transparent', border: '1px solid #2a2f3e', borderRadius: 6, padding: '3px 8px', color: '#999', cursor: 'pointer', fontSize: '0.72rem', marginLeft: 6 }}>↻ Refresh</button>
      </div>

      {/* Top metric cards */}
      <div style={s.grid}>
        <div style={s.card}>
          <div style={s.title}>Total Endpoints</div>
          <div style={{ ...s.big, color: '#3b82f6' }}>{data ? data.total_endpoints.toLocaleString() : 1}</div>
          <div style={s.sub}>{data ? data.healthy_endpoints : 1} healthy · {deadCount} dead</div>
        </div>

        <div style={s.card}>
          <div style={s.title}>Health Rate</div>
          <div style={{ ...s.big, color: parseFloat(healthPct) > 80 ? '#10b981' : parseFloat(healthPct) > 50 ? '#f59e0b' : '#ef4444' }}>{healthPct}%</div>
          <div style={s.sub}>{data ? data.healthy_endpoints : 1} of {data ? data.total_endpoints : 1}</div>
        </div>

        <div style={s.card}>
          <div style={s.title}>Total RPC Requests</div>
          <div style={{ ...s.big, color: '#a78bfa' }}>{realStats ? realStats.total_requests.toLocaleString() : 0}</div>
          <div style={s.sub}>{realStats ? `${realStats.total_rejected.toLocaleString()} rate limited` : 'Processed by local node'}</div>
        </div>

        <div style={s.card}>
          <div style={s.title}>Active Connections</div>
          <div style={{ ...s.big, color: '#f59e0b' }}>{realStats ? realStats.active_connections.toLocaleString() : 0}</div>
          <div style={s.sub}>Connected WebSocket clients</div>
        </div>

        <div style={s.card}>
          <div style={s.title}>Avg Latency</div>
          <div style={{ ...s.big, color: (safeData ? safeData.avg_latency_ms : 0) < 200 ? '#10b981' : '#f59e0b' }}>{safeData ? safeData.avg_latency_ms : '<1'}ms</div>
          <div style={s.sub}>best: {safeData ? safeData.min_latency_ms : '<1'}ms</div>
        </div>

        <div style={{ ...s.card, border: '1px solid #065f46', background: 'linear-gradient(135deg, #064e3b22, #111827)' }}>
          <div style={s.title}>💰 Monthly Savings</div>
          <div style={{ ...s.big, color: '#10b981' }}>${safeData.gas_savings.total_monthly_saved.toLocaleString()}</div>
          <div style={{ ...s.sub, color: '#10b981', fontWeight: 700 }}>Your cost: $0.00</div>
        </div>
      </div>

      {/* Gas Savings Comparison */}
      <div style={s.section}>
        <div style={{ ...s.card, background: 'linear-gradient(135deg, #064e3b0d, #111827)' }}>
          <div style={s.title}>💰 Monthly Cost Comparison — You vs. Paid Providers</div>
          <div style={{ marginTop: 8 }}>
            {plans.map((plan, i) => (
              <div key={i} style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 8 }}>
                <span style={{ width: 130, textAlign: 'right', color: '#9ca3af', fontSize: '0.72rem' }}>{plan.name}</span>
                <div style={{ flex: 1, height: 18, background: '#1f2937', borderRadius: 4, overflow: 'hidden', position: 'relative' as const }}>
                  <div style={{ width: `${plan.cost === 0 ? 1 : pct(plan.cost, maxPlanCost)}%`, height: '100%', background: plan.color, borderRadius: 4, transition: 'width 0.6s ease' }} />
                </div>
                <span style={{ width: 90, textAlign: 'right', fontWeight: 700, fontSize: '0.78rem', color: plan.cost === 0 ? '#10b981' : plan.color }}>
                  {plan.cost === 0 ? 'FREE 🎯' : `$${plan.cost.toLocaleString()}`}
                </span>
                <span style={{ width: 160, fontSize: '0.65rem', color: '#555' }}>{plan.desc}</span>
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* Provider Breakdown + Tier Breakdown */}
      <div style={{ display: 'grid', gridTemplateColumns: '2fr 1fr', gap: 10, padding: '0 16px 16px' }}>
        {/* Provider table */}
        <div style={s.card}>
          <div style={s.title}>📊 Provider Breakdown</div>
          <table style={s.table}>
            <thead>
              <tr>
                <th style={s.th}>Provider</th>
                <th style={{ ...s.th, textAlign: 'right' as const }}>Endpoints</th>
                <th style={{ ...s.th, width: 160 }}>Distribution</th>
                <th style={{ ...s.th, textAlign: 'right' as const }}>RPS</th>
                <th style={{ ...s.th, textAlign: 'right' as const }}>Avg Latency</th>
              </tr>
            </thead>
            <tbody>
              {safeData.by_provider.map((p, i) => (
                <tr key={i}>
                  <td style={{ ...s.td, color: '#e0e0e0', fontWeight: 500 }}>{p.provider || 'unknown'}</td>
                  <td style={{ ...s.td, textAlign: 'right', color: '#9ca3af' }}>{p.count}</td>
                  <td style={s.td}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
                      <div style={{ flex: 1, height: 8, background: '#1f2937', borderRadius: 3, overflow: 'hidden' }}>
                        <div style={{ width: `${pct(p.count, maxProviderCount)}%`, height: '100%', background: ['#3b82f6','#8b5cf6','#f59e0b','#ec4899','#10b981','#06b6d4','#f97316','#84cc16'][i % 8], borderRadius: 3 }} />
                      </div>
                      <span style={{ fontSize: '0.6rem', color: '#555', width: 30 }}>{((p.count / safeData.healthy_endpoints) * 100).toFixed(0)}%</span>
                    </div>
                  </td>
                  <td style={{ ...s.td, textAlign: 'right', color: '#3b82f6' }}>{(p.rps || 0).toLocaleString()}</td>
                  <td style={{ ...s.td, textAlign: 'right', color: p.avg_latency < 100 ? '#10b981' : p.avg_latency < 300 ? '#f59e0b' : '#ef4444' }}>
                    {Math.round(p.avg_latency)}ms
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        {/* Tier breakdown */}
        <div style={s.card}>
          <div style={s.title}>🏷️ Tier Breakdown</div>
          {safeData.by_tier.map((t, i) => {
            const tierColors: Record<string, string> = { public: '#10b981', authenticated: '#f59e0b', premium: '#8b5cf6' };
            const color = tierColors[t.tier] || '#6b7280';
            return (
              <div key={i} style={{ marginBottom: 10 }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 3 }}>
                  <span style={{ color, fontWeight: 600, fontSize: '0.72rem', textTransform: 'capitalize' }}>{t.tier}</span>
                  <span style={{ color: '#9ca3af', fontSize: '0.72rem' }}>{t.count}</span>
                </div>
                <div style={{ height: 10, background: '#1f2937', borderRadius: 4, overflow: 'hidden' }}>
                  <div style={{ width: `${pct(t.count, safeData.total_endpoints)}%`, height: '100%', background: color, borderRadius: 4 }} />
                </div>
              </div>
            );
          })}

          {/* RPS breakdown by tier would go here when data available */}
          <div style={{ marginTop: 16, paddingTop: 12, borderTop: '1px solid #1f2937' }}>
            <div style={s.title}>⚡ Capacity Summary</div>
            <div style={{ fontSize: '0.72rem', color: '#9ca3af', lineHeight: 1.8 }}>
              <div>Total RPS: <span style={{ color: '#f59e0b', fontWeight: 700 }}>{safeData.combined_rps.toLocaleString()}</span></div>
              <div>Healthy: <span style={{ color: '#10b981', fontWeight: 700 }}>{safeData.healthy_endpoints}</span></div>
              <div>Dead: <span style={{ color: '#ef4444', fontWeight: 700 }}>{deadCount}</span></div>
              <div>Chains: <span style={{ color: '#a78bfa', fontWeight: 700 }}>{safeData.chains_covered}</span></div>
            </div>
          </div>
        </div>
      </div>

      {/* Fastest Endpoints */}
      <div style={s.section}>
        <div style={s.card}>
          <div style={s.title}>🏆 Fastest Endpoints (Top 10)</div>
          <table style={s.table}>
            <thead>
              <tr>
                <th style={{ ...s.th, width: 30 }}>#</th>
                <th style={{ ...s.th, width: 70 }}>Latency</th>
                <th style={s.th}>Provider</th>
                <th style={s.th}>Chain</th>
                <th style={s.th}>URL</th>
                <th style={{ ...s.th, textAlign: 'right' as const }}>RPS</th>
              </tr>
            </thead>
            <tbody>
              {safeData.top_fastest.map((ep, i) => (
                <tr key={i}>
                  <td style={{ ...s.td, color: i < 3 ? '#f59e0b' : '#555', fontWeight: i < 3 ? 700 : 400 }}>
                    {i < 3 ? ['🥇','🥈','🥉'][i] : `#${i+1}`}
                  </td>
                  <td style={{ ...s.td, color: '#10b981', fontWeight: 700 }}>{Math.round(ep.latency_ms)}ms</td>
                  <td style={{ ...s.td, color: '#e0e0e0' }}>{ep.provider || 'unknown'}</td>
                  <td style={{ ...s.td, color: '#a78bfa' }}>{ep.chain_id}</td>
                  <td style={{ ...s.td, color: '#6b7280', fontSize: '0.65rem', maxWidth: 300, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' as const }}>
                    {ep.url}
                  </td>
                  <td style={{ ...s.td, textAlign: 'right', color: '#3b82f6' }}>{(ep.rate_limit_rps || 0).toLocaleString()}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {/* Provider RPS Distribution Chart */}
      <div style={s.section}>
        <div style={s.card}>
          <div style={s.title}>📈 Provider RPS Distribution</div>
          <div style={{ display: 'flex', alignItems: 'flex-end', gap: 3, height: 120, marginTop: 8, padding: '0 8px' }}>
            {safeData.by_provider.filter(p => (p.rps || 0) > 0).slice(0, 15).map((p, i) => {
              const h = pct(p.rps || 0, maxRps);
              const colors = ['#3b82f6','#8b5cf6','#f59e0b','#ec4899','#10b981','#06b6d4','#f97316','#84cc16'];
              return (
                <div key={i} style={{ flex: 1, display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 2 }}>
                  <span style={{ fontSize: '0.55rem', color: '#555', transform: 'rotate(-45deg)', transformOrigin: 'center', whiteSpace: 'nowrap' as const }}>{(p.rps || 0).toLocaleString()}</span>
                  <div style={{ width: '100%', height: `${h}%`, background: colors[i % colors.length], borderRadius: '3px 3px 0 0', minHeight: 2, transition: 'height 0.5s ease' }} />
                  <span style={{ fontSize: '0.5rem', color: '#555', transform: 'rotate(-45deg)', transformOrigin: 'center', whiteSpace: 'nowrap' as const, maxWidth: 60, overflow: 'hidden', textOverflow: 'ellipsis' }}>{p.provider}</span>
                </div>
              );
            })}
          </div>
        </div>
      </div>

      {/* Footer */}
      <div style={{ padding: '8px 16px', borderTop: '1px solid #1a1f2e', textAlign: 'center', color: '#444', fontSize: '0.6rem', flexShrink: 0 }}>
        X3 Chain RPC Pool — {safeData.healthy_endpoints} healthy endpoints across {safeData.chains_covered} chains — refreshes every 10s
      </div>
    </div>
  );
};

export default RpcStatsPanel;
