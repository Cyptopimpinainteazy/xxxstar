# X3 repo scan

Findings carry an id, a severity, the exact file and symbol, why it matters, the fix, the test that would prove the fix and the gate that catches a regression. Sorted by severity, kind, path and line, so two runs over the same tree are byte-identical.

Root: `/home/lojak/Desktop/xxxstar-main`
Findings: 4

## Counts

| kind | count | ratcheted here |
|---|---|---|
| `pallet-call-without-weights` | 4 | yes |

## Related ratchets (not re-reported here)

| gate | status | detail |
|---|---|---|
| stub / marker ratchet | pass | critical-marker=443, explicit-stub=72, marker=1076 |
| fake-code scan | pass | constant-assert=14, noop-test=2, skip=142 |
| panic / unwrap ratchet | pass | pallet-call=0, production=440, runtime-hook=0 |

## Findings

### LOW — `pallet-call-without-weights` — pallets/depin-marketplace/src/lib.rs:368

- **id:** `1f4c6126e754ae38`
- **symbol:** `depin-marketplace::T::DbWeight::get().reads_writes(2, 2`
- **why it matters:** 11 extrinsic(s) charge the documented pre-benchmark `DbWeight::reads_writes` estimate; real numbers need a benchmark run
- **suggested fix:** add a WeightInfo trait, generate weights with the FRAME benchmark CLI (`scripts/run-frame-benchmarks.sh`), and point the runtime at SubstrateWeight<Runtime>
- **test required:** cargo test -p depin-marketplace --features runtime-benchmarks
- **release gate affected:** runtime identity / benchmarks

### LOW — `pallet-call-without-weights` — pallets/private-execution/src/lib.rs:607

- **id:** `d1b03300ae0c8bcf`
- **symbol:** `private-execution::T::DbWeight::get().reads_writes(2, 2`
- **why it matters:** 8 extrinsic(s) charge the documented pre-benchmark `DbWeight::reads_writes` estimate; real numbers need a benchmark run
- **suggested fix:** add a WeightInfo trait, generate weights with the FRAME benchmark CLI (`scripts/run-frame-benchmarks.sh`), and point the runtime at SubstrateWeight<Runtime>
- **test required:** cargo test -p private-execution --features runtime-benchmarks
- **release gate affected:** runtime identity / benchmarks

### LOW — `pallet-call-without-weights` — pallets/x3-jury-anchor/src/lib.rs:97

- **id:** `9e99332b7fcb730a`
- **symbol:** `x3-jury-anchor::T::DbWeight::get().reads_writes(1, 2`
- **why it matters:** 2 extrinsic(s) charge the documented pre-benchmark `DbWeight::reads_writes` estimate; real numbers need a benchmark run
- **suggested fix:** add a WeightInfo trait, generate weights with the FRAME benchmark CLI (`scripts/run-frame-benchmarks.sh`), and point the runtime at SubstrateWeight<Runtime>
- **test required:** cargo test -p x3-jury-anchor --features runtime-benchmarks
- **release gate affected:** runtime identity / benchmarks

### LOW — `pallet-call-without-weights` — pallets/x3-rebalance/src/lib.rs:303

- **id:** `7cf43cd34802bacc`
- **symbol:** `x3-rebalance::T::DbWeight::get().reads_writes(2, 2`
- **why it matters:** 3 extrinsic(s) charge the documented pre-benchmark `DbWeight::reads_writes` estimate; real numbers need a benchmark run
- **suggested fix:** add a WeightInfo trait, generate weights with the FRAME benchmark CLI (`scripts/run-frame-benchmarks.sh`), and point the runtime at SubstrateWeight<Runtime>
- **test required:** cargo test -p x3-rebalance --features runtime-benchmarks
- **release gate affected:** runtime identity / benchmarks

## Documented decisions

These `pallets/` crates are deliberately absent from `runtime/src/lib.rs`. The scanner reports an entry the moment the runtime names the pallet, so the list can only shrink.

- `pallets/pallet-x3-control/Cargo.toml` — the control plane is fail-closed and carries 12 tests, but nothing on a chain reads `ControlState`, so wiring it means deciding who acts on `Frozen`/`Paused` — a design decision the owning row records, not an oversight (owning document: `feature-matrix/agents-experimental.toml`)
## What this scan does not cover

`TODO`/`stub`/test-cheat markers and reachable `unwrap()`/`panic!` counts are owned by the two ratchets above; this report cites their verdicts instead of duplicating their debt.
