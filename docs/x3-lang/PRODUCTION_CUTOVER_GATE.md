# X3Lang Production Cutover Gate

**Status:** mandatory pre-Alpha gate  
**Decision:** X3 Public Testnet Alpha does not launch until the Rust X3Lang pipeline is the single production-authoritative `.x3` path and a real validator network proves source-to-finality execution.

## Production authority decision

```text
.x3 source
  -> x3-lang/compiler
  -> canonical X3BC
  -> canonical bytecode verification
  -> X3Lang VM semantics
  -> root crates/x3-integration host/runtime boundary
  -> X3 runtime state mutation
  -> block inclusion
  -> GRANDPA finality
  -> signed/verifiable execution receipt
```

Authority rules:

1. `x3-lang/compiler` owns production grammar, semantic verification, IR, lowering and X3BC emission.
2. `x3-lang/vm` owns production X3Lang bytecode semantics.
3. `x3-lang/crates/x3-tools/src/bin/x3c.rs` is the production developer CLI.
4. Root `crates/x3-*` code may integrate canonical X3Lang artifacts with Substrate, but may not define a competing language or bytecode meaning.
5. The Python `x3-lang/*.py` MVP remains a development/reference surface only after cutover. It cannot produce artifacts accepted as production X3Lang launch evidence.
6. Root `crates/x3-compiler` cannot remain an independent production compiler. It must delegate to or consume artifacts from the canonical `x3-lang/compiler`.
7. `crates/x3-integration/src/mini_x3.rs` cannot silently define consensus semantics independently of `x3-lang/vm`.

## Current divergence that blocks launch

At the current audited head:

- `docs/x3-lang/architecture-authority.md` declares `x3-lang/compiler` and `x3-lang/vm` authoritative for Rust X3Lang.
- `crates/x3-integration/src/compiler_bridge.rs` still compiles through root `x3_compiler::{Compiler, CompilationOptions}`.
- `crates/x3-integration/src/executor.rs` executes std builds with root `x3_vm::VM`.
- the no-std path executes with the separate `mini_x3` interpreter.
- `x3c run` is a dry-run surface; fixture capabilities are rejected in production mode.
- therefore a green compiler/unit-test suite does not yet prove that a `.x3` program is the exact program the validator finalized.

Until these are reconciled:

```text
X3LANG_PRODUCTION_CUTOVER: FAIL
PUBLIC_TESTNET_ALPHA: NO-GO
```

## Required implementation spine

### X0 — Freeze semantics

Freeze X3BC version/header, opcode IDs, operand encoding, gas rules, traps, numeric rules, control flow, hostcall ABI, receipt version and capability-policy encoding. Version any future incompatible change.

### X1 — Canonical compiler bridge

Production compilation must delegate to the canonical Rust compiler:

```text
crates/x3-integration
  -> std-only canonical compiler bridge
  -> x3-lang/compiler
```

No fallback to root compatibility compilation.

Required tests: direct canonical compiler bytecode equals bridge bytecode; mode is preserved; semantic failure produces no artifact; provenance binds source hash, compiler version, artifact hash and repository commit.

### X2 — One executable semantic contract

Preferred architecture: extract/share a no-std X3BC semantic core from the canonical X3Lang workspace so developer execution and runtime integration consume the same decoder, opcode and gas definitions.

```text
x3-lang canonical semantic core (no_std-compatible)
        |                         |
        v                         v
x3-lang developer VM      X3 runtime integration
```

A second hand-maintained interpreter is not accepted merely because its tests pass. If `mini_x3` remains temporarily, Alpha requires exhaustive differential conformance against canonical X3Lang VM behavior and fail-closed rejection of unsupported canonical opcodes.

### X3 — Runtime host boundary

Production hostcalls must reach deterministic runtime-backed state/effects, not fixture-only state. The first launch proof should be X3-native and deterministic; external RPC/DEX calls must not be invoked nondeterministically from consensus execution.

Required host evidence: storage reads/writes, events, balances/assets, gas/weight, atomic begin/commit/rollback and failure behavior.

### X4 — Canonical receipt

Define one `X3ExecutionReceiptV1` that binds at least: version, chain/network IDs, source hash, compiler identity, bytecode hash, policy hash, plan hash, tx hash, extrinsic index, block number/hash, pre/post state roots, gas, fee, hostcall transcript root, economic/result commitment, finality proof reference, attestor public key and signature.

