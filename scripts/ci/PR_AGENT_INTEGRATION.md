# PR-Agent + CI Autopilot: integration contract

Status: proposed architecture, not a deployed or production-admitted feature.
Owner: X3 engineering. Initial tracking branch: PR #683. Existing entry points:
`services/pr-agent/`, `.github/workflows/pr-agent-auto.yml`,
`scripts/ci/autopilot_triage.py`, `scripts/ci/autopilot_capacity.py`,
`scripts/x3-cluster/x3jobs.py`, and `scripts/local-ci.sh`.

## Decision

PR-Agent remains the **advisory code-review interface**; CI Autopilot supplies
observed job/test evidence and safe work placement. Neither can independently
approve releases, merge, bypass required checks, alter production runners or
declare consensus readiness. Preserve GitHub as the authoritative source of
required checks and exact-head status.

**First priority:** evidence-backed reviews. Every serious finding must link to
an exact file and line at the reviewed head, distinguish a hypothesis from a
reproduced defect, describe the invariant at risk, and attach either a
reproducer/test result or an explicit `UNVERIFIED` marker. An AI all-clear is
never a substitute for required checks.

## Data flow

1. Receive an event for a specific repository, PR number and immutable head SHA.
2. Fetch PR diff/metadata using read-only GitHub API access. Never checkout
   untrusted PR code on a privileged self-hosted runner.
3. Run existing deterministic static tests and PR-Agent review independently.
4. Ingest GitHub Actions job metadata, observed conclusions and bounded
   redacted logs. A queued job stays `NOT_FINISHED`; an unknown failure stays
   `UNKNOWN`, never `PASS`.
5. Join findings and test evidence by `repository + head_sha + workflow_run_id
   + attempt`. Reject or mark stale findings when the PR head moves.
6. Publish one structured, advisory evidence artifact and optionally a
   deduplicated PR comment. Never include raw tokens, full environment values,
   raw log bodies or private runner addresses.
7. Only on an explicit repair request: generate an isolated candidate branch,
   introduce a reproducing test, verify it fails before the change and passes
   after, run independent gates, then open a **draft PR**. No unattended merge.

## Minimal proposed evidence schema (v1)

```json
{
  "schema_version": 1,
  "repo": "Cyptopimpinainteazy/xxxstar",
  "pr_number": 683,
  "head_sha": "<verified full commit SHA>",
  "workflow_run_id": null,
  "run_attempt": null,
  "findings": [
    {
      "fingerprint": "<stable hash of location and defect category>",
      "path": "path/to/file.rs",
      "line": 1,
      "severity": "high",
      "invariant": "atomic rollback",
      "claim": "A failed leg might leave a partial state change",
      "verification": "UNVERIFIED",
      "evidence_refs": []
    }
  ],
  "checks": [],
  "release_authorized": false
}
```

This is a **specification**, not observed findings. The sample must never be
posted as a real review or passing result.

## Implementation slices and acceptance

1. **Evidence joiner (P0):** strict schema and source identifiers, stale-head
   detection, bounded log handling, reject malformed/mismatched evidence.
   Unit tests: queued, failed, stale SHA, missing logs, secrets redaction.
2. **Consensus/trading rules (P0):** exact integer money, canonical
   gas/state/receipt consistency, WASM `no_std` checks, atomic rollback,
   runtime weight configuration and test weakening. Verify against independent
   tests; model assertions alone are insufficient.
3. **Independent review (P1):** optional second model for high-risk changes,
   reconcile disagreements; never turn model majority into proof.
4. **Reproducer loop (P1):** create candidate tests in an isolated checkout,
   classify deterministic failure vs environment issue, preserve command,
   SHA and sanitized output digest.
5. **Repair proposals (P2):** draft PR only, no merge or deployment.
6. **Measurements (P2):** tracked precision, false positives, missed defects,
   turnaround, model spend and reproducibility over a representative fixture set.

## Security and operational requirements

- `pull_request_target` uses default-branch-controlled scripts exclusively;
  never execute or source PR-head code within a secret-bearing job.
- Scope tokens by job. Treat PR text, diffs, comments and logs as untrusted
  input; no LLM-generated command receives privileged runner credentials.
- Bound analysis per PR/commit and cache by exact SHA to avoid duplicate paid
  calls. Explicitly disclose incomplete large-diff coverage.
- A cancelled, skipped, queued or unavailable required check blocks admission.
- Approval for automatic repair is separate from approval to merge or release.
- Mark live GitHub ingestion **unimplemented** until an end-to-end exact-head
  demonstration passes, including a deliberately failing fixture.
