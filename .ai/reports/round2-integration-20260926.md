# Round-2 workstreams D / E / F — integration and independent reproduction

Date: 2026-09-26
Branch: `feat/x3-prelaunch-economics-x3lang-cutover`
Scope source: `.ai/tasks/2026-09-26-round2-workstreams.md`
Integrator: root agent (this pass). Three workstream agents ran in parallel against the
same working tree; every result below was **re-run by the integrator** rather than taken
from the agent's own report. Old proofs are not new proofs.

## What the three workstreams delivered, and what was checked

| WS | Deliverable | Agent claim | Integrator re-ran | Verdict |
|---|---|---|---|---|
| D | runtime upgrade rehearsed through the chain's own governance path | 13/13 PASS, spec 20→21 | `bash scripts/mainnet/runtime_upgrade_rehearsal.sh` and the gate via `local-ci --live --only runtime-upgrade-through-governance` | reproduced: PASS, 47s through the gate |
| D | the check can say no | negative control via `X3_UPGRADE_WASM` = the running blob | re-ran with a scratch report path | reproduced: rc=1, `System::SpecVersionNeedsToIncrease` |
| E | every external EVM/SVM path refuses by default, machine-checked | checker FAILED before the gate existed, OK after | `local-ci --only external-paths-disabled,test-x3-external-chains` + `check-external-paths-disabled.py` | reproduced: both PASS; checker reports 5 paths / 32 gate shapes |
| F | native supply conserved on every validator under distributed traffic | 15 comits, per-validator identity, corrupted-ledger control | `cargo test -p x3-chain-node --test supply_invariant_distributed -- --ignored` and the gate via `local-ci --cross --only supply-invariant-across-validators` | reproduced: 1 passed 193s; gate PASS 190s |

## Exact numbers the integrator reproduced

### D — runtime upgrade (governance, not subxt+sudo)

```
[PASS] governance_upgrade_enacted
[PASS] spec_version_incremented
[PASS] code_hash_changed
[PASS] all_validators_upgraded
[PASS] blocks_and_finality_after_upgrade
[PASS] post_upgrade_state_operation
  runtime_upgrade_rehearsal: PASS
```

Negative control (same script, `X3_UPGRADE_WASM` pointed at the blob the chain already runs):

```
[note] the chain refused the code swap with System::SpecVersionNeedsToIncrease
[FAIL] governance_upgrade_enacted / spec_version_incremented / code_hash_changed / all_validators_upgraded
[SKIP] blocks_and_finality_after_upgrade — no enactment block
  runtime_upgrade_rehearsal: FAIL      RC=1
```

Why the "old" runtime is genuinely older: `runtime/src/lib.rs` at HEAD declares
`spec_version: 20`, and the rehearsal's next-runtime artifact is built by
`build_runtime_upgrade_artifact.sh` in a **detached worktree** with the version incremented by
exactly one and nothing else touched, so the working tree's `runtime/src/lib.rs` is never
edited. The artifact's provenance (revision, both spec versions, sha256) is recorded in
`target/upgrade-artifact/x3_chain_runtime.next.compact.compressed.wasm.provenance.json`.

### E — external paths closed by default

```
check-external-paths-disabled: OK — 5 external paths closed (32 gate shapes, proofs and flags checked;
  no settlement proof is accepted that this workspace cannot verify)      EXIT=0
```

The defect it found is the real one: `x3-external-chains` (every external EVM adapter and the
settlement verifier) ran in **no** `local-ci.sh` gate, so its refusals were not evidence. Both
gate lines are now in the fast array and both PASS.

### F — supply conservation under distributed traffic

```
consensus: finalized alice=69 bob=69 charlie=69, all agree on 69:0x2fdd12fd…
baseline at 69  — 8 accounts, 8000000000000000000 == TotalIssuance 8000000000000000000 (all three)
15 comits accepted; all 15 have execution receipts on a validator that never saw the submission
:19964/:19965/:19966 at 101:0x7ad4f9b4… — 7999999998338644985 == 7999999998338644985 — conserved
//Alice //Bob //Charlie each paid 553785005; total 1661355015
TotalIssuance 8000000000000000000 -> 7999999998338644985 (burned 1661355015)
negative control caught it: 8 accounts sum to 8000000000000000000 but TotalIssuance is
  8000000000000000001 (delta 1)
test result: ok. 1 passed; 0 failed; 0 ignored   (193.44s)
```

