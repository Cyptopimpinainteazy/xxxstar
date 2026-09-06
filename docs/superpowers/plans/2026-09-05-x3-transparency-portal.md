# X3 Transparency Portal Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create a static X3 transparency portal that turns local readiness evidence and a validated treasury ledger into public, honest funding and completion views.

**Architecture:** A Node generator validates the existing readiness snapshot and a hand-authored zero-value treasury ledger before writing a static JSON payload. A React/Vite front end reads only that payload and renders funding, evidence, and completion views with accessible CSS charts. The generator is the policy boundary; the UI has no authority to create financial or verification claims.

**Tech Stack:** React 18, Vite 6, TypeScript, Node built-in test runner, CSS, existing readiness JSON.

**Spec:** `docs/superpowers/specs/2026-09-05-x3-transparency-portal-design.md`

## Global Constraints

- Create `apps/x3-transparency`; do not modify or reuse the existing `apps/x3-funding` presentation or its claims.
- Do not add payment processing, wallet connections, donation forms, fabricated financial values, chain API calls, or a launch-ready claim.
- Treat `audit-artifacts/mainnet-readiness/live/current.json` as a local source only and label its freshness explicitly.
- Do not mark money as spent without a proof record reference.
- Keep all production data validation in `scripts/transparency/generate.mjs`; tests use Node's real filesystem and subprocesses.

---

### Task 1: Add the validated treasury and readiness data boundary

**Files:**
- Create: `apps/x3-transparency/data/treasury-ledger.json`
- Create: `apps/x3-transparency/scripts/transparency-data.mjs`
- Create: `apps/x3-transparency/test/transparency-data.test.mjs`

**Interfaces:**
- Produces: `validateLedger(ledger)`, `buildPortalData({ ledger, readiness, snapshotPath })`, and `summarizeFunding(workPackages)`.
- Consumes: a ledger record `{ id, title, requested, pledged, received, allocated, spent, status, proofIds }` and readiness snapshot fields `readiness_score`, `task_count`, `completed_tasks`, `open_findings`, `checks`.

- [ ] **Step 1: Write failing tests**

```js
assert.throws(() => validateLedger({ workPackages: [{ id: 'wp', requested: 1, pledged: 0, received: 0, allocated: 0, spent: 1, proofIds: [] }] }), /spent amount requires proof/)
assert.equal(buildPortalData({ ledger, readiness, snapshotPath: 'snapshots/current' }).funding.spent, 0)
```

- [ ] **Step 2: Run tests to verify failure**

Run: `node --test apps/x3-transparency/test/transparency-data.test.mjs`
Expected: FAIL because the module does not exist.

- [ ] **Step 3: Implement the minimal validator and derived totals**

```js
export function validateLedger(ledger) {
  for (const item of ledger.workPackages) {
    if (item.spent > 0 && item.proofIds.length === 0) throw new Error(`${item.id}: spent amount requires proof`)
  }
  return ledger
}
```

- [ ] **Step 4: Run tests to verify pass**

Run: `node --test apps/x3-transparency/test/transparency-data.test.mjs`
Expected: PASS.

### Task 2: Generate static public portal data from evidence

**Files:**
- Create: `scripts/transparency/generate.mjs`
- Modify: `apps/x3-transparency/package.json`
- Test: `apps/x3-transparency/test/transparency-data.test.mjs`

**Interfaces:**
- Consumes: `buildPortalData`, `apps/x3-transparency/data/treasury-ledger.json`, and `audit-artifacts/mainnet-readiness/live/current.json`.
- Produces: `apps/x3-transparency/public/data/portal.json`.

- [ ] **Step 1: Write the failing generator contract test**

```js
const result = spawnSync('node', ['scripts/transparency/generate.mjs'], { encoding: 'utf8' })
assert.equal(result.status, 0, result.stderr)
assert.equal(JSON.parse(readFileSync(output)).completion.readinessScore, expectedScore)
```

