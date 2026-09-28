# X3 repo scan

Findings carry an id, a severity, the exact file and symbol, why it matters, the fix, the test that would prove the fix and the gate that catches a regression. Sorted by severity, kind, path and line, so two runs over the same tree are byte-identical.

Root: `/tmp/x3lang-finish`
Findings: 12

## Counts

| kind | count | ratcheted here |
|---|---|---|
| `pallet-call-without-weights` | 12 | yes |

## Related ratchets (not re-reported here)

| gate | status | detail |
|---|---|---|
| stub / marker ratchet | pass | critical-marker=442, explicit-stub=72, marker=1035 |
| fake-code scan | pass | constant-assert=14, prod-mock=239, skip=137 |
| panic / unwrap ratchet | pass | pallet-call=0, production=442, runtime-hook=0 |

## Findings

### HIGH — `pallet-call-without-weights` — pallets/x3-account-registry/src/lib.rs:143

- **id:** `b0dc1fd9e36a697e`
- **symbol:** `x3-account-registry::10_000`
- **why it matters:** 3 extrinsic(s) charge an invented literal weight and the pallet has no generated weights at all, while being registered in runtime/src/lib.rs
- **suggested fix:** add a WeightInfo trait, generate weights with the FRAME benchmark CLI (`scripts/run-frame-benchmarks.sh`), and point the runtime at SubstrateWeight<Runtime>
- **test required:** cargo test -p x3-account-registry --features runtime-benchmarks
- **release gate affected:** runtime identity / benchmarks

### HIGH — `pallet-call-without-weights` — pallets/x3-asset-registry/src/lib.rs:180

- **id:** `b574f1efddd4ee97`
- **symbol:** `x3-asset-registry::Weight::from_parts(25_000, 0`
- **why it matters:** 7 extrinsic(s) charge an invented literal weight and the pallet has no generated weights at all, while being registered in runtime/src/lib.rs
- **suggested fix:** add a WeightInfo trait, generate weights with the FRAME benchmark CLI (`scripts/run-frame-benchmarks.sh`), and point the runtime at SubstrateWeight<Runtime>
- **test required:** cargo test -p x3-asset-registry --features runtime-benchmarks
- **release gate affected:** runtime identity / benchmarks

### HIGH — `pallet-call-without-weights` — pallets/x3-cross-vm-router/src/lib.rs:619

- **id:** `dfbf068d1a222e53`
- **symbol:** `x3-cross-vm-router::Weight::from_parts(40_000, 0`
- **why it matters:** 8 extrinsic(s) charge an invented literal weight and the pallet has no generated weights at all, while being registered in runtime/src/lib.rs
- **suggested fix:** add a WeightInfo trait, generate weights with the FRAME benchmark CLI (`scripts/run-frame-benchmarks.sh`), and point the runtime at SubstrateWeight<Runtime>
- **test required:** cargo test -p x3-cross-vm-router --features runtime-benchmarks
- **release gate affected:** runtime identity / benchmarks

### HIGH — `pallet-call-without-weights` — pallets/x3-crosschain-gateway/src/lib.rs:803

- **id:** `1f2a607c7f59c765`
- **symbol:** `x3-crosschain-gateway::frame_support::weights::Weight::from_parts(20_000, 0`
- **why it matters:** 11 extrinsic(s) charge an invented literal weight and the pallet has no generated weights at all, while being registered in runtime/src/lib.rs
- **suggested fix:** add a WeightInfo trait, generate weights with the FRAME benchmark CLI (`scripts/run-frame-benchmarks.sh`), and point the runtime at SubstrateWeight<Runtime>
- **test required:** cargo test -p x3-crosschain-gateway --features runtime-benchmarks
- **release gate affected:** runtime identity / benchmarks

### HIGH — `pallet-call-without-weights` — pallets/x3-custody/src/lib.rs:535

- **id:** `abfc948c61561e62`
- **symbol:** `x3-custody::Weight::from_parts(10_000, 0`
- **why it matters:** 10 extrinsic(s) charge an invented literal weight and the pallet has no generated weights at all, while being registered in runtime/src/lib.rs
- **suggested fix:** add a WeightInfo trait, generate weights with the FRAME benchmark CLI (`scripts/run-frame-benchmarks.sh`), and point the runtime at SubstrateWeight<Runtime>
- **test required:** cargo test -p x3-custody --features runtime-benchmarks
- **release gate affected:** runtime identity / benchmarks

### HIGH — `pallet-call-without-weights` — pallets/x3-dapp-hub/src/lib.rs:247

- **id:** `ae270b6375793ccb`
- **symbol:** `x3-dapp-hub::Weight::from_parts(10_000, 0`
- **why it matters:** 8 extrinsic(s) charge an invented literal weight and the pallet has no generated weights at all, while being registered in runtime/src/lib.rs
- **suggested fix:** add a WeightInfo trait, generate weights with the FRAME benchmark CLI (`scripts/run-frame-benchmarks.sh`), and point the runtime at SubstrateWeight<Runtime>
- **test required:** cargo test -p x3-dapp-hub --features runtime-benchmarks
- **release gate affected:** runtime identity / benchmarks

### HIGH — `pallet-call-without-weights` — pallets/x3-partner/src/lib.rs:256

- **id:** `5a4a4346f22f5a32`
- **symbol:** `x3-partner::Weight::from_parts(60_000_000, 0`
- **why it matters:** 8 extrinsic(s) charge an invented literal weight and the pallet has no generated weights at all, while being registered in runtime/src/lib.rs
- **suggested fix:** add a WeightInfo trait, generate weights with the FRAME benchmark CLI (`scripts/run-frame-benchmarks.sh`), and point the runtime at SubstrateWeight<Runtime>
- **test required:** cargo test -p x3-partner --features runtime-benchmarks
- **release gate affected:** runtime identity / benchmarks

### HIGH — `pallet-call-without-weights` — pallets/x3-wallet-pallet/src/lib.rs:218

- **id:** `b9033896ff6205da`
- **symbol:** `x3-wallet-pallet::10_000`
- **why it matters:** 12 extrinsic(s) charge an invented literal weight and the pallet has no generated weights at all, while being registered in runtime/src/lib.rs
- **suggested fix:** add a WeightInfo trait, generate weights with the FRAME benchmark CLI (`scripts/run-frame-benchmarks.sh`), and point the runtime at SubstrateWeight<Runtime>
- **test required:** cargo test -p x3-wallet-pallet --features runtime-benchmarks
- **release gate affected:** runtime identity / benchmarks

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
