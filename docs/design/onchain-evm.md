# A real EVM on chain — design for sign-off

Status: **draft, for the owner's decision.** Nothing here is built yet. Measured 2026-09-29 on
`origin/master` 264b3166a.

## Why

`.x3` programs cannot call EVM contracts (X3-LANG-001, closed as a documented non-goal in #553)
because the chain has no stateful EVM to call. The multi-VM L1 the project describes (X3VM + EVM +
SVM) needs one: an EVM whose contracts and balances persist across blocks, that users can deploy to
and call, and that X3 programs can reach atomically.

## What exists today (measured)

| Piece | State |
|---|---|
| `pallet_evm` + `pallet_ethereum` config, `construct_runtime!` entries, precompiles (simple, modexp, sha3fips) | written in `runtime/src/lib.rs`, compiled only with the `frontier` feature |
| Production (WASM) runtime | built **without** `frontier` (`default = ["std"]`) |
| Kernel EVM on chain | `WasmEvmAdapter` → `mini_evm`: fresh empty storage every execution, caller and value zero, nothing persists (`the_chains_evm_keeps_no_state_between_executions`) |
| Kernel EVM with `frontier` | `NativeEvmAdapter` → `pallet_evm` Runner, but `#[cfg(all(feature = "std", feature = "frontier"))]` (uses `std::collections::HashMap`), runs every call as `H160::zero()`, and guesses create-or-call from the payload |
| WASM runtime with `frontier` | **builds** (exit 0; the `.features` sidecar lists `pallet-evm`, `pallet-ethereum`, `fp-evm`, the three precompiles). Compressed size 1,688,493 bytes vs 1,530,390 today (+158 KB, +10%) |
| EVM origins | `CallOrigin = EnsureAddressRoot`: only Root can call `pallet_evm::call`/`create` |
| Address model | `AccountId32` with `HashedAddressMapping<BlakeTwo256>` (one-way: an H160 maps to an account, not back) |
| Fees | `FixedGasPrice`, `BlockGasLimit` 15M, `FindAuthor = ()` |
| Extrinsics | `OpaqueExtrinsic`: no self-contained Ethereum transactions |
| Ethereum JSON-RPC | hand-written `eth_*` module (`node/src/rpc_frontier.rs`, ~2k lines) behind the node's `frontier` feature |

So the pieces exist; they are switched off, and the kernel's path into them is std-only and
identity-less.

## Decisions the owner has to make

**D1 — Who can deploy and call.** Recommended: **signed users, through the kernel**, with the sender
being the signer's mapped H160 (not `H160::zero()`); `pallet_evm`'s own extrinsics stay Root-only.
Alternative: open `pallet_evm::call`/`create` to signed origins as well (two entry points to secure
instead of one).

**D2 — Address model.** Recommended: **keep `AccountId32` + `HashedAddressMapping`.** Every chain
account gets a deterministic EVM address; nothing about existing accounts changes. Alternative:
unified 20-byte accounts (Moonbeam-style), which is what MetaMask-native signing wants but is a
chain-wide account-model migration.

**D3 — Ethereum-signed transactions (MetaMask).** Recommended: **not in the first release.** It needs
self-contained extrinsics (`fp-self-contained`) and a Frontier-compatible RPC; the existing `eth_*`
module can serve reads. Revisit after D2.

**D4 — Fees.** Recommended: **keep a fixed gas price** first, with EVM gas mapped to weight
(`FixedGasWeightMapping`) and a per-block EVM gas cap; add EIP-1559 (`pallet-base-fee`) later if the
market needs it. Set `FindAuthor` so fees are accounted, not silently dropped.

**D5 — Where it ships.** Recommended: **testnet runtime first**, behind the existing `frontier`
feature turned on for the testnet build; `mainnet-rc1` only after an external audit of the EVM
entry points.

**D6 — Live chain.** Adding pallets is a runtime upgrade: new pallets start with empty storage, so
no data migration is needed, but `spec_version` bumps and the governance upgrade rehearsal
(`scripts/mainnet/runtime_upgrade_rehearsal.sh`) must pass with the EVM-carrying runtime.

## Phases

**Phase 1 — the chain has a stateful EVM.**
- Turn `frontier` on for the chosen runtime build(s) (D5).
- Make the kernel's EVM adapter compile without `std` (replace `HashMap`, drop the std gate), so
  native and WASM run the **same** EVM. The two-adapter divergence disappears.
- Replace create-or-call guessing with explicit `EvmCall { target, input, value }` /
  `EvmCreate { init_code, value }` payloads, sender = the signer's mapped address (D1).
- Weight: benchmark the EVM path; charge by gas via `GasWeightMapping`.
- Evidence: a live-node test deploys a contract in one block, calls it in a later block and reads
  the storage it wrote (the opposite of today's `the_chains_evm_keeps_no_state_between_executions`,
  which flips); a balance moves between a native account and its mapped EVM address; the WASM
  runtime is re-attested; the upgrade rehearsal passes with it.

**Phase 2 — `.x3` calls EVM contracts atomically.**
- A host-call interface in `mini_x3` (no_std callback), implemented by the kernel's X3 adapter with
  the EVM adapter behind it.
- `evm_call(target, input) -> bytes` from source, which needs a bytes value in the language (today:
  i64/bool/float, strings as constants only).
- Atomicity: the X3 run and every EVM call it makes run inside one storage transaction; an X3 fault
  rolls back the EVM's writes too.
- Gas: EVM gas spent inside an X3 run counts against the X3 gas limit.
- Evidence: an `.x3` program writes an EVM contract's storage and reads it back; the same program
  with a fault after the call leaves the contract unchanged; `UnavailableCrossVmCall` is retired for
  the EVM names.

**Phase 3 (optional) — Ethereum wallets.** Self-contained Ethereum transactions and the RPC surface
MetaMask needs, after D2/D3.

A stateful **SVM** is a separate, similar project; this document does not cover it.

## Risks

- **Consensus-critical.** Every phase changes what a block computes; each needs the runtime
  re-attested and the upgrade rehearsal green before it lands.
- **Weight and DoS.** EVM execution must be charged by gas mapped to weight, with a block cap;
  today's kernel path passes a gas limit but has no benchmarked weight for a real EVM.
- **State growth and PoV.** Contract storage grows chain state; `GasLimitPovSizeRatio` and
  `GasLimitStorageGrowthRatio` are set but unmeasured here.
- **Audit.** New signed entry points into the EVM need an external review before mainnet (D5).
- **Size.** +158 KB compressed WASM (measured), within limits but not free.

## Ask

Sign off on D1–D6 (or change them), and Phase 1 starts on a branch with its evidence list as the
merge bar.
