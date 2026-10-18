# Source-bound application packages

This module creates editable private working drafts, not approved applications.
It reuses the gateway funding grant, official source snapshot and operator gate.
It sends no content to AI services, funders or external contacts.

Begin research on a genuine source opportunity. With the operator credential,
open its details and fill Project title, Research question, Technical approach,
Evaluation plan and Organization facts/supporting evidence. Use verified facts
and clearly identify proposed work, unknown identity and unverified capability.
The requested amount is optional and uses USD cents; it does not authorize a
budget. Save creates a new version and a private audit event atomically.

Every package contains its grant ID, version, source snapshot/hash/normalizer
version, source facts, separate operator self-report, UNKNOWN eligibility,
explicit unverified requirements and external_submission=false. Download exports
that exact saved package plus its SHA256 content checksum. The checksum detects
content changes; it is not an attestation, eligibility decision or execution proof.
The history shows the latest25 versions, without silently overwriting older rows.

GET/POST `/api/v1/admin/funding-swarm/grants/{grant_id}/proposals` requires the
funding operator token. Saving requires research/draft stage, expected_version,
current source_snapshot_id and bounded nonempty sections. Unknown fields,
negative/unsafe monetary values, stale versions and stale source bindings reject.
The JSON monetary ceiling matches the maximum exact JavaScript integer, to
preserve cents across the browser/API boundary. Grant row locks serialize saves
and source refreshes. A failed audit insert rolls back the proposed version.
Draft bodies are never added to public funding timeline events.

Current source identity is independent of immutable snapshot creation time: a
revisited earlier source document must become current without rewriting it.
Legacy source rows without a current reference require an official refresh before
proposal preparation. Source review is still a human responsibility; no software
can infer missing organizational eligibility from topical relevance.

Limitations: one shared operator role, no organizational attestation/claim review,
no immutable version approval/signature, no funder-specific form completion,
no submission transport and no confidential-data encryption beyond deployment
storage controls. Internal grant-stage approval does not approve a specific
package hash; do not treat it as authorization to submit. Production private
proposal storage requires managed access, encrypted storage/backups and retention
policy before public deployment.

## Executable verification

Real PostgreSQL integration exercised discovery-stage rejection, foreign source
rejection, immutable version creation, checksum recomputation, conflicting edits
(exactly one winner), stale source rejection, injected audit failure rollback,
restart/reconnect retention and private audit counts. Independent review found
A→B→A source selection; a new regression failed before the explicit-current-pointer
fix and passed afterward. Gateway library55, discovery contracts7, proposal
contract1 and migration coverage1 passed. Dashboard72 tests, lint/build and
gateway/indexer all-target clippy passed. Gateway checks used SKIP_WASM_BUILD;
production runtime WASM and whole-workspace readiness remain unestablished.

The rebuilt loopback pilot refreshed actual Grants.gov records, prepared one
internal working draft on the discovered NSF opportunity (grants.gov:356055),
saved version1, downloaded it through the browser and independently recomputed
its checksum. It labels unimplemented research as PROPOSED WORK and legal/entity
eligibility as UNKNOWN. No budget was requested, no application was submitted,
and no human approval was impersonated. Browser runs had no JavaScript errors
or horizontal overflow at desktop/mobile widths. Evidence logs are under
`.ai/runlogs/proposal-packages-20261018/`; screen captures and the downloaded
working package remain outside the repository in `/tmp/x3-proposal-browser-20261018/`.
