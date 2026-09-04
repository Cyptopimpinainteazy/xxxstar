import React, { useState, useEffect, useCallback } from 'react';
import { invoke, isTauri } from '../ipc/tauri';

// ── Types ────────────────────────────────────────────────────────────────────

interface FoundryProject {
  id: string;
  name: string;
  dapp_type: string;
  description: string;
  creator_wallet: string;
  status: string;
  chain: string;
  contract_addresses: Record<string, string>;
  frontend_url: string | null;
  marketplace_listing_id: string | null;
  risk_score: number;
  created_at: string;
  updated_at: string;
}

interface TemplateEntry {
  name: string;
  description: string;
  icon: string;
}

interface SimulationResult {
  expected_daily_volume: string;
  expected_daily_fees: string;
  estimated_gas_cost: string;
  days_to_break_even: number;
  is_profitable: boolean;
  confidence_score: number;
  monthly_projections: MonthlyProjection[];
}

interface MonthlyProjection {
  month: number;
  volume: string;
  revenue: string;
  gas_cost: string;
}

interface AuditResult {
  passed: boolean;
  risk_score: number;
  warnings: string[];
  critical_findings: string[];
  fee_findings: string[];
  static_analysis_score: number;
  auditor_signature: string;
}

interface DeployResult {
  success: boolean;
  chain: string;
  contract_addresses: Record<string, string>;
  frontend_url: string | null;
  tx_hashes: string[];
  block_number: number;
  gas_used: number;
  manifest_hash: string;
}

interface RevenueSummary {
  total_volume: string;
  total_fees: string;
  platform_revenue: string;
  creator_revenue: string;
  unclaimed_revenue: string;
  transaction_count: number;
}

interface MarketplaceListing {
  id: string;
  project_id: string;
  title: string;
  description: string;
  dapp_type: string;
  price: string;
  price_token: string;
  creator_wallet: string;
  rating: number;
  download_count: number;
  verified: boolean;
  listed_at: string;
}

// ── Icons ─────────────────────────────────────────────────────────────────────

const DAPP_ICONS: Record<string, string> = {
  token: '💰',
  nft: '🖼️',
  staking: '🏦',
  subscription: '📅',
  escrow: '🤝',
  ai: '🤖',
  trading: '📈',
  yield: '🌾',
  'cross-chain': '🌉',
  domain: '🌐',
  prediction: '🔮',
  affiliate: '🔗',
  data: '📊',
  custom: '⚙️',
};

const STATUS_COLORS: Record<string, string> = {
  draft: 'text-gray-400 bg-gray-500/20',
  generating: 'text-yellow-400 bg-yellow-500/20',
  auditing: 'text-orange-400 bg-orange-500/20',
  simulating: 'text-blue-400 bg-blue-500/20',
  deploying: 'text-purple-400 bg-purple-500/20',
  deployed: 'text-green-400 bg-green-500/20',
  failed: 'text-red-400 bg-red-500/20',
  audited: 'text-cyan-400 bg-cyan-500/20',
};

// ── Main Component ────────────────────────────────────────────────────────────

