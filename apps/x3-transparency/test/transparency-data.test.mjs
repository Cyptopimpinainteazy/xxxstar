import assert from 'node:assert/strict'
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { spawnSync } from 'node:child_process'
import test from 'node:test'

import { buildPortalData, summarizeFunding, validateLedger } from '../scripts/transparency-data.mjs'
import { toDisplayModel } from '../src/portal-model.mjs'
import { generate } from '../../../scripts/transparency/generate.mjs'

const readiness = {
  generated_at: '2026-09-06T00:00:00.000Z',
  source: { fingerprint: 'f'.repeat(64), commit: 'test-commit' },
  readiness_score: 20,
  uncapped_score: 24.29,
  task_count: 29,
  completed_tasks: 0,
  task_progress_percent: 0,
  open_findings: { Critical: 3, High: 18, Medium: 7, Low: 1 },
  release_decision: 'NO-GO',
  checks: [{ id: 'rpc-unit', status: 'passed', reason: 'Fresh execution', receipt: { id: 'receipt-1', log: 'evidence/receipt-1.log', exit_code: 0 } }],
}

const ledger = {
  currency: 'USD',
  asOf: '2026-09-06',
  workPackages: [{
    id: 'build-integrity', title: 'Build integrity', requested: 120000,
    pledged: 0, received: 0, allocated: 0, spent: 0,
    status: 'seeking_funding', proofIds: [], evidenceStatus: 'unverified',
  }],
  proofs: [],
}

test('rejects a spent amount without a proof record', () => {
  const invalid = structuredClone(ledger)
  invalid.workPackages[0].requested = 1
  invalid.workPackages[0].pledged = 1
  invalid.workPackages[0].received = 1
  invalid.workPackages[0].allocated = 1
  invalid.workPackages[0].spent = 1
  assert.throws(() => validateLedger(invalid), /spent amount requires proof/i)
})

test('derives funnel totals from ledger records without inventing receipts', () => {
  assert.deepEqual(summarizeFunding(ledger.workPackages), {
    requested: 120000, pledged: 0, received: 0, allocated: 0, spent: 0,
  })
})

test('exposes completion evidence with its current source reference', () => {
  const portal = buildPortalData({ ledger, readiness, snapshotPath: 'snapshots/current' })
  assert.equal(portal.completion.readinessScore, 20)
  assert.equal(portal.funding.spent, 0)
  assert.equal(portal.source.snapshotPath, 'snapshots/current')
  assert.deepEqual(portal.source.evidenceAssets, [
    'data/evidence-assets/readiness.svg',
    'data/evidence-assets/subsystems.svg',
    'data/evidence-assets/findings.svg',
    'data/evidence-assets/tasks.svg',
    'data/evidence-assets/checks.svg',
  ])
  assert.equal(portal.proofLedger.length, 0)
})

test('keeps unverified zero-value funding separate from evidence-backed spending', () => {
  const portal = buildPortalData({ ledger, readiness, snapshotPath: 'snapshots/current' })
  const model = toDisplayModel(portal)
  assert.equal(model.funnel.find((stage) => stage.id === 'spent').value, 0)
  assert.equal(model.funnel.find((stage) => stage.id === 'spent').label, 'Spent with proof')
  assert.equal(model.fundingDisclosure, 'No funding, allocation, spending, or payment receipts are recorded in this portal yet.')
  assert.equal(model.completion.releaseDecision, 'NO-GO')
})

test('generator emits current readiness data from a supplied evidence root', async () => {
  const root = await mkdtemp(join(tmpdir(), 'x3-transparency-'))
  const readinessRoot = join(root, 'readiness')
  const output = join(root, 'portal.json')
  try {
    const snapshot = 'snapshots/current'
    await writeFile(join(root, 'ledger.json'), JSON.stringify(ledger))
    await mkdir(join(readinessRoot, snapshot, 'assets'), { recursive: true })
    await writeFile(join(readinessRoot, 'current.json'), JSON.stringify({ snapshot }))
    await writeFile(join(readinessRoot, snapshot, 'summary.json'), JSON.stringify(readiness))
    await Promise.all(['readiness.svg', 'subsystems.svg', 'findings.svg', 'tasks.svg', 'checks.svg'].map((name) => writeFile(join(readinessRoot, snapshot, 'assets', name), `<svg id="${name}" />`)))
    const result = spawnSync('node', ['scripts/transparency/generate.mjs', '--ledger', join(root, 'ledger.json'), '--readiness-root', readinessRoot, '--output', output], {
      cwd: resolve(import.meta.dirname, '../../..'), encoding: 'utf8',
    })
    assert.equal(result.status, 0, result.stderr)
    assert.equal(JSON.parse(await readFile(output, 'utf8')).completion.readinessScore, 20)
    assert.match(await readFile(join(root, 'evidence-assets', 'readiness.svg'), 'utf8'), /readiness\.svg/)
  } finally {
    await rm(root, { recursive: true, force: true })
  }
})

test('generator refuses a malformed readiness pointer', async () => {
  const root = await mkdtemp(join(tmpdir(), 'x3-transparency-'))
  try {
    await writeFile(join(root, 'ledger.json'), JSON.stringify(ledger))
    await writeFile(join(root, 'current.json'), JSON.stringify({ snapshot: '../../outside' }))
    await assert.rejects(
      generate({ ledgerPath: join(root, 'ledger.json'), readinessRoot: root, outputPath: join(root, 'portal.json') }),
      /snapshot/i,
    )
  } finally {
    await rm(root, { recursive: true, force: true })
  }
})
