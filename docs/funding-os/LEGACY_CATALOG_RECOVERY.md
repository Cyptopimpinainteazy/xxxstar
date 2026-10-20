# Funding OS — historical catalog recovery checkpoint

This is a **read-only research-data recovery** checkpoint, **not** a live funding connector or an application pipeline.

## Verified repository findings

- `grants/INDEX.md` reports approximately 1,772 historic leads across 12 files (September 5, 2026). An independent table-ID count on master `70423054e2d9560a212ab763d4f265f9cdc448d9` found **1,617 parseable program IDs**. These counts are not equivalent to 1,617 open grants; the discrepancy needs reconciliation.
- The existing files contain grants, VC prospects, bug bounties, credits, free tiers, paid services and defunct programs. `C25-vc-accelerators.md` explicitly acknowledges shallow verification.
- `apps/x3-funding/` already has funding angles, sponsorship lanes, bounty and service descriptions, and eight historical hardware sponsor routes.
- `apps/x3-desktop/src-tauri/migrations/hardware_acquisition.sql` describes hardware campaigns, sources, equipment, shipments and inventory. The existing V4 audit warns that the desktop hardware CRM contains hardcoded, unverified contacts and success estimates. This is not a live prospect verification source.
- The active funding PR stack is **#630 → #632 → #633**, all draft/unmerged when reviewed. #631 is closed, superseded by #632. #632 contains Grants.gov discovery; it does not discover Web3 funding or browser-navigate general sources.

## Export historical leads without promoting them

From the repository root:

```sh
python3 scripts/funding/legacy_catalog.py \
  --catalog-dir grants \
  --output /tmp/x3-historical-leads.jsonl \
  --summary /tmp/x3-historical-leads-summary.json
python3 -m unittest discover -s tests/funding -v
```

Every exported row carries legacy ID, source file and line, original fields, referenced URLs, archive status and claimed historical verification date. **Current availability and current eligibility are always UNKNOWN**; applications remain unauthorized. The exporter makes **no network requests, database mutations or outbound communications**. A historical QUALIFIED/AWARDED label is not evidence of X3 eligibility or an actual current award.

This CLI is deliberately separate from the persistent PostgreSQL system in the funding PR stack. Do not import into public live opportunity or financial tables until the reviewed staging importer, current source verification, funding-kind separation and deduplication are implemented and tested.

## Required follow-up work

1. Reconcile the source-file count discrepancy and flag overlapping/duplicate providers without altering archived records.
2. Add an explicitly reviewed staging importer on top of the #630/#632/#633 migrations. Preserve immutable official snapshots, source lineage and UNKNOWN eligibility. Do not count historical records as live opportunities.
3. Add **official Web3/governance** discovery for Polkadot OpenGov, Solana, Rootstock, Stellar, Arbitrum and applicable public-goods programs. Web3 Foundation's former general Grants Program is recorded as discontinued; do not draft against that closed intake.
4. Recover hardware sponsorship and acquisition workflows with verified program URLs, actual equipment compatibility, power/shipping costs and secure inventory intake. Quarantine hardcoded contacts and unsupported donor/revenue claims.
5. Recover advanced hardware R&D (FPGA/CXL/DPU, X3A-P1) as *proposed research*, never as deployed hardware, and connect it to eligibility-checked technical funding tracks.
6. Reuse existing sponsor pages and funding product descriptions, but audit any readiness percentages, acceptance claims, grant values and tax-deductible donation representations before publication.

## Verification and limits

The offline exporter passed four fixture-based standard-library unit tests and Python compilation locally. The complete real Markdown corpus was counted separately through GitHub file inspection; a full local corpus execution of this new CLI, PostgreSQL integration, live Web3 source checks and exact-head CI are **not yet verified**. No external application, outreach, donation or purchase took place.