export function FoundryPanel({ onClose }: { onClose: () => void }) {
  const [tab, setTab] = useState<'generate' | 'projects' | 'marketplace' | 'templates'>('generate');
  const [prompt, setPrompt] = useState('');
  const [wallet, setWallet] = useState('5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY');
  const [generating, setGenerating] = useState(false);
  const [projects, setProjects] = useState<FoundryProject[]>([]);
  const [templates, setTemplates] = useState<TemplateEntry[]>([]);
  const [listings, setListings] = useState<MarketplaceListing[]>([]);
  const [selectedProject, setSelectedProject] = useState<FoundryProject | null>(null);
  const [simResult, setSimResult] = useState<SimulationResult | null>(null);
  const [auditResult, setAuditResult] = useState<AuditResult | null>(null);
  const [deployResult, setDeployResult] = useState<DeployResult | null>(null);
  const [revenue, setRevenue] = useState<RevenueSummary | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [listingTitle, setListingTitle] = useState('');
  const [listingDesc, setListingDesc] = useState('');

  // Load projects and templates on mount
  useEffect(() => {
    if (!isTauri()) return;
    loadProjects();
    loadTemplates();
    loadListings();
  }, []);

  const loadProjects = async () => {
    try {
      const p = await invoke<FoundryProject[]>('foundry_list_projects');
      setProjects(p);
    } catch (e) {
      console.error('Failed to load projects:', e);
    }
  };

  const loadTemplates = async () => {
    try {
      const t = await invoke<TemplateEntry[]>('foundry_list_templates');
      setTemplates(t);
    } catch (e) {
      console.error('Failed to load templates:', e);
    }
  };

  const loadListings = async () => {
    try {
      const l = await invoke<MarketplaceListing[]>('foundry_list_listings');
      setListings(l);
    } catch (e) {
      console.error('Failed to load listings:', e);
    }
  };

  const handleGenerate = useCallback(async () => {
    if (!prompt.trim()) return;
    setGenerating(true);
    setError(null);
    try {
      const project = await invoke<FoundryProject>('foundry_generate', {
        request: { prompt: prompt.trim(), creator_wallet: wallet, target_chain: null },
      });
      setSelectedProject(project);
      setTab('projects');
      await loadProjects();
    } catch (e: any) {
      setError(e?.toString() || 'Generation failed');
    } finally {
      setGenerating(false);
    }
  }, [prompt, wallet]);

  const handleAudit = async (projectId: string) => {
    setError(null);
    setAuditResult(null);
    try {
      const result = await invoke<AuditResult>('foundry_audit', {
        request: { project_id: projectId },
      });
      setAuditResult(result);
      await loadProjects();
    } catch (e: any) {
      setError(e?.toString() || 'Audit failed');
    }
  };

  const handleSimulate = async (projectId: string) => {
    setError(null);
    setSimResult(null);
    try {
      const result = await invoke<SimulationResult>('foundry_simulate', {
        request: { project_id: projectId, user_base_estimate: null },
      });
      setSimResult(result);
    } catch (e: any) {
      setError(e?.toString() || 'Simulation failed');
    }
  };

  const handleDeploy = async (projectId: string) => {
    setError(null);
    setDeployResult(null);
    try {
      const result = await invoke<DeployResult>('foundry_deploy', {
        request: { project_id: projectId, chain: null },
      });
      setDeployResult(result);
      await loadProjects();
    } catch (e: any) {
      setError(e?.toString() || 'Deployment failed');
    }
  };

  const handleRevenue = async (projectId: string) => {
    setError(null);
    setRevenue(null);
    try {
      const result = await invoke<RevenueSummary>('foundry_revenue_summary', {
        projectId,
      });
      setRevenue(result);
    } catch (e: any) {
      setError(e?.toString() || 'Revenue fetch failed');
    }
  };

  const handleDelete = async (projectId: string) => {
    try {
      await invoke<void>('foundry_delete_project', { projectId });
      setSelectedProject(null);
      await loadProjects();
    } catch (e: any) {
      setError(e?.toString() || 'Delete failed');
    }
  };

  const handleCreateListing = async (projectId: string) => {
    if (!listingTitle.trim()) return;
    try {
      await invoke<MarketplaceListing>('foundry_create_listing', {
        request: {
          project_id: projectId,
          title: listingTitle.trim(),
          description: listingDesc.trim(),
          price: null,
          price_token: null,
        },
      });
      setListingTitle('');
      setListingDesc('');
      await loadListings();
    } catch (e: any) {
      setError(e?.toString() || 'Listing failed');
    }
  };

  const formatToken = (val: string) => {
    const n = parseInt(val, 10);
    if (isNaN(n)) return val;
    return (n / 1e18).toLocaleString(undefined, { maximumFractionDigits: 4 }) + ' X3';
  };

  // ── Render ──────────────────────────────────────────────────────────────────

  return (
    <div className="h-full w-full bg-black/80 backdrop-blur-md flex flex-col pointer-events-auto overflow-hidden">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-white/10">
        <div className="flex items-center gap-2">
          <div className="w-6 h-6 rounded bg-gradient-to-br from-cyan-400 to-blue-600 flex items-center justify-center text-white font-bold text-[10px]">
            XF
          </div>
          <h2 className="text-white font-semibold text-sm">X3 Foundry</h2>
        </div>
        <button onClick={onClose} className="text-white/40 hover:text-white/80 text-lg leading-none">&times;</button>
      </div>

      {/* Tabs */}
      <div className="flex border-b border-white/10">
        {(['generate', 'projects', 'marketplace', 'templates'] as const).map((t) => (
          <button
            key={t}
            onClick={() => { setTab(t); setSelectedProject(null); setError(null); }}
            className={`flex-1 py-2 text-[10px] font-semibold uppercase tracking-wider transition-all ${
              tab === t ? 'text-cyan-400 border-b-2 border-cyan-400 bg-white/5' : 'text-white/40 hover:text-white/70'
            }`}
          >
            {t === 'generate' ? '✨ Generate' : t === 'projects' ? '📦 Projects' : t === 'marketplace' ? '🏪 Marketplace' : '📋 Templates'}
          </button>
        ))}
      </div>

      {/* Error */}
      {error && (
        <div className="mx-4 mt-2 px-3 py-2 bg-red-900/30 border border-red-500/30 rounded text-red-400 text-[11px]">
          {error}
        </div>
      )}

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        {tab === 'generate' && (
          <div className="p-4 space-y-4">
            <div>
              <label className="text-white/50 text-[10px] font-semibold uppercase tracking-wider block mb-1">
                Describe your dApp
              </label>
              <textarea
                value={prompt}
                onChange={(e) => setPrompt(e.target.value)}
                placeholder="e.g. Create a token launchpad called 'MoonRocket' with presale and vesting..."
                className="w-full h-24 bg-white/5 border border-white/10 rounded-lg p-3 text-white text-xs font-mono placeholder-white/20 resize-none focus:outline-none focus:border-cyan-500/50"
              />
            </div>
            <div>
              <label className="text-white/50 text-[10px] font-semibold uppercase tracking-wider block mb-1">
                Creator Wallet
              </label>
              <input
                value={wallet}
                onChange={(e) => setWallet(e.target.value)}
                className="w-full bg-white/5 border border-white/10 rounded-lg p-2 text-white text-xs font-mono focus:outline-none focus:border-cyan-500/50"
              />
            </div>
            <button
              onClick={handleGenerate}
              disabled={generating || !prompt.trim()}
              className="w-full py-2.5 rounded-lg text-xs font-bold bg-gradient-to-r from-cyan-500 to-blue-600 text-white hover:from-cyan-400 hover:to-blue-500 disabled:opacity-40 disabled:cursor-not-allowed transition-all"
            >
              {generating ? '✨ Generating...' : '🚀 Generate dApp'}
            </button>

            {/* Template quick picks */}
            <div>
              <h3 className="text-white/50 text-[10px] font-semibold uppercase tracking-wider mb-2">Quick Start Templates</h3>
              <div className="grid grid-cols-2 gap-2">
                {templates.slice(0, 6).map((t) => (
                  <button
                    key={t.name}
                    onClick={() => setPrompt(`Create a ${t.name.toLowerCase()} dApp`)}
                    className="flex items-center gap-2 p-2 bg-white/5 hover:bg-white/10 rounded-lg border border-white/5 text-left transition-all"
                  >
                    <span className="text-lg">{DAPP_ICONS[t.icon] || '⚙️'}</span>
                    <div>
                      <div className="text-white text-[11px] font-medium">{t.name}</div>
                      <div className="text-white/30 text-[9px] truncate">{t.description}</div>
                    </div>
                  </button>
                ))}
              </div>
            </div>
          </div>
        )}

        {tab === 'projects' && (
          <div className="p-4 space-y-3">
            {projects.length === 0 && (
              <div className="text-center py-8">
                <div className="text-4xl mb-2">🏗️</div>
                <p className="text-white/40 text-xs">No projects yet. Generate your first dApp!</p>
              </div>
            )}

            {selectedProject ? (
              <div className="space-y-3">
                {/* Project detail header */}
                <div className="flex items-center justify-between">
                  <div>
                    <h3 className="text-white font-semibold text-sm">{selectedProject.name}</h3>
                    <span className="text-white/40 text-[10px] font-mono">{selectedProject.dapp_type}</span>
                  </div>
                  <div className="flex items-center gap-2">
                    <span className={`px-2 py-0.5 rounded text-[10px] font-mono font-bold ${STATUS_COLORS[selectedProject.status] || 'text-gray-400 bg-gray-500/20'}`}>
                      {selectedProject.status.toUpperCase()}
                    </span>
                    <button onClick={() => setSelectedProject(null)} className="text-white/30 hover:text-white/60 text-xs">← Back</button>
                  </div>
                </div>

                {/* Action buttons */}
                <div className="flex flex-wrap gap-2">
                  <ActionBtn label="🔍 Audit" onClick={() => handleAudit(selectedProject.id)} color="orange" />
                  <ActionBtn label="📊 Simulate" onClick={() => handleSimulate(selectedProject.id)} color="blue" />
                  <ActionBtn label="🚀 Deploy" onClick={() => handleDeploy(selectedProject.id)} color="green" disabled={selectedProject.status === 'deployed'} />
                  <ActionBtn label="💰 Revenue" onClick={() => handleRevenue(selectedProject.id)} color="purple" />
                  <ActionBtn label="🗑️ Delete" onClick={() => handleDelete(selectedProject.id)} color="red" />
                </div>

                {/* Audit results */}
                {auditResult && (
                  <div className={`p-3 rounded-lg border ${auditResult.passed ? 'bg-green-900/20 border-green-500/30' : 'bg-red-900/20 border-red-500/30'}`}>
                    <div className="flex items-center justify-between mb-2">
                      <span className="text-white text-xs font-semibold">Security Audit</span>
                      <span className={`text-[10px] font-mono font-bold ${auditResult.passed ? 'text-green-400' : 'text-red-400'}`}>
                        {auditResult.passed ? '✅ PASSED' : '❌ FAILED'}
                      </span>
                    </div>
                    <div className="flex gap-2 mb-2">
                      <MiniStat label="Risk" value={`${auditResult.risk_score}/100`} />
                      <MiniStat label="Static Analysis" value={`${auditResult.static_analysis_score}/100`} />
                    </div>
                    {auditResult.warnings.length > 0 && (
                      <div className="mb-1">
                        <span className="text-yellow-400 text-[10px] font-semibold">Warnings:</span>
                        {auditResult.warnings.map((w, i) => (
                          <p key={i} className="text-yellow-300/70 text-[10px] ml-2">⚠ {w}</p>
                        ))}
                      </div>
                    )}
                    {auditResult.critical_findings.length > 0 && (
                      <div>
                        <span className="text-red-400 text-[10px] font-semibold">Critical:</span>
                        {auditResult.critical_findings.map((c, i) => (
                          <p key={i} className="text-red-300/70 text-[10px] ml-2">🔴 {c}</p>
                        ))}
                      </div>
                    )}
                  </div>
                )}

                {/* Simulation results */}
                {simResult && (
                  <div className="p-3 bg-blue-900/20 border border-blue-500/30 rounded-lg">
                    <h4 className="text-white text-xs font-semibold mb-2">📊 Economic Simulation</h4>
                    <div className="grid grid-cols-2 gap-2 mb-2">
                      <MiniStat label="Daily Volume" value={formatToken(simResult.expected_daily_volume)} />
                      <MiniStat label="Daily Fees" value={formatToken(simResult.expected_daily_fees)} />
                      <MiniStat label="Gas Cost" value={formatToken(simResult.estimated_gas_cost)} />
                      <MiniStat label="Break Even" value={simResult.days_to_break_even < 365 ? `${simResult.days_to_break_even} days` : '>1 year'} />
                      <MiniStat label="Profitable" value={simResult.is_profitable ? '✅ Yes' : '❌ No'} />
                      <MiniStat label="Confidence" value={`${(simResult.confidence_score * 100).toFixed(0)}%`} />
                    </div>
                    {simResult.monthly_projections.length > 0 && (
                      <div>
                        <span className="text-white/50 text-[10px] font-semibold">12-Month Projection:</span>
                        <div className="mt-1 space-y-1">
                          {simResult.monthly_projections.filter((_, i) => i % 3 === 0 || i === 11).map((m) => (
                            <div key={m.month} className="flex justify-between text-[10px] font-mono">
                              <span className="text-white/50">Month {m.month}</span>
                              <span className="text-green-400">Vol: {formatToken(m.volume)}</span>
                              <span className="text-cyan-400">Rev: {formatToken(m.revenue)}</span>
                            </div>
                          ))}
                        </div>
                      </div>
                    )}
                  </div>
                )}

                {/* Deploy results */}
                {deployResult && (
                  <div className="p-3 bg-green-900/20 border border-green-500/30 rounded-lg">
                    <h4 className="text-white text-xs font-semibold mb-2">🚀 Deployment</h4>
                    <div className="space-y-1 text-[10px] font-mono">
                      <div className="flex justify-between"><span className="text-white/50">Chain</span><span className="text-white">{deployResult.chain}</span></div>
                      <div className="flex justify-between"><span className="text-white/50">Block</span><span className="text-white">#{deployResult.block_number}</span></div>
                      <div className="flex justify-between"><span className="text-white/50">Gas Used</span><span className="text-white">{deployResult.gas_used.toLocaleString()}</span></div>
                      {deployResult.frontend_url && (
                        <div className="flex justify-between"><span className="text-white/50">Frontend</span><span className="text-cyan-400">{deployResult.frontend_url}</span></div>
                      )}
                    </div>
                    {Object.entries(deployResult.contract_addresses).length > 0 && (
                      <div className="mt-2">
                        <span className="text-white/50 text-[10px] font-semibold">Contracts:</span>
                        {Object.entries(deployResult.contract_addresses).map(([name, addr]) => (
                          <div key={name} className="flex justify-between text-[10px] font-mono">
                            <span className="text-white/60">{name}</span>
                            <span className="text-white/80">{addr.slice(0, 16)}...</span>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                )}

                {/* Revenue */}
                {revenue && (
                  <div className="p-3 bg-purple-900/20 border border-purple-500/30 rounded-lg">
                    <h4 className="text-white text-xs font-semibold mb-2">💰 Revenue Summary</h4>
                    <div className="grid grid-cols-2 gap-2">
                      <MiniStat label="Total Volume" value={formatToken(revenue.total_volume)} />
                      <MiniStat label="Total Fees" value={formatToken(revenue.total_fees)} />
                      <MiniStat label="Platform Revenue" value={formatToken(revenue.platform_revenue)} />
                      <MiniStat label="Creator Revenue" value={formatToken(revenue.creator_revenue)} />
                      <MiniStat label="Unclaimed" value={formatToken(revenue.unclaimed_revenue)} />
                      <MiniStat label="Transactions" value={revenue.transaction_count.toLocaleString()} />
                    </div>
                  </div>
                )}

                {/* Marketplace listing form */}
                {selectedProject.status === 'deployed' && !selectedProject.marketplace_listing_id && (
                  <div className="p-3 bg-white/5 rounded-lg border border-white/10">
                    <h4 className="text-white text-xs font-semibold mb-2">🏪 List on Marketplace</h4>
                    <input
                      value={listingTitle}
                      onChange={(e) => setListingTitle(e.target.value)}
                      placeholder="Listing title"
                      className="w-full mb-2 bg-white/5 border border-white/10 rounded p-2 text-white text-[11px] font-mono placeholder-white/20 focus:outline-none focus:border-cyan-500/50"
                    />
                    <textarea
                      value={listingDesc}
                      onChange={(e) => setListingDesc(e.target.value)}
                      placeholder="Description"
                      rows={2}
                      className="w-full mb-2 bg-white/5 border border-white/10 rounded p-2 text-white text-[11px] font-mono placeholder-white/20 resize-none focus:outline-none focus:border-cyan-500/50"
                    />
                    <button
                      onClick={() => handleCreateListing(selectedProject.id)}
                      disabled={!listingTitle.trim()}
                      className="w-full py-1.5 rounded text-[11px] font-bold bg-gradient-to-r from-purple-500 to-pink-600 text-white disabled:opacity-40"
                    >
                      📢 Create Listing
                    </button>
                  </div>
                )}

                {/* Project metadata */}
                <div className="p-3 bg-white/5 rounded-lg border border-white/10">
                  <h4 className="text-white/50 text-[10px] font-semibold uppercase tracking-wider mb-2">Details</h4>
                  <div className="space-y-1 text-[10px] font-mono">
                    <div className="flex justify-between"><span className="text-white/40">ID</span><span className="text-white/60">{selectedProject.id.slice(0, 16)}...</span></div>
                    <div className="flex justify-between"><span className="text-white/40">Wallet</span><span className="text-white/60">{selectedProject.creator_wallet.slice(0, 20)}...</span></div>
                    <div className="flex justify-between"><span className="text-white/40">Chain</span><span className="text-white/60">{selectedProject.chain}</span></div>
                    <div className="flex justify-between"><span className="text-white/40">Risk Score</span><span className={`${selectedProject.risk_score < 30 ? 'text-green-400' : selectedProject.risk_score < 60 ? 'text-yellow-400' : 'text-red-400'}`}>{selectedProject.risk_score}/100</span></div>
                    <div className="flex justify-between"><span className="text-white/40">Created</span><span className="text-white/60">{new Date(selectedProject.created_at).toLocaleDateString()}</span></div>
                  </div>
                </div>
              </div>
            ) : (
              /* Project list */
              projects.map((p) => (
                <div
                  key={p.id}
                  onClick={() => { setSelectedProject(p); setAuditResult(null); setSimResult(null); setDeployResult(null); setRevenue(null); }}
                  className="p-3 bg-white/5 hover:bg-white/10 rounded-lg border border-white/5 cursor-pointer transition-all"
                >
                  <div className="flex items-center justify-between mb-1">
                    <h4 className="text-white text-xs font-semibold">{p.name}</h4>
                    <span className={`px-1.5 py-0.5 rounded text-[9px] font-mono font-bold ${STATUS_COLORS[p.status] || 'text-gray-400 bg-gray-500/20'}`}>
                      {p.status}
                    </span>
                  </div>
                  <div className="flex items-center gap-3 text-[10px] font-mono">
                    <span className="text-white/40">{p.dapp_type}</span>
                    <span className="text-white/30">|</span>
                    <span className="text-white/40">{p.chain}</span>
                    {p.risk_score > 0 && (
                      <>
                        <span className="text-white/30">|</span>
                        <span className={p.risk_score < 30 ? 'text-green-400' : p.risk_score < 60 ? 'text-yellow-400' : 'text-red-400'}>
                          Risk: {p.risk_score}
                        </span>
                      </>
                    )}
                  </div>
                </div>
              ))
            )}
          </div>
        )}

        {tab === 'marketplace' && (
          <div className="p-4 space-y-3">
            {listings.length === 0 && (
              <div className="text-center py-8">
                <div className="text-4xl mb-2">🏪</div>
                <p className="text-white/40 text-xs">No listings yet. Deploy a project and list it!</p>
              </div>
            )}
            {listings.map((l) => (
              <div key={l.id} className="p-3 bg-white/5 rounded-lg border border-white/10">
                <div className="flex items-center justify-between mb-1">
                  <h4 className="text-white text-xs font-semibold">{l.title}</h4>
                  <div className="flex items-center gap-2">
                    {l.verified && <span className="text-blue-400 text-[10px]">✅ Verified</span>}
                    <span className="text-white/40 text-[10px] font-mono">{l.dapp_type}</span>
                  </div>
                </div>
                <p className="text-white/50 text-[10px] mb-2">{l.description}</p>
                <div className="flex items-center justify-between text-[10px] font-mono">
                  <span className="text-white/40">Price: <span className="text-cyan-400">{l.price} {l.price_token}</span></span>
                  <span className="text-white/40">Rating: <span className="text-yellow-400">{l.rating.toFixed(1)} ⭐</span></span>
                  <span className="text-white/40">Downloads: <span className="text-white/60">{l.download_count}</span></span>
                </div>
              </div>
            ))}
          </div>
        )}

        {tab === 'templates' && (
          <div className="p-4 space-y-2">
            {templates.map((t) => (
              <div key={t.name} className="flex items-center gap-3 p-3 bg-white/5 rounded-lg border border-white/5">
                <span className="text-2xl">{DAPP_ICONS[t.icon] || '⚙️'}</span>
                <div>
                  <h4 className="text-white text-xs font-semibold">{t.name}</h4>
                  <p className="text-white/40 text-[10px]">{t.description}</p>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

// ── Sub-components ────────────────────────────────────────────────────────────

function ActionBtn({ label, onClick, color, disabled }: { label: string; onClick: () => void; color: string; disabled?: boolean }) {
  const colorMap: Record<string, string> = {
    orange: 'from-orange-500 to-red-600',
    blue: 'from-blue-500 to-cyan-600',
    green: 'from-green-500 to-emerald-600',
    purple: 'from-purple-500 to-violet-600',
    red: 'from-red-500 to-rose-600',
  };
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      className={`px-3 py-1.5 rounded text-[10px] font-bold text-white bg-gradient-to-r ${colorMap[color] || colorMap.blue} disabled:opacity-40 disabled:cursor-not-allowed hover:opacity-90 transition-all`}
    >
      {label}
    </button>
  );
}

function MiniStat({ label, value }: { label: string; value: string }) {
  return (
    <div className="bg-white/5 rounded p-2">
      <div className="text-white/40 text-[9px]">{label}</div>
      <div className="text-white font-mono text-[10px] font-bold truncate">{value}</div>
    </div>
  );
}
