/**
 * AirdropsPanel — Airdrop & faucet discovery dashboard.
 *
 * Shows: Discovered airdrops with claim dates, testnet faucets with auto-claim status,
 * wallet balances across chains, chain discoveries log, claim history.
 */
import React, { useState, useEffect, useCallback } from 'react';

/* ── Types ─────────────────────────────────────────── */
interface Airdrop {
  id: number;
  chain_id: string;
  name: string;
  project: string | null;
  token_symbol: string | null;
  airdrop_type: string;
  status: string;
  source: string | null;
  source_url: string | null;
  claim_url: string | null;
  claim_start: string | null;
  claim_deadline: string | null;
  estimated_value: number;
  actual_value: number;
  discovered_at: string;
}

interface AirdropStats {
  total: number;
  discovered: number;
  eligible: number;
  claimed: number;
  expired: number;
  active_deadlines: number;
  total_estimated_value: number;
  total_actual_value: number;
}

interface Faucet {
  id: number;
  chain_id: string;
  name: string;
  provider: string | null;
  url: string;
  faucet_type: string;
  token_symbol: string | null;
  amount_per_claim: string | null;
  cooldown_hours: number;
  requires_auth: boolean;
  status: string;
  last_claimed: string | null;
  total_claims: number;
  total_received: string;
  discovered_at: string;
}

interface FaucetStats {
  total: number;
  active: number;
  dead: number;
  total_claims: number;
  chains_covered: number;
}

interface Wallet {
  id: number;
  chain_id: string;
  address: string;
  label: string;
  ecosystem: string;
  is_active: boolean;
  balance: string;
  balance_usd: number;
  last_balance_check: string | null;
}

interface WalletStats {
  total: number;
  chains: number;
  active: number;
  total_balance_usd: number;
}

interface ChainDiscovery {
  id: number;
  chain_id: string | null;
  chain_name: string;
  chain_numeric_id: number | null;
  ecosystem: string;
  chain_type: string;
  is_testnet: boolean;
  source: string;
  rpc_url: string | null;
  status: string;
  discovered_at: string;
}

interface DiscoveryStats {
  total: number;
  new_chains: number;
  added: number;
  testnets: number;
}

type Tab = 'airdrops' | 'faucets' | 'wallets' | 'discoveries';

/* ── API ───────────────────────────────────────────── */
const API = 'http://localhost:7070';

async function fetchJSON<T>(url: string): Promise<T | null> {
  try {
    const res = await fetch(url, { signal: AbortSignal.timeout(5000) });
    if (!res.ok) return null;
    return await res.json();
  } catch { return null; }
}

