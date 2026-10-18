import { useEffect, useRef, useState, type FormEvent } from 'react';

interface Draft {
  project_title: string; research_question: string; technical_approach: string;
  evaluation_plan: string; organization_facts: string; requested_usd_cents: number | null;
}
interface Version {
  version: number; source_snapshot_id: string; package_sha256: string;
  package: { operator_self_report: Draft; [key: string]: unknown };
  created_at: string;
}
interface Context {
  source: { snapshot_id: string; content_sha256: string; official_url: string };
  versions: Version[]; history_limit: number;
}
const empty: Draft = { project_title: '', research_question: '', technical_approach: '', evaluation_plan: '', organization_facts: '', requested_usd_cents: null };
const gateway = (import.meta.env.VITE_X3_GATEWAY_URL || '/money-api').replace(/\/$/, '');
const fields = [
  ['project_title', 'Project title'], ['research_question', 'Research question'],
  ['technical_approach', 'Technical approach'], ['evaluation_plan', 'Evaluation plan'],
  ['organization_facts', 'Organization facts and supporting evidence'],
] as const;

export function FundingProposalEditor({ grantId, token }: { grantId: string; token: string }) {
  const requests = useRef<AbortController | null>(null);
  const [loadedCredential, setLoadedCredential] = useState('');
  const [loadedPath, setLoadedPath] = useState('');
  const [context, setContext] = useState<Context | null>(null);
  const [draft, setDraft] = useState<Draft>(empty);
  const [cents, setCents] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');
  const path = `${gateway}/api/v1/admin/funding-swarm/grants/${encodeURIComponent(grantId)}/proposals`;

  useEffect(() => {
    const controller = new AbortController();
    requests.current = controller;
    setContext(null); setDraft(empty); setCents(''); setNotice(''); setError(''); setBusy(true);
    void (async () => {
      try {
        const response = await fetch(path, { headers: { 'x-admin-token': token }, signal: controller.signal });
        if (!response.ok) throw new Error(`Proposal evidence unavailable (HTTP ${response.status}). Check your credential and the official source record.`);
        const result: Context = await response.json();
        if (controller.signal.aborted) return;
        if (!result.source?.snapshot_id || !Array.isArray(result.versions)) throw new Error('Invalid proposal evidence response.');
        const previous = result.versions[0]?.package.operator_self_report;
        setLoadedCredential(token); setLoadedPath(path); setContext(result); setDraft(previous || empty);
        setCents(previous?.requested_usd_cents == null ? '' : String(previous.requested_usd_cents));
      } catch (failure) {
        if (!controller.signal.aborted) setError(failure instanceof Error ? failure.message : 'Proposal evidence could not be loaded.');
      } finally { if (!controller.signal.aborted) setBusy(false); }
    })();
    return () => controller.abort();
  }, [path, token]);

  async function save(event: FormEvent) {
    event.preventDefault();
    const controller = requests.current;
    if (!context || loadedCredential !== token || loadedPath !== path || busy || !controller || controller.signal.aborted) return;
    if (cents && (!/^\d+$/.test(cents) || !Number.isSafeInteger(Number(cents)))) {
      setError('Requested amount must be a nonnegative safe integer in USD cents.'); return;
    }
    setBusy(true); setError(''); setNotice('');
    try {
      const response = await fetch(path, { method: 'POST', signal: controller.signal, headers: { 'Content-Type': 'application/json', 'x-admin-token': token },
        body: JSON.stringify({ ...draft, requested_usd_cents: cents === '' ? null : Number(cents),
          expected_version: context.versions[0]?.version || 0, source_snapshot_id: context.source.snapshot_id }) });
      if (!response.ok) throw new Error(`Version was not saved (HTTP ${response.status}). Reload the proposal before retrying; source or draft evidence may have changed.`);
      const version: Version = await response.json();
      if (controller.signal.aborted) return;
      setContext({ ...context, versions: [version, ...context.versions].slice(0, context.history_limit) });
      setNotice(`Version ${version.version} saved. No application was submitted.`);
    } catch (failure) { if (!controller.signal.aborted) setError(failure instanceof Error ? failure.message : 'Version was not saved.'); }
    finally { if (!controller.signal.aborted) setBusy(false); }
  }

  function download(version: Version) {
    const blob = new Blob([JSON.stringify({ package: version.package, package_sha256: version.package_sha256 }, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement('a'); anchor.href = url;
    anchor.download = `x3-proposal-${grantId}-v${version.version}.json`;
    anchor.click(); setTimeout(() => URL.revokeObjectURL(url), 1000);
  }

  return <section className="money-proposal" aria-label="Application package editor">
    <h3>Application package</h3>
    <p>Working research content and organization self-reports. Eligibility, budget approval and technical claims require separate verification.</p>
    {error && <p role="alert" className="money-error">{error}</p>}
    {notice && <p role="status" className="money-notice">{notice}</p>}
    {busy && !context && <p>Loading private proposal evidence…</p>}
    {context && loadedCredential === token && loadedPath === path && <>
      <p className="money-limit">Source snapshot <code>{context.source.snapshot_id}</code>. Saving binds this exact source evidence; a changed source requires reloading.</p>
      <form onSubmit={event => void save(event)}>
        {fields.map(([key, label]) => <label key={key}>{label}<textarea required maxLength={key === 'project_title' ? 200 : 16384} value={draft[key]} onChange={event => setDraft({ ...draft, [key]: event.target.value })} /></label>)}
        <label>Requested amount in USD cents (blank means unknown)<input inputMode="numeric" value={cents} onChange={event => setCents(event.target.value)} /></label>
        <button type="submit" className="money-button primary" disabled={busy}>{busy ? 'Saving…' : 'Save new version'}</button>
      </form>
      <h4>Saved versions</h4>
      {context.versions.length === 0 ? <p>No versions saved.</p> : <ol>{context.versions.map(version => <li key={version.version}><strong>Version {version.version}</strong><code>{version.package_sha256}</code><button className="money-button" onClick={() => download(version)}>Download version {version.version}</button></li>)}</ol>}
      <p className="money-limit">Latest {context.history_limit} versions shown. Downloads include a content checksum, which does not attest eligibility or execution authenticity.</p>
    </>}
  </section>;
}
