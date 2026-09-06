const MONEY_FIELDS = ['requested', 'pledged', 'received', 'allocated', 'spent']
const EVIDENCE_ASSETS = ['readiness.svg', 'subsystems.svg', 'findings.svg', 'tasks.svg', 'checks.svg']

function isAmount(value) {
  return Number.isFinite(value) && value >= 0
}

export function summarizeFunding(workPackages) {
  return workPackages.reduce((totals, item) => {
    for (const field of MONEY_FIELDS) totals[field] += item[field]
    return totals
  }, Object.fromEntries(MONEY_FIELDS.map((field) => [field, 0])))
}

export function validateLedger(ledger) {
  if (!ledger || typeof ledger !== 'object' || !Array.isArray(ledger.workPackages) || !Array.isArray(ledger.proofs)) {
    throw new Error('ledger requires workPackages and proofs arrays')
  }
  const proofIds = new Set()
  for (const proof of ledger.proofs) {
    if (!proof?.id || !proof?.status || !proof?.reference) throw new Error('every proof requires id, status, and reference')
    if (proofIds.has(proof.id)) throw new Error(`duplicate proof id: ${proof.id}`)
    proofIds.add(proof.id)
  }
  const packageIds = new Set()
  for (const item of ledger.workPackages) {
    if (!item?.id || !item?.title || !Array.isArray(item.proofIds) || !item.evidenceStatus) {
      throw new Error('every work package requires id, title, proofIds, and evidenceStatus')
    }
    if (packageIds.has(item.id)) throw new Error(`duplicate work package id: ${item.id}`)
    packageIds.add(item.id)
    for (const field of MONEY_FIELDS) {
      if (!isAmount(item[field])) throw new Error(`${item.id}: ${field} must be a non-negative finite number`)
    }
    if (item.spent > item.allocated || item.allocated > item.received || item.received > item.pledged || item.pledged > item.requested) {
      throw new Error(`${item.id}: funding values must follow requested ≥ pledged ≥ received ≥ allocated ≥ spent`)
    }
    if (item.spent > 0 && item.proofIds.length === 0) throw new Error(`${item.id}: spent amount requires proof`)
    for (const proofId of item.proofIds) {
      if (!proofIds.has(proofId)) throw new Error(`${item.id}: unknown proof id ${proofId}`)
    }
  }
  return ledger
}

function normalizeChecks(checks) {
  if (!Array.isArray(checks)) throw new Error('readiness checks must be an array')
  return checks.map((check) => ({
    id: String(check.id),
    status: String(check.status),
    reason: String(check.reason ?? ''),
    exitCode: check.receipt?.exit_code ?? null,
    evidenceId: check.receipt?.id ?? null,
    log: check.receipt?.log ?? null,
  }))
}

function validateReadiness(readiness) {
  const required = ['readiness_score', 'uncapped_score', 'task_count', 'completed_tasks', 'open_findings', 'release_decision', 'source']
  for (const field of required) if (!(field in readiness)) throw new Error(`readiness missing ${field}`)
  if (!Number.isFinite(readiness.readiness_score) || !Number.isFinite(readiness.uncapped_score)) throw new Error('readiness scores must be finite')
  if (!Number.isInteger(readiness.task_count) || !Number.isInteger(readiness.completed_tasks)) throw new Error('readiness task counts must be integers')
  if (!readiness.source?.fingerprint) throw new Error('readiness source fingerprint is required')
  return readiness
}

export function buildPortalData({ ledger, readiness, snapshotPath }) {
  validateLedger(ledger)
  validateReadiness(readiness)
  if (!/^snapshots\/[A-Za-z0-9-]+$/.test(snapshotPath)) throw new Error('snapshot path is invalid')
  return {
    schemaVersion: 1,
    generatedAt: new Date().toISOString(),
    source: {
      snapshotPath,
      sourceFingerprint: readiness.source.fingerprint,
      sourceCommit: readiness.source.commit ?? null,
      readinessGeneratedAt: readiness.generated_at ?? null,
      evidenceAssets: EVIDENCE_ASSETS.map((name) => `data/evidence-assets/${name}`),
    },
    funding: { currency: ledger.currency, asOf: ledger.asOf, ...summarizeFunding(ledger.workPackages) },
    workPackages: ledger.workPackages,
    proofLedger: ledger.proofs,
    completion: {
      readinessScore: readiness.readiness_score,
      uncappedScore: readiness.uncapped_score,
      taskCount: readiness.task_count,
      completedTasks: readiness.completed_tasks,
      taskProgressPercent: readiness.task_progress_percent ?? 0,
      openFindings: readiness.open_findings,
      releaseDecision: readiness.release_decision,
      checks: normalizeChecks(readiness.checks),
      baselineEligible: Boolean(readiness.baseline_eligible),
    },
  }
}