/* ── Component ─────────────────────────────────────── */
const AirdropsPanel: React.FC = () => {
  const [tab, setTab] = useState<Tab>('airdrops');
  const [airdrops, setAirdrops] = useState<Airdrop[]>([]);
  const [airdropStats, setAirdropStats] = useState<AirdropStats | null>(null);
  const [faucets, setFaucets] = useState<Faucet[]>([]);
  const [faucetStats, setFaucetStats] = useState<FaucetStats | null>(null);
  const [wallets, setWallets] = useState<Wallet[]>([]);
  const [walletStats, setWalletStats] = useState<WalletStats | null>(null);
  const [discoveries, setDiscoveries] = useState<ChainDiscovery[]>([]);
  const [discoveryStats, setDiscoveryStats] = useState<DiscoveryStats | null>(null);
  const [loading, setLoading] = useState(true);
  const [lastUpdate, setLastUpdate] = useState('');
  const [online, setOnline] = useState(false);

  const load = useCallback(async () => {
    const [adResp, fcResp, wResp, dResp] = await Promise.all([
      fetchJSON<{ airdrops: Airdrop[]; stats: AirdropStats }>(`${API}/api/airdrops?limit=100`),
      fetchJSON<{ faucets: Faucet[]; stats: FaucetStats }>(`${API}/api/faucets?limit=100`),
      fetchJSON<{ wallets: Wallet[]; stats: WalletStats }>(`${API}/api/wallets`),
      fetchJSON<{ discoveries: ChainDiscovery[]; stats: DiscoveryStats }>(`${API}/api/discoveries?limit=100`),
    ]);

    const hasAny = adResp || fcResp || wResp || dResp;
    setOnline(!!hasAny);

    if (adResp) { setAirdrops(adResp.airdrops); setAirdropStats(adResp.stats); }
    if (fcResp) { setFaucets(fcResp.faucets); setFaucetStats(fcResp.stats); }
    if (wResp) { setWallets(wResp.wallets); setWalletStats(wResp.stats); }
    if (dResp) { setDiscoveries(dResp.discoveries); setDiscoveryStats(dResp.stats); }

    setLastUpdate(new Date().toLocaleTimeString());
    setLoading(false);
  }, []);

  useEffect(() => {
    load();
    const iv = setInterval(load, 15000);
    return () => clearInterval(iv);
  }, [load]);

  /* ── Styles ── */
  const c = {
    root: { display: 'flex', flexDirection: 'column' as const, height: '100%', background: '#0a0e17', color: '#e0e0e0', fontFamily: '-apple-system, BlinkMacSystemFont, monospace', fontSize: '0.8rem', overflow: 'auto' },
    header: { display: 'flex', alignItems: 'center', gap: 8, padding: '10px 14px', borderBottom: '1px solid #1a1f2e', flexShrink: 0 },
    tabs: { display: 'flex', gap: 2, padding: '8px 14px', borderBottom: '1px solid #1a1f2e', flexShrink: 0 },
    tab: (active: boolean) => ({ padding: '6px 14px', borderRadius: 6, background: active ? '#1f2937' : 'transparent', color: active ? '#e0e0e0' : '#6b7280', cursor: 'pointer' as const, fontSize: '0.72rem', fontWeight: active ? 700 : 400, border: active ? '1px solid #374151' : '1px solid transparent', transition: 'all 0.15s ease' }),
    grid: { display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(160px, 1fr))', gap: 10, padding: 14 },
    card: { background: '#111827', border: '1px solid #1f2937', borderRadius: 10, padding: '12px 14px' },
    cardT: { fontSize: '0.68rem', color: '#9ca3af', textTransform: 'uppercase' as const, letterSpacing: 1, marginBottom: 6 },
    big: { fontSize: '1.3rem', fontWeight: 700 },
    sub: { fontSize: '0.7rem', color: '#6b7280', marginTop: 2 },
    table: { width: '100%', borderCollapse: 'collapse' as const, fontSize: '0.72rem' },
    th: { textAlign: 'left' as const, padding: '6px 8px', color: '#555', fontWeight: 600, borderBottom: '1px solid #1f2937', fontSize: '0.68rem' },
    td: { padding: '5px 8px', borderBottom: '1px solid #1a1f2e' },
    badge: (color: string) => ({ display: 'inline-block', padding: '1px 6px', borderRadius: 4, fontSize: '0.6rem', fontWeight: 600, background: `${color}22`, color, border: `1px solid ${color}44` }),
  };

  const statusColors: Record<string, string> = {
    discovered: '#3b82f6', eligible: '#f59e0b', claimed: '#10b981', expired: '#ef4444', ineligible: '#6b7280',
    active: '#10b981', dead: '#ef4444', rate_limited: '#f59e0b', maintenance: '#6b7280',
    new: '#3b82f6', added: '#10b981', ignored: '#6b7280', invalid: '#ef4444',
    pending: '#f59e0b', submitted: '#3b82f6', confirmed: '#10b981', failed: '#ef4444',
  };

  const fmtDate = (d: string | null) => {
    if (!d) return '—';
    try { return new Date(d).toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: '2-digit' }); }
    catch { return d; }
  };

  const daysUntil = (d: string | null) => {
    if (!d) return null;
    try {
      const diff = Math.ceil((new Date(d).getTime() - Date.now()) / 86400000);
      return diff;
    } catch { return null; }
  };

  if (loading) {
    return (
      <div style={c.root}>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', flex: 1, color: '#666' }}>Loading airdrop data…</div>
      </div>
    );
  }

  return (
    <div style={c.root}>
      {/* Header */}
      <div style={c.header}>
        <span style={{ fontSize: '1.1rem' }}>🪂</span>
        <span style={{ fontWeight: 700, fontSize: '0.95rem' }}>Airdrops & Faucets</span>
        <div style={{ flex: 1 }} />
        <span style={{ width: 8, height: 8, borderRadius: '50%', background: online ? '#10b981' : '#ef4444', display: 'inline-block' }} />
        <span style={{ fontSize: '0.72rem', color: online ? '#10b981' : '#ef4444', marginLeft: 4 }}>{online ? 'Online' : 'Offline'}</span>
        <span style={{ fontSize: '0.65rem', color: '#555', marginLeft: 8 }}>{lastUpdate}</span>
        <button onClick={load} style={{ background: 'transparent', border: '1px solid #2a2f3e', borderRadius: 6, padding: '3px 8px', color: '#999', cursor: 'pointer', fontSize: '0.72rem', marginLeft: 4 }}>↻</button>
      </div>

      {/* Tab bar */}
      <div style={c.tabs}>
        {(['airdrops', 'faucets', 'wallets', 'discoveries'] as Tab[]).map(t => (
          <div key={t} style={c.tab(tab === t)} onClick={() => setTab(t)}>
            {{ airdrops: '🪂 Airdrops', faucets: '🚰 Faucets', wallets: '👛 Wallets', discoveries: '🔍 Discoveries' }[t]}
            {t === 'airdrops' && airdropStats ? ` (${airdropStats.total})` : ''}
            {t === 'faucets' && faucetStats ? ` (${faucetStats.total})` : ''}
            {t === 'wallets' && walletStats ? ` (${walletStats.total})` : ''}
            {t === 'discoveries' && discoveryStats ? ` (${discoveryStats.total})` : ''}
          </div>
        ))}
      </div>

      {!online ? (
        <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', flex: 1, gap: 8 }}>
          <span style={{ fontSize: '2rem' }}>🔌</span>
          <span style={{ color: '#ef4444', fontWeight: 700 }}>Chain DB Offline</span>
          <span style={{ color: '#6b7280', fontSize: '0.72rem' }}>Start the chain-db server on port 7070</span>
        </div>
      ) : (
        <>
          {/* ── Airdrops Tab ── */}
          {tab === 'airdrops' && (
            <>
              <div style={c.grid}>
                <div style={c.card}>
                  <div style={c.cardT}>Discovered</div>
                  <div style={{ ...c.big, color: '#3b82f6' }}>{airdropStats?.discovered ?? 0}</div>
                </div>
                <div style={c.card}>
                  <div style={c.cardT}>Eligible</div>
                  <div style={{ ...c.big, color: '#f59e0b' }}>{airdropStats?.eligible ?? 0}</div>
                </div>
                <div style={c.card}>
                  <div style={c.cardT}>Claimed</div>
                  <div style={{ ...c.big, color: '#10b981' }}>{airdropStats?.claimed ?? 0}</div>
                </div>
                <div style={c.card}>
                  <div style={c.cardT}>Active Deadlines</div>
                  <div style={{ ...c.big, color: '#ec4899' }}>{airdropStats?.active_deadlines ?? 0}</div>
                </div>
                <div style={{ ...c.card, border: '1px solid #065f46', background: 'linear-gradient(135deg, #064e3b22, #111827)' }}>
                  <div style={c.cardT}>Est. Value</div>
                  <div style={{ ...c.big, color: '#10b981' }}>${(airdropStats?.total_estimated_value ?? 0).toLocaleString()}</div>
                  <div style={c.sub}>Claimed: ${(airdropStats?.total_actual_value ?? 0).toLocaleString()}</div>
                </div>
                <div style={c.card}>
                  <div style={c.cardT}>Expired</div>
                  <div style={{ ...c.big, color: '#ef4444' }}>{airdropStats?.expired ?? 0}</div>
                </div>
              </div>

              {airdrops.length === 0 ? (
                <div style={{ padding: 40, textAlign: 'center', color: '#555' }}>
                  <div style={{ fontSize: '2rem', marginBottom: 8 }}>🪂</div>
                  <div style={{ fontWeight: 700, marginBottom: 4 }}>No airdrops discovered yet</div>
                  <div style={{ fontSize: '0.72rem' }}>The crawler daemon will auto-detect airdrops from aggregator sites and Google dorks.</div>
                </div>
              ) : (
                <div style={{ padding: '0 14px 14px' }}>
                  <div style={c.card}>
                    <table style={c.table}>
                      <thead>
                        <tr>
                          <th style={c.th}>Airdrop</th>
                          <th style={c.th}>Chain</th>
                          <th style={c.th}>Type</th>
                          <th style={c.th}>Status</th>
                          <th style={c.th}>Token</th>
                          <th style={{ ...c.th, textAlign: 'right' as const }}>Est. Value</th>
                          <th style={c.th}>Deadline</th>
                          <th style={c.th}>Claim</th>
                        </tr>
                      </thead>
                      <tbody>
                        {airdrops.map(a => {
                          const days = daysUntil(a.claim_deadline);
                          const urgent = days !== null && days >= 0 && days <= 7;
                          return (
                            <tr key={a.id}>
                              <td style={{ ...c.td, fontWeight: 500 }}>
                                {a.name}
                                {a.project && <span style={{ color: '#555', fontSize: '0.6rem', marginLeft: 4 }}>({a.project})</span>}
                              </td>
                              <td style={{ ...c.td, color: '#a78bfa' }}>{a.chain_id}</td>
                              <td style={c.td}><span style={c.badge('#6b7280')}>{a.airdrop_type}</span></td>
                              <td style={c.td}><span style={c.badge(statusColors[a.status] || '#6b7280')}>{a.status}</span></td>
                              <td style={{ ...c.td, color: '#e0e0e0' }}>{a.token_symbol || '—'}</td>
                              <td style={{ ...c.td, textAlign: 'right', color: '#10b981' }}>{a.estimated_value > 0 ? `$${a.estimated_value.toLocaleString()}` : '—'}</td>
                              <td style={{ ...c.td, color: urgent ? '#ef4444' : '#9ca3af', fontWeight: urgent ? 700 : 400 }}>
                                {fmtDate(a.claim_deadline)}
                                {days !== null && days >= 0 && <span style={{ fontSize: '0.6rem', marginLeft: 3 }}>({days}d)</span>}
                                {days !== null && days < 0 && <span style={{ fontSize: '0.6rem', color: '#ef4444', marginLeft: 3 }}>expired</span>}
                              </td>
                              <td style={c.td}>
                                {a.claim_url ? (
                                  <a href={a.claim_url} target="_blank" rel="noreferrer" style={{ color: '#3b82f6', textDecoration: 'none', fontSize: '0.68rem' }}>Claim →</a>
                                ) : '—'}
                              </td>
                            </tr>
                          );
                        })}
                      </tbody>
                    </table>
                  </div>
                </div>
              )}
            </>
          )}

          {/* ── Faucets Tab ── */}
          {tab === 'faucets' && (
            <>
              <div style={c.grid}>
                <div style={c.card}>
                  <div style={c.cardT}>Active Faucets</div>
                  <div style={{ ...c.big, color: '#10b981' }}>{faucetStats?.active ?? 0}</div>
                </div>
                <div style={c.card}>
                  <div style={c.cardT}>Dead</div>
                  <div style={{ ...c.big, color: '#ef4444' }}>{faucetStats?.dead ?? 0}</div>
                </div>
                <div style={c.card}>
                  <div style={c.cardT}>Total Claims</div>
                  <div style={{ ...c.big, color: '#3b82f6' }}>{(faucetStats?.total_claims ?? 0).toLocaleString()}</div>
                </div>
                <div style={c.card}>
                  <div style={c.cardT}>Chains Covered</div>
                  <div style={{ ...c.big, color: '#a78bfa' }}>{faucetStats?.chains_covered ?? 0}</div>
                </div>
              </div>

              {faucets.length === 0 ? (
                <div style={{ padding: 40, textAlign: 'center', color: '#555' }}>
                  <div style={{ fontSize: '2rem', marginBottom: 8 }}>🚰</div>
                  <div style={{ fontWeight: 700, marginBottom: 4 }}>No faucets discovered yet</div>
                  <div style={{ fontSize: '0.72rem' }}>The crawler will auto-detect testnet faucets and attempt auto-claiming.</div>
                </div>
              ) : (
                <div style={{ padding: '0 14px 14px' }}>
                  <div style={c.card}>
                    <table style={c.table}>
                      <thead>
                        <tr>
                          <th style={c.th}>Faucet</th>
                          <th style={c.th}>Chain</th>
                          <th style={c.th}>Provider</th>
                          <th style={c.th}>Status</th>
                          <th style={c.th}>Token</th>
                          <th style={c.th}>Per Claim</th>
                          <th style={c.th}>Cooldown</th>
                          <th style={{ ...c.th, textAlign: 'right' as const }}>Claims</th>
                          <th style={c.th}>Last Claimed</th>
                          <th style={c.th}>Link</th>
                        </tr>
                      </thead>
                      <tbody>
                        {faucets.map(f => (
                          <tr key={f.id}>
                            <td style={{ ...c.td, fontWeight: 500 }}>{f.name}</td>
                            <td style={{ ...c.td, color: '#a78bfa' }}>{f.chain_id}</td>
                            <td style={{ ...c.td, color: '#9ca3af' }}>{f.provider || '—'}</td>
                            <td style={c.td}><span style={c.badge(statusColors[f.status] || '#6b7280')}>{f.status}</span></td>
                            <td style={{ ...c.td, color: '#e0e0e0' }}>{f.token_symbol || '—'}</td>
                            <td style={{ ...c.td, color: '#f59e0b' }}>{f.amount_per_claim || '—'}</td>
                            <td style={{ ...c.td, color: '#9ca3af' }}>{f.cooldown_hours}h</td>
                            <td style={{ ...c.td, textAlign: 'right', color: '#3b82f6' }}>{f.total_claims}</td>
                            <td style={{ ...c.td, color: '#6b7280', fontSize: '0.68rem' }}>{fmtDate(f.last_claimed)}</td>
                            <td style={c.td}>
                              <a href={f.url} target="_blank" rel="noreferrer" style={{ color: '#3b82f6', textDecoration: 'none', fontSize: '0.68rem' }}>Open →</a>
                            </td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                </div>
              )}
            </>
          )}

          {/* ── Wallets Tab ── */}
          {tab === 'wallets' && (
            <>
              <div style={c.grid}>
                <div style={c.card}>
                  <div style={c.cardT}>Total Wallets</div>
                  <div style={{ ...c.big, color: '#3b82f6' }}>{walletStats?.total ?? 0}</div>
                </div>
                <div style={c.card}>
                  <div style={c.cardT}>Chains</div>
                  <div style={{ ...c.big, color: '#a78bfa' }}>{walletStats?.chains ?? 0}</div>
                </div>
                <div style={c.card}>
                  <div style={c.cardT}>Active</div>
                  <div style={{ ...c.big, color: '#10b981' }}>{walletStats?.active ?? 0}</div>
                </div>
                <div style={{ ...c.card, border: '1px solid #065f46', background: 'linear-gradient(135deg, #064e3b22, #111827)' }}>
                  <div style={c.cardT}>Total Balance</div>
                  <div style={{ ...c.big, color: '#10b981' }}>${(walletStats?.total_balance_usd ?? 0).toLocaleString()}</div>
                </div>
              </div>

              {wallets.length === 0 ? (
                <div style={{ padding: 40, textAlign: 'center', color: '#555' }}>
                  <div style={{ fontSize: '2rem', marginBottom: 8 }}>👛</div>
                  <div style={{ fontWeight: 700, marginBottom: 4 }}>No wallets yet</div>
                  <div style={{ fontSize: '0.72rem' }}>Wallets are auto-created for each chain when faucets or airdrops are claimed.</div>
                </div>
              ) : (
                <div style={{ padding: '0 14px 14px' }}>
                  <div style={c.card}>
                    <table style={c.table}>
                      <thead>
                        <tr>
                          <th style={c.th}>Chain</th>
                          <th style={c.th}>Address</th>
                          <th style={c.th}>Label</th>
                          <th style={c.th}>Ecosystem</th>
                          <th style={{ ...c.th, textAlign: 'right' as const }}>Balance</th>
                          <th style={{ ...c.th, textAlign: 'right' as const }}>USD</th>
                          <th style={c.th}>Last Check</th>
                        </tr>
                      </thead>
                      <tbody>
                        {wallets.map(w => (
                          <tr key={w.id}>
                            <td style={{ ...c.td, color: '#a78bfa', fontWeight: 500 }}>{w.chain_id}</td>
                            <td style={{ ...c.td, color: '#e0e0e0', fontSize: '0.65rem', fontFamily: 'monospace' }}>
                              {w.address.length > 16 ? `${w.address.slice(0, 8)}…${w.address.slice(-6)}` : w.address}
                            </td>
                            <td style={c.td}><span style={c.badge(w.label === 'auto' ? '#3b82f6' : '#f59e0b')}>{w.label}</span></td>
                            <td style={{ ...c.td, color: '#9ca3af' }}>{w.ecosystem}</td>
                            <td style={{ ...c.td, textAlign: 'right', color: '#e0e0e0' }}>{w.balance}</td>
                            <td style={{ ...c.td, textAlign: 'right', color: '#10b981' }}>${w.balance_usd.toFixed(2)}</td>
                            <td style={{ ...c.td, color: '#6b7280', fontSize: '0.68rem' }}>{fmtDate(w.last_balance_check)}</td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                </div>
              )}
            </>
          )}

          {/* ── Discoveries Tab ── */}
          {tab === 'discoveries' && (
            <>
              <div style={c.grid}>
                <div style={c.card}>
                  <div style={c.cardT}>New Chains</div>
                  <div style={{ ...c.big, color: '#3b82f6' }}>{discoveryStats?.new_chains ?? 0}</div>
                </div>
                <div style={c.card}>
                  <div style={c.cardT}>Added to DB</div>
                  <div style={{ ...c.big, color: '#10b981' }}>{discoveryStats?.added ?? 0}</div>
                </div>
                <div style={c.card}>
                  <div style={c.cardT}>Testnets Found</div>
                  <div style={{ ...c.big, color: '#f59e0b' }}>{discoveryStats?.testnets ?? 0}</div>
                </div>
                <div style={c.card}>
                  <div style={c.cardT}>Total Discovered</div>
                  <div style={{ ...c.big, color: '#a78bfa' }}>{discoveryStats?.total ?? 0}</div>
                </div>
              </div>

              {discoveries.length === 0 ? (
                <div style={{ padding: 40, textAlign: 'center', color: '#555' }}>
                  <div style={{ fontSize: '2rem', marginBottom: 8 }}>🔍</div>
                  <div style={{ fontWeight: 700, marginBottom: 4 }}>No chain discoveries yet</div>
                  <div style={{ fontSize: '0.72rem' }}>The crawler daemon will log new chains, testnets, and devnets as they are found.</div>
                </div>
              ) : (
                <div style={{ padding: '0 14px 14px' }}>
                  <div style={c.card}>
                    <table style={c.table}>
                      <thead>
                        <tr>
                          <th style={c.th}>Chain</th>
                          <th style={c.th}>Chain ID</th>
                          <th style={c.th}>Ecosystem</th>
                          <th style={c.th}>Type</th>
                          <th style={c.th}>Status</th>
                          <th style={c.th}>Source</th>
                          <th style={c.th}>RPC</th>
                          <th style={c.th}>Discovered</th>
                        </tr>
                      </thead>
                      <tbody>
                        {discoveries.map(d => (
                          <tr key={d.id}>
                            <td style={{ ...c.td, fontWeight: 500 }}>{d.chain_name}</td>
                            <td style={{ ...c.td, color: '#9ca3af' }}>{d.chain_numeric_id ?? d.chain_id ?? '—'}</td>
                            <td style={{ ...c.td, color: '#a78bfa' }}>{d.ecosystem}</td>
                            <td style={c.td}>
                              <span style={c.badge(d.is_testnet ? '#f59e0b' : '#10b981')}>{d.chain_type}{d.is_testnet ? ' 🧪' : ''}</span>
                            </td>
                            <td style={c.td}><span style={c.badge(statusColors[d.status] || '#6b7280')}>{d.status}</span></td>
                            <td style={{ ...c.td, color: '#6b7280' }}>{d.source}</td>
                            <td style={{ ...c.td, fontSize: '0.65rem', color: '#555', maxWidth: 200, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' as const }}>{d.rpc_url || '—'}</td>
                            <td style={{ ...c.td, color: '#6b7280', fontSize: '0.68rem' }}>{fmtDate(d.discovered_at)}</td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                </div>
              )}
            </>
          )}
        </>
      )}
    </div>
  );
};

export default AirdropsPanel;