## Gate list after this pass

`bash scripts/local-ci.sh --all --dry-run` → 118 gates, no duplicate slugs. Two live gates
were added and both are in `SERIAL_GATES` because they bind node ports:

```
"runtime upgrade through governance:bash scripts/mainnet/runtime_upgrade_rehearsal.sh"
"supply invariant across validators:env -u SKIP_WASM_BUILD cargo test -p x3-chain-node --test supply_invariant_distributed -- --ignored --nocapture --test-threads=1"
```

Fail-closed detail found while wiring F: the other committer added the `GATES_CROSS` line but
not the `SERIAL_GATES` entry. A multi-node gate that is not serialised loses a port bind and
then talks past the other node instead of failing loudly — that is a silent-red hazard, so the
entry was added here.

## Commits from this pass

* `370cabec7` fix(mainnet): rehearse the runtime upgrade through council governance, not subxt+sudo
  (`scripts/mainnet/runtime_upgrade_rehearsal.sh`, `build_runtime_upgrade_artifact.sh`,
  `runtime_upgrade_governance_driver.cjs`, `scripts/local-ci.sh`, `.gitignore`)
* `cb856ba66` test(node): prove native supply conservation on every validator under distributed traffic
  (`node/tests/supply_invariant_distributed.rs`, `node/Cargo.toml`, `Cargo.lock`, `scripts/local-ci.sh`)

E's two files were committed by the tree's other committer as `3504d8b6c`; the integrator
verified that commit's gates rather than re-committing them.

## Not proven, and not claimed

1. The seven physical validators do not exist. Every live result here is the built-in
   three-validator `local3` chain. The bullet "performed successfully on the live 7-node
   testnet" is **not** literally satisfied.
2. D: spec 20→21 has no migration work to do, so the migration tuple runs without rewriting a
   key. A rehearsal that upgrades over existing state with a pallet whose `on_runtime_upgrade`
   really rewrites keys is a stronger, separate test.
3. D: nothing enforces that the artifact's revision matches the node binary's revision; HEAD
   moved twice during the workstream. `--rev` exists for the operator.
4. F: the value-moving operation is the kernel comit's burned fee, not a `Balances::transfer`.
   The only generic-call path the runtime signer exposes reaches Root and is not `ensure_signed`.
5. F: only **native** supply is covered end to end. The kernel's per-asset `SupplySnapshot`
   invariant (`bridge_locked` / `external_locked`) still has no per-asset RPC read surface.
6. E: no external path is proven against a real public testnet; none can be on this box. The
   deliverable is therefore the explicit, tested closure.

## Branch-state blocker found while verifying (not fixed here, on purpose)

`cargo fmt --all -- --check` is **RED at HEAD**, in exactly one place, and it is not any
round-2 workstream's file:

```
Diff in /home/lojak/Desktop/xxxstar-main/pallets/x3-kernel/src/tests.rs:2334:
        let stored = crate::SubmittedComits::<Test>::get(comit_id)
            .expect("a submitted comit is recorded");
   ->
        let stored =
            crate::SubmittedComits::<Test>::get(comit_id).expect("a submitted comit is recorded");
```

It arrived with `3224d6ecf test(x3-kernel): a pause must not strand a caller's nonce…`, which the
other committer landed without running the format gate (`cargo fmt --all -- --check` → exit 1,
1 file). The file is clean in the working tree, so this is committed state, not someone's
in-flight edit.

