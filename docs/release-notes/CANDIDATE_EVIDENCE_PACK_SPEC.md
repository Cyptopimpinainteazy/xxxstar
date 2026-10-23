# Candidate Evidence Pack Specification

## Purpose

Define the minimum evidence contract for a public X3 mainnet promotion and bind
that evidence to the exact commit and artifacts being launched.

The machine-readable launch record is schema 2. It is produced and verified by
`scripts/mainnet/launch_record.py`; the evidence pack is the input index from
which the record is compiled.

## Candidate Directory Layout

```text
candidate-evidence/
  metadata/
    run.json
  gates/
    receipt-schema.txt
    claim-status.txt
    parity-vectors.txt
    replay-failure.txt
    gpu-cpu-determinism.txt
    status-doc-truth-lint.txt
  signoff/
    candidate-signoff.md
```

The candidate rehearsal layout remains required. A mainnet launch record adds
the schema-2 files below.

## Launch Evidence Pack

`launch_record.py build --evidence-pack <path>` accepts this structure. Paths
are relative to `--root` or absolute.

```json
{
  "feature_matrix": {
    "path": "FEATURE_MATRIX.toml"
  },
  "test_gates": [
    {
      "name": "release",
      "command": "make mainnet-check",
      "status": "PASS",
      "exit_code": 0,
      "log": "audit-artifacts/launch/test-gates/release.log"
    }
  ],
  "audits": [
    {
      "provider": "auditor-a",
      "scope": ["runtime", "consensus", "atomic"],
      "critical_open": 0,
      "high_open": 0,
      "report": "audit-artifacts/launch/audits/runtime.pdf"
    },
    {
      "provider": "auditor-b",
      "scope": ["bridges", "contracts", "operations"],
      "critical_open": 0,
      "high_open": 0,
      "report": "audit-artifacts/launch/audits/bridges.pdf"
    }
  ],
  "bounty_window": {
    "url": "https://bounty.example/x3",
    "pool_usd": 100000,
    "starts_at": "2026-08-01T00:00:00Z",
    "ends_at": "2027-01-01T00:00:00Z",
    "evidence": "audit-artifacts/launch/bounty/window.json"
  },
  "public_testnet_window": {
    "started_at": "2026-06-01T00:00:00Z",
    "ended_at": "2026-08-01T00:00:00Z",
    "validator_count": 50,
    "continents": 5,
    "evidence": "audit-artifacts/launch/public-testnet/window.json"
  },
  "bridge_configuration": {
    "path": "audit-artifacts/launch/bridges/genesis.json"
  },
  "validators": {
    "path": "audit-artifacts/launch/validators/inventory.json"
  },
  "approvals": [
    {
      "approver": "release-approver",
      "role": "release",
      "approved_commit": "<40-or-64-char commit>",
      "evidence": "audit-artifacts/launch/approvals/release.json"
    },
    {
      "approver": "security-approver",
      "role": "security",
      "approved_commit": "<same commit>",
      "evidence": "audit-artifacts/launch/approvals/security.json"
    },
    {
      "approver": "operations-approver",
      "role": "operations",
      "approved_commit": "<same commit>",
      "evidence": "audit-artifacts/launch/approvals/operations.json"
    }
  ]
}
```

## Evidence Pack Rules

- The feature matrix must describe exactly 82 P0 rows. Every P0 row must record
  `implemented`, `tested`, and `mainnet_ready` at 95 or above with no blockers.
- Every test-gate entry must name a command, exit zero, carry `PASS`, and point
  to a readable log. The launch run must include every gate required by the
  release policy, including the mainnet release gate and pre-push CI.
- At least two distinct audit providers are required. Their scopes must cover
  runtime/consensus/atomic execution and bridges/contracts/operations, with no
  open critical or high findings.
- The bounty pool must be at least USD 100,000, have been live for four weeks
  before the record, and remain open at promotion time.
- The public testnet must have run at least eight weeks, ended before the
  record, and evidence at least 50 validators across five continents.
- Bridge configuration must activate exactly `ethereum-mainnet`,
  `solana-mainnet-beta`, and `bitcoin-mainnet`, with initial limits of USD 1,000
  per transfer, USD 10,000 per route per day, and USD 50,000 total exposure, and
  with the kill switch enabled.
- Validator evidence must contain seven distinct non-loopback hosts, operators,
  and identities.
- Approvals must be distinct and include release, security, and operations
  roles. Each approval names the promoted commit. `--approved-by` must identify
  one of the approvers.

## Launch Record Schema 2

The record contains:

- `commit`, the genesis artifacts, and the RC gate log with hashes.
- `evidence_pack` and `feature_matrix` hashes.
- A canonical 82-row `p0_snapshot` and its digest.
- Normalized test gates, audit reports, bounty window, public-testnet window,
  bridge configuration, validator inventory, and approvals, each with file
  hashes.
- `record_sha256`, computed over the compact, sorted JSON representation of
  every other field.

JSON is written with sorted keys and a trailing newline. The record must not be
hand-edited.

## Verification

```bash
python3 scripts/mainnet/launch_record.py verify --record reports/launch/promotion.json
```

`verify` recomputes the record from current files and rejects:

- a changed record digest, spec, raw spec, RC log, matrix, gate log, audit,
  window, bridge configuration, validator inventory, or approval;
- a tree at a commit other than the recorded commit;
- a P0 snapshot that no longer describes the current matrix;
- a generated record whose evidence pack has moved or disappeared.

Hashes detect local tampering and drift. A named local approval is not a
cryptographic identity proof; authenticated operator signatures remain separate
release evidence.

## Rejection Conditions

- Missing metadata keys or missing evidence classes.
- Missing, stale, or mismatched artifact and evidence hashes.
- P0 scores below 95, open P0 blockers, or a P0 count other than 82.
- Fewer than two independent audit providers or unresolved critical/high
  findings.
- Bounty or public-testnet windows below the required duration or coverage.
- Bridge routes or limits different from the approved initial configuration.
- Any validator that is missing, duplicated, or loopback.
- An approval for another commit or a missing approval role.
- A record whose schema is not 2 or whose `record_sha256` does not verify.
