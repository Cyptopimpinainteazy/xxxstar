# Release Gates

**Canonical source: `FEATURE_REGISTRY.toml`** — all readiness scores and blockers derive from it. Run `scripts/check-readiness-consistency.sh` to validate.

**Overall readiness: 62.74%** (arithmetic mean across the 23 scored entries in `FEATURE_REGISTRY.toml`). A mainnet-ready claim is forbidden unless every feature scores ≥95%.

> This figure is not maintained by hand. `scripts/check-readiness-status-current.py` recomputes it from the
> registry and fails the `readiness status current` gate when this file disagrees, so a registry edit cannot
> leave a stale number here. The previous revision read 58.5% across 22 entries, and the section below still
> said 55.5% — both had survived several registry changes because nothing checked downward drift.

## Gate commands

- `make guard` — agent/stub/test-cheat guards
- `make test` — focused Python + Rust compiler tests
- `make audit` — invariant guard + mainnet release gate
- `make mainnet-check` — mainnet release gate
- `make fresh-machine-check` — bootstrap validation on fresh machine
- `python3 scripts/mainnet/swarm_reactor_gate.py` — Northern Swarm/Reactor on-chain compute hard gate

## Mainnet release gate (`make mainnet-check` → `scripts/mainnet_release_gate.py`)

Exit 0 = PASS. Exit 1 = FAIL — do NOT cut a release.

Validates the P0 release contract before doing any expensive build work: exactly 82 approved P0
rows, each with `implemented`, `tested`, and `mainnet_ready` at 95 or above, no recorded blockers,
resolvable named tests, and generated audit-matrix artifacts that still match their canonical
sources. A P0 scope failure stops the gate immediately.

After that early check, validates documentation existence, build, chain-spec, critical test suites,
Northern Swarm/Reactor on-chain compute, reproducible builds, and secret hygiene.

The granular matrix may record a P0 row below 95 while it is being completed. `mainnet-check`
will reject that state; the recorded score must only rise when its named evidence is current.


## Northern Swarm + Reactor on-chain compute hard gate

`scripts/mainnet/swarm_reactor_gate.py` is a **mainnet blocker**, not a documentation score. It is intentionally red until the deployed compute path is real.

Required before PASS:

- `pallet-northern-swarm` and `northern-swarm` remain workspace/runtime-integrated and both test suites pass.
- The on-chain pallet has real mock/unit coverage and benchmark-derived weights; hard-coded `Weight::from_parts` call weights are not accepted.
- Task results are finalized through an M-of-N verification/quorum path rather than accepting the first executor result.
- Reserved task rewards are actually settled to accepted executor(s), with balance invariants tested.
- The chain watcher consumes the pallet's real `Tasks` storage/events through metadata-compatible access; a fabricated/hard-coded `PendingTasks` key is forbidden.
- `submit_result` is signed by the registered executor and call encoding is metadata-driven; hard-coded pallet indices and unsigned-call shortcuts are forbidden.
- `TaskKind` survives into execution routing, with a typed compute-backend abstraction, a GPU backend, and an explicit CPU fallback.
- Accelerator use changes performance only. Consensus correctness and deterministic work must retain a CPU-verifiable path.

Current canonical score: `northern_swarm_reactor = 55` in `FEATURE_REGISTRY.toml`, derived on the matrix formula (`implemented*0.35 + tested*0.25 + mainnet_ready*0.40`) with implemented=83, tested=78, mainnet_ready=17 -> 55.35, recorded as 55, still capped below 70 because nothing on the row has run against a node. Passing the structural gate alone does not raise a score; evidence and tests must justify any readiness change, and the row's blockers name what is still open (no live-node test, hash-equality-only result acceptance, self-declared hardware, host-specific weights).

The gate reaches the pallet and the executor crate only. It does **not** reach `crates/x3-bench`
(`[x3_reactor]`), so its name is not a claim about the Reactor benchmark crate.

## CI enforcement

Enforced in `.github/workflows/mainnet-readiness.yml` on every push/PR to main.

## Mainnet-ready claims

Forbidden unless all gates pass AND `FEATURE_REGISTRY.toml` scores ≥95% for every feature. Currently: 62.74% registry-scored readiness; this is **not** a mainnet-ready score.
