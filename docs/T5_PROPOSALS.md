# T5+ Review-Required Proposals

This document summarizes proposed, non-invasive changes and next steps for the review-required T5+ items imported to the session todos (t5-1..t5-7).

## t5-1 (node/src/service.rs:1355)
- Current: TODO to implement process detection + RPC probe for GPU sidecar health check.
- Proposal: Leave runtime health check invocation as-is; add tracking comment (done). Long-term: implement process liveness checks and an optional RPC probe to the sidecar control endpoint. Create separate PR for wiring orch.trigger_restart to an orchestrator API with retries and backoff.

## t5-2 (node/src/service.rs:1395)
- Current: C-002 comment to replace no-op keep-alive loop with real dispatcher.
- Proposal: Add tracking comment (done). Long-term: implement RuntimeCrossVmDispatcher integration, ensure preflight/postflight safety gates, and add metrics and monotonic backoff for failures.

## t5-3 (runtime/src/precompiles.rs:129)
- Current: (Review required) Precompile setup includes TODO markers — may affect gas accounting or address checks.
- Proposal: Do not change runtime logic without governance/consensus review. Draft concrete changes and tests here for maintainers.

## t5-5..t5-7 (pallets/x3-invariants/src/lib.rs lines ~337..381)
- Current: Invariant-related governance controls and emergency authority APIs. Scanner flagged severity; likely historical panic->error changes.
- Proposal: Create PR with documentation and clarifying comments; add unit tests that exercise invalid constitution hash / authority expiry. Do not enable chain-halting by default.

---

Next steps (recommended):
1. Review these proposals and approve which ones to implement as PRs.
2. For runtime/pallet changes, open RFCs or governance proposals as required by the project.
3. I will now create a temp branch, commit the annotations and proposals doc, and run `cargo check` to ensure edits compile.