Use domain-separated signing such as `X3:EXECUTION_RECEIPT:V1`. Only a receipt bound to a finalized block is final launch evidence.

### X5 — Real source-to-finality E2E

Required chain:

```text
source.x3
 -> x3c check
 -> x3c build --mode testnet
 -> canonical X3BC
 -> submit to live validator network
 -> bytecode verifier
 -> production runtime execution
 -> actual state mutation
 -> block inclusion
 -> GRANDPA finality
 -> X3ExecutionReceiptV1
 -> independent receipt verification
```

The proof must use the same binary, runtime and chain spec intended for Public Testnet Alpha.

## First launch programs

First prove the language/runtime/finality spine with a deterministic X3-native program:

```text
BEGIN
  bind X3-native asset/account
  debit/lock known amount
  execute deterministic operation
  assert invariant
  write state/event
  COMMIT
END
```

Then prove the Trading Core economic safety path:

```text
BEGIN
  borrow 1000 units
  execute deterministic test route
  repay principal + declared cost
  require all debt closed
  require realized profit/cost rule
  COMMIT
END
```

## Differential VM gate

Execute the same X3BC through every implementation still present. Compare accept/reject, return value, gas, trap/error, events/logs, storage writes, balances, rollback and final state commitment.

Required invariant:

```text
canonical VM observable result == runtime observable result
```

Cover every opcode, malformed headers, bad checksums, unsupported versions, bad jumps, bounds, arithmetic edges, gas exhaustion, nested calls, rollback, hostcall failures, truncated bytecode and randomized generated programs. Any divergence blocks launch.

## Mutation gate

Mutate source, compiler version, bytecode, bytecode version, chain/network ID, nonce, policy, capability manifest, asset, amount, deadline, gas/fee limits, hostcall transcript, pre/post roots, tx hash, extrinsic index, block hash, finalized block, signer and signature. Every mutation must reject or produce a distinct receipt; none may verify as the original execution.

## Required evidence

```text
audit-artifacts/x3lang-production-cutover/<commit>/<timestamp>/
  summary.json
  source.x3
  source.sha256
  compiler-version.txt
  compiler-commit.txt
  bytecode.x3b
  bytecode.sha256
  policy.json
  execution-plan.json
  pre-state-root.txt
  hostcalls.json
  hostcall-transcript-root.txt
  post-state-root.txt
  tx.json
  tx-hash.txt
  inclusion.json
  block-hash.txt
  grandpa-finality.json
  receipt.json
  receipt.bin
  receipt.sig
  replay.json
  differential-vm.json
  mutation-results.json
  commands.txt
  logs/
```

## Development checks

Add the final launch script as `scripts/mainnet/x3lang_production_cutover_gate.sh`. Until then, these are necessary development checks but are not sufficient launch evidence:

```bash
cargo test --manifest-path x3-lang/Cargo.toml
cargo run --manifest-path x3-lang/Cargo.toml -p x3-tools --bin x3c -- \
  --mode testnet --deny-warnings check x3-lang/examples/trading_core_v1.x3
cargo run --manifest-path x3-lang/Cargo.toml -p x3-tools --bin x3c -- \
  --mode testnet --deny-warnings build x3-lang/examples/trading_core_v1.x3 \
  --out /tmp/trading_core_v1.x3b \
  --provenance /tmp/trading_core_v1.provenance.json
```

## Alpha launch criterion

- [ ] Rust `x3-lang/compiler` is the only production compiler authority
- [ ] Rust `x3-lang/vm` semantic contract is canonical
- [ ] root integration delegates rather than redefines compilation
- [ ] runtime execution has no unproven alternate semantics
- [ ] differential VM gate passes
- [ ] source provenance is bound to bytecode
- [ ] production host path is non-fixture
- [ ] state mutation is visible in actual runtime state
- [ ] transaction inclusion is bound to block hash + extrinsic index
- [ ] GRANDPA finality is bound to the receipt
- [ ] receipt signature independently verifies
- [ ] economic replay verifies
- [ ] mutation gate passes
- [ ] one X3-native source-to-finality E2E passes on the Alpha candidate
- [ ] one Trading Core source-to-finality E2E passes on the Alpha candidate

If any item is red:

```text
X3LANG_PRODUCTION_CUTOVER: FAIL
PUBLIC_TESTNET_ALPHA: NO-GO
```
