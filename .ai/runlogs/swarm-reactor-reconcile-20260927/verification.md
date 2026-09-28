# Northern Swarm / Reactor reconciliation — verification record

Session: openclaw-agent. Host: x3star1. Date: 2026-09-27 (local America/Denver).
Base: `origin/master` = `975408a9b` (PR #519 already landed; see the memory record).

## 1. The weights are measured, and the fix is the pinned template

Before this lane: `pallets/northern-swarm/src/weights.rs` at `a5f6ddca9` carried
`Weight::from_parts(462388000, )` — an empty proof-size argument, because the repo's
`.maintain/frame-weight-template.hbs` read `{{cmd.execution}}` and
`{{benchmark.base_proof_size}}`, which benchmark CLI 53.0.0 does not emit. Build failure
recorded in `pre-fix-template-build-failure.log` (8 × E0061).

Fix: `.maintain/frame-weight-template.hbs` replaced with the pinned polkadot-sdk revision
verbatim. Regenerated with the repo's own driver:

    BENCHMARK_STEPS=50 BENCHMARK_REPEAT=20 bash scripts/run-frame-benchmarks.sh run pallet-northern-swarm

Raw CLI output: `benchmark-cli-run.log`. Output file kept as
`weights-regenerated-by-primary.rs`. Result carries real proof sizes —
`register_executor` Measured 142 / Estimated 3567, `claim_task` Estimated 4149,
`submit_result` Measured 1043 / Estimated 11402 — and matches the committed file in shape
and proof size, differing in ref_time by benchmark noise (78_998_000 vs 90_536_000 for
`register_executor` on two runs the same afternoon). The committed file was left at
`origin/master`'s version rather than churning it with a third measurement.

## 2. Commands and results on the final tree

    cargo check -p pallet-northern-swarm
      Finished `dev` profile — clean
    cargo test -p pallet-northern-swarm --all-targets --no-fail-fast
      7 passed; 0 failed; 0 ignored
    cargo test -p northern-swarm --all-targets --no-fail-fast
      13 passed; 0 failed; 0 ignored
    python3 scripts/mainnet/swarm_reactor_gate.py
      ✅ swarm_reactor_gate: PASS
    bash scripts/check-readiness-consistency.sh
      PASS: All status documents are consistent with FEATURE_REGISTRY.toml.
    python3 scripts/check-registry-tests-are-gated.py
      19 registry feature(s) cite a crate a gate tests, 0 on KNOWN_UNGATED
    python3 scripts/feature_matrix.py check
      PASS: 149 features, 35 warning(s)
    python3 scripts/ci/check-matrix-tests-exist.py
      OK - 403 required_tests citation(s) resolve to real fn names
    python3 scripts/ci/check-matrix-test-evidence.py
      OK - 196 citation(s) in test_evidence notes resolve
    python3 scripts/x3_audit_matrix.py --check
      PASS: artifacts match their sources

## 3. Break-it-first: the weight checks are load-bearing

Mutating the files and re-running the gate's own `check_pallet_contract()`, then restoring
byte-identically:

    baseline                       failures: []
    weights.rs loses CLI provenance failures: ['Northern Swarm weights.rs lacks benchmark CLI provenance']
    hand-written call weight added  failures: ['pallet-northern-swarm still uses hand-written
                                               Weight::from_parts constants; benchmark-generated
                                               weights are required']
    restored                       failures: []

## 4. What this record does not claim

Nothing here ran against a node. The executor↔chain contract is proven at the
encoder/signer/decoder boundary only. `check-registry-tests-are-gated.py` was red before this
lane because `northern_swarm_reactor` cited `crates/northern-swarm` while no gate ran it; that
is now closed by `test northern-swarm` and `test pallet-northern-swarm` in `scripts/local-ci.sh`.

## 5. Post-commit gate sweep

Run on the committed tree (HEAD = the four reconciliation commits on top of `975408a9b`):

    bash scripts/check-readiness-consistency.sh              rc=0  PASS
    python3 scripts/check-registry-tests-are-gated.py        rc=0  19 citing features gated, 0 ungated
    python3 scripts/feature_matrix.py check                  rc=0  PASS 149 features, 35 warnings
    python3 scripts/ci/check-matrix-tests-exist.py           rc=0  403 citations resolve
    python3 scripts/ci/check-matrix-test-evidence.py         rc=0  196 citations resolve
    python3 scripts/x3_audit_matrix.py --check               rc=0  PASS
    python3 scripts/mainnet/swarm_reactor_gate.py            rc=0  swarm_reactor_gate: PASS

Not run here: `cargo check -p x3-chain-runtime --features mainnet-rc1`. It was started twice and
lost both times to the shared `target/` lock — three other lanes were building in this worktree
(a cross-vm-coordinator test run, an srtool reproducible build, a nested `x3-swarm-core` check).
It is also not a check this change set can move: the four commits touch no Rust. The palette of
results above is the pallet and executor compiling and testing, which is what these edits could
affect.
