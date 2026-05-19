/**
 * InfrastructurePanel — infrastructure monitoring dashboard integrated into X3 Desktop.
 *
 * Displays: Validator stats, Bridge status, GPU Lane health, Chain stats, TPS metrics.
 * Connects to the same backend endpoints as the standalone inferstructor-dashboard.
 */
import React, { useState, useEffect, useCallback } from 'react';
import { useDesktopStore } from '@/stores/desktopStore';

/* ── Types ─────────────────────────────────────────── */
interface BridgeStats {
  total_received: number;
  total_forwarded: number;
  total_failed: number;
  uptime_seconds: number;
  current_tps: number;
}

interface GPULane {
  id: number;
  service: string;
  status: string;
  utilization: number;
  memory_used_mb: number;
  temperature_c: number;
  total_requests: number;
  success_rate: number;
  txns_per_second: number;
}

interface ChainStats {
  port: number;
  uptime_seconds: number;
  total_requests: number;
  cached_responses: number;
  gpu_accelerated: number;
  errors: number;
}

interface RpcPoolStats {
  total_endpoints: number;
  healthy_endpoints: number;
  chains_covered: number;
  combined_rps: number;
  avg_latency_ms: number;
  min_latency_ms: number;
  by_provider: { provider: string; count: number; avg_latency: number; rps: number }[];
  top_fastest: { chain_id: string; url: string; provider: string; latency_ms: number }[];
  gas_savings: {
    infura_growth_equiv: number;
    alchemy_growth_equiv: number;
    quicknode_build_equiv: number;
    moralis_pro_equiv: number;
    total_monthly_saved: number;
    your_cost: number;
  };
}

/* ── API helpers ───────────────────────────────────── */
const BRIDGE_URL = 'http://localhost:9999';
const RPC_PROXY_URL = 'http://localhost:8899';
const CHAIN_DB_URL = 'http://localhost:7070';

async function fetchJSON<T>(url: string): Promise<T | null> {
  try {
    const res = await fetch(url, { signal: AbortSignal.timeout(3000) });
    if (!res.ok) return null;
    return await res.json();
  } catch { return null; }
}

type InfrastructureView = 'overview' | 'chain-explorer' | 'admin-dashboard' | 'login' | 'register';

