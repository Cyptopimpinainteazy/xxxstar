import { useCallback, useEffect, useRef, useState } from 'react';
import { ArrowUpRight, Check, Database, RefreshCw, Search, ShieldCheck, X } from 'lucide-react';
import './MoneySwarmWorkspace.css';
import { FundingProposalEditor } from './FundingProposalEditor';

interface Grant {
  grant_id: string; external_id: string; title: string; sponsor: string;
  status: string; stage: string; score: number; amount_usd: number;
  metadata: Record<string, unknown>; created_at: string; updated_at: string;
}
interface OfficialPortalSnapshot {
  snapshot_id: string; source_id: string; official_url: string; content_sha256: string;
  retrieved_at: string; last_observed_at: string;
  normalized: { program_name?: string; source_verification?: string;
    application_guidance_status?: string; eligibility_status?: string;
    approved_for_submission?: boolean; round?: string | null; deadline?: string | null; };
}
interface OfficialPortalResult {
  new_snapshot: boolean; source_id: string; official_url: string; snapshot_id: string;
  application_guidance_status: string; eligibility_status: string;
  awarded_funding_verified: boolean;
}
interface HistoricalLead {
  source_file: string; legacy_id: string; name: string; category: string;
  historical_status: string; current_availability: string;
  current_eligibility: string; review_state: string;
}
interface Scoreboard {
  total_grants: number; open_grants: number; submitted_proposals: number;
  approved_grants: number; funded_grants: number; total_awarded_usd: number;
  last_event_at: string | null;
}
interface TimelineItem {
  item_id: string; title: string; occurred_at: string; data: Record<string, unknown>;
}

const gateway = (import.meta.env.VITE_X3_GATEWAY_URL || '/money-api').replace(/\/$/, '');
const money = new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD', maximumFractionDigits: 0 });
function text(value: unknown, fallback = 'Not provided by source'): string {
  return typeof value === 'string' && value.length > 0 ? value : fallback;
}
function officialUrl(value: unknown): string | null {
  if (typeof value !== 'string') return null;
  try { const url = new URL(value); return url.protocol === 'https:' && !url.username && !url.password ? url.href : null; }
  catch { return null; }
}
function date(value: unknown): string {
  if (typeof value !== 'string' || !Number.isFinite(Date.parse(value))) return 'Not verified';
  return new Intl.DateTimeFormat('en-US', { dateStyle: 'medium', timeZone: 'UTC' }).format(new Date(value));
}

async function request<T>(path: string, token = '', payload?: unknown): Promise<T> {
  const response = await fetch(`${gateway}${path}`, {
    method: payload === undefined ? 'GET' : 'POST',
    headers: { ...(token ? { 'x-admin-token': token } : {}),
      ...(payload === undefined ? {} : { 'Content-Type': 'application/json' }) },
    ...(payload === undefined ? {} : { body: JSON.stringify(payload) }),
  });
  if (!response.ok) throw new Error(`Funding service unavailable (HTTP ${response.status}). Check the gateway connection and operator credential, then retry.`);
  return response.json() as Promise<T>;
}

