# Funding workspace verification checkpoint

Base: `6c2eec195e2068a49065f60b7dae7e4f2d474f6a`, stacked on PR630.
The original dirty compiler/runtime checkout was preserved; implementation lives
in the isolated `codex/money-swarm-live-workspace-20261018` worktree.

Observed before the final review repairs:

- Official live discovery imported ten Grants.gov opportunities into persistent
  PostgreSQL. These are opportunities, not awards or eligibility determinations.
- Real browser checks compared dashboard cards against API records, searched a
  real title, inspected evidence, exercised Escape/focus restoration and refreshed
  official records with authentication. Desktop/tablet/mobile had no horizontal
  overflow or JavaScript errors. Screenshots and action journal are kept outside
  the repository in `/tmp/x3-money-ui-proof-20261018/`.
- Gateway/indexer all-target clippy passed. It used SKIP_WASM_BUILD and therefore
  does not establish a freshly built production runtime WASM artifact.
- PostgreSQL integration passed after migration0010, including evidence binding,
  idempotency, retained immutable versions and persistence after reconnect.
- Dashboard unit/component suite passed 68 tests across nine files after the
  logout callback fix. A regression first failed when registration occurred
  before the hook mounted; callback registration now survives mounting.

Independent read-only review found two source deadline/availability defects.
Regression and post-fix results are recorded in the final appended checkpoint;
no RUNNING command is treated as PASS. Raw local logs are retained under
`.ai/runlogs/grant-compute-20261018/` (ignored unless explicitly curated).

Broader workspace, public volunteer workloads, hardware donations, research
execution, energy measurement, mainnet and exact-head CI are not established by
these focused checks. No external application, outreach, purchase, contract or
GPU enrollment occurred.

## Review repairs verified

Malformed deadline regression first failed (six passed, one failed); the
normalizer now accepts only valid dates or complete valid Grants.gov timestamps,
and rejects numeric/object/array values and arbitrary suffixes. Missing/null
remains unknown. Final contract suite: seven PASS. Gateway library:55 PASS;
migration table coverage:one PASS. Real PostgreSQL regression first reproduced
expired research counting as open, then separately reproduced expiry without a
source refresh. Final PostgreSQL integration:PASS, with researched stage retained,
three immutable source snapshots/audit events and expired opportunity excluded.
UTC-date filtering handles overnight expiry independently of source refresh.

Full dashboard ESLint:PASS after correcting declaration ordering, state-based
clock updates, stable sort-icon components and early logout registration.
Dashboard production build:PASS. No tests, checks or CI jobs were removed or
suppressed. These focused results do not establish whole-workspace readiness.

The rebuilt gateway restarted against the same persistent pilot volume; readiness
and canonical route checks passed. The final browser run passed again, including
actual authenticated source refresh. All ten persisted records now carry
normalization_version2 and structured agency identities. Final gateway/indexer
clippy and the deployable gateway binary build both exited0. Required control
boundaries were corrected to exceed3:1 contrast; text contrast exceeds9:1.

Review focused on fixed upstream binding, bounded transport, source interpretation,
transactional audit and focused dashboard lifecycle changes. Two deadline defects
were repaired with recorded failing regressions. Important remaining risks include
single-token operator authority, absent end-user RBAC, one upstream connector,
first100-record client filtering, source freshness requiring an operator refresh,
and no complete hardware/research/compute workflow. No production-ready claim.
