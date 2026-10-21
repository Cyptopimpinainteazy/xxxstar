# Canonical trading architecture

This describes the source at receipt correction `354da22bac9b80e8fd06b28fe7d16beff50269b5`, following RED commit `b04d9e1210f037a2f8c171808b91594d68111e6d`. It documents the implemented trading bridge, not full language canonicalization or a completed deployment. See [MASTER_FEATURE_AUDIT.md](MASTER_FEATURE_AUDIT.md) for scope and evidence limits.

## Before and after

Before the production-unification bridge, two separate paths existed:

```text
nested .x3 frontend -> nested trading IR/encoding -> nested reference TradingVm
root compatibility frontend -> root backend X3BC -> root verifier/VM -> chain integration
```

The implemented trading path joins the nested frontend to the chain's bytecode and engine:

```text
.x3 source
  -> x3-lang/compiler parser and pre-emission verification layers
  -> compile_trading_x3bc (verified trading IR -> canonical X3BC 1.1)
  -> serialized artifact
  -> kernel submit_trade -> integration execute_trade
  -> canonical decoder -> on_chain_trading verifier -> production x3-vm
  -> committed delta -> kernel settlement
  -> CanonicalLedger + runtime DEX/flashloan/oracle host
```

The kernel seeds execution from chain state; it does not compile source inside a block. Unsupported constructs, including bridge venues without proof-verified execution, are explicitly refused by the canonical trading lowering.

## Ownership and delegation

| Responsibility | Implementation and boundary |
|---|---|
| Source frontend | `x3-lang/compiler/src/parser.rs`, semantic/trading verification and lowering modules own the nested source/IR path. `x3bc.rs::compile_trading_x3bc` runs pre-emission verification before lowering one atomic trade. |
| Canonical bytecode contract | `crates/x3bc-core` is the shared `no_std`, Substrate-independent core: container/version, opcodes, operand and gas spec, admission metadata, trading section, AMM arithmetic, state encoding, receipt and traps. Compiler and production VM consume it. |
| Production execution | `crates/x3-vm/src/verifier.rs` consumes the canonical spec. `vm.rs` and `trading.rs` implement the execution engine and trading session. Shared definitions do not imply that all nested reference opcodes are production-admitted. |
| Integration | `crates/x3-integration/src/trading.rs::execute_trade` decodes canonical bytes, applies `VerifyOptions::on_chain_trading`, seeds the VM and returns receipt, outcome and optional committed delta. |
| Chain host | `pallets/x3-kernel/src/trading.rs` reads registered asset metadata, balances, pool custody, oracle observations and configured costs; `submit_trade` in the pallet invokes that path. Runtime host uses `pallet_x3_dex` and `pallet_x3_flashloan`. |
| Retained reference path | `x3-lang/vm` remains the reference/test oracle. Differential tests provide bounded comparison; its separate encoding/ISA is not the chain artifact contract. |
| Retained compatibility compiler | `crates/x3-compiler` and `crates/x3-integration/src/compiler_bridge.rs` retain the root general compiler pipeline. The trading bridge does not remove all duplicate frontend representations or migrate every caller. |
| Current developer CLI | `x3-lang/crates/x3-tools/src/bin/x3c.rs::cmd_build` still calls `compile_with_mode_diagnostics`, not `compile_trading_x3bc`. Its deployment command prints a plan and optionally writes bytes. CLI availability does not establish canonical artifact selection or submitted/finalized deployment. |

## Transaction and settlement boundaries

The compiler emits an atomic trading entry around its section operations, explicit cost charging, profit checks and trade commit. The VM keeps economic changes in its trading journal. `execute_trade` returns a delta only for a committed session; rollback outcomes return no economic delta and no released storage writes.

The kernel applies committed ledger intents and venue changes against the canonical ledger and host, then checks settlement against the engine's expected final state. Failure aborts the extrinsic's storage changes. Engine rollback leaves economic balances unchanged while the kernel can record its receipt. The current kernel trading path refuses contract-slot writes rather than silently dropping them.

Custody and execution rely on configured chain state: trading is refused until governance supplies trading configuration. Public submissions set `private_channel: false`; private-required artifacts are refused. This is an admission limit, not evidence of a private channel. These boundaries require runtime and failure-path execution evidence in addition to library tests.

## Receipts and trust

`x3bc-core/src/receipt.rs` defines canonical receipt encoding and digest. The off-chain integration attestation API signs that digest and resolves active trusted attestor keys. Replay checks signature and artifact hash, executes the artifact from the receipt's own prestate, compares the recomputed receipt, and independently audits economic identities.

The current correction additionally requires execution anchor height to equal prestate height and, when finality is attached, finalized-header height to equal execution height. It preserves provisional receipts with `finality: None` and signature/artifact error precedence. These checks reject contradictory metadata; they do not authenticate the claimed header, state roots or prestate against external finalized chain storage. A registered signature and deterministic replay are bounded evidence, not standalone proof of live inclusion or GRANDPA finality.

## Verification checkpoint

The parent execution recorded 51 local trading tests passing with the correction and strict library Clippy passing. Other named runtime/compiler/CLI suites are not claimed as executed for this correction here. Remote RED run `39625154680` remains queued at this checkpoint; exact-head remote GREEN is absent. Untouched-master Local CI run `39623913764` failed six gates; that baseline is not evidence that this change passes them. No current finalized deployment, completed live soak, or production-readiness claim follows from this document.