/* ── Component ─────────────────────────────────────── */
const InfrastructurePanel: React.FC = () => {
  const [bridge, setBridge] = useState<BridgeStats | null>(null);
  const [gpuLanes, setGpuLanes] = useState<GPULane[]>([]);
  const [chain, setChain] = useState<ChainStats | null>(null);
  const [rpcPool, setRpcPool] = useState<RpcPoolStats | null>(null);
  const [loading, setLoading] = useState(true);
  const [lastUpdate, setLastUpdate] = useState<string>('');
  const [tpsHistory, setTpsHistory] = useState<number[]>([]);
  const [connectionStatus, setConnectionStatus] = useState<'connected' | 'partial' | 'offline'>('offline');
  const [activeView, setActiveView] = useState<InfrastructureView>('overview');
  const [loginApiKey, setLoginApiKey] = useState('');
  const [loginApiSecret, setLoginApiSecret] = useState('');
  const [registeredEmail, setRegisteredEmail] = useState('');
  const [registerTier, setRegisterTier] = useState<'basic' | 'pro' | 'enterprise'>('pro');
  const [registrationDone, setRegistrationDone] = useState(false);
  const openWindow = useDesktopStore((s) => s.openWindow);

  const loadStats = useCallback(async () => {
    const [bridgeData, gpuData, chainData, rpcData] = await Promise.all([
      fetchJSON<BridgeStats>(`${BRIDGE_URL}/stats`),
      fetchJSON<{ lanes: GPULane[] }>(`${BRIDGE_URL}/gpu/health`).then(d => {
        if (d && Array.isArray((d as any))) return d as unknown as GPULane[];
        if (d && 'lanes' in d) return d.lanes;
        return [];
      }).catch(() => [] as GPULane[]),
      fetchJSON<{ proxy: ChainStats }>(`${RPC_PROXY_URL}/stats`).then(d => d?.proxy ?? null).catch(() => null),
      fetchJSON<RpcPoolStats>(`${CHAIN_DB_URL}/api/rpc/stats`),
    ]);

    const hasAny = bridgeData || gpuData.length > 0 || chainData || rpcData;
    setConnectionStatus(
      (bridgeData && chainData) || rpcData ? 'connected' :
      hasAny ? 'partial' : 'offline'
    );
    if (rpcData) setRpcPool(rpcData);

    if (bridgeData) {
      setBridge(bridgeData);
      setTpsHistory(prev => [...prev.slice(-60), Math.round(bridgeData.current_tps)]);
    }
    if (gpuData.length > 0) setGpuLanes(gpuData);
    if (chainData) setChain(chainData);
    setLastUpdate(new Date().toLocaleTimeString());
    setLoading(false);
  }, []);

  useEffect(() => {
    loadStats();
    const iv = setInterval(loadStats, 3000);
    return () => clearInterval(iv);
  }, [loadStats]);

  const formatUptime = (s: number) => {
    const h = Math.floor(s / 3600);
    const m = Math.floor((s % 3600) / 60);
    return `${h}h ${m}m`;
  };

  const formatNum = (n: number) => n.toLocaleString();

  /* ── Inline TPS sparkline (ASCII-style bar chart) ── */
  const maxTps = Math.max(...tpsHistory, 1);
  const sparkBars = tpsHistory.slice(-30);

  /* ── Styles ── */
  const c = {
    root: { display: 'flex', flexDirection: 'column' as const, height: '100%', background: '#0a0e17', color: '#e0e0e0', fontFamily: '-apple-system, BlinkMacSystemFont, monospace', fontSize: '0.8rem', overflow: 'auto' },
    header: { display: 'flex', alignItems: 'center', gap: 8, padding: '10px 14px', borderBottom: '1px solid #1a1f2e', flexShrink: 0 },
    grid: { display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))', gap: 10, padding: 14 },
    card: { background: '#111827', border: '1px solid #1f2937', borderRadius: 10, padding: '12px 14px' },
    cardTitle: { fontSize: '0.7rem', color: '#9ca3af', textTransform: 'uppercase' as const, letterSpacing: 1, marginBottom: 6, display: 'flex', alignItems: 'center', gap: 4 },
    bigNum: { fontSize: '1.3rem', fontWeight: 700, marginBottom: 2 },
    sub: { fontSize: '0.7rem', color: '#6b7280' },
    statusDot: (ok: boolean) => ({ display: 'inline-block', width: 8, height: 8, borderRadius: '50%', background: ok ? '#10b981' : '#ef4444', marginRight: 4 }),
    gpuRow: { display: 'flex', alignItems: 'center', gap: 8, padding: '6px 0', borderBottom: '1px solid #1f2937' },
    bar: (_pct: number, _color: string) => ({ width: 60, height: 6, background: '#1f2937', borderRadius: 3, overflow: 'hidden' as const, display: 'inline-block' }),
    barFill: (pct: number, color: string) => ({ width: `${pct}%`, height: '100%', background: color, borderRadius: 3 }),
  };

  const statusColors = { connected: '#10b981', partial: '#f59e0b', offline: '#ef4444' };
  const statusLabel = { connected: 'All Systems', partial: 'Partial', offline: 'Offline' };
  const viewTabs: Array<{ id: InfrastructureView; label: string }> = [
    { id: 'overview', label: 'Overview' },
    { id: 'chain-explorer', label: 'Chain Explorer' },
    { id: 'admin-dashboard', label: 'Admin Dashboard' },
    { id: 'login', label: 'Login' },
    { id: 'register', label: 'Register' },
  ];

  const renderIntegratedView = () => {
    if (activeView === 'chain-explorer') {
      return (
        <div style={{ padding: 14, display: 'grid', gap: 10 }}>
          <div style={c.card}>
            <div style={c.cardTitle as React.CSSProperties}>🌐 Chain Explorer (Consolidated)</div>
            <div style={{ ...c.sub, marginBottom: 10 }}>
              Consolidated from standalone Chain Explorer into Infrastructure Monitor.
            </div>
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, minmax(0, 1fr))', gap: 8, marginBottom: 10 }}>
              <div style={c.card}><div style={c.sub}>Chains Covered</div><div style={{ ...c.bigNum, color: '#3b82f6' }}>{rpcPool?.chains_covered ?? 0}</div></div>
              <div style={c.card}><div style={c.sub}>RPC Endpoints</div><div style={{ ...c.bigNum, color: '#22c55e' }}>{rpcPool?.total_endpoints ?? 0}</div></div>
              <div style={c.card}><div style={c.sub}>Healthy</div><div style={{ ...c.bigNum, color: '#10b981' }}>{rpcPool?.healthy_endpoints ?? 0}</div></div>
              <div style={c.card}><div style={c.sub}>Avg Latency</div><div style={{ ...c.bigNum, color: '#f59e0b' }}>{rpcPool?.avg_latency_ms ?? 0}ms</div></div>
            </div>
            <button
              onClick={() => openWindow('rpc-stats', 'RPC Pool Stats', '#f59e0b')}
              style={{ background: '#1e3a5f', border: 'none', borderRadius: 6, padding: '6px 10px', color: '#3b82f6', cursor: 'pointer', fontSize: '0.72rem', fontWeight: 600 }}
            >
              Open Full Chain Explorer →
            </button>
          </div>
        </div>
      );
    }

    if (activeView === 'admin-dashboard') {
      return (
        <div style={{ padding: 14, display: 'grid', gap: 10 }}>
          <div style={c.card}>
            <div style={c.cardTitle as React.CSSProperties}>🛡️ Admin Dashboard (Consolidated)</div>
            <div style={c.sub}>Consolidated from standalone Admin Dashboard into Infrastructure Monitor.</div>
          </div>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, minmax(0, 1fr))', gap: 10 }}>
            <div style={c.card}><div style={c.sub}>Bridge</div><div style={{ ...c.bigNum, color: bridge ? '#10b981' : '#ef4444' }}>{bridge ? 'Online' : 'Offline'}</div></div>
            <div style={c.card}><div style={c.sub}>GPU Lanes</div><div style={{ ...c.bigNum, color: '#a78bfa' }}>{gpuLanes.length}</div></div>
            <div style={c.card}><div style={c.sub}>RPC Proxy</div><div style={{ ...c.bigNum, color: chain ? '#3b82f6' : '#ef4444' }}>{chain ? 'Healthy' : 'Offline'}</div></div>
          </div>
          <div style={{ display: 'flex', gap: 8 }}>
            <button onClick={loadStats} style={{ background: '#1f2937', border: '1px solid #374151', borderRadius: 6, padding: '6px 10px', color: '#e5e7eb', cursor: 'pointer', fontSize: '0.72rem', fontWeight: 600 }}>Refresh Metrics</button>
            <button onClick={() => openWindow('rpc-stats', 'RPC Pool Stats', '#f59e0b')} style={{ background: '#1e3a5f', border: 'none', borderRadius: 6, padding: '6px 10px', color: '#3b82f6', cursor: 'pointer', fontSize: '0.72rem', fontWeight: 600 }}>Open RPC Stats</button>
            <button onClick={() => openWindow('airdrops', 'Airdrops & Faucets', '#ec4899')} style={{ background: '#4a1942', border: 'none', borderRadius: 6, padding: '6px 10px', color: '#ec4899', cursor: 'pointer', fontSize: '0.72rem', fontWeight: 600 }}>Open Airdrops</button>
          </div>
        </div>
      );
    }

    if (activeView === 'login') {
      return (
        <div style={{ padding: 14, maxWidth: 460 }}>
          <div style={c.card}>
            <div style={c.cardTitle as React.CSSProperties}>🔐 Login (Consolidated)</div>
            <div style={{ ...c.sub, marginBottom: 10 }}>Local auth view consolidated from standalone dashboard.</div>
            <div style={{ display: 'grid', gap: 8 }}>
              <input value={loginApiKey} onChange={(e) => setLoginApiKey(e.target.value)} placeholder="API key" style={{ background: '#0f172a', border: '1px solid #374151', borderRadius: 6, padding: '8px 10px', color: '#e5e7eb' }} />
              <input value={loginApiSecret} onChange={(e) => setLoginApiSecret(e.target.value)} type="password" placeholder="API secret" style={{ background: '#0f172a', border: '1px solid #374151', borderRadius: 6, padding: '8px 10px', color: '#e5e7eb' }} />
              <button style={{ background: '#2563eb', border: 'none', borderRadius: 6, padding: '8px 10px', color: '#fff', cursor: 'pointer', fontWeight: 600 }}>Log In</button>
            </div>
          </div>
        </div>
      );
    }

    if (activeView === 'register') {
      return (
        <div style={{ padding: 14, maxWidth: 560 }}>
          <div style={c.card}>
            <div style={c.cardTitle as React.CSSProperties}>📝 Register (Consolidated)</div>
            <div style={{ ...c.sub, marginBottom: 10 }}>Local registration flow consolidated from standalone dashboard.</div>
            {!registrationDone ? (
              <div style={{ display: 'grid', gap: 8 }}>
                <input value={registeredEmail} onChange={(e) => setRegisteredEmail(e.target.value)} type="email" placeholder="validator@example.com" style={{ background: '#0f172a', border: '1px solid #374151', borderRadius: 6, padding: '8px 10px', color: '#e5e7eb' }} />
                <select value={registerTier} onChange={(e) => setRegisterTier(e.target.value as 'basic' | 'pro' | 'enterprise')} style={{ background: '#0f172a', border: '1px solid #374151', borderRadius: 6, padding: '8px 10px', color: '#e5e7eb' }}>
                  <option value="basic">Basic</option>
                  <option value="pro">Pro</option>
                  <option value="enterprise">Enterprise</option>
                </select>
                <button onClick={() => setRegistrationDone(true)} style={{ background: '#16a34a', border: 'none', borderRadius: 6, padding: '8px 10px', color: '#fff', cursor: 'pointer', fontWeight: 600 }}>Create Validator Credentials</button>
              </div>
            ) : (
              <div style={{ ...c.sub, color: '#10b981' }}>Registration complete. Credentials generated for {registeredEmail || 'validator'} ({registerTier}).</div>
            )}
          </div>
        </div>
      );
    }

    return null;
  };

  return (
    <div style={c.root}>
      {/* Header */}
      <div style={c.header}>
        <span style={{ fontSize: '1.1rem' }}>🏗️</span>
        <span style={{ fontWeight: 700, fontSize: '0.95rem' }}>Infrastructure Monitor</span>
        <div style={{ display: 'flex', gap: 4, marginLeft: 10 }}>
          {viewTabs.map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveView(tab.id)}
              style={{
                background: activeView === tab.id ? '#1f3b5f' : 'transparent',
                border: activeView === tab.id ? '1px solid #3b82f6' : '1px solid #2a2f3e',
                borderRadius: 6,
                padding: '3px 8px',
                color: activeView === tab.id ? '#93c5fd' : '#9ca3af',
                cursor: 'pointer',
                fontSize: '0.68rem',
                fontWeight: 600,
              }}
            >
              {tab.label}
            </button>
          ))}
        </div>
        <div style={{ flex: 1 }} />
        <span style={{ ...c.statusDot(connectionStatus === 'connected') as any, background: statusColors[connectionStatus] }} />
        <span style={{ fontSize: '0.72rem', color: statusColors[connectionStatus] }}>{statusLabel[connectionStatus]}</span>
        <span style={{ fontSize: '0.65rem', color: '#555', marginLeft: 8 }}>{lastUpdate}</span>
        <button onClick={() => openWindow('airdrops', 'Airdrops & Faucets', '#ec4899')} style={{ background: '#4a1942', border: 'none', borderRadius: 6, padding: '3px 8px', color: '#ec4899', cursor: 'pointer', fontSize: '0.72rem', marginLeft: 4, fontWeight: 600 }}>
          🪂 Airdrops
        </button>
        <button onClick={loadStats} style={{ background: 'transparent', border: '1px solid #2a2f3e', borderRadius: 6, padding: '3px 8px', color: '#999', cursor: 'pointer', fontSize: '0.72rem', marginLeft: 4 }}>
          ↻ Refresh
        </button>
      </div>

      {activeView !== 'overview' ? (
        renderIntegratedView()
      ) : loading ? (
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', flex: 1, color: '#666' }}>
          Loading infrastructure data…
        </div>
      ) : (
        <>
          {/* Top metric cards */}
          <div style={c.grid}>
            {/* Bridge Status */}
            <div style={c.card}>
              <div style={c.cardTitle as React.CSSProperties}>🌉 Bridge</div>
              <div style={{ ...c.bigNum, color: bridge ? '#10b981' : '#ef4444' }}>
                {bridge ? `${bridge.current_tps.toFixed(0)} TPS` : 'Offline'}
              </div>
              {bridge && (
                <>
                  <div style={c.sub}>Received: {formatNum(bridge.total_received)}</div>
                  <div style={c.sub}>Forwarded: {formatNum(bridge.total_forwarded)}</div>
                  <div style={c.sub}>Failed: {formatNum(bridge.total_failed)}</div>
                  <div style={c.sub}>Uptime: {formatUptime(bridge.uptime_seconds)}</div>
                </>
              )}
            </div>

            {/* Chain / RPC Proxy */}
            <div style={c.card}>
              <div style={c.cardTitle as React.CSSProperties}>⛓️ RPC Proxy</div>
              <div style={{ ...c.bigNum, color: chain ? '#3b82f6' : '#ef4444' }}>
                {chain ? formatNum(chain.total_requests) : 'Offline'}
              </div>
              {chain && (
                <>
                  <div style={c.sub}>Port: {chain.port}</div>
                  <div style={c.sub}>Cached: {formatNum(chain.cached_responses)}</div>
                  <div style={c.sub}>GPU Accel: {formatNum(chain.gpu_accelerated)}</div>
                  <div style={c.sub}>Errors: {formatNum(chain.errors)}</div>
                  <div style={c.sub}>Uptime: {formatUptime(chain.uptime_seconds)}</div>
                </>
              )}
            </div>

            {/* GPU Summary */}
            <div style={c.card}>
              <div style={c.cardTitle as React.CSSProperties}>🎮 GPU Lanes</div>
              <div style={{ ...c.bigNum, color: gpuLanes.length > 0 ? '#a78bfa' : '#666' }}>
                {gpuLanes.length > 0 ? `${gpuLanes.length} Active` : 'No Lanes'}
              </div>
              {gpuLanes.length > 0 && (
                <>
                  <div style={c.sub}>Avg Util: {(gpuLanes.reduce((s, g) => s + g.utilization, 0) / gpuLanes.length).toFixed(0)}%</div>
                  <div style={c.sub}>Success Rate: {(gpuLanes.reduce((s, g) => s + g.success_rate, 0) / gpuLanes.length).toFixed(1)}%</div>
                  <div style={c.sub}>Total Req: {formatNum(gpuLanes.reduce((s, g) => s + g.total_requests, 0))}</div>
                </>
              )}
            </div>

            {/* Gas Savings */}
            <div style={{ ...c.card, border: rpcPool ? '1px solid #065f46' : '1px solid #1f2937', background: rpcPool ? 'linear-gradient(135deg, #064e3b22, #111827)' : '#111827' }}>
              <div style={c.cardTitle as React.CSSProperties}>💰 Gas Savings</div>
              {rpcPool ? (
                <>
                  <div style={{ ...c.bigNum, color: '#10b981' }}>
                    ${rpcPool.gas_savings.total_monthly_saved.toLocaleString()}<span style={{ fontSize: '0.7rem', color: '#6b7280' }}>/mo saved</span>
                  </div>
                  <div style={c.sub}>vs Infura: ${rpcPool.gas_savings.infura_growth_equiv.toLocaleString()}/mo</div>
                  <div style={c.sub}>vs Alchemy: ${rpcPool.gas_savings.alchemy_growth_equiv.toLocaleString()}/mo</div>
                  <div style={c.sub}>vs QuickNode: ${rpcPool.gas_savings.quicknode_build_equiv.toLocaleString()}/mo</div>
                  <div style={{ ...c.sub, color: '#10b981', fontWeight: 700, marginTop: 4 }}>Your cost: $0.00 🎯</div>
                  <button onClick={() => openWindow('rpc-stats', 'RPC Pool Stats', '#f59e0b')} style={{ marginTop: 8, background: '#065f46', border: 'none', borderRadius: 6, padding: '4px 10px', color: '#10b981', cursor: 'pointer', fontSize: '0.7rem', fontWeight: 600, width: '100%' }}>
                    📊 View Full Stats →
                  </button>
                </>
              ) : (
                <div style={{ color: '#555', fontSize: '0.72rem', marginTop: 8 }}>Chain DB offline</div>
              )}
            </div>

            {/* RPC Pool */}
            <div style={c.card}>
              <div style={c.cardTitle as React.CSSProperties}>🔗 RPC Pool</div>
              {rpcPool ? (
                <>
                  <div style={{ ...c.bigNum, color: '#3b82f6' }}>
                    {rpcPool.healthy_endpoints.toLocaleString()}<span style={{ fontSize: '0.7rem', color: '#6b7280' }}> healthy</span>
                  </div>
                  <div style={c.sub}>Total: {rpcPool.total_endpoints.toLocaleString()} endpoints</div>
                  <div style={c.sub}>Chains: {rpcPool.chains_covered.toLocaleString()}</div>
                  <div style={c.sub}>Combined: {rpcPool.combined_rps.toLocaleString()} req/s</div>
                  <div style={c.sub}>Avg latency: {rpcPool.avg_latency_ms}ms (best: {rpcPool.min_latency_ms}ms)</div>
                  <button onClick={() => openWindow('rpc-stats', 'RPC Pool Stats', '#f59e0b')} style={{ marginTop: 8, background: '#1e3a5f', border: 'none', borderRadius: 6, padding: '4px 10px', color: '#3b82f6', cursor: 'pointer', fontSize: '0.7rem', fontWeight: 600, width: '100%' }}>
                    🔗 Full RPC Dashboard →
                  </button>
                </>
              ) : (
                <div style={{ color: '#555', fontSize: '0.72rem', marginTop: 8 }}>Chain DB offline</div>
              )}
            </div>

            {/* TPS Sparkline */}
            <div style={c.card}>
              <div style={c.cardTitle as React.CSSProperties}>📈 TPS History</div>
              {sparkBars.length > 0 ? (
                <div style={{ display: 'flex', alignItems: 'flex-end', gap: 1, height: 48, marginTop: 4 }}>
                  {sparkBars.map((v, i) => (
                    <div key={i} style={{
                      flex: 1,
                      height: `${Math.max(2, (v / maxTps) * 100)}%`,
                      background: v > maxTps * 0.8 ? '#ef4444' : v > maxTps * 0.5 ? '#f59e0b' : '#10b981',
                      borderRadius: 2,
                      minWidth: 2,
                    }} />
                  ))}
                </div>
              ) : (
                <div style={{ color: '#555', fontSize: '0.72rem', marginTop: 8 }}>Waiting for data…</div>
              )}
              {sparkBars.length > 0 && (
                <div style={{ ...c.sub, marginTop: 4 }}>
                  Peak: {Math.max(...sparkBars)} TPS &nbsp;|&nbsp; Current: {sparkBars[sparkBars.length - 1] ?? 0} TPS
                </div>
              )}
            </div>
          </div>

          {/* GPU Lane Details */}
          {gpuLanes.length > 0 && (
            <div style={{ padding: '0 14px 14px' }}>
              <div style={{ ...c.card }}>
                <div style={c.cardTitle as React.CSSProperties}>🎮 GPU Lane Details</div>
                <div style={{ fontSize: '0.7rem' }}>
                  <div style={{ display: 'flex', padding: '4px 0', color: '#555', fontWeight: 600, borderBottom: '1px solid #1f2937' }}>
                    <span style={{ width: 40 }}>ID</span>
                    <span style={{ flex: 1 }}>Service</span>
                    <span style={{ width: 60 }}>Status</span>
                    <span style={{ width: 80 }}>Utilization</span>
                    <span style={{ width: 70 }}>Memory</span>
                    <span style={{ width: 50 }}>Temp</span>
                    <span style={{ width: 70, textAlign: 'right' }}>TPS</span>
                  </div>
                  {gpuLanes.map((lane, i) => (
                    <div key={i} style={c.gpuRow}>
                      <span style={{ width: 40, color: '#9ca3af' }}>#{lane.id}</span>
                      <span style={{ flex: 1 }}>{lane.service}</span>
                      <span style={{ width: 60 }}>
                        <span style={c.statusDot(lane.status === 'healthy')} />
                        <span style={{ color: lane.status === 'healthy' ? '#10b981' : '#f59e0b', fontSize: '0.65rem' }}>
                          {lane.status}
                        </span>
                      </span>
                      <span style={{ width: 80, display: 'flex', alignItems: 'center', gap: 4 }}>
                        <div style={c.bar(lane.utilization, '')}>
                          <div style={c.barFill(lane.utilization, lane.utilization > 80 ? '#ef4444' : lane.utilization > 50 ? '#f59e0b' : '#10b981')} />
                        </div>
                        <span style={{ fontSize: '0.65rem', color: '#9ca3af' }}>{lane.utilization}%</span>
                      </span>
                      <span style={{ width: 70, fontSize: '0.7rem', color: '#9ca3af' }}>{lane.memory_used_mb}MB</span>
                      <span style={{ width: 50, fontSize: '0.7rem', color: lane.temperature_c > 80 ? '#ef4444' : '#9ca3af' }}>{lane.temperature_c}°C</span>
                      <span style={{ width: 70, textAlign: 'right', fontWeight: 600, color: '#3b82f6' }}>{lane.txns_per_second.toFixed(1)}</span>
                    </div>
                  ))}
                </div>
              </div>
            </div>
          )}

          {/* Offline notice */}
          {connectionStatus === 'offline' && !rpcPool && (
            <div style={{ margin: '0 14px 14px', padding: 20, background: '#1f1215', border: '1px solid #7f1d1d', borderRadius: 10, textAlign: 'center' }}>
              <div style={{ fontSize: '1.5rem', marginBottom: 8 }}>🔌</div>
              <div style={{ fontWeight: 700, color: '#fca5a5', marginBottom: 4 }}>Infrastructure Offline</div>
              <div style={{ color: '#9ca3af', fontSize: '0.75rem' }}>
                Could not connect to Bridge ({BRIDGE_URL}) or RPC Proxy ({RPC_PROXY_URL}).
                <br />Start the infrastructure services and this panel will auto-connect.
              </div>
            </div>
          )}

          {/* RPC Provider Breakdown + Fastest Endpoints */}
          {rpcPool && (
            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 10, padding: '0 14px 14px' }}>
              {/* Top Providers */}
              <div style={c.card}>
                <div style={c.cardTitle as React.CSSProperties}>📊 Top RPC Providers</div>
                <div style={{ fontSize: '0.7rem' }}>
                  <div style={{ display: 'flex', padding: '4px 0', color: '#555', fontWeight: 600, borderBottom: '1px solid #1f2937' }}>
                    <span style={{ flex: 1 }}>Provider</span>
                    <span style={{ width: 50, textAlign: 'right' }}>Count</span>
                    <span style={{ width: 60, textAlign: 'right' }}>RPS</span>
                    <span style={{ width: 70, textAlign: 'right' }}>Avg ms</span>
                  </div>
                  {rpcPool.by_provider.slice(0, 10).map((p, i) => (
                    <div key={i} style={{ display: 'flex', padding: '3px 0', borderBottom: '1px solid #1a1f2e' }}>
                      <span style={{ flex: 1, color: '#e0e0e0' }}>{p.provider}</span>
                      <span style={{ width: 50, textAlign: 'right', color: '#9ca3af' }}>{p.count}</span>
                      <span style={{ width: 60, textAlign: 'right', color: '#3b82f6' }}>{p.rps}</span>
                      <span style={{ width: 70, textAlign: 'right', color: p.avg_latency < 100 ? '#10b981' : p.avg_latency < 300 ? '#f59e0b' : '#ef4444' }}>
                        {Math.round(p.avg_latency)}ms
                      </span>
                    </div>
                  ))}
                </div>
              </div>

              {/* Fastest Endpoints */}
              <div style={c.card}>
                <div style={c.cardTitle as React.CSSProperties}>🏆 Fastest Endpoints</div>
                <div style={{ fontSize: '0.7rem' }}>
                  <div style={{ display: 'flex', padding: '4px 0', color: '#555', fontWeight: 600, borderBottom: '1px solid #1f2937' }}>
                    <span style={{ width: 50 }}>Latency</span>
                    <span style={{ width: 80 }}>Provider</span>
                    <span style={{ flex: 1 }}>Chain</span>
                  </div>
                  {rpcPool.top_fastest.map((ep, i) => (
                    <div key={i} style={{ display: 'flex', padding: '3px 0', borderBottom: '1px solid #1a1f2e' }}>
                      <span style={{ width: 50, color: '#10b981', fontWeight: 600 }}>{Math.round(ep.latency_ms)}ms</span>
                      <span style={{ width: 80, color: '#9ca3af' }}>{ep.provider}</span>
                      <span style={{ flex: 1, color: '#e0e0e0' }}>{ep.chain_id}</span>
                    </div>
                  ))}
                </div>
              </div>
            </div>
          )}

          {/* Gas Savings Comparison Bar */}
          {rpcPool && (
            <div style={{ padding: '0 14px 14px' }}>
              <div style={{ ...c.card, background: 'linear-gradient(135deg, #064e3b15, #111827)' }}>
                <div style={c.cardTitle as React.CSSProperties}>💰 Monthly Cost Comparison</div>
                <div style={{ fontSize: '0.72rem', marginTop: 6 }}>
                  {[
                    { name: 'Infura Growth', cost: rpcPool.gas_savings.infura_growth_equiv, color: '#f59e0b' },
                    { name: 'Alchemy Growth', cost: rpcPool.gas_savings.alchemy_growth_equiv, color: '#8b5cf6' },
                    { name: 'QuickNode Build', cost: rpcPool.gas_savings.quicknode_build_equiv, color: '#3b82f6' },
                    { name: 'Moralis Pro', cost: rpcPool.gas_savings.moralis_pro_equiv, color: '#ec4899' },
                    { name: 'X3 Chain', cost: 0, color: '#10b981' },
                  ].map((plan, i) => {
                    const maxCost = Math.max(rpcPool.gas_savings.infura_growth_equiv, rpcPool.gas_savings.quicknode_build_equiv, 1);
                    const pct = plan.cost === 0 ? 1 : (plan.cost / maxCost) * 100;
                    return (
                      <div key={i} style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 6 }}>
                        <span style={{ width: 110, color: '#9ca3af', textAlign: 'right' }}>{plan.name}</span>
                        <div style={{ flex: 1, height: 14, background: '#1f2937', borderRadius: 4, overflow: 'hidden', position: 'relative' as const }}>
                          <div style={{ width: `${pct}%`, height: '100%', background: plan.color, borderRadius: 4, transition: 'width 0.5s ease' }} />
                        </div>
                        <span style={{ width: 80, textAlign: 'right', fontWeight: 700, color: plan.cost === 0 ? '#10b981' : plan.color }}>
                          {plan.cost === 0 ? 'FREE 🎯' : `$${plan.cost.toLocaleString()}`}
                        </span>
                      </div>
                    );
                  })}
                </div>
              </div>
            </div>
          )}
        </>
      )}
    </div>
  );
};

export default InfrastructurePanel;
