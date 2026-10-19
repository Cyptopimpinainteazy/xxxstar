# X3 Funding OS — official Web3 portal monitor

**Scope:** source evidence for two fixed public programs; NOT a general browser crawler, a confirmed open grant feed, a funding award, a confirmed eligibility assessment, a proposal submission or hardware/volunteer GPU execution.

## First verified official program pages

| Program | Official page | Evidence classification |
|---|---|---|
| Solana Foundation grants | https://solana.org/grants-funding | Rolling-application guidance, if actually present on retrieved page; funder status/eligibility UNKNOWN |
| Stellar Community Fund Build Awards | https://communityfund.stellar.org/awards | If page confirms a round, records its label and deadline; interest/applicant eligibility UNKNOWN |

These URLs were independently researched against official provider pages before this implementation. The program definitions are manually allowlisted in `crates/x3-gateway/src/funding_web3.rs` and are not user-supplied URLs. Do not add Rootstock/Polkadot/Arbitrum without verifying exact official page, program type and terms.

## Private gateway endpoints

- `POST /api/v1/admin/funding-swarm/discovery/web3` body `{"provider":"stellar"}` or `{"provider":"solana"}`. Requires `X-Admin-Token` **and** server-side `X3_FUNDING_WEB3_DISCOVERY_ENABLED=1`. The operator must first review the relevant site access policy and rate limits.
- `GET /api/v1/admin/funding-swarm/discovery/web3/sources` lists the latest persisted official program-source snapshot per provider, without exposing the raw HTML document.

Fetching uses HTTPS only, no cross-domain redirects, a single global in-process refresh lock, a 20-second timeout, maximum 1 MiB source document, static pages, and content-type validation. It does not follow links, submit forms, bypass access restrictions, download attachments, or accept arbitrary URLs.

## Evidence and data model

Migration `0013_funding_web3_source_snapshots.sql` is mirrored in gateway and indexer migration directories. Its private table stores the exact HTML document, SHA-256 digest, first observed timestamp, last observed timestamp, normalized guidance and official source URL. Duplicate bodies retain their original snapshot ID and update the last observation timestamp. Source identity has a foreign key to the existing `funding_sources` table. A new source fingerprint produces one private audit event.

The source parser strips scripts and styles, verifies the program's expected official heading/phrases, and extracts only supported public guidance. Missing or changed layout means UNKNOWN; no invented deadline, award, live-availability or organization eligibility.

Crucially, these observations **never enter** `funding_swarm_grants`, public scoreboards, applications, financial events or equipment records. Source-page observation is not an official approval or submitted application. The dashboard displays the records in a credential-gated **Official Web3 portals** view inside the existing Money Swarm workspace.

## Tests and known limitations

Test fixtures cover allowlisted providers, source identity checks, HTML script stripping, Stellar round deadline extraction, expired/stale dates and unsupported pages. The disposable-PostgreSQL regression checks idempotence and that grant count and awarded USD totals remain unchanged. Dashboard tests cover credential gating, source display and rejection of forged eligibility.

Run before merge:

```bash
cargo fmt --all -- --check
SKIP_WASM_BUILD=1 cargo test -p x3-gateway --lib funding_web3
SKIP_WASM_BUILD=1 cargo test -p x3-gateway --test migrations_cover_tables
python3 -m unittest discover -s tests/funding -p 'test_stage_legacy_catalog.py' -v
# Also run the existing disposable PostgreSQL integration gate:
scripts/gateway-postgres-integration.sh
cd apps/inferstructor-dashboard
npm ci && npm test -- --run MoneySwarmWorkspace.test.tsx && npm run build
```

Live production source-page smoke tests were not performed by writing this code. Some sites may render incomplete HTML, block automated access, redirect to another same-site path or exceed the size budget; these conditions must fail safely and be recorded as blockers rather than reported as working live discovery.

## Verified hardware-credit eligibility warning

The official NVIDIA Inception page at https://www.nvidia.com/en-us/startups/ describes partner cloud credits and preferred hardware pricing, **not an unconditional equipment donation or cash grant**. The same published FAQ excludes companies associated with cryptocurrency and requires an incorporated applicant. Until a qualified legal applicant and any exception are independently confirmed, **do not mark X3 eligible or generate an approved application** for Inception. This should be a negative eligibility test when typed hardware/credit discovery is implemented; do not rely on historical sponsor CRM success percentages.

## Next milestones

1. Verify exact-head targeted CI and disposable PostgreSQL tests.
2. Perform a controlled manual source refresh on an authorized gateway, preserving the real fetch result and timestamp.
3. Add official Rootstock funding, Polkadot OpenGov treasury, Arbitrum and public-goods adapters where verified.
4. Add a separate reviewed promotion path from an observed official program to a typed, source-snapshot-bound opportunity. Do NOT automatically convert pages into eligible grants.
5. Add hardware equipment-donation/credit discovery to the same funding-source registry, with distinct in-kind accounting and verified physical inventory.
6. Bind any future proposal packages to exact official opportunity or round rules, subject to human approval. No external submission/outreach without authorization.
