# X3 Atomic Star — Executive Summary

Evidence-based review by Codex / AI-assisted analysis, 2026-09-05.
Base commit `6a24d8cf38f2522ddf9ae0b47011fd59a9984208` plus pre-existing working-tree changes.

**Public testnet: NO-GO. Mainnet: NO-GO. Evidence readiness: 20/100.**

X3 has real Substrate node/runtime architecture and substantial cross-VM/accounting code. However, authenticated external proof verification, finality-anchor provenance, rollback-state integrity and several execution/submission paths are not production-safe. Three Critical findings remain open. The register contains 29 findings (3 Critical, 18 High, 7 Medium, 1 Low).

Observed evidence: four RPC middleware unit tests pass; selected Python DSL tests produce six passes and four failures; three adversarial production proof-router rejection tests fail because invalid payloads are accepted. Workspace check/test/clippy and release/testnet builds fail in the WASM build (E0152 duplicate core). Configured dependency audit suppresses 35 IDs; an unsuppressed offline scan finds 53 advisory/package-version matches, not 53 proven exploitable node flaws.

The 20/100 score is an explicit evidence-criteria score capped for open Critical findings; it is not percent code written or an investment valuation. Two narrowly scoped capabilities are VERIFIED. No live network, contract deployment, finalized end-to-end transfer, restore drill or sustained blockchain TPS was proven.

Fundable milestones: (1) reproducible builds and honest gates, (2) authenticated proof and rollback boundaries, (3) one real signed/finalized native and internal atomic route, (4) four-validator fault/recovery evidence, (5) usable canonical SDK/gateway tooling, and (6) independent audit closure with custody/genesis/upgrade assurance. Resource estimates require staffing and performance baselines; no delivery dates or partnerships are asserted.

Sponsors should request commit-bound logs, reproducible artifacts, canonical independent proof fixtures, cross-node finalized roots, recovery drills and independent closure retests. Do not claim mainnet readiness, trustless external bridges or production TPS from the present evidence. See the full booklet and machine-readable register for exact files, scenarios, acceptance tests and ownership.
