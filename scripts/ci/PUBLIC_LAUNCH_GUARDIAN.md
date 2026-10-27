# X3 Public Launch Guardian — evidence gate (v0.1)

This is a **read-only admission assessment**, not an autonomous launch agent.
It complements `scripts/mainnet_release_gate.py`, the existing production checks,
`scripts/ci/github_evidence.py` and the CI Autopilot foundation in PR #683.
It never merges, deploys, starts validators, changes firewall rules, publishes
endpoints, rotates keys, or starts paid infrastructure.

## Policy and decisions

The versioned policy is `scripts/ci/public-launch-policy.json`. It requires:
- Latest completed-success GitHub Actions run for **the same full candidate SHA**
  for Local CI, production compiler, compiled storage, atomic trading,
  security audit, production gate and Zombienet integration.
- **Job-level** evidence: each required run must have the required count of
  completed-success jobs. Skipped, queued, cancelled, failed, zero-job and
  incomplete-paginated workflows are blocking. The latest rerun wins.
- Workflow runs no older than 14 days; a different SHA never inherits green.
- At least seven distinct observed peer IDs and on-chain authority keys, one
  matching finalized checkpoint and genesis, plus recorded finality and fault
  evidence. **Submitted metadata is not proof that those nodes exist.**
- Operator evidence for external RPC rate limiting, key/firewall review,
  monitoring, rollback, faucet abuse controls and an independent security review.

Results:
- `BLOCKED` (exit 1): any evidence missing, contradictory or non-green.
- `INDEPENDENT_REVIEW_REQUIRED` (exit 0): *claims* for every required lane
  are present. **Never a GO decision**; an independent human must verify
  the physical fleet and source attestations before release.
- Invalid or unavailable input (exit 2): no partial evaluation or silent green.

Every report includes `release_authorized: false`, whether tests pass or not.
The tool does not verify operator signatures or the cryptographic validity of
the submitted physical-finality evidence. Do not use its exit status alone to
make a public-testnet deployment decision.

## Run the tests

```bash
python3 -m unittest discover -s scripts/ci -p 'test_public_launch_guardian.py' -v
python3 -m json.tool scripts/ci/public-launch-policy.json >/dev/null
```

## Read-only assessment using GitHub Actions

On a trusted operator machine with GitHub read permission:

```bash
GH_TOKEN="$(gh auth token)" python3 scripts/ci/public_launch_guardian.py \
  --candidate-sha "<40-character-lowercase-commit-SHA>" \
  --fetch-github \
  --output /tmp/x3-public-launch-assessment.json
```

The workflow `.github/workflows/x3-public-launch-guardian.yml` has a manual
dispatch input `candidate_sha`. It checks out only the workflow branch, **not**
the candidate's source, and queries GitHub metadata with a read-only token.
A manually dispatched assessment without operator evidence must fail closed.

For an offline rehearsal (fixtures are *never* authenticated GitHub evidence):

```bash
python3 scripts/ci/public_launch_guardian.py \
  --candidate-sha "<commit-SHA>" \
  --runs-json /tmp/actions-runs.json \
  --fleet-json /tmp/fleet-observations.json \
  --operations-json /tmp/operations-attestations.json
```

The offline result remains `BLOCKED` even if every supplied check says pass.
Do **not** store secrets, validator keys, API tokens, wallet seed phrases, or
sensitive raw logs in the observations or CI artifacts.

## Operator evidence schemas

Fleet file, prepared from **independent operator observation** of the physical
fleet at an agreed finalized checkpoint:

```json
{
  "candidate_sha": "<40-character-commit>",
  "genesis_hash": "0x<64-hex-digits>",
  "evidence_sha256": "<64-hex-digits-of-external-evidence-bundle>",
  "validators": [
    {
      "peer_id": "<unique-network-peer-id>",
      "authority_key": "0x<64-hex-digits-public-key>",
      "finalized_number": 100,
      "finalized_hash": "0x<64-hex-digits>"
    }
  ],
  "finality_progress_verified": true,
  "fault_recovery_verified": true,
  "onchain_authority_set_verified": true,
  "chain_spec_verified": true
}
```

The fleet array needs **at least seven actual entries** with distinct peer IDs,
distinct authority keys and identical finalized height/hash. The example above
contains one entry for brevity and intentionally fails the gate. Authority keys
here are *public* identifiers, **never private signing keys**. Also verify
validator authority ownership and the corresponding on-chain session set;
distinct peer IDs alone do not prove independent validators.

Operations file, prepared only after **outside verification**:

```json
{
  "candidate_sha": "<40-character-commit>",
  "evidence_sha256": "<64-hex-digits-of-external-evidence-bundle>",
  "rpc_rate_limits_verified": true,
  "keys_and_firewall_reviewed": true,
  "monitoring_alerts_verified": true,
  "rollback_rehearsed": true,
  "faucet_abuse_controls_verified": true,
  "independent_security_reviewed": true
}
```

These values are operator assertions, not proof. Their digests are identifiers;
this version does not fetch, authenticate or validate the evidence bundles.
Final launch approval must be independent of the code author and CI agent.

## Promotion sequence

1. Choose immutable candidate commit; reconcile any open implementation PRs.
2. Run `make guard`, the mainnet/testnet release gates and exact-head workflow
   suite; no skipped jobs or unexplained advisory exceptions.
3. Prove seven real authority nodes agree on genesis/finality under adverse
   conditions and verify rollback from a snapshot on nonproduction hardware.
4. Verify RPC/faucet limits, external exposure, operator isolation, monitoring,
   security findings and key custody. Audit bridge finality independently.
5. Review the Guardian JSON and primary artifacts independently; obtain a human
   GO/NO-GO sign-off that cites the candidate SHA and evidence bundle digests.
6. Only after that separate approval may an operator schedule a public testnet.
   **This workflow intentionally contains no deployment operation.**

### Known limits

The Guardian does not establish that GitHub workflow definitions themselves are
correct or immutable, determine branch-protection required checks, prove binary
reproducibility, inspect physical validators, verify signature ownership or
guarantee audit completeness. Those remain separate mandatory reviews. A run
with no job metadata is `NO_JOBS`, not success. Artifact upload failures may
also block evidence retention and require operator remediation.
