# Runtime WASM reproducibility — verified 2026-09-19

Mainnet governance attests to a runtime hash; that hash is only meaningful if
anyone can rebuild the same WASM from the same source. This records a
reproducible build of `x3-chain-runtime` and the hashes it produced.

## How

```bash
# one pinned image, so every builder uses the same toolchain
docker pull paritytech/srtool:1.93.0-0.18.4   # = paritytech/srtool:1.93.0
docker run --rm \
  -e PACKAGE=x3-chain-runtime -e RUNTIME_DIR=runtime \
  -v "$PWD":/build paritytech/srtool:1.93.0-0.18.4 build
```

`scripts/run-srtool.sh build` wraps this (it also creates `runtime/target` with
the permissions the image's `builder` user needs — see that script).

| | |
| --- | --- |
| image | `paritytech/srtool:1.93.0-0.18.4` |
| image digest | `sha256:8638a668bd6d29111dc01953fbead6eb08c062e1cc62d3047a245a52b6edb3bf` |
| srtool | 0.18.4 |
| rustc in image | 1.93.0 |
| source | `master` @ the commit this file was added on |

## Result: reproducible at a given revision

The second run was a from-scratch rebuild (`runtime/target/srtool` removed
first), so this is not one artifact reported twice.

The values below were first established at `deab51f7f`, the revision that added
the cross-chain-gateway header anchor (`EvmHeaderAnchor`), re-attested unchanged at
`04fb44907` (the minimal-RLP EVM signature fix, which lives behind `std` and is
never instantiated by the runtime), and then **re-attested twice with different
bytes**:

* `5e0f86760` — the Bitcoin header path: proof of work is checked over Bitcoin's 80
  wire bytes instead of the SCALE encoding of a struct carrying a non-wire `height`
  field, negative/overflowing/zero targets are rejected as Bitcoin rejects them, and
  `spec_version` moves to 12.
* `9fe02ac37` — the cross-chain gateway derives withdrawal ids with
  domain-separated Blake2b-256 instead of XOR mixing, which collided (`"A" * 64` and
  `"B" * 64` produced the same id, and that id keys both the pallet's `Withdrawals`
  map and the relayer's processed set), and `spec_version` moves to 13.
* `93fe86c9bd` — the BTC SPV path gets a trust root: `anchor_btc_checkpoint`
  (root) commits this chain to a Bitcoin height/hash in write-once
  `BtcCheckpoints`, `BtcPoWLimitBits` carries Bitcoin's per-network `powLimit`, the
  single admission path `btc_admit_header` derives heights from the parent link and
  enforces Bitcoin's `nBits` and median-time-past rules, and both SPV entry points
  accept evidence only from a checkpoint-anchored chain. New storage and a new call,
  so `spec_version` moves to 14.
* `e74bb199f9` — the BTC tests now run on data a real Bitcoin Core v28.1.0 regtest
  node produced (header, merkle path and a confirmed transaction), and the SPV entry
  points document which transaction serialization they take. `spec_version` is
  unchanged at 14: **the bytes still move**, because pallet documentation is part of
  the runtime metadata, and metadata is in the WASM. That is the reason this gate
  watches the whole dependency graph rather than a hand-kept list of "runtime files".
* `2a4296b630` — the SPV trust root can be pinned in genesis: `X3SettlementEngine`'s
  `GenesisConfig` carries `btc_checkpoints`, validated as genesis is built and then
  admitted onto the anchored chain, so a testnet can start with the root of trust in its
  spec. `BtcBlockHeader` gains serde for the spec's benefit only. `spec_version` moves to
  15.
* `269d611d96` — formatting only, and the runtime still reports `x3-chain-15`; but the
  bytes moved by two. `cargo fmt` re-wrapped lines in the pallet, and an `assert!`'s
  message carries the `file:line` it was written at, so where a line sits is part of the
  artifact. Nothing about the format of a change is invisible to this gate, which is the
  point of measuring bytes rather than intentions.
* `d471adfec4` — validator key rotation is wired to the on-chain custody registry:
  `pallet_x3_custody` gains `KeyRotationPeriod` and `rotate_validator_key` grants
  `current_block + period` instead of inheriting an overdue due block; the node gains
  the `validator rotate` operator command. `spec_version` moves to 16.
* `93d97edd34` — the merge of the formatting fix with that key-rotation change. Both
  revisions were attested separately and neither record described the other's tree, which
  is what a single record for a single artifact means: at any moment the file describes
  **the revision it names**, and a merge of two attested revisions is a third revision
  that has to be rebuilt. Two from-scratch builds of the merge agree, and the compact
  artifact is 8,468,726 bytes.
* `e28a0907be` — the Bitcoin header path gains a batch submission
  (`x3SettlementEngine.submitBtcHeaders`, call_index 35, up to 100 headers per call, atomic so
  a batch refused partway applies nothing) and a `BtcHeaderOrigin` config item, which this
  runtime sets to `EnsureRoot`. `spec_version` moves to 17. Two from-scratch builds of
  `e28a0907be` agree, and the compact artifact was 8,475,400 bytes at that revision.
* `a7878e933e` — the BTC median-time-past rule stops being stricter than Bitcoin's: with fewer than
  eleven ancestors (the first headers above a checkpoint) the median is not computable from stored
  history, so the check is skipped instead of using the parent as a stand-in, which refused real
  headers. `spec_version` moves to 18. Two from-scratch builds of `a7878e933e` agree, and the compact
  artifact is 8,474,849 bytes.
* `5ec1b61576` — formatting only, and the hashes did not move: collapsing a multi-line `ensure!` into
  one line leaves the macro's `file:line` where it was, so this is the first revision in this list
  whose bytes are identical to its predecessor's. `recorded_revision` moved; nothing else did.
* `7ea1819169` — Bitcoin merkle verification in `x3-bitcoin-vault` becomes position-aware, and the
  pallet's tests now compare all three implementations on one real regtest block. The hashes are
  again identical to the previous revision's: the vault is a *dev*-dependency of the pallet, and the
  intent crate's change is a doc comment, so nothing the runtime instantiates moved. Two revisions
  in a row where only `recorded_revision` changes is worth noticing: it means the gate is measuring
  what it claims to measure rather than reacting to any edit in the graph.
* `0d296d236b` — the atomic kernel's `do_finalize_bundle` requires the bundle to be `Executing`
  instead of accepting a `Pending` one and leaving the refusal to `verify_bundle_consistency` two
  checks later, and five tests now cover the finalization entry point (the alarm TICKET-097 has to
  change). **The bytes move again**: the pallet is instantiated by the runtime, so this is not a
  dev-dependency revision. `spec_version` deliberately stays at 18 — every reachable call accepts
  and refuses exactly what it did before, a `Pending` bundle being refused both before and after,
  only earlier and with a different error code, and no state transition changes. Two from-scratch
  builds of `0d296d236b` agree: compact 8,474,849 bytes (same size as the previous revision,
  different bytes) and compressed 1,453,609.
* `752a334759` — the runtime stops holding a privileged account. `X3LangOrigin` and
  `SettlementOrigin` were `EnsureSignedBy<{X3Lang,Settlement}GatewayAccount, _>`, and those
  constants are the accounts of the public phrases `//x3-atomic-gateway` and
  `//x3-settlement-gateway`, so every chain this runtime built granted the atomic kernel, the
  router and settlement finalization to anyone who read the repository. They now read
  `pallet-x3-custody`'s genesis-configured `AuthorizedGateways`; a chain that names no gateway can
  no longer assign or finalize a bundle. `spec_version` moves to 19 — new storage, two new calls,
  and an origin that answers differently for the same input — so the runtime reports `x3-chain-19`
  and the compact artifact grows to 8,482,706 bytes. Two from-scratch builds agree.
* `ae4a4c912b` — the unsigned finalization extrinsic is removed. `submit_finalization_result`
  was `ensure_none`, its off-chain marker had no writer anywhere in the repository, and the
  certificate it checked was anchored by an equally unsigned call — so a caller planted the value
  it was about to be checked against and any account could finalize any `Executing` bundle
  (TICKET-097). Finalization is only the signed `finalize_atomic_bundle` (`X3LangOrigin`) and
  `finalize_with_settlement` (`SettlementOrigin`) now. One call and one weight entry leave the
  metadata, `spec_version` moves to 20, and the artifact moves again — compact 8,482,928 bytes
  (was 8,482,706), compressed 1,459,662 (was 1,458,035) — which makes it the first
  revision in this list where a *removal* changes the bytes.
  Two from-scratch builds agree.
* `cc19883faf` — two test files that never compiled are replaced by four that do, and the
  formatting drift the last three merges left behind is repaired. The hashes are **identical to
  `ae4a4c912b`'s**: `runtime/src/tests.rs` and `pallets/x3-atomic-kernel/src/tests.rs` are
  `#[cfg(test)]` code, which the WASM build does not compile, and the rest is `cargo fmt`. Only
  `recorded_revision` moves. That is the third revision in this list where that happens, and it is
  the property the gate exists for: it watches the runtime's dependency graph rather than the
  timestamp of the last edit.
* `8ba9f897f` — the storage/RPC/validator-ops audit. The treasury, agent-accounts and agent-memory
  migration modules read their pallet's declared `STORAGE_VERSION` instead of restating a literal,
  and five `migrations.rs` files that no crate declared are deleted. The bytes move this time:
  compact 8,485,072 bytes (was 8,482,928), compressed 1,458,341 (was 1,459,662) — the compressed
  artifact shrinks while the compact one grows, which is exactly why the record is rebuilt rather
  than reasoned about. Two from-scratch builds agree.
* `877c37035` — the confidential-execution attestation is verified rather than merely non-empty,
  and the wallet's signature verification is real. `pallet-private-execution` gains a required
  `TeeAttestationVerifier` whose shipped default refuses every report, so the runtime registers no
  confidential validators instead of accepting `vec![1]`; the wallet crate's verifier stops
  returning "not implemented" and checks Ed25519/Sr25519 over a stated message. The bytes move:
  compact 8,488,288 bytes (was 8,485,072), compressed 1,460,543 (was 1,458,341). Two from-scratch
  builds agree.
* `3e3ecb8a6` — the kernel's API surface is consolidated and the X3 receipt gains a client accessor.
  `pallets/x3-kernel/src/runtime_api.rs` (a second, uncompiled trait declaration) is deleted, the TS
  SDK calls `AtlasKernelRuntimeApi_get_canonical_balance` instead of a method no runtime metadata has,
  and `AtlasKernelRuntimeApi` gains `get_x3_execution_receipt`, asserted over the wire by
  `state_call` in the live lifecycle test. The bytes move: compact 8,501,495 bytes (was 8,496,845),
  compressed 1,461,704 (was 1,461,148). Two from-scratch builds agree — and the compressed BLAKE2_256
  (`0xd0996f91...`) matches the cache-mounted single build taken before the record was written, which
  is a second check that mounting a cargo cache does not change the artifact.
* `215c01fe6` — the X3 execution receipt is persisted. `submit_comit_v2` writes
  `X3ExecutionReceipts[comit_id]` only for an accepted comit, `STORAGE_VERSION` moves 1 -> 2 (the map
  starts empty; the migration only records the version), and the extra write is declared at the call
  site. The bytes move: compact 8,496,845 bytes (was 8,489,301), compressed 1,461,148 (was 1,460,962).
  Two from-scratch builds agree.
* `496242b64` — the X3 payload convention is fixed: `submit_comit_v2` validated a routing packet and
  then handed those bytes to an adapter that parses X3BC, so no real chain could execute an X3
  program. The payload is the compiled program now and validation is the adapter's own. The bytes
  move: compact 8,489,301 bytes (was 8,488,288), compressed 1,460,962 (was 1,460,543). Two
  from-scratch builds agree.
* `6b3e241eb` — the EVM interpreter the runtime executes is the maintained crate.
  `crates/evm-integration` pinned `evm` to a rev of `rust-blockchain/evm`, which the upstream project
  has left, so the node compiled two interpreters: ours (`evm 0.39.1` + `ethereum 0.14.0`, the pair
  carrying the Dependabot advisories) and Frontier's (`evm 0.43.4`). It depends on
  `rust-ethereum/evm.git` `branch = "v0.x"` now — Frontier's spec verbatim, because a `rev` is a
  different git SourceId and would have kept the second copy of the interpreter in the node.
  `mini_evm::execute_evm`, which had no test at all, now has 13 unit tests plus five across the two
  `tests/` files that had been disabled by `#![cfg(any())]`. The bytes move: compact 8,506,569 bytes
  (was 8,501,495), compressed 1,463,589 (was 1,461,704). Two from-scratch builds agree.
* `971ae2908` — the X3 executor's gas limit is enforced, and the two bytecode readers stop
  allocating from the module. `X3Executor::execute` built its VM with `VM::from_bytes`, which uses
  `VMConfig::default()`, so the gas limit, call depth and stack size in `X3ExecutorConfig` were
  discarded: a measured 100-gas limit admitted a 2,000-instruction program. Separately,
  `mini_x3` (the reader `pallets/x3-kernel` executes on chain) and `x3-backend` both did
  `Vec::with_capacity(count)` on a count read straight out of the module, so `const_count =
  0xFF00_0001` asked for 95 GB — the on-chain one aborted with "memory allocation of
  102676561944 bytes failed". Both are bounded by the remaining input now. The bytes move:
  compact 8,506,628 bytes (was 8,506,569), compressed 1,463,682 (was 1,463,589). Two
  from-scratch builds agree.
* `6dbdbf051` — the SVM interpreter charges what it burned. `execute_bpf` caps its fuel at
  MAX_INSN_FUEL and then reported `config.compute_unit_limit - vm.fuel`, so any limit above the
  cap counted the gap as spent: a two-instruction program under a 2,000,000 unit limit reported
  1,000,002 units, and the pallet charges that number. It reports against the fuel actually granted
  now, and `validate_program` applies the same `.text` checks `execute_bpf` does, so a payload that
  validates can no longer be refused at execution for a reason validation could have seen. The
  bytes move: compact 8,506,630 bytes (was 8,506,628), compressed 1,463,693 (was 1,463,682). Two
  from-scratch builds agree.
* `a7ff06572` — the runtime's build script stops embedding a WASM blob from another feature set.
  `substrate-wasm-builder` decides freshness from source timestamps and this crate's feature set is
  not a source file, so a blob built with `runtime-benchmarks` survived a build without it and the
  node embedded a runtime whose host functions it does not provide — `target/release/x3-chain-node`
  could not start, on any `--chain`. The build script now reads its feature sidecar *before* the
  build as well, and removes the stale outputs so the builder has to produce one for this feature
  set. **The bytes do not move**: the new code runs only on the host path (`TARGET` is
  `wasm32-unknown-unknown` inside the WASM build, where the script returns early), so this is a
  revision-only record. compact 8,506,630 bytes, compressed 1,463,693, unchanged. Two from-scratch
  builds agree.

* `aaece429d` — the kernel stops calling a packet an execution. `submit_comit_v2` validates a
  non-empty EVM/SVM payload as a SCALE-encoded `Packet` while the adapters execute their input as EVM
  bytecode / eBPF, and a packet run as code halts on its own enum discriminant — `0x00` is EVM `STOP`
  — so the chain recorded `success: true` with base gas for an operation that never ran
  (`PayloadValidation`… measured: `WasmEvmAdapter::execute(wrap_evm_payload(&[0xAA; 64]), 6_000_000)`
  returned success, 22,576 gas, for a 124-byte packet). All four adapter sites now refuse a packet by
  name. This is a runtime change — `packet_adapters.rs` and `wasm_adapters.rs` are compiled into the
  blob — so the bytes move: compact 8,508,157 bytes (was 8,506,630), compressed 1,463,840 (was
  1,463,693). Two from-scratch builds agree.

* `a3d918abb` — a payload is the artifact its adapter executes (X3-LANG-004). The kernel required a
  non-empty EVM/SVM payload to SCALE-decode as a `Packet`, while every adapter executes its input as
  code; SCALE puts the enum discriminant first, so `Packet::Evm(..)` begins `0x00` — EVM `STOP` — and
  an accepted payload was receipted as a success for work that never happened. Both validation sites
  now ask `T::EvmAdapter::validate` / `T::SvmAdapter::validate`, the fixtures are real bytecode and
  eBPF programs, and the mock and native adapters refuse packets by name. This is a runtime change:
  the bytes move — compact 8,508,725 bytes (was 8,508,157), compressed 1,465,616 (was 1,463,840). Two
  from-scratch builds agree.

* `6ebdd35bd` — an external root needs a verifier, and the per-asset ledger gets a read
  surface. `register_external_root` accepted any non-empty byte string as "proof against chain
  consensus" and stored the caller's root where the bridge surface trusts it; it now calls
  `Config::ExternalRootVerifier`, and this runtime wires `RefuseExternalRoots`, so the call
  fails closed with `ExternalRootVerificationUnavailable` until a per-chain light client exists.
  `AtlasKernelRuntimeApi::get_asset_supply_ledger(asset_id)` returns the ledger's own record, so
  the per-asset invariant has a read path over RPC. The bytes move: compact 8,512,083 bytes (was
  8,508,732), `set_code 0xfb92492d…`. Two from-scratch builds agree.

  Note on the record: `docs/reports/runtime-wasm-hashes.json` names the revision the *script* was
  run at (`335a27d8c`), while these bytes were built from the working tree that became commit
  `6ebdd35bd` — the record was written before that commit existed, and the field cannot name a
  commit that contains it. The check that matters is stage 6b of `make mainnet-check`, which
  rebuilds from the revision and requires every hash to match; it is green on `6ebdd35bd`.

* `43099919a` — the adapter story told the truth. `pallets/x3-kernel/src/adapters.rs` and
  `lib.rs` still advertised a `FrontierEvmAdapter` as the production EVM adapter; it returned
  `ExecutionReceipt { success: true, gas_used: <derived from payload length>, .. }` for code it never
  executed, and the EVM arm is executed instead by the runtime's own `NativeEvmAdapter` (std) or
  `WasmEvmAdapter` (wasm). It is deleted, the SDK's `evm_deploy` builds init code rather than a
  `Packet` that halts on byte 0, and two accept-anything verifiers on the proposal and receipt paths
  were replaced with real ones. The bytes move: compact 8,508,732 bytes (was 8,508,725), compressed
  1,465,245 (was 1,465,616). Two from-scratch builds agree.

* `9125c487a` — the record catches up with a day of runtime-graph work, and eighteen pallets stop
  charging zero. `docs/reports/runtime-wasm-hashes.json` had stood at `335a27d8c` while the gate
  counted **64 runtime-graph files** moved under it (the halt's exemption list and its recovery
  path — the
  finality anchor derived from the chain's own `block_hash`, the finality-certificate producer, the
  ordering window's settle-weight fix, the wallet's signature verification, and finally the
  `WeightInfo` wiring). `runtime hash freshness` named every one of them and refused to pass; it
  passes now.
  The bytes move: compact 8,748,372 bytes (was 8,512,083), compressed 1,512,695 (was 1,464,553).
  Two from-scratch builds agree, and `recorded_revision` names this commit — the field can only name
  a revision that already exists, so this is the first record in this list written from a tree that
  was already committed.

  The weight change is worth naming because it is the one that altered *behaviour* without altering
  the ABI: `runtime/src/lib.rs` now points eighteen pallet configs at the `SubstrateWeight` their
  pallets already shipped instead of `type WeightInfo = ();`, whose `impl WeightInfo for ()` returns
  `Weight::zero()`. Dispatchables that were invisible to the block weight limit are now charged.
  Those values are the generated stubs (hand-sized `ref_time` with real read/write counts), not
  benchmarks — `X3-GPU-003` is the row that measures them.

* `248435935` — the private-submission policy reaches the chain, and the bytes move for real. Since
  the previous record the runtime graph gained the whole MEV-002 path
  (`crates/x3-common`'s feature-word reader, `crates/x3-backend`'s alias for the same bit,
  `crates/x3-compiler`'s policy-carrying entry, `pallets/x3-kernel`'s intake guard and its
  `PrivateSubmissionChannel` binding in `runtime/src/lib.rs`), the cross-VM router's fee fix, the
  RPC-quorum fix, and the `mainnet-rc1` variant compile fix. The bytes move:
  compact 8,742,756 bytes, `0x1ea62909…`; compressed 1,510,586 bytes, `0x50c5a499…` (was
  `0xce698445…` / `0x4971c1fc…`). Two from-scratch builds agree, and `runtime hash freshness`
  passes against the new record.

  One operational note this run produced: the checkout's root had lost its `o+rx` bit
  (mode `700`), which the srtool image cannot enter as uid 1001 — the build failed in **3 seconds**
  with a message naming the exact `chmod`. Worth keeping in mind: the failure is not a build
  failure, but it looks like one in a log that only says `build 1/2`.

* `06b79a939` — **the bytes move this time, and the reason is real.** This record was written by
  `./scripts/update-runtime-hashes.sh` after two from-scratch `srtool` builds of `06b79a939`
  agreed, and it replaces the `d62fecf6b` entry that had been carried since the halt-recovery
  commit. Seventeen files in the runtime's dependency graph changed between the two revisions, and
  unlike the test-only moves above, these are compiled into the artifact: the private-submission
  demand now reaches the chain-intake compiler (`crates/x3-compiler`, `crates/x3-backend`,
  `crates/x3-common`, `crates/x3-integration`) and the kernel enforces it at the door
  (`pallets/x3-kernel`), the router no longer fails a transfer on a fee the treasury cannot accept
  (`pallets/x3-cross-vm-router`), the secret-release firewall's quorum bar became policy
  (`crates/x3-atomic-swap`), and the `mainnet-rc1` runtime variant was fixed so it compiles
  (`runtime/src/lib.rs`). So:
  compact 8,742,756 bytes (`0x1ea6290946e35b06fb2b24446ab2e58721f8236a890cd87e5b72ec11ae448b10`),
  compressed 1,510,586 (`0x50c5a4999cbe17fe804aae5535a3ac0de8e8f0ff897a4ea502559df573193021`) —
  both different from `d62fecf6b`'s `0xce698445…` / `0x4971c1fc…`, which is what a revision with
  real runtime changes is supposed to look like.

  One thing this entry is *not*: a statement that the two builds used the same sources as each
  other. An earlier attempt at this re-attestation was refused by the script for exactly that
  reason — `c85c7b207` landed between its two builds, so they disagreed by one build's worth of
  source, and the script wrote nothing. That is the guard working, and it is worth knowing that the
  window is real: this repository takes commits while a 20-minute double build runs.

* `c62f93200` — **the Northern Swarm / Reactor merge reaches the runtime, and `spec_version` stays
  at 20 on purpose.** `construct_runtime!` gains `NorthernSwarm` in every variant, which is new
  storage and new calls, so the bytes move for real: compact 8,882,167 bytes
  (`0x3a6d6d13a30237307e40b4559e3a752f448fff227034648c0ee5b49b642a18b2`) — was 8,742,756 — and
  compressed 1,524,730
  (`0x7c3b8721360bc3a603ca59f0a9701724bf6aff0068b26bb98da0a3cfc2e10e78`) — was 1,510,586. The
  pallet's own weights are *measured*: the shared template had been reading `{{cmd.low_range}}` and
  `{{benchmark.base_proof_size}}`, fields the pinned CLI no longer emits, so it rendered
  `Weight::from_parts(N, )` and eight E0611-class compile errors; the template is now the pinned
  polkadot-sdk one and every generated weight carries a proof size.

* `4e1f5bdcf` — **a disputed task refunds its submitter, and the runtime import order is
  formatted.** `pallet-northern-swarm` gains `resolve_disputed_task` (call index 8), so the
  runtime the launch binary builds has a new call and the bytes move again: compact 8,888,727 bytes
  (`0x18054a6594839f4d793a62ac221ee8fdb50f45a5a81acbcbc1270a464af55f0e`) — was 8,882,167 — and
  compressed 1,527,036
  (`0xba83a129e83dc98b61415bb15110658d53ab4d582ef2c58bee43a848d3955496`) — was 1,524,730. The
  extrinsic's weight is measured, not hand-written: the first attempt put a
  `Weight::from_parts(0, 0)` entry into the generated file, and it was replaced by benchmark CLI
  output (`Measured: 548`, `Estimated: 4149`, `Weight::from_parts(96_270_000, 4149)`). The second
  half of this revision is cosmetic: the merge had appended
  `use pallet_northern_swarm::Pallet as NorthernSwarm;` out of order in `mod benches`, which left
  `cargo fmt --all -- --check` red on master from `975e58eea` until `3707883a8`.

  Why no `spec_version` bump here, despite the rule two entries up ("new storage and a new call, so
  `spec_version` moves"): this repository does not bump in-tree for a release. `spec_version` in
  `runtime/src/lib.rs` is the **before** side, and
  `scripts/mainnet/build_runtime_upgrade_artifact.sh` builds the **after** side by incrementing it by
  exactly one on top of the revision being released — it refuses a worktree that already carries a
  bump, or one whose `runtime/src/lib.rs` differs from the revision by more than that one line. So
  the upgrade rehearsal's `spec_version_before`/`spec_version_after` pair is 20 → 21, and the
  identity baseline gate (`scripts/ci/verify_runtime_identity_baseline.sh`) is what stops the
  in-tree value moving silently — it pins `RuntimeVersion`, the `construct_runtime!` pallet
  order that decides every pallet index, and the `SignedExtra` order that is part of every
  signed payload. It had been dead: it required `RuntimeVersion.state_version`, a field the
  pinned polkadot-sdk revision removed, so every run exited with
  `::error::RuntimeVersion.state_version missing`, and nothing ran it to notice. It now runs in
  the default gate set (`runtime identity baseline`), and `docs/reports/runtime-identity.baseline.json`
  was regenerated at the value above with `accepted_reasons` naming why each field reads what it
  reads.

  Both records were written by `./scripts/update-runtime-hashes.sh` after two from-scratch builds
  agreed. The current one names `23197c6d3`, the revision the runtime's dependency graph last moved
  at; `runtime hash freshness` is what keeps that true, and it is the reason the `c62f93200` record
  was retaken rather than assumed. Earlier in the night it read as a 40-minute false positive on
  `runtime/runtime-identity.baseline.json` — a checked-in *record* that no source names, which
  `83de5a64a` moved out of the graph's package directory and taught the checker to recognise.

Bytes changing is the intended behaviour: a revision that a mainnet governance
motion attests to has to be named, and the hash has to describe the artifact that
revision builds.

`recorded_at` and the top-level `runtime_version` are now written by
`./scripts/update-runtime-hashes.sh` from the same srtool block the hashes come
from (each runtime block also carries its own `version`/`metadata`, and stage 6b
compares them). Until 2026-09-22 those fields were carried over untouched: the
record described `x3-chain-11` for a wasm that reported `x3-chain-12`, and nothing
noticed.

They are **not** a fixed property of the project: any runtime-affecting change
produces different WASM, which is why `docs/reports/runtime-wasm-hashes.json`
names the revision it was recorded at, and why
`scripts/mainnet_release_gate.py` **rebuilds and compares** rather than trusting
the file. Cutting a release means rebuilding from the revision being released,
confirming two builds of *that* revision agree, and updating the record in the
same change.

Earlier pairs of builds (compact `0x78feb683…`, `0xb1f0348c…`, `0xb1777a09…`) were
taken at older revisions and are kept here only as the record of how this was
established. `setCode` for the compact artifact is `0xac13ee1b…`, quoted with the
other values below.

**Compact (`x3_chain_runtime.compact.wasm`, 8,512,083 bytes — revision `6ebdd35bd`)**

```
Version          : x3-chain-20 (x3-chain-1.tx1.au1)
Metadata         : V14
setCode          : 0x568105227d11285fbad532780b7646f2b8311dc565d2ac225e4d9441f421ca11
authorizeUpgrade : 0x76b489c790351b452482dae7981e2824cf9ec76b8568afeb37a987fa6473ff14
IPFS             : QmbrH7E8EfV7ivRcz5qvfDfHwHFpxaidTQHUV6iRmPrrzo
BLAKE2_256       : 0xf272e2f05eef5302664670e5b5a3a90b8b26afdeb8089397370d09c9a1d906e5
```

**Compressed (`x3_chain_runtime.compact.compressed.wasm`, 1,465,245 bytes — revision `43099919a`)**

```
setCode          : 0x77247760d5d0c876eabc7a490fc63c3c23a1dfc554d003dd92ebfe662245bcf3
authorizeUpgrade : 0x43adaa2e231de7642e1fa5cfb37e04f5dfe85b6d62270db932068229fc8ba0a2
IPFS             : QmYas4jvNZmXkUMbGPxxGpbdj8ckzcgaob2MpcC1Qee4qL
BLAKE2_256       : 0xad72d272abfa9a3c4e9b8b5ddc8f299cba1d59f5462a1ea8c70f14a788818bf8
```

Both runs produced these values byte for byte. The compressed artifact is the
one whose `setCode`/`BLAKE2_256` a release would attest to.

## What this does and does not prove

* **Does:** the runtime source in this revision builds deterministically inside
  that image — nothing in the workspace's build scripts injects a timestamp,
  path or machine-specific value into the WASM.
* **Does:** the release gate enforces it. Stage 6b of
  `scripts/mainnet_release_gate.py` runs this build and compares every value
  against `docs/reports/runtime-wasm-hashes.json`, so `make mainnet-check` fails
  if the runtime no longer builds to the recorded bytes. It adds about ten
  minutes to that gate; that is the price of the claim.
* **Does not:** prove the hash is the one an *audited* release commits to.
  Updating the record is part of cutting a release, and an audit is a separate
  step.

## Re-attesting

Any change that alters the runtime's bytes invalidates the record, and the
release gate will say so. Re-recording it is one command:

```bash
./scripts/update-runtime-hashes.sh          # build twice, then rewrite the record
./scripts/update-runtime-hashes.sh --check  # build twice, write nothing
```

It clears srtool's target directory between the two builds, compares every
field, and refuses to write anything if they disagree — a disagreement is a
reproducibility failure, not a stale record. Run it in the same change that
alters the runtime, so the record and the code land together.
* Toolchain note: the image must be able to build this dependency graph. Both
  `1.75.0` (the previous pin) and `1.88.0` refuse with
  `enum-ordinalize@4.4.2 requires rustc 1.89`.

* `ddec9b166` — **two pallets stop charging hand-written weights, and the record moves with them.**
  `pallets/atomic-trade-engine` had four calls (`register_liquidity_pool`, `update_liquidity_pool`,
  `sync_pool_price`, `submit_price_observation`) and `pallets/x3-kernel` had three
  (`emergency_pause`, `emergency_unpause`, `emergency_halt`) charging literals — the kernel's three
  were 10,000/15,000 picoseconds, i.e. a *freeze of the chain* for essentially nothing. Both pallets
  now carry benchmark-derived weights, so the bytes move for real: compact 8,884,011 bytes
  (`0xd45f2cc766c5531a74b97ae0fb58cb21c86b9d04020f444b2dc022b1bff431d4`) — was 8,888,727 — and
  compressed 1,525,686
  (`0xcbbe7219852b60bf436f3d76f07eb6642c290a9f428194ee525b7eff7bb7e5ae`) — was 1,527,036.

  `pallet-atomic-trade-engine` could not be benchmarked at all before this: it ships a
  `benchmarking.rs` but was missing from `mod benches` in `runtime/src/lib.rs`, so the CLI answered
  "No benchmarks found which match your input" and the four literals were never re-measurable.
  `pallet_x3_kernel` *was* registered, but two of its own benchmarks failed when re-run, so its
  weights could not be regenerated either: `register_asset` registered the default asset the
  development genesis already holds (`AssetAlreadyRegistered`), and `submit_comit` presented
  `prepare_root = H256::zero()`, which `verify_dual_vm_with_receipts` refuses on any build without
  the `dev-bypass` feature (`ComitVerificationFailed`) — the same trap `submit_comit_v2` had already
  had fixed. Both benchmarks now establish their own preconditions, and the full 13-benchmark pallet
  run rewrites `pallets/x3-kernel/src/weights.rs`. Scanner finding `pallet-call-without-weights`
  25 → 23.

* `730c608e5` — **the supply ledger's weights are measured, and its mint stops being free.**
  `pallets/x3-supply-ledger` charged literals for a governance **mint** (20,000 picoseconds), a
  **burn** (15,000) and three switches (10,000) while every one of them reads and writes `Ledgers`.
  The pallet now carries a generated `weights.rs`, a `benchmarking.rs` and a `type WeightInfo`, so
  the bytes move: compact 8,885,606 bytes
  (`0x227964acfaa64bde9a11e8ad8f4fe3b92f8312e0e5da871453cf056b8f812ac9`) — was 8,884,011 — and
  compressed 1,526,168
  (`0xd1019855e47ca229a7b3af6dba9c768a49d77a05d24a21f793cea89c3b187ac4`) — was 1,525,686.

  It could not be benchmarked at all before this, and the reason is worth keeping: the runtime's
  `runtime-benchmarks` feature list did not enable `pallet-x3-supply-ledger/runtime-benchmarks`, so
  the pallet's generated `impl Benchmarking` was `#[cfg(any(feature = "runtime-benchmarks", test))]`
  -gated out and `define_benchmarks!` failed with
  `Pallet<Runtime>: Benchmarking is not satisfied` — a message that names neither the feature list nor
  the pallet's own manifest. The mint and burn benchmarks register an asset through the asset registry
  (the development genesis holds none, and the ledger refuses an unknown asset), so they measure the
  state a chain can reach rather than a ledger seeded by hand. Scanner
  `pallet-call-without-weights` 23 → 22.

* `af80888a6` — **the treasury policy's eight calls stop charging literals.** `pallets/x3-treasury-policy`
  had hand-typed weights on every dispatchable — a settlement vault funding at 80,000,000 picoseconds,
  a cap at 50,000,000 — behind a `runtime-benchmarks` feature that had nothing in it and that the
  runtime's feature list did not enable either, so `define_benchmarks!` could not see the pallet at
  all. It carries a generated `weights.rs`, a `benchmarking.rs` and a `type WeightInfo` now, and the
  bytes move: compact 8881232 bytes (`0x88164e76c0d4207735324e0413cfe58fe8fd0bec4ed6a61d56bb49101bc92f98`) — was 8,885,606 — and compressed
  1526056 (`0xa764ea628bde1995afe7fd41e654acb92a37ec85d4d585e9771c610cad7bd4d5`) — was 1,526,168.

  The vault-funding benchmark creates its vault with the inventory pallet's own root-only
  `create_vault` and sets an allocation cap and an operator threshold first, because the default
  threshold of zero sends every non-zero funding action into the governance queue — measuring that
  branch would have missed the immediate-apply path a chain takes. Scanner
  `pallet-call-without-weights` 22 → 21.

* `69a58d4fe` — **the token factory measures its launch, and a cross-crate regression the gate missed.**
  `pallets/x3-token-factory` charged literals for a token launch (60,000 picoseconds), a mint and a
  burn (20,000 each) and an authority handover (10,000), behind a `runtime-benchmarks` feature that
  had nothing in it. It carries a generated `weights.rs`, a `benchmarking.rs` and a `type WeightInfo`
  now: compact 8894628 bytes (`0xcaab18521a8364f1f0ea1ef3e036a357478ed50749c9fff8fa32de53433746c7`) — was 8,881,232 — and compressed 1524541
  (`0x6399941a9a001b39f5cca3bd224a287d23c83417d12ec60842f146da58f05824`) — was 1,526,056.

  The launch benchmark drives the whole path a chain takes — register the asset, activate it,
  configure the internal routes, mint the initial supply — and the mint and burn benchmarks use the
  class that permits each (`CappedMintable` for a post-launch mint, `Burnable` for a burn), because no
  single class allows both.

  This revision also repairs a regression the release gate did not catch: the supply ledger's new
  `type WeightInfo` (two revisions back) broke `cargo test -p pallet-x3-token-factory` and
  `cargo test -p pallet-x3-cross-vm-router`, whose test runtimes implement that `Config`, while
  `make mainnet-check` stayed green because it runs a subset of packages. Adding an associated type
  to a pallet's `Config` touches every crate that wires it. `cargo check --workspace --all-targets` is
  the check that sees it, and it is now run after every weights pass. Scanner
  `pallet-call-without-weights` 21 → 20.

* `891a47f96` — **the sequencer and the DA pallet measure their calls.** `pallets/x3-sequencer`
  charged 10,000 picoseconds for `submit_transaction` while reserving a per-byte fee, bumping the
  global sequence and pushing into the pending batch; `pallets/x3-da` charged 15,000 for a blob
  commitment and 10,000 for a shard proof. Both declared `runtime-benchmarks` features with nothing
  behind them. They carry generated weight files now: compact 8888569 bytes (`0x1570b59530b954d0e6e99ed64158728461acd3eb282a76320a08d040dc742d6a`)
  — was 8,894,628 — and compressed 1524965 (`0xb8088b091b6116540d0a2e693e4e3bff9c0a1eca92c3848c34e9eb7ca23b8124`) — was 1,524,541.

  The DA pallet's shard-proof benchmark commits the blob it attests to first, through the pallet's own
  extrinsic, because `BlobNotFound` is the guard and the commitment has to exist. Both benchmarks
  measure the fee reserve as part of the call, since `ReservableCurrency::reserve` is inside it.
  Scanner `pallet-call-without-weights` 20 → 18.

* `9f5446270` — **the flash loan and the reservation pallet measure their calls.** `pallets/x3-flashloan`
  charged 10,000 picoseconds to borrow or repay and 5,000 to add liquidity; `pallets/x3-reservation`
  charged 10,000 for each of its three root transitions — a request that locks real vault inventory
  and increments the lane's unsettled notional, and the two terminal transitions that undo it. Both
  declared `runtime-benchmarks` features with nothing behind them. They carry generated weight files
  now: compact 8895788 bytes (`0x343f40ba483afd6550234e05810269abc4b73c4512a1ee8d196eaeb46673cebf`) — was 8,888,569 — and compressed 1524612
  (`0x942851a920761e1d35ec741852572e4296c5f6de63bad1870aacbc193084b094`) — was 1,524,965.

  Two traps live in this revision. `define_benchmarks!`'s location name must match the
  `construct_runtime!` pallet alias exactly — `X3FlashLoan`, not `X3Flashloan` — because the macro
  resolves that name at the crate root, where a `use` alias declared inside `mod benches` is not in
  scope; the failure is reported as `cannot find type X3Flashloan in this scope` on the macro line.
  And a benchmark's amounts have to clear the chain's existential deposit: the flash loan's 1,000-unit
  pool passed in the mock and failed on the dev chain with "Account cannot exist with the funds that
  would be given", so its amounts are multiples of `minimum_balance()` now. The reservation benchmark
  requires the chain's balance to be `u128` and uses real amounts for the same class of reason: the
  inventory helpers return early for zero, and a zero-amount benchmark measures the no-op path.
  Scanner `pallet-call-without-weights` 18 → 16.

* `382f4c8da` — **the domain registry and the reconciliation pallet measure their calls.**
  `pallets/x3-domain-registry` charged 20,000 picoseconds to register a domain and 30,000 to set its
  records; `pallets/x3-reconciliation` charged 10,000-30,000 across six calls — a chain supply report,
  the canonical supply, the reconciliation run itself, the halt lift, and two governance-power calls.
  Both declared `runtime-benchmarks` features with nothing behind them. They carry generated weight
  files now: compact 8890492 bytes (`0x84ae71b3d391d407c06d7a4cb4789ae7d2e58cd480c6f13fc811e37201785db5`) — was 8,895,788 — and compressed
  1526313 (`0xbe7d4de0bd5020c3ebc2c86ac4f21e83c1719da09e40f60328844d52fbd25a02`) — was 1,524,612.

  A benchmark module also compiles into this runtime's WASM build, where `Vec` and `vec!` are not in
  the prelude: the domain registry's `Vec<X3DnsRecord<T>>` helper needed `use sp_std::{vec, vec::Vec}`,
  and the failure surfaced as `cannot find type Vec in this scope` from the build script, not from the
  pallet's own test run, which passes with `std`. The reconciliation benchmarks set the state their
  calls read through the pallet's own extrinsics. Scanner `pallet-call-without-weights` 16 → 14.

* `eac5255ce` — **the wrapped pallet and the sentinel measure their calls.** `pallets/x3-wrapped`
  charged 8,000-20,000 picoseconds across seven calls; `pallets/x3-sentinel` charged 15,000 on each
  of seven — and each of those is a security power: freezing an authority's supply-changing rights on
  an asset, freezing the asset, enrolling it for guardian review, granting an approval. Both declared
  `runtime-benchmarks` features with nothing behind them. They carry generated weight files now:
  compact 8888748 bytes (`0xd15e080511871e250c3d6674bc5bf3bdf115107cde087c49801518b01be59478`) — was 8,890,492 — and compressed 1528293
  (`0xe3d3a6f7bcd1ca623dbcbed037cfc15db729b2e621314831af2fb76371c66d8c`) — was 1,526,313.

  The sentinel has no `mock.rs`; its test runtime lives in `tests.rs`, and `new_test_ext` had to
  become `pub` for the benchmark test suite to link against it. Its argument shapes are not guessable
  from the call names either — `freeze_authority(origin, asset, who, reason)` takes four and
  `freeze_asset(origin, asset, reason)` three. Scanner `pallet-call-without-weights` 14 → 12.

* `41f386c3d` — **the settlement proof fixture moves out of `cfg(test)`, and the record catches up with
  the merge.** PR #520 tightened the EVM settlement path to walk a real receipts trie, which left that
  pallet's `submit_proof` benchmark handing the verifier a proof with no trie path and no receipt
  index; the `benchmarks!` macro asserts the extrinsic succeeds, so the benchmark could never pass, and
  `cargo test -p pallet-x3-settlement-engine --features runtime-benchmarks` was red on master. The
  fixture (`receipt_trie`, `create_evm_receipt_proof`, their two constants) now lives in
  `src/proof_fixtures.rs`, compiled under `cfg(any(test, feature = "runtime-benchmarks"))`, so the
  benchmark's CLI run — which is not `cfg(test)` — can reach the same evidence the tests use. The suite
  is 176 passed with the feature on, where it was 175 and one failing.

  The same change carries the record forward: the merge's attestation (`eac5255ce`) was taken inside
  the PR branch before master was merged into it, so the merged tree's bytes are new — compact
  8899733 bytes (`0x18d31490b724d8e5af91f0eb77436a3a3e95947d5576bb973022f125a16e63b1`) — was 8,880,606 — and compressed 1528341
  (`0x73df026fa625427bb4e02aaadda76b53be0a788d7876e3b7f46823af5bfac7b6`) — was 1,528,293, from two builds that agreed.

* `beec521ad` — **the asset registry joins the registered benchmark pallets, and the record follows.**
  `pallets/x3-asset-registry`'s seven dispatchables charged literals with no `runtime-benchmarks`
  feature at all. The benchmarks exist and pass in the pallet's own suite now (seven entries), and the
  runtime registers the pallet in `mod benches`, which moves the recorded bytes by one: compact
  8899733 (`0x9433d24621838ef7d41e04d838635952ead942b97ee199410d89546f9eedaf93`) — was 8,888,737 — and compressed 1530460 (`0x89363b9616de3a28e6703cd82bcbfda4a9c7a45c95308e8f3c3b345a1b5cd8eb`)
  — was 1,528,341, from two builds that agreed.

  **No weights file yet, and the reason is a defect in this repository's benchmark build, not in the
  pallet.** `cargo build --release -p x3-chain-node --features runtime-benchmarks` fails in the nested
  WASM build with `E0463: can't find crate for 'std'` from `rustc-hex`/`bytes`, reached through
  `evm/std`; the same command fails with every uncommitted edit stashed, so it predates this change,
  and clearing the nested build cache does not help. The weights CLI cannot work around it either: a
  node built with `SKIP_WASM_BUILD=1` refuses to start without an embedded runtime. Until that build is
  fixed, no pallet's weights can be regenerated.

* (2026-09-28) **Six gate failures master was carrying, cleared.** None was a re-baseline: the stub
  ratchet had grown by one word in a benchmark comment (reworded); the test-cheat ratchet had grown by
  the merge's one `#[ignore]` (the test is behind a `live-node` node feature now, which is stronger —
  it either compiles and must pass or does not exist, and gating it orphaned a helper that clippy then
  rejected, fixed with the same cfg); `FEATURE_MATRIX.toml` pinned 149 rows against 153 and
  `X3-GPU-003` crossed the `tested >= 80` bar without naming evidence; `crates/x3-sidecar`'s nested
  lockfile went stale when its manifest gained local `[patch]` entries; and this record had to move
  because a comment changed a runtime-graph file — two builds agreed the bytes were identical, and the
  attestation that followed the #518 merge is the one this file now names.

* `e39297afb` — **the unused `ed25519-dalek` dependency goes, and the record follows.** Every red run
  on `master` was a Dependabot Updates job, and the `cargo in /.` one could not resolve
  `ed25519-dalek`: `crates/x3-common` requested `ed25519-dalek/std` (a feature 3.0.0 dropped) for a
  dependency no source file in that crate uses (`rg 'dalek' crates/x3-common` matched only the
  manifest), while `agave-precompiles 3.0.14` pins the vulnerable 1.0.1 copy to `^1.0`. Removing the
  unused dependency moves the bytes: compact 8899373
  (`0x1528ad65a4b2fb656a928aa728cc225d6f23e97e69a704b414b0e85361c89fb4`) — was 8,899,733 — and
  compressed 1530897
  (`0xdc247b530ea5a31613fd255b31b6e5ac19bca630a89899682d2963e8e0173a67`) — was 1,530,460, from two
  builds that agreed. A *second* attestation run of the committed tree reproduced the same pair
  exactly, and the same change also moves `lru` from 0.12 to 0.16.3 in `crates/gpu-swarm` and
  `crates/x3-gulfstream` (GHSA-rhfx-m35p-ff5j) and records the two unfixable alerts in
  `.github/dependabot.yml`; those three files are outside the runtime's graph, which is why only this
  one moved the hashes. Detail: `.ai/reports/dependabot-jobs-triage-20260928.md`.

* `4c76fe691` — **the asset registry charges measured weights, and the benchmark build works again.**
  Two changes travel together because neither is useful alone. `pallets/x3-settlement-engine` enabled
  its optional `rlp` dependency from `runtime-benchmarks` with `default-features` on, and the
  runtime's `runtime-benchmarks` feature is linked into the WASM blob built for `wasm32v1-none`;
  `rlp/std` turns on `bytes/std` and `rustc-hex/std`, so that build died with E0463 before any
  pallet's weights could be regenerated. With `rlp` on its `no_std` path the benchmark node builds
  again, and `pallets/x3-asset-registry` (seven benchmarks that already existed) finally has the
  generated file — real measurements at STEPS 50 / REPEAT 20, not the
  `Weight::from_parts(10_000, 0)` literals it charged before. The runtime now points
  `type WeightInfo` at them, which is why the bytes move: compact 8902436
  (`0x49f94a7524542c747ce42ed2e470f9d2b86c6d00449d65b7b20dea046579d0b1`) — was 8,899,373 — and
  compressed 1529033
  (`0x7c38de390e6a5cd4afbcdf0ecc7d09a5bfd10103d917a21573a3410a36fa51a9`) — was 1,530,897, from two
  builds that agreed.

* `10b7f5c5d` — **the account registry and the dApp hub charge measured weights too.** Both pallets
  charged literals on every dispatchable and neither had a `benchmarking.rs`, while this runtime
  already enabled their `runtime-benchmarks` features — a feature that named a capability the pallet
  could not provide. `pallet-x3-account-registry` now carries three benchmarks and
  `pallet-x3-dapp-hub` eight, both are registered in `mod benches`, and both charge
  `type WeightInfo` from the generated files. Scanner `pallet-call-without-weights` reached 9 from
  12; runtime configs wired to generated weights 50 -> 53. The bytes move: compact 8903183
  (`0x0871ced5f0fcea528a13125d54d61d113f73cc750b9e25f029b9e720288ab38f`) — was 8,902,436 — and
  compressed 1529715
  (`0x63f5ad963d515a4667f31fa2c261e699db09454996adbf36f6fd28ee2a39b3c8`) — was 1,529,033, from two
  builds that agreed.

* `8981b29c9` — **the capacity manager and the custody registry charge measured weights.** They
  charged 45,000,000-70,000,000 and 4,000-12,000 picoseconds respectively on every call, neither was
  registered in `mod benches`, and `pallet-x3-partner`'s runtime feature was not enabled at all.
  Both carry benchmarks now and charge `type WeightInfo`; `custody`'s governance/operator calls take
  their origin from `try_successful_origin()`, because the mock models an operator as any signed
  account while this runtime requires root or half the council. Scanner
  `pallet-call-without-weights` reached 7 from 9 (four of the seven are the documented
  `T::DbWeight::get().reads_writes` form). The bytes move — and *shrink*, because the measured
  numbers replace 60,000,000-picosecond guesses: compact 8898068
  (`0x9a109392dab2c1dbe474f7c49e97fb811775bb0d25428be94046c2fa82aec4fc`) — was 8,903,183 — and
  compressed 1529260
  (`0xafdc7d331d9bf8b5cfa5cd6b69c22f0178d03f8887499fd7bea337b073963d39`) — was 1,529,715, from two
  builds that agreed.

* `1b9b04e5d` — **the wallet pallet charges measured weights.** `pallet-x3-wallet` (aliased here as
  `pallet-x3-wallet-pallet`) charged 5,000-15,000 picoseconds on all twelve calls behind a
  `runtime-benchmarks` feature that pulled `frame-benchmarking` in and never used it, and it was not
  registered in `mod benches`. Its four recovery benchmarks walk the real flow
  (`register_recovery_guardians` -> `initiate_recovery` -> `approve_recovery`) with a zero delay, so
  `finalize_recovery` reaches its executable block without the benchmark moving the chain's block
  number. Scanner `pallet-call-without-weights` reached 6 from 7 (four of the six are the documented
  `T::DbWeight::get().reads_writes` form). The bytes move: compact 8899712
  (`0xc0889ffe389ce6ee96d7a20ff521aab1257d95669c887fc2e8d46e11ef27bec6`) — was 8,898,068 — and
  compressed 1530072
  (`0x0e737994439fae21e99ecd597ae8cc8c9d7bac04d777d7a4a9de4291c42002f5`) — was 1,529,260, from two
  builds that agreed.

* `b8a73a01d` — **the cross-chain gateway charges measured weights.** All eleven dispatchables
  charged `Weight::from_parts(20_000..60_000, 0)` with no read or write count. The benchmarks build
  their state through the pallet's own extrinsics, and the route they use is `X3Internal` — the
  verification level whose verifier this repository can actually satisfy, which is what makes the
  deposit and release proof paths measurable rather than mocked. Scanner
  `pallet-call-without-weights` reached 5 from 6; `x3-cross-vm-router` is the last pallet that still
  charges a literal, and the other four findings are the documented `T::DbWeight::get().reads_writes`
  form. The bytes move: compact 8905964
  (`0x108225d763aa5f41b39d59b4ad69e284b3da1546d4af3dab84f7df1470986bec`) — was 8,899,712 — and
  compressed 1530061
  (`0x7904649cf82fb1dc80076f6c7b16c1cf104d9705988e78d20c9a38fdc87b0acd`) — was 1,530,072, from two
  builds that agreed.

* `23197c6d3` — **the cross-VM router's three reachable calls are measured, and the five that are not
  are recorded.** Its `benchmarking.rs` turned out to be dead code — `lib.rs` declared no
  `pub mod benchmarking;`, so nothing compiled it, and it measured helper functions rather than the
  pallet's dispatchables; the crate never declared `frame-benchmarking` either. It now carries real
  benchmarks for `set_external_bridge_audit_gate`, `set_external_bridges_enabled` and
  `emergency_pause_bridge`, and a new `UNMEASURABLE_CALL_WEIGHTS` record (with its own staleness
  tripwire and tests in `scripts/swarm/x3_repo_scan.py`) for the four custody-origin-gated calls plus
  `register_external_root`, which this runtime refuses at its `RefuseExternalRoots` verifier by
  policy. Scanner `pallet-call-without-weights` reached 4 from 5, and none of the four is high: the
  whole twelve-pallet burndown is closed, leaving only the documented
  `T::DbWeight::get().reads_writes` estimates. The bytes move: compact 8906533
  (`0x80f6336ce883c7b7602b2e14c197bd7b09e4b35258e5dda7d73ccd1848a993ca`) — was 8,905,964 — and
  compressed 1530390
  (`0x5efad00fe7ab70f779eeb0f16318ec43b7857e809c22397180b81bea256519ee`) — was 1,530,061, from two
  builds that agreed.

* `7332aa279` — **the guardian pallets are attested.** `x3-app-registry`, `x3-security-gate` and
  `x3-trust-gate` (with their weights) are wired into the runtime; the record's exemption for
  test-only files kept this from being demanded for the test edits on the way here. Two
  independent from-scratch srtool builds agreed. The bytes move: compact 9084047
  (`0xd6ba85e1c4ecd1b86eb5708aee4158cc1ced81b7003de3ba0a1e1564202459f9`) — was 8,906,533 — and
  compressed 1553413
  (`0x4f6e6b08ef8b27a9433c59a481adbabb53e2be52dd3109806a37ca13ee63a2ff`) — was 1,530,390, from
  two builds that agreed.
