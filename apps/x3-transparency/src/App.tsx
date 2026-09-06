import { useEffect, useMemo, useState } from 'react'
import { toDisplayModel } from './portal-model.mjs'
import type { PortalData } from './types'

const currency = new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD', maximumFractionDigits: 0 })

function statusLabel(status: string) {
  return status.split('_').join(' ')
}

function Bar({ value, max, label }: { value: number; max: number; label: string }) {
  const width = max > 0 ? Math.max(0, Math.min(100, (value / max) * 100)) : 0
  return <div className="bar" aria-label={`${label}: ${value}`}><span style={{ width: `${width}%` }} /></div>
}

function DataState({ children }: { children: React.ReactNode }) {
  return <main className="page state">{children}</main>
}

export default function App() {
  const [data, setData] = useState<PortalData | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let active = true
    const load = () => fetch('./data/portal.json', { cache: 'no-store' })
      .then(async (response) => {
        if (!response.ok) throw new Error(`Data request failed with ${response.status}`)
        return response.json() as Promise<PortalData>
      })
      .then((portal) => { if (active) { setData(portal); setError(null) } })
      .catch((reason: unknown) => { if (active && !data) setError(reason instanceof Error ? reason.message : 'Unable to load portal data') })
    load()
    const refresh = window.setInterval(load, 30_000)
    return () => { active = false; window.clearInterval(refresh) }
  }, [])

  const model = useMemo(() => data ? toDisplayModel(data) : null, [data])
  if (error) return <DataState><h1>Evidence data unavailable</h1><p>{error}</p><p>Run <code>npm run generate:data</code> from this application directory, then refresh.</p></DataState>
  if (!data || !model) return <DataState><p className="eyebrow">X3 / TRANSPARENCY PORTAL</p><h1>Loading current evidence…</h1></DataState>

  const maxFunding = Math.max(...model.funnel.map((stage) => stage.value), 1)
  const currentSnapshot = `../../audit-artifacts/mainnet-readiness/live/${data.source.snapshotPath}`
  const evidenceLabels: Record<string, string> = {
    'readiness.svg': 'Readiness score',
    'subsystems.svg': 'Subsystem coverage',
    'findings.svg': 'Open findings',
    'tasks.svg': 'Task completion',
    'checks.svg': 'Verification checks',
  }

  return <main className="page">
    <header className="hero" id="overview">
      <nav aria-label="Primary navigation">
        <a className="brand" href="#overview">X3 <span>Transparency</span></a>
        <div className="nav-links"><a href="#funding">Funding</a><a href="#proofs">Proof ledger</a><a href="#completion">Completion</a><a href="#disclosure">Disclosure</a></div>
      </nav>
      <div className="hero-grid">
        <div>
          <p className="eyebrow">Evidence, funding, and completion</p>
          <h1>Show the work.<br /><em>Prove the path.</em></h1>
          <p className="lede">A public record of what X3 has verified, what still needs work, and where documented funding is assigned. This portal does not accept payments or infer funds from promises.</p>
          <div className="hero-actions"><a className="button primary" href="#completion">Review completion evidence</a><a className="button secondary" href="#funding">View grant funnel</a></div>
        </div>
        <aside className="readiness-card" aria-labelledby="readiness-heading">
          <p className="card-label" id="readiness-heading">Current evidence readiness</p>
          <p className="score">{model.completion.readinessScore}<span>/100</span></p>
          <p className={`decision ${model.completion.releaseDecision === 'NO-GO' ? 'no-go' : ''}`}>{model.completion.releaseDecision}</p>
          <dl><div><dt>Completed tasks</dt><dd>{model.completion.completedTasks}/{model.completion.taskCount}</dd></div><div><dt>Evidence source</dt><dd>{model.completion.checks.length} recorded checks</dd></div></dl>
        </aside>
      </div>
      <p className="freshness">Generated {new Date(data.generatedAt).toLocaleString()} · Snapshot <code>{data.source.snapshotPath}</code></p>
    </header>

    <section className="section" id="funding" aria-labelledby="funding-title">
      <div className="section-heading"><div><p className="eyebrow">Grant funnel</p><h2 id="funding-title">Follow every recorded dollar</h2></div><p>As of {data.funding.asOf}</p></div>
      <div className="disclosure" role="note"><strong>Current record:</strong> {model.fundingDisclosure}</div>
      <div className="funnel" aria-label="Funding funnel">
        {model.funnel.map((stage) => <article className="funnel-stage" key={stage.id}><div className="stage-top"><h3>{stage.label}</h3><strong>{stage.formatted}</strong></div><Bar label={stage.label} value={stage.value} max={maxFunding} /><p>{stage.detail}</p></article>)}
      </div>
    </section>

    <section className="section work-section" aria-labelledby="work-title">
      <div className="section-heading"><div><p className="eyebrow">Grant work packages</p><h2 id="work-title">What funding is intended to unlock</h2></div><p>Amounts remain unpriced until a reviewed budget is published.</p></div>
      <div className="work-grid">
        {data.workPackages.map((work) => <article className="work-card" key={work.id}><div className="work-meta"><span>{statusLabel(work.status)}</span><span>{work.evidenceStatus}</span></div><h3>{work.title}</h3><p>{work.description}</p><dl><div><dt>Linked audit findings</dt><dd>{work.linkedFindings.join(', ')}</dd></div><div><dt>Recorded spending</dt><dd>{currency.format(work.spent)}</dd></div><div><dt>Linked proofs</dt><dd>{work.proofIds.length || 'None recorded'}</dd></div></dl></article>)}
      </div>
    </section>

    <section className="section proof-section" id="proofs" aria-labelledby="proof-title">
      <div className="section-heading"><div><p className="eyebrow">Proof ledger</p><h2 id="proof-title">Evidence is a record, not a promise</h2></div><p>Records must include a public reference and status.</p></div>
      {data.proofLedger.length === 0 ? <div className="empty-proof"><strong>No funding proofs published yet.</strong><p>When a receipt, transaction reference, deliverable, or review is recorded, it will appear here with its source and status.</p></div> : <div className="proof-table-wrap"><table><thead><tr><th>Proof</th><th>Status</th><th>Reference</th><th>Reviewer</th></tr></thead><tbody>{data.proofLedger.map((proof) => <tr key={proof.id}><td>{proof.description ?? proof.id}</td><td>{proof.status}</td><td><a href={proof.reference}>{proof.reference}</a></td><td>{proof.reviewer ?? 'Not recorded'}</td></tr>)}</tbody></table></div>}
    </section>

    <section className="section completion-section" id="completion" aria-labelledby="completion-title">
      <div className="section-heading"><div><p className="eyebrow">Completion dashboard</p><h2 id="completion-title">Proof-backed protocol progress</h2></div><a href={`${currentSnapshot}/X3-LIVE-READINESS.pdf`}>Open current evidence PDF</a></div>
      <div className="completion-grid">
        <article className="metric-panel"><p>Evidence readiness</p><strong>{model.completion.readinessScore}<span>/100</span></strong><Bar label="Evidence readiness" value={model.completion.readinessScore} max={100} /><small>Uncapped score: {data.completion.uncappedScore}/100. Open Critical findings cap readiness.</small></article>
        <article className="metric-panel"><p>Task completion</p><strong>{model.completion.completedTasks}<span>/{model.completion.taskCount}</span></strong><Bar label="Task completion" value={model.completion.completedTasks} max={model.completion.taskCount} /><small>{data.completion.taskProgressPercent}% verified completion.</small></article>
        <article className="metric-panel"><p>Evidence freshness</p><strong>{data.completion.baselineEligible ? 'Current' : 'Stale'}</strong><small>{data.completion.baselineEligible ? 'Historical baseline is eligible on the current source fingerprint.' : 'Historical baseline does not currently earn evidence credit.'}</small></article>
      </div>
      <div className="findings-grid" aria-label="Open findings by severity">{model.completion.openFindings.map(([severity, count]) => <div key={severity}><span className={`severity ${severity.toLowerCase()}`}>{severity}</span><strong>{count}</strong><small>open findings</small></div>)}</div>
      <div className="evidence-charts" aria-label="Generated evidence charts">
        {data.source.evidenceAssets.map((asset) => {
          const fileName = asset.split('/').pop() ?? asset
          return <figure key={asset}><img src={asset} alt={`${evidenceLabels[fileName] ?? 'Evidence'} chart generated from the current readiness snapshot`} /><figcaption>{evidenceLabels[fileName] ?? 'Evidence chart'}</figcaption></figure>
        })}
      </div>
      <div className="check-list"><h3>Latest verification checks</h3>{model.completion.checks.map((check: PortalData['completion']['checks'][number]) => <article key={check.id}><div><strong>{check.id}</strong><span className={`check-status ${check.status}`}>{check.status}</span></div><p>{check.reason}</p><small>{check.exitCode === null ? 'No exit code recorded' : `Exit code ${check.exitCode}`}{check.log ? ` · ${check.log}` : ''}</small></article>)}</div>
    </section>

    <section className="section disclosure-section" id="disclosure" aria-labelledby="disclosure-title">
      <p className="eyebrow">Method and disclosure</p><h2 id="disclosure-title">Read the numbers in context</h2>
      <div className="disclosure-grid"><p><strong>Funding:</strong> Ledger totals are source data. Pledged, received, allocated, and spent are separate states. Spending is rejected by the generator without a linked proof record.</p><p><strong>Completion:</strong> The score is evidence coverage, not code volume, investment value, or a guarantee of safety. Failed, stale, or unreviewed checks do not count as completion proof.</p><p><strong>Scope:</strong> This static portal consumes local repository evidence. It is not an on-chain treasury, an accounting system, an independent audit, or an offer to sell securities.</p></div>
      <p className="source-note">Source fingerprint <code>{data.source.sourceFingerprint}</code> · Commit <code>{data.source.sourceCommit ?? 'not recorded'}</code></p>
    </section>
    <footer><span>X3 Transparency Portal</span><span>Static evidence site · No payments accepted</span></footer>
  </main>
}