export function MoneySwarmWorkspace() {
  const [grants, setGrants] = useState<Grant[]>([]);
  const [stats, setStats] = useState<Scoreboard | null>(null);
  const [timeline, setTimeline] = useState<TimelineItem[]>([]);
  const [query, setQuery] = useState('');
  const [token, setToken] = useState('');
  const [sourceKeyword, setSourceKeyword] = useState('distributed computing');
  const [selected, setSelected] = useState<Grant | null>(null);
  const [view, setView] = useState<'opportunities' | 'review' | 'activity' | 'archive' | 'web3'>('opportunities');
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');
  const [officialSources, setOfficialSources] = useState<OfficialPortalSnapshot[]>([]);
  const [portalProvider, setPortalProvider] = useState<'solana' | 'stellar'>('stellar');
  const [portalBusy, setPortalBusy] = useState(false);
  const [portalError, setPortalError] = useState('');
  const portalRequest = useRef(0);
  const [archive, setArchive] = useState<HistoricalLead[]>([]);
  const [archiveOffset, setArchiveOffset] = useState(0);
  const [archiveSearch, setArchiveSearch] = useState('');
  const [archiveLoading, setArchiveLoading] = useState(false);
  const [archiveError, setArchiveError] = useState('');
  const archiveRequest = useRef(0);
  const detailsRef = useRef<HTMLElement>(null);

  useEffect(() => {
    if (!selected) return;
    const previous = document.activeElement;
    detailsRef.current?.querySelector<HTMLButtonElement>('button')?.focus();
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setSelected(null);
      if (event.key !== 'Tab') return;
      const controls = Array.from(detailsRef.current?.querySelectorAll<HTMLElement>('button:not(:disabled),a[href],input:not(:disabled)') || []);
      const first = controls[0], last = controls.at(-1);
      if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last?.focus(); }
      if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first?.focus(); }
    };
    document.addEventListener('keydown', onKey);
    return () => { document.removeEventListener('keydown', onKey); if (previous instanceof HTMLElement) previous.focus(); };
  }, [selected]);

  const load = useCallback(async () => {
    setLoading(true); setError('');
    try {
      const [summary, records, events] = await Promise.all([
        request<Scoreboard>('/api/public/funding-swarm/scoreboard'),
        request<Grant[]>('/api/public/funding-swarm/grants?limit=100'),
        request<TimelineItem[]>('/api/public/funding-swarm/timeline?limit=50'),
      ]);
      if (!Array.isArray(records) || !Array.isArray(events) || typeof summary.total_grants !== 'number') throw new Error('Funding service returned an invalid response. Retry after checking the gateway.');
      setStats(summary); setGrants(records); setTimeline(events);
    } catch (failure) { setStats(null); setGrants([]); setTimeline([]); setError(failure instanceof Error ? failure.message : 'Funding service unavailable. Retry the connection.'); }
    finally { setLoading(false); }
  }, []);
  useEffect(() => { void load(); }, [load]);

  const loadOfficialSources = async () => {
    if (!token) return;
    const requestId = ++portalRequest.current;
    setPortalBusy(true); setPortalError('');
    try {
      const records = await request<OfficialPortalSnapshot[]>(
        '/api/v1/admin/funding-swarm/discovery/web3/sources', token
      );
      if (requestId !== portalRequest.current) return;
      if (!Array.isArray(records) || records.some(row =>
        row.normalized?.eligibility_status !== 'UNKNOWN' ||
        row.normalized?.approved_for_submission !== false ||
        row.normalized?.source_verification !== 'OFFICIAL_PAGE_OBSERVED'
      )) throw new Error('Official portal records have unexpected verification or approval state.');
      setOfficialSources(records);
    } catch (failure) {
      if (requestId === portalRequest.current) {
        setOfficialSources([]);
        setPortalError(failure instanceof Error ? failure.message : 'Official portal research unavailable.');
      }
    } finally {
      if (requestId === portalRequest.current) setPortalBusy(false);
    }
  };

  const refreshOfficialSource = async () => {
    if (!token || busy || portalBusy) return;
    setBusy(true); setPortalError(''); setNotice('');
    try {
      const result = await request<OfficialPortalResult>(
        '/api/v1/admin/funding-swarm/discovery/web3', token, { provider: portalProvider }
      );
      if (result.eligibility_status !== 'UNKNOWN' || result.awarded_funding_verified) {
        throw new Error('Official portal probe returned an unauthorized eligibility or award claim.');
      }
      setNotice(`Official ${portalProvider} page observed (${result.new_snapshot ? 'new source snapshot' : 'unchanged evidence'}). Application availability and eligibility require review.`);
      await loadOfficialSources();
    } catch (failure) {
      setPortalError(failure instanceof Error ? failure.message : 'Official portal refresh unavailable.');
    } finally { setBusy(false); }
  };

  const loadArchive = async (offset: number) => {
    if (!token) return;
    const requestId = ++archiveRequest.current;
    setArchiveLoading(true);
    setArchiveError('');
    try {
      const records = await request<HistoricalLead[]>(`/api/v1/admin/funding-swarm/legacy-leads?limit=100&offset=${offset}&search=${encodeURIComponent(archiveSearch.trim())}`, token);
      if (requestId !== archiveRequest.current) return;
      if (!Array.isArray(records) || records.some(record =>
        record.current_availability !== 'UNKNOWN' || record.current_eligibility !== 'UNKNOWN' ||
        record.review_state !== 'UNVERIFIED'
      )) throw new Error('Archived records have an unexpected verification state. Stop and investigate.');
      setArchive(records);
      setArchiveOffset(offset);
    } catch (failure) {
      if (requestId === archiveRequest.current) {
        setArchive([]);
        setArchiveError(failure instanceof Error ? failure.message : 'Historical research unavailable.');
      }
    } finally {
      if (requestId === archiveRequest.current) setArchiveLoading(false);
    }
  };

  const discover = async () => {
    if (!token || busy) return;
    setBusy(true); setNotice(''); setError('');
    try {
      const result = await request<{ imported: number; changed: number }>('/api/v1/admin/funding-swarm/discovery/grants-gov', token, { keyword: sourceKeyword, pages: 1 });
      setNotice(`Source refresh complete: ${result.imported} opportunities checked, ${result.changed} new or changed. Eligibility remains subject to review.`);
      await load();
    } catch (failure) { setError(failure instanceof Error ? failure.message : 'Source refresh failed.'); }
    finally { setBusy(false); }
  };

  const advance = async (grant: Grant, action: string) => {
    if (!token || busy) return;
    setBusy(true); setError('');
    try {
      const updated = await request<Grant>(`/api/v1/admin/funding-swarm/grants/${encodeURIComponent(grant.grant_id)}/${action}`, token, {});
      setSelected(updated); setNotice(`Updated review stage for ${grant.title}. No application was submitted.`);
      await load();
    } catch (failure) { setError(failure instanceof Error ? failure.message : 'Review action failed.'); }
    finally { setBusy(false); }
  };

  const filtered = grants.filter(grant => `${grant.title} ${grant.sponsor} ${grant.external_id}`.toLowerCase().includes(query.toLowerCase()) && (view !== 'review' || grant.stage !== 'discovery'));
  const sourceCount = new Set(grants.map(grant => text(grant.metadata.source, 'manual'))).size;

  return <main className="money-workspace">
    <aside className="money-nav" aria-label="Money Swarm navigation">
      <a className="money-wordmark" href="/money-swarm"><span aria-hidden="true">✳</span> X3 <strong>Money Swarm</strong></a>
      <p className="money-nav-label">FUNDING OPERATIONS</p>
      <nav>{([['opportunities', 'Opportunity ledger', Search], ['review', 'Application review', ShieldCheck], ['web3', 'Official Web3 portals', Search], ['archive', 'Historical leads', Database], ['activity', 'Audit activity', Database]] as const).map(([id, label, Icon]) =>
        <button key={id} className={view === id ? 'active' : ''} aria-current={view === id ? 'page' : undefined} onClick={() => { setView(id); archiveRequest.current++; setArchive([]); setArchiveError(''); setArchiveLoading(false);
          portalRequest.current++; setOfficialSources([]); setPortalError(''); setPortalBusy(false); }}><Icon size={17} aria-hidden="true" />{label}</button>)}</nav>
      <div className="money-nav-note"><ShieldCheck size={20} aria-hidden="true" /><strong>Human decisions. Verified evidence.</strong><p>External submissions, spending and outreach require separate authorization.</p></div>
      <a className="money-back" href="/">← Inferstructor</a>
    </aside>
    <div className="money-content">
      <header className="money-header"><div><p className="money-eyebrow">X3 / INFRASTRUCTURE FUNDING</p><h1>{view === 'activity' ? 'Audit activity' : view === 'review' ? 'Application review' : view === 'archive' ? 'Historical research' : view === 'web3' ? 'Official Web3 sources' : 'Funding intelligence'}</h1><p>Find the next opportunity. Keep the evidence in view.</p></div>
        <button className="money-button" disabled={loading || archiveLoading || portalBusy || busy || ((view === 'archive' || view === 'web3') && !token)} onClick={() => { if (view === 'archive') void loadArchive(archiveOffset); else if (view === 'web3') void loadOfficialSources(); else void load(); }}><RefreshCw size={16} aria-hidden="true" />{loading || archiveLoading ? 'Loading records…' : view === 'archive' ? 'Load private research' : view === 'web3' ? 'Load official source records' : 'Refresh records'}</button></header>

      <div className="money-trust"><span className="money-dot" aria-hidden="true" /><span>Database records · Source timestamps shown per opportunity</span><span>Eligibility unknown until reviewed</span></div>
      {error && <div className="money-error" role="alert">{error}</div>}
      {notice && <div className="money-notice" role="status"><Check size={18} aria-hidden="true" />{notice}</div>}
      {stats && view !== 'archive' && view !== 'web3' && <section className="money-summary" aria-label="Funding pipeline">
        <div><span>Opportunities recorded</span><strong>{stats.total_grants.toLocaleString()}</strong><small>{sourceCount} sources in this view</small></div>
        <div><span>Open opportunities</span><strong>{stats.open_grants.toLocaleString()}</strong><small>Not confirmed eligibility</small></div>
        <div><span>Approved drafts</span><strong>{stats.approved_grants.toLocaleString()}</strong><small>Submission remains a human action</small></div>
        <div><span>Verified awards</span><strong>{money.format(stats.total_awarded_usd)}</strong><small>{stats.funded_grants === 0 ? 'No verified awards' : `${stats.funded_grants} verified award records`}</small></div>
      </section>}

      <section className="money-operator" aria-label="Operator access"><div><h2>Operator access</h2><p>A credential enables source discovery and review actions. It stays in memory for this session.</p></div>
        <label>Operator credential<input type="password" autoComplete="off" value={token} onChange={event => { archiveRequest.current++; setArchive([]); setArchiveOffset(0); setArchiveLoading(false); setArchiveError(''); portalRequest.current++; setOfficialSources([]); setPortalBusy(false); setPortalError(''); setToken(event.target.value); }} placeholder="Enter your operator credential" /></label></section>

      {view !== 'activity' && view !== 'archive' && view !== 'web3' && <>
        <section className="money-discovery" aria-label="Official source discovery"><div><h2>Discover from Grants.gov</h2><p>Read official opportunities. No application is sent.</p></div><label>Research keyword<input maxLength={120} value={sourceKeyword} onChange={event => setSourceKeyword(event.target.value)} /></label><button className="money-button primary" onClick={() => void discover()} disabled={!token || !sourceKeyword.trim() || busy || loading}>{busy ? 'Working…' : 'Discover opportunities'}<ArrowUpRight size={16} aria-hidden="true" /></button>{!token && <small>Enter an operator credential to refresh the official source.</small>}</section>
        <section className="money-ledger" aria-label="Opportunity ledger"><div className="money-section-heading"><div><h2>{view === 'review' ? 'Review pipeline' : 'Opportunity ledger'}</h2><p>{loading ? 'Retrieving records from the gateway…' : `${filtered.length} records shown · First 100 opportunities`}</p></div><label className="money-search"><Search size={16} aria-hidden="true" /><input type="search" aria-label="Search opportunities" placeholder="Search title, agency or identifier" value={query} onChange={event => setQuery(event.target.value)} /></label></div>
          {filtered.length === 0 && !loading && !error && <div className="money-empty"><Database size={26} aria-hidden="true" /><h3>{query ? 'No opportunities match your search' : view === 'review' ? 'No applications in review' : 'No opportunities recorded yet'}</h3><p>{query ? 'Try another research term or agency.' : 'Discover from an official source, then open a record to begin research.'}</p></div>}
          {filtered.map(grant => {
            const url = officialUrl(grant.metadata.official_url);
            return <article className="money-opportunity" key={grant.grant_id}><div className="money-opportunity-main"><p className="money-agency">{grant.sponsor}<span>{grant.external_id}</span></p><button className="money-title" onClick={() => setSelected(grant)}>{grant.title}</button><div className="money-opportunity-meta"><span>Deadline {text(grant.metadata.deadline, 'not provided')}</span><span className="money-status">{grant.stage.replaceAll('_', ' ')}</span></div></div><div className="money-evidence"><span className="money-unknown">Eligibility unknown</span><small>Source verified {date(grant.metadata.last_verified_at)}</small>{url ? <a href={url} target="_blank" rel="noopener noreferrer">Official source <ArrowUpRight size={14} aria-hidden="true" /></a> : <small>Official URL requires verification</small>}</div></article>;
          })}
        </section>
      </>}
      {view === 'web3' && <section className="money-ledger" aria-label="Official Web3 funding portals">
        <div className="money-section-heading"><div><h2>Official ecosystem funding sources</h2><p>Live page observations are source evidence, not verified open opportunities, eligibility or awards. Discovery requires an operator-enabled backend.</p></div></div>
        {!token && <div className="money-empty"><ShieldCheck size={26} aria-hidden="true" /><h3>Operator credential required</h3><p>These records are private research. Enter a credential before requesting official source pages.</p></div>}
        {portalError && <div className="money-error" role="alert">{portalError}</div>}
        {token && <div className="money-discovery">
          <div><h2>Inspect an official source</h2><p>Only fixed, reviewed Solana and Stellar pages. Never sends applications or messages.</p></div>
          <label>Funding portal
            <select aria-label="Funding portal" value={portalProvider} onChange={event => setPortalProvider(event.target.value as 'solana' | 'stellar')}>
              <option value="stellar">Stellar Community Fund</option>
              <option value="solana">Solana Foundation</option>
            </select>
          </label>
          <button className="money-button primary" disabled={busy || portalBusy} onClick={() => void refreshOfficialSource()}>Inspect official funding page</button>
        </div>}
        {officialSources.map(record => {
          const link = officialUrl(record.official_url);
          return <article className="money-opportunity" key={record.source_id}>
            <div className="money-opportunity-main">
              <p className="money-agency">{record.source_id}</p>
              <strong className="money-title">{text(record.normalized.program_name)}</strong>
              <div className="money-opportunity-meta"><span>{text(record.normalized.application_guidance_status, 'UNKNOWN')}</span>
                {record.normalized.round && <span>{record.normalized.round}</span>}
                {record.normalized.deadline && <span>Source deadline {record.normalized.deadline}</span>}</div>
            </div>
            <div className="money-evidence"><span className="money-unknown">Eligibility UNKNOWN</span><small>Page observed {date(record.last_observed_at)}</small>
              {link ? <a href={link} target="_blank" rel="noopener noreferrer">Official source <ArrowUpRight size={14} aria-hidden="true" /></a> : <small>Invalid source URL</small>}</div>
          </article>;
        })}
        {token && !portalBusy && !portalError && officialSources.length === 0 && <div className="money-empty"><Database size={26} aria-hidden="true" /><h3>No official pages loaded</h3><p>Choose Load official source records to inspect stored snapshots.</p></div>}
      </section>}
      {view === 'archive' && <section className="money-ledger" aria-label="Historical research leads">
        <div className="money-section-heading"><div><h2>Historical research queue</h2><p>Private source-labelled candidates, not verified opportunities or awards. Search, then select Load private research.</p></div>
          <label className="money-search"><Search size={16} aria-hidden="true" /><input type="search" aria-label="Search historical leads" maxLength={120} value={archiveSearch} placeholder="Search archive titles and categories" onChange={event => { archiveRequest.current++; setArchiveSearch(event.target.value); setArchive([]); setArchiveOffset(0); setArchiveLoading(false); setArchiveError(''); }} /></label>
        </div>
        {!token && <div className="money-empty"><ShieldCheck size={26} aria-hidden="true" /><h3>Operator access required</h3><p>Enter the operator credential, then load the private research records.</p></div>}
        {archiveError && <div className="money-error" role="alert">{archiveError}</div>}
        {token && archive.length === 0 && !archiveLoading && !archiveError && <div className="money-empty"><Database size={26} aria-hidden="true" /><h3>No historical leads loaded</h3><p>Choose Load private research. Records are never displayed as live funding opportunities.</p></div>}
        {token && archive.map(record => <article className="money-opportunity" key={record.source_file + ':' + record.legacy_id}>
          <div className="money-opportunity-main"><p className="money-agency">{record.category}<span>{record.legacy_id}</span></p>
            <strong className="money-title">{record.name}</strong>
            <div className="money-opportunity-meta"><span>Historical label only: {record.historical_status}</span><span>Archived research</span></div>
          </div>
          <div className="money-evidence"><span className="money-unknown">Current availability UNKNOWN</span><small>Eligibility UNKNOWN</small><small>Source: {record.source_file}</small></div>
        </article>)}
        {token && <div className="money-review-actions">
          <button className="money-button" disabled={archiveLoading || archiveOffset === 0} onClick={() => void loadArchive(Math.max(0, archiveOffset - 100))}>Previous 100</button>
          <span>{archive.length ? `Showing ${archiveOffset + 1}–${archiveOffset + archive.length}` : 'No archived records on this page'}</span>
          <button className="money-button" disabled={archiveLoading || archive.length < 100} onClick={() => void loadArchive(archiveOffset + 100)}>Next 100</button>
        </div>}
      </section>}
      {view === 'activity' && <section className="money-ledger"><div className="money-section-heading"><h2>Public audit events</h2><p>Private review notes are excluded.</p></div>{timeline.length === 0 && <div className="money-empty"><h3>No public events recorded</h3><p>Approved workflow transitions will appear here.</p></div>}{timeline.map(event => <article className="money-event" key={event.item_id}><time>{date(event.occurred_at)}</time><strong>{event.title.replaceAll('_', ' ')}</strong><code>{event.item_id}</code></article>)}</section>}
      <footer className="money-footer">X3 Money Swarm · No automatic submissions or financial commitments · GPU execution is not activated by this workspace</footer>
    </div>
    {selected && <div className="money-detail-backdrop"><section ref={detailsRef} className="money-detail" role="dialog" aria-modal="true" aria-labelledby="money-detail-title"><button className="money-close money-button" aria-label="Close opportunity details" onClick={() => setSelected(null)}><X size={18} /></button><p className="money-eyebrow">OPPORTUNITY EVIDENCE</p><h2 id="money-detail-title">{selected.title}</h2><p>{selected.sponsor}</p><dl><dt>Eligibility</dt><dd>Unknown — verified organizational facts and funder review required.</dd><dt>Requirements from source</dt><dd className="money-source-text">{text(selected.metadata.eligibility_requirements)}</dd><dt>Description</dt><dd className="money-source-text">{text(selected.metadata.description)}</dd><dt>Review stage</dt><dd>{selected.stage}</dd><dt>Source last verified</dt><dd>{date(selected.metadata.last_verified_at)} UTC</dd></dl>{token && (selected.stage === 'research' || selected.stage === 'draft') && <FundingProposalEditor key={selected.grant_id} grantId={selected.grant_id} token={token} />}<div className="money-review-actions">{selected.stage === 'discovery' && <button className="money-button primary" disabled={!token || busy} onClick={() => void advance(selected, 'research')}>Begin research</button>}{selected.stage === 'research' && <button className="money-button primary" disabled={!token || busy} onClick={() => void advance(selected, 'draft')}>Mark drafting stage</button>}{selected.stage === 'draft' && <button className="money-button primary" disabled={!token || busy} onClick={() => void advance(selected, 'approve')}>Approve draft for review</button>}</div><p className="money-limit">Approval changes the internal review record. It does not establish eligibility, submit an application or commit funds.</p></section></div>}
  </main>;
}
