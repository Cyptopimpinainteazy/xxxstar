# TODO — X3 Funding Swarm (Grant Hunter OS)

## Execution order (current)
1. Step 2 backend public ledger + seed data.
2. Step 3 admin routes with approval gates.
3. Step 4 compliant swarm state-machine split.
4. Step 5 frontend integration.
5. Step 6 tests.
6. Step 7 run verification.

## Step 1 — Locate integration points
- [x] Reviewed existing swarm control-plane scaffold (`x3-swarm-api`).
- [x] Find Next.js app/router + static vs dynamic site integration.
- [x] Find/confirm which Rust service serves the site + API base path.

### Step 1 findings (verified)
- Next.js app/router exists at `apps/dex` (App Router). No `site/funding-swarm.html` file currently exists.
- Gateway API base path is `/api/v1` in `crates/x3-gateway/src/rest.rs`; `x3-swarm-api` exists as a separate service in `crates/x3-swarm-core/services/x3-swarm-api/src/main.rs`.

## Step 2 — Backend: Funding Swarm public ledger endpoints
- [x] Add DB migrations + tables for grant/ledger/event/publication.
- [x] Add REST routes under existing gateway: `/api/public/funding-swarm/{scoreboard,grants,timeline}`.
- [x] Implement demo/seed responses so `site/funding-swarm.html` boot() works immediately.

## Step 3 — Backend: Admin endpoints (stub + auth placeholder)
- [ ] Add admin routes for discovery/research/draft/approve/submit-award-paid/publication.
- [ ] Enforce manual human approval gates.

## Step 4 — Swarm job flow (compliant)
- [ ] Implement swarm task state machine and audit event split (public vs private).
- [ ] Integrate OpenRouter via AI provider interface (draft-only, no external submissions).

## Step 5 — Frontend
- [ ] Replace `site/funding-swarm.html` static placeholder with real API-connected components (or keep as static + fetch).
- [ ] Add Recharts graphs + funnel once APIs return data.

## Step 6 — Tests + verification
- [ ] Unit tests: scoring/dedupe/public filtering.
- [ ] Integration test: endpoint returns JSON with expected shapes.

## Step 7 — Run verification
- [ ] Start gateway + Postgres; apply migrations; seed demo rows.
- [ ] Smoke test with curl for the three public endpoints.

## Live Context (Auto-synced)
<!-- MARKDOWN-AUTO-DOCS:START (CODE:src=./docs/_autodocs/PENDING_SYNC.md&syntax=md) -->
<!-- The below code snippet is automatically added from ./docs/_autodocs/PENDING_SYNC.md -->
```md
> Auto-doc placeholder: this section is managed by markdown-autodocs.
>
> Next update: replace this shared placeholder with a file-specific snippet source once the related implementation lands.
```
<!-- MARKDOWN-AUTO-DOCS:END -->

