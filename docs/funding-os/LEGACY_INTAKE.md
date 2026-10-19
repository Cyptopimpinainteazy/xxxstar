# X3 Funding OS — private historical research intake

Status: implementation on a stacked review branch over PR #633. This is a private research-stage integration, not live Web3 grant discovery, proof of eligibility or financial reporting.

## The single-system data path

1. The historical Markdown research collection under grants/ is exported by scripts/funding/legacy_catalog.py in PR #636 into an explicitly nonlive JSONL research feed.
2. scripts/funding/stage_legacy_catalog.py previews the research feed without network access unless --execute --confirm-stage is explicitly supplied.
3. X3 gateway POST /api/v1/admin/funding-swarm/legacy-leads accepts at most 32 leads in a single authenticated request.
4. Mirrored migration 0012 stores original documents and SHA-256 digests in the private funding_legacy_leads staging table.
5. X3 gateway GET /api/v1/admin/funding-swarm/legacy-leads provides the existing operator credential holder a private, paginated review queue.
6. A later review/official-source verification stage may convert an eligible lead into a real canonical funding opportunity; this is NOT implemented here.

## One backend, no competing financial ledgers

Historical leads never enter funding_swarm_grants, funding_source_snapshots, funding_proposal_versions or public funding totals through this import. They cannot claim current availability, current eligibility, funded awards or payments.
Original source file/line, legacy ID, descriptive fields, candidate URLs, archival classification and historical timestamp are retained. Conflicting duplicates roll back the entire request rather than silently overwriting provenance. A private event records staging counts.
Candidate URLs are stored as inert data. This stage makes no requests to upstream funders, does not crawl portals, never sends outreach, and does not access GPU hardware.

## Quick usage

First obtain the archived export using the script on PR #636. Keep the generated JSONL in a protected local working directory.

    python3 scripts/funding/legacy_catalog.py --catalog-dir grants --output /tmp/x3-archived-leads.jsonl --summary /tmp/x3-archived-summary.json
    python3 scripts/funding/stage_legacy_catalog.py --input /tmp/x3-archived-leads.jsonl

After the gateway has migration 0012 deployed and an operator has enabled the existing admin credential, explicitly stage privately:

    python3 scripts/funding/stage_legacy_catalog.py --input /tmp/x3-archived-leads.jsonl --endpoint http://127.0.0.1:3000/api/v1/admin/funding-swarm/legacy-leads --execute --confirm-stage

The credential is read from FUNDING_SWARM_ADMIN_TOKEN environment variable; never put tokens into command arguments, Git, screenshots, logs or shared documentation. Remote endpoints require HTTPS; HTTP is limited to loopback.

## Tests that must pass before merge

    python3 -m unittest discover -s tests/funding -p 'test_stage_legacy_catalog.py' -v
    python3 -m py_compile scripts/funding/stage_legacy_catalog.py
    cargo fmt --all -- --check
    SKIP_WASM_BUILD=1 cargo test -p x3-gateway --lib funding_legacy
    SKIP_WASM_BUILD=1 cargo test -p x3-gateway --test migrations_cover_tables
    # Then run disposable PostgreSQL integration: scripts/gateway-postgres-integration.sh

The PostgreSQL regression test covers replay, tamper rejection, forged OPEN status rejection and no effect on public funding or award totals. The tests are written but not independently verified until executed on the exact PR head.

## Follow-up implementation order

1. Reconcile historical count and classification gaps between 1,617 parsed IDs and approximately 1,772 editorial leads.
2. Add live official-source verification and source-bound promotion from this staging table to the existing grants/official-snapshot ledger.
3. Implement current Polkadot OpenGov, Solana, Rootstock, Stellar, Arbitrum and public-goods discovery adapters.
4. Add explicit cash, crypto treasury, hardware donation, hardware loan, cloud credits, investment, bounty and paid-service types with correct unit accounting.
5. Add verified hardware sponsorship sources and asset intake using the existing hardware acquisition domain (quarantining hardcoded contacts).
6. Integrate verified volunteer compute receipts as project evidence via Inferstructor; no automatic enrollment or unsafe remote shell execution.
7. Extend the current Money Swarm operator dashboard and the approved public x3-funding presentation; do not build another grant database or dashboard.

Security and deployment prerequisites: preserve #630 → #632 → #633 review ordering; never merge until exact-head CI and security gates pass. No auto-submissions, auto-outreach, spending, fictional awards, public participant enrollment or unreviewed legal/tax representations.
