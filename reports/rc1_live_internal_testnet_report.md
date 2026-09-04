# RC1 Live Internal Testnet Report

## Result

PARTIAL PASS / RC1 BLOCKED

The local 3-validator network booted, peered, produced blocks, and finalized blocks. The full RC1 gate is still blocked because the only runnable node artifact available in the workspace is the older `bin/x3-chain-node-fresh` binary, and that runtime metadata does not expose all required RC1 pallets.

## Scope

- One local X3 development network
- Alice, Bob, and Charlie validator authorities
- External bridge storage check against RPC
- Runtime metadata check for RC1 internal settlement pallets

## Build And Binary Status

- Current source build target: `x3-chain-node`
- Pinned Rust toolchain: `1.90.0`
- Source build status: BLOCKED by repeated rustc crashes/ICEs while compiling Substrate/runtime dependencies.
- Most useful source-build target dir preserved: `target/rc1-debug-rustixpatch`
- Runnable fallback binary used for local network proof: `bin/x3-chain-node-fresh`
- Fallback binary version: `X3 Chain Node 0.1.0`
- Fallback binary limitation: stale runtime metadata; it does not expose `X3SupplyLedger` or `X3CrossVmRouter`.

Implemented source-build mitigations so far:

- Patched `rustix v1.1.4` locally to avoid Rust 1.90 `rustc_diagnostics` ICE path.
- Patched `deranged v0.5.8` locally to avoid a Rust 1.90 macro-generated unchecked-negation ICE path.
- Removed the `rocksdb` feature from `sc-service` in the node package to avoid the `librocksdb-sys` native build blocker.
- Used single-job, no-incremental Cargo builds with clang/lld and elevated rustc stack sizes.
- Tried a separate Rust `1.92.0` side target; it also hit compiler crashes in dependency code.

## Chain Specs

Generated and validated:

- `chain-specs/x3-local3-plain.json`
- `chain-specs/x3-local3-raw.json`

The generated `local3` spec contains three validator authorities:

- Alice: Aura/account `5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY`, GRANDPA `5FA9nQDVg267DEd8m1ZypXLBnvN7SFxYwV7ndqSYGiN9TTpu`
- Bob: Aura/account `5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty`, GRANDPA `5GoNkf6WdbxCFnPdAnYYQyCjAKPJgLNxXwPjwTh6DGg6gN3E`
- Charlie: Aura/account `5FLSigC9HGRKVhB9FiEo4Y3koPsNmBmLJbpXg2mp1hXcS59Y`, GRANDPA `5DbKjhNLpqX3zqZdNBc9BGb4fHU1cRBaDhJUskrvkwfraDi6`

The authority list in `chain-specs/x3-local3-plain.json` is the source of truth.

## Local Validator Proof

Command used:

```bash
COUNT=3 \
NODE_BIN="$PWD/bin/x3-chain-node-fresh" \
CHAIN_SPEC="$PWD/chain-specs/x3-local3-raw.json" \
CHAIN_SPEC_PLAIN="$PWD/chain-specs/x3-local3-plain.json" \
BASE_DIR="$PWD/.local/rc1-local3" \
LOG_DIR="$PWD/logs/rc1-local3" \
SUBKEY_BIN="$PWD/scripts/testnet/subkey-js-shim.cjs" \
scripts/testnet/run-7-validators-local.sh --wipe
```

Validators started successfully:

- Node 1: RPC `9944`, P2P `30333`
- Node 2: RPC `9945`, P2P `30334`
- Node 3: RPC `9946`, P2P `30335`

Latest health snapshot:

- `9944`: peers `2`, syncing `false`, head `0x301`
- `9945`: peers `2`, syncing `false`, head `0x301`
- `9946`: peers `2`, syncing `false`, head `0x301`

Finality was observed advancing in the smoke test.

## Smoke Test Result

Artifact: `reports/rc1_smoke_test.log`

Result: FAILED, with 3 passing checks and 2 failing checks.

Passed:

- Block production advanced from `#670` to `#700`.
- Finality advanced from `0x1728bd9ed9185b3c7e7acbb5fa3c56f6f58fef10acedbb391e2242f47150316f` to `0xb2e4d806679b47bb871121669bd62aa5b08700d04cec9ab237b11122f3da4e8b`.
- `ExternalBridgesEnabled` storage resolved to `null`, treated as disabled/absent.

Failed:

- `X3SupplyLedger` was not found in runtime metadata.
- `X3CrossVmRouter` was not found in runtime metadata.

Additional metadata snapshot: `reports/rc1_metadata.hex`

Observed metadata names:

- Present: `System`, `Timestamp`, `Aura`, `Grandpa`, `Balances`, `X3AtomicKernel`, `X3SettlementEngine`
- Missing: `X3SupplyLedger`, `X3CrossVmRouter`, `X3AssetRegistry`, `X3AccountRegistry`

## Script Fixes Made During Proof

- Fixed `scripts/mainnet/rc1_smoke_test.sh` pass/fail counters so `set -e` does not abort on the first `((PASS++))` result.
- Replaced the incorrect `shake_128` storage hash with a dependency-free TwoX128 implementation.
- Normalized JSON `null` from `state_getStorage` to the expected `null` string.
- Added `scripts/testnet/subkey-js-shim.cjs`, a testnet-only `subkey inspect` shim backed by existing Polkadot JS packages, because `/home/lojak/.cargo/bin/subkey` is not installed.

## Blockers

1. A fresh source-built `x3-chain-node` is still required for RC1.
2. The current source build is blocked by Rust compiler crashes/ICEs across Substrate/runtime dependency crates.
3. The old runnable binary can prove local consensus/finality, but cannot prove the full RC1 runtime pallet set.

## Final Verdict

RC1 is not green yet.

The local network path is proven: specs generate, three validators boot, peers connect, blocks advance, and GRANDPA finality advances. The release remains blocked until a fresh node binary embeds the current runtime and the smoke test passes the missing runtime metadata checks.
