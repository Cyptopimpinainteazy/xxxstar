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

**Compact (`x3_chain_runtime.compact.wasm`, 8,482,928 bytes)**

```
Version          : x3-chain-20 (x3-chain-1.tx1.au1)
Metadata         : V14
setCode          : 0xac2e46e9366fa6a31129599faf552dc568157dfc656e52deffea8b7e7b04361c
authorizeUpgrade : 0x9c2bb5f158a63d5281e0e4948c92e75830e02707c7ebb2455ba9f48888c3d6de
IPFS             : QmYBepxKZR7wB6RJZE7QdnWqQocmG3bZ9mxLCH9PsauLsk
BLAKE2_256       : 0xeb9c27c2a89fe1803f4923bf32cc9c3d09124fa3f17b5f04f56bfd5b4db6f390
```

**Compressed (`x3_chain_runtime.compact.compressed.wasm`, 1,459,662 bytes)**

```
setCode          : 0x465d6fbc78fdec2652447cb6bd7d57576e53dfc0736d2c04665414fe7fc523a9
authorizeUpgrade : 0x354782abcad1b712a5edb78bbfb4b7e2adbbeae55692bb25cf6703a947f8e80d
IPFS             : QmeZCmK1QA7zQ8DiNgTZGSc9ZXc5QmTGT9acA9iNYo1mXm
BLAKE2_256       : 0x532b70e61d1a95bab79cbff721ffbbbfb652e79283b8f19f5876b64850e0d2de
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
