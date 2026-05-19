# TODO — X3 Funding Swarm (Grant Hunter OS)

## Step 1 — Locate integration points
- [x] Reviewed existing swarm control-plane scaffold (`x3-swarm-api`).
- [ ] Find Next.js app/router + static vs dynamic site integration.
- [ ] Find/confirm which Rust service serves the site + API base path.

## Step 2 — Backend: Funding Swarm public ledger endpoints
- [ ] Add DB migrations + tables for grant/ledger/event/publication.
- [ ] Add REST routes under existing gateway: `/api/public/funding-swarm/{scoreboard,grants,timeline}`.
- [ ] Implement demo/seed responses so `site/funding-swarm.html` boot() works immediately.

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