It was deliberately **not** fixed here. `pallets/x3-kernel/**` is another workstream's path
(round-2 ground rule 3), and that crate is under active edit by a second session. The fix is one
rustfmt reflow of a single `let`: run `cargo fmt --all`, confirm `pallets/x3-kernel/src/tests.rs`
is the only file that moves, and commit it as `style(x3-kernel): reflow a let the format gate
caught`. Until it lands, the fast `format check` gate is red for everyone on this branch.

## Follow-up tickets

Each is a concrete, independently verifiable item with its acceptance test.

1. **X3-RT-002 — a migration that actually rewrites keys under an upgrade.**
   *Why:* D's rehearsal proves the upgrade path and the version move, but spec 20→21 has no
   migration work, so the migration tuple runs without touching a key.
   *Acceptance:* a pallet in the `Migrations` tuple is rolled to version 0 with seeded state, the
   hooks run, and the rewritten keys are read back over RPC afterwards on the upgraded chain.
   *Validation:* extend `scripts/mainnet/runtime_upgrade_rehearsal.sh` with a check whose failure
   mode is observable (corrupt a migrated key in a scratch copy and require it to fail).

2. **X3-RT-003 — pin the upgrade artifact to the node binary's revision.**
   *Why:* D's item 3 — nothing enforces that the "next runtime" and the frozen node binary come
   from the same revision, and HEAD moved twice during the workstream.
   *Acceptance:* the rehearsal refuses to start when the artifact's provenance revision differs
   from the revision the node binary was built from (or the operator passes the pairing
   explicitly).
   *Validation:* run the gate with a deliberately mismatched `--rev` and require exit 1.

3. **X3-SUP-002 — a signed `Balances::transfer` traffic stream.**
   *Why:* F's item 1 — the only value-moving operation proven under distributed traffic is the
   kernel comit's burned fee, because the only generic-call path the runtime signer exposes
   reaches Root and is not `ensure_signed`.
   *Acceptance:* `crates/x3-runtime-signer` gains `sign_balances_transfer`, and the distributed
   test drives a transfer stream that creates accounts.
   *Validation:* the same per-validator conservation check, with the transfer stream included.

4. **X3-SUP-003 — a per-asset supply read surface.**
   *Why:* F's item 3 — only **native** supply is covered; the kernel's `SupplySnapshot`
   (`circulating + bridge_locked + pending_transfer + external_locked == total_issued`) has no
   per-asset read surface over RPC.
   *Acceptance:* a runtime API returns the per-asset snapshot, and the distributed check asserts
   the identity per asset on every validator.
   *Validation:* the negative control must still catch an injected unit, per asset.

5. **X3-EXT-003 — `register_external_root` validates a data shape, not a proof.**
   *Why:* E's item 3 — `pallet-x3-cross-vm-router`'s `register_external_root` checks
   `!proof.is_empty()` and calls that "proof against chain consensus", the same class of bug
   `settlement.rs` already had fixed. Unreachable today only because `ExternalBridgesEnabled` is
   false.
   *Acceptance:* the gate verifies a real proof (or refuses by name with a typed error), and the
   existing `ExternalBridgeAuditGate` continues to block enablement.
   *Validation:* a test proving the permissive body is rejected — the same shape of test that
   caught the settlement verifier.

6. **X3-EXT-004 — delete the stale test reference in the settlement mock.**
   *Why:* E's item 2 — `pallets/x3-settlement-engine/src/mock.rs` cites
   `allow_unattested_cross_domain_proofs_is_false_by_default_in_live_genesis`, a test that exists
   nowhere in the tree. The new checker enforces what the comment claimed, but the comment is
   still a phantom test name.
   *Acceptance:* the comment points at `scripts/ci/check-external-paths-disabled.py`, or is gone.
   *Validation:* `rg` for the old test name returns nothing.

7. **CI-FMT-001 — the branch is format-red at HEAD.**
   *Why:* the blocker section above; `3224d6ecf` landed without the format gate.
   *Acceptance:* `cargo fmt --all -- --check` exits 0 at HEAD.
   *Validation:* `bash scripts/local-ci.sh --only format-check`.
   *Owner:* whoever owns `pallets/x3-kernel/**` this session — **not** the round-2 integrator.