- [ ] **Step 2: Run test to verify failure**

Run: `node --test apps/x3-transparency/test/transparency-data.test.mjs`
Expected: FAIL because the generator does not exist.

- [ ] **Step 3: Implement a fail-closed generator**

```js
const pointer = JSON.parse(readFileSync(readinessRoot + '/current.json'))
const report = JSON.parse(readFileSync(readinessRoot + '/' + pointer.snapshot + '/summary.json'))
const portal = buildPortalData({ ledger, readiness: report, snapshotPath: pointer.snapshot })
writeFileSync(output, JSON.stringify(portal, null, 2) + '\n')
```

- [ ] **Step 4: Run test and generator**

Run: `node --test apps/x3-transparency/test/transparency-data.test.mjs && node scripts/transparency/generate.mjs`
Expected: PASS and a generated data file.

### Task 3: Build the transparency portal UI

**Files:**
- Create: `apps/x3-transparency/index.html`
- Create: `apps/x3-transparency/src/main.tsx`
- Create: `apps/x3-transparency/src/App.tsx`
- Create: `apps/x3-transparency/src/styles.css`
- Create: `apps/x3-transparency/src/types.ts`
- Modify: `apps/x3-transparency/package.json`
- Modify: `apps/x3-transparency/vite.config.ts`

**Interfaces:**
- Consumes: `PortalData` fetched from `/data/portal.json`.
- Produces: five anchored semantic sections and external artifact links.

- [ ] **Step 1: Write a failing structural test**

```js
assert.match(readFileSync('apps/x3-transparency/src/App.tsx', 'utf8'), /id="funding"/)
assert.match(readFileSync('apps/x3-transparency/src/App.tsx', 'utf8'), /id="completion"/)
```

- [ ] **Step 2: Run test to verify failure**

Run: `node --test apps/x3-transparency/test/transparency-data.test.mjs`
Expected: FAIL because the UI does not exist.

- [ ] **Step 3: Implement the page**

```tsx
<section id="funding" aria-labelledby="funding-title"><h2 id="funding-title">Grant funnel</h2></section>
<section id="completion" aria-labelledby="completion-title"><h2 id="completion-title">Completion evidence</h2></section>
```

- [ ] **Step 4: Build and typecheck**

Run: `npm --prefix apps/x3-transparency run typecheck && npm --prefix apps/x3-transparency run build`
Expected: PASS.

### Task 4: Make evidence accessible and verify generated output

**Files:**
- Create: `apps/x3-transparency/README.md`
- Modify: `apps/x3-transparency/test/transparency-data.test.mjs`
- Modify: `apps/x3-transparency/src/styles.css`

**Interfaces:**
- Consumes: generated static output.
- Produces: documented regeneration path and an artifact link per current evidence record.

- [ ] **Step 1: Write a failing accessibility/output test**

```js
assert.match(html, /Evidence status/)
assert.match(html, /aria-label=/)
assert.match(readFileSync('apps/x3-transparency/public/data/portal.json', 'utf8'), /"source"/)
```

- [ ] **Step 2: Run test to verify failure**

Run: `node --test apps/x3-transparency/test/transparency-data.test.mjs`
Expected: FAIL until the interface and generated source links exist.

- [ ] **Step 3: Add clear labels, source links, and regeneration instructions**

```md
npm run generate:data
npm run typecheck
npm run build
```

- [ ] **Step 4: Run the complete verification set**

Run: `node --test apps/x3-transparency/test/transparency-data.test.mjs && npm --prefix apps/x3-transparency run generate:data && npm --prefix apps/x3-transparency run typecheck && npm --prefix apps/x3-transparency run build`
Expected: PASS.

## Self-review

The tasks cover funding data validation, generated evidence data, the website, and documentation/output validation. No task permits a funding amount to appear as spent without proof. The interfaces use the same function and property names throughout. No placeholder work is left in this plan.
