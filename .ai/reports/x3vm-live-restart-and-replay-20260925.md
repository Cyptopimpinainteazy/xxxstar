# The X3VM live suite proved inclusion, not survival: restart and replay proofs added

Date: 2026-09-25
Subsystem: X3Lang -> X3BC -> X3VmAdapter -> X3AtomicKernel -> finality (matrix row `X3-LANG-004`)

## What was already proven, and what was not

`node/tests/x3vm_live_lifecycle.rs` boots the real dev node and proves a compiled `.x3`
program travels source -> compiler -> X3BC -> the production adapter -> the kernel's
extrinsic -> block inclusion -> GRANDPA finality, with the receipt read back from
finalized state and over the runtime API. That is the hardest half of PRIORITY 1.

Two items on the directive's PRIORITY 1 list had no live test:

* **restart behaviour.** The suite's only restart proof re-opened a *relayer's* persisted
  proof ledger. "The relayer can revalidate a lock it wrote down" is a different claim from
  "the chain still has the comit after the chain node exits".
* **replay rejection.** `duplicate refund must fail closed` covers a second refund call on
  the adapter. Nothing asserted that the same signed extrinsic cannot be mined twice.

## What the two new tests assert

`a_finalized_x3_comit_survives_a_node_restart` gives the node a `--base-path` that outlives
it (`spawn_dev_node` passes `--tmp`, so no existing test could have proved persistence; the
binary was checked first — `--dev --base-path DIR` creates `DIR/chains/x3_chain_dev`).
It finalizes a comit, kills the process, starts a new one on the same database and ports,
and requires:

* the finalized height to resume **at or past** where it stopped, not at genesis;
* the receipt to come back with the same `return_data`, `gas_used` and `version` — gas is
  state, not something a restarted node re-derives;
* a further comit to finalize, so the restarted node is a running chain and not just a
  readable database.

`a_replayed_x3_comit_extrinsic_is_not_mined_twice` submits the identical signed bytes a
second time and counts, across every finalized block, how many contain them. The count must
stay exactly 1 — asserted once before the replay so the test cannot pass vacuously. The
pool's answer is reported rather than asserted: it arrives as `JSON-RPC error 1012:
Transaction is temporarily banned`, and "the pool refused it" is not "the chain did not
mine it twice". The first version of this test panicked on that refusal, which is exactly
the confusion the chain-level assertion exists to avoid.

## Evidence

```
$ bash scripts/local-ci.sh --cross --only x3-native-lifecycles
PASS X3-native lifecycles   362s
  test result: ok. 6 passed; 0 failed; 0 ignored; ... finished in 325.68s

(the same gate was run twice: 327s / 324.73s before a clippy literal fix, and 362s / 325.68s after it,
 on the exact committed bytes; the two new tests also pass individually, 80.29s and 48.10s)

$ ... --exact a_finalized_x3_comit_survives_a_node_restart
test result: ok. 1 passed; ... finished in 80.29s
  (node started 17:21:38, killed, restarted 17:22:18, receipt unchanged, chain advanced)

$ ... --exact a_replayed_x3_comit_extrinsic_is_not_mined_twice
the pool refused the replay: JSON-RPC error 1012: Transaction is temporarily banned
test result: ok. 1 passed; ... finished in 48.10s

$ bash scripts/local-ci.sh --only 'audit-matrix-freshness,format-check,test-node'
PASS audit matrix freshness 1s / PASS format check 6s / PASS test node 45s
```

Matrix: `X3-LANG-004` gains both test names in `required_tests`, a note describing what the
live file now covers, and a CLOSED entry for the pair. `scripts/x3_audit_matrix.py`
regenerated `docs/audit/X3_FEATURE_COMPLETION_MATRIX.md`, `docs/audit/X3_AGENT_QUEUE.md` and
`audit-artifacts/current/feature-status.json` (3 lines total; 145 rows, COMPLETE=10,
FUNCTIONAL BUT UNHARDENED=56, PARTIAL=60, STUB=11, NOT INTEGRATED=8).

## Still open on this row

* The network is **one local node**, so inclusion, finality, restart and replay are proven
  but multi-validator agreement is not. That is ROADMAP PRIORITY 4.
* `scripts/mainnet/runtime_upgrade_rehearsal.sh` still has not run (it wants a release build
  and `subxt`, absent here), so the live-chain upgrade path keeps its own open item.
* `submit_comit_v2`'s benchmark still stops on its SVM payload fixture (`SvmExecutionFailed`),
  which is why the weight entry remains a placeholder.
* Gas accounting is proven live only as "the receipt reports metered gas". An out-of-gas
  refusal is asserted at the unit level (`test_gas_exhausted`) and not yet on a live chain.
