# V4 capability evidence matrix

Audited master: `70423054e2d9560a212ab763d4f265f9cdc448d9`. Source presence
means implementation evidence only. No full workflow is TESTED until an exact-head
end-to-end command has passed. Milestone test results will be recorded separately.

| Capability | Baseline status | Primary source / missing proof |
|---|---|---|
| Funding ledger and public API | BROKEN | gateway `db.rs` / seeded examples and arbitrary stage writes |
| Official grant discovery | NOT IMPLEMENTED | public Grants.gov APIs reachable; canonical repository adapter absent |
| Funding eligibility | NOT IMPLEMENTED | no verified applicant-fact evidence model found |
| Proposal factory | PARTIALLY IMPLEMENTED | gateway draft text/prompt provider; no claim retrieval/versioned package |
| Hardware sourcing | PARTIALLY IMPLEMENTED | desktop hardware schema and constructors; no demonstrated durable source ingestion |
| Sponsor contacts | BROKEN for verified research use | hard-coded unverified contacts/probabilities in hardware_sources_db.rs |
| Hardware inventory | PARTIALLY IMPLEMENTED | cluster scripts/artifacts; live fleet and custody reconciliation absent |
| Compatibility/gap analysis | NOT IMPLEMENTED as integrated workflow | historical hardware lists cannot prove operational capability |
| Energy funding / measured costs | NOT IMPLEMENTED as integrated workflow | rate, load, cooling and utility eligibility need evidence |
| Controlled outreach | PARTIALLY IMPLEMENTED | desktop SMTP/outreach code; no demonstrated V4 approval/suppression path |
| Money Swarm agents | PARTIALLY IMPLEMENTED primitives | swarm authority/permissions exist; bounded durable funding queues absent |
| Volunteer leases | PARTIALLY IMPLEMENTED, UNMERGED | PR #626; #627 CI changes unverified |
| Volunteer enrollment | NOT IMPLEMENTED end to end | no persistent owner-consent/identity/revocation proof |
| Real volunteer GPU execution | BLOCKED for live validation | x3gpu1 network route failed; backend/isolation remain unverified |
| Research receipts and contribution accounting | NOT IMPLEMENTED end to end | atomic receipt primitives require compatible research binding |
| Project funding/compute workflow | NOT IMPLEMENTED | award-to-budget-to-job transaction evidence absent |
| GrantProof / SwarmProof | NOT IMPLEMENTED end to end | proof manifest/export pipeline absent |
| Unified dashboard | NOT IMPLEMENTED | existing React apps require real funding/hardware/research API wiring |
| CI | IMPLEMENTED infrastructure, not all PASS | required x3/local-ci, exact PR head checkout; master run cancelled at audit |
| Reproducible deployment | NOT IMPLEMENTED for V4 | local containers support tests; fleet consent/capacity/connectivity unverified |

This is an explicit gap matrix, not a project-completion estimate.
