# Phase 0 baseline — X3 pre-launch economics + X3Lang cutover

* Date: 2026-09-24
* Branch: `feat/x3-prelaunch-economics-x3lang-cutover`
* Starting commit: `4b74f025b39630531326fa087bfc39574f27575a` (= `origin/master` at the time of writing)
* Method: every claim below was read out of the tree at this commit. Nothing is taken from a
  report, a README, or a comment that says "production" without a caller traced to it.

## The twelve questions

| Question | Answer | Evidence |
| --- | --- | --- |
| Economics actually runtime-active | Balances, `pallet_transaction_payment`, `X3Coin`, `Treasury` (Substrate), `X3TreasuryPolicy`, `X3SupplyLedger`, `X3SettlementEngine`, `X3Slash`, `X3AtomicKernel`. No fee market, no staking, no authorship. | `runtime/src/lib.rs:516+` (`construct_runtime!`), `runtime/Cargo.toml` |
| Economics present only as libraries | `crates/x3-fees` and `crates/x3-economics` are depended on by **nothing** (their names appear only in their own manifests). `crates/x3-revenue-sharing` is a runtime dependency but is used only by `pallets/x3-dapp-hub` as a split-policy type. | `runtime/Cargo.toml:131,263`; `pallets/x3-dapp-hub/src/lib.rs:33` |
| Total supply / genesis | `8,888,888,888` X3, four-way allocation of `2,222,222,222` each, in both genesis manifests. A stale `pallets/x3-coin/README.md:16` still claims 2,000,000,000. | `deployment/genesis/x3-testnet-allocations.json:8-9,25`; `...baseline.json:7-8,13`; `pallets/x3-coin/README.md:16` |
| Transaction fee behaviour | `IdentityFee<Balance>` for weight (1:1) and `ConstantMultiplier<Balance, TransactionByteFee>` with `TransactionByteFee = 10 * MICRO_ATLAS` for length; `OperationalFeeMultiplier = 5`; `FeeMultiplierUpdate = ()` — no congestion adjustment at all. | `runtime/src/lib.rs:1165-1176`, `:408`, `:471` |
| Fee destination | **Burned, implicitly.** `DealWithFees::on_unbalanced` is `drop(amount)` on the `NegativeImbalance`; the collected fee never reaches treasury or the block author. | `runtime/src/lib.rs:1042-1047`, `:1170` |
| Runtime inflation | None found. The only mint-time extrinsic, `X3Coin::mint`, is minter-allowlisted, proof-validated, replay-guarded, funded from a treasury balance, and asserts `verify_supply_invariant()` — it moves value, it does not issue it. The `deposit_creating` calls in `runtime/src/lib.rs` sit inside `#[cfg(all(test, feature = "std"))] mod native_supply_contract_tests`. | `pallets/x3-coin/src/lib.rs:487-530`; `runtime/src/lib.rs:5144,5258-5317` |
| Runtime staking | Absent. `pallet_staking`, `pallet_authorship`, `pallet_bags_list`, `pallet_nomination_pools` appear nowhere in `runtime/Cargo.toml`. `sp-staking` is present but that is session-key/offence typing, not economics. | `runtime/Cargo.toml` |
| What compensates validators today | Nothing. Aura + Grandpa + Session + Offences are wired; there is no authorship, no reward pallet, no fee share (fees are dropped). | `runtime/src/lib.rs:518-521`; `:1042-1047` |
| Compiler the runtime integration invokes | The **root** `crates/x3-compiler` (`x3_compiler::{CompilationOptions, Compiler}`), not `x3-lang/compiler`. | `crates/x3-integration/src/compiler_bridge.rs:9,20` |
| VM used on std | `x3-vm` (root) and `x3-lang-vm` are both outside the runtime graph; consumers are `x3-bench`, `x3-opt`, `x3-gpu-validator-swarm`, `atomic-swap-orchestrator`, `x3-bridge-adapters` (x3-vm) and `x3-lang/crates/x3-tools`, `x3-lang/compiler` (x3-lang-vm). | `Cargo.toml` graph sweep, see Commands |
| VM used on no_std/WASM | `x3-integration::mini_x3`, "the one the runtime uses" — a second, hand-written reader of the same X3BC envelope, reached through `pallets/x3-kernel`. | `crates/x3-common/src/lib.rs:200-207`; `pallets/x3-kernel/Cargo.toml` (depends on `x3-integration`) |
| Where Python enters | The `x3-lang/` Python track (20 files / 2,542 lines: `cli.py`, `compiler/`, `emitter/`, `numeric.py`) — a separate track from `x3-lang/compiler` (99 Rust files / 52,311 lines) and from root `crates/x3-compiler` (17 files / 4,470 lines). Exercised by the `test x3-lang python` gate only. | `scripts/local-ci.sh:248-252`; line counts measured |

## Divergence register — every place two implementations can disagree

1. **Compiler**: root `crates/x3-compiler` (what `x3-integration` compiles with) vs `x3-lang/compiler`
   (the canonical one this mission freezes). The `x3-lang` tree is a **separate cargo workspace**:
   the root `Cargo.toml` does not reference it, so the runtime cannot reach the canonical compiler
   at all today.
2. **Bytecode reader**: `x3-backend::bc_format` (std) vs `x3-integration::mini_x3` (no_std, the
   runtime's). The comment at `crates/x3-common/src/lib.rs:200-207` records that these two already
   disagreed once about header validation (TICKET-108).
3. **VM**: four surfaces named in the mission (`x3-lang/vm`, root `crates/x3-vm`, `mini_x3`,
   TradingVm); only the last is reachable from the runtime.
4. **Supply claims**: genesis `8,888,888,888` vs `pallets/x3-coin/README.md` `2,000,000,000`.
5. **Fee claims**: `crates/x3-fees::Eip1559FeeMarket` (70/30) exists as a library; the runtime
   burns 100% and has `FeeMultiplierUpdate = ()`.
6. **Language authority**: `README.md:11` calls the Python/pipeline track "the current
   authoritative implementation" and `crates/x3-compiler` experimental. `LAUNCH_SCOPE.md` — the
   README's own "single authoritative scope statement" — never mentions x3-lang.

## Measured launch-gate state (this commit)

RC6 was run on this tree (not inferred): `RC6_PUBLIC_TESTNET_READINESS: FAIL`.
Release node build PASS, runtime WASM build PASS, docs PASS — and then:
`X3_TESTNET_AUTHORITIES not set and no validator key summaries found`, so the public plain/raw
chain specs do not generate; dev-key evaluation and the external-bridge disable flag also fail.
That is a *config/deployment* blocker, not a code blocker, and it is not caused by anything in
this mission.

## Not yet established (must not be claimed)

* Which of `x3-vm` / `x3-lang-vm` is intended to become the single production VM, and what
  `mini_x3` would need to be replaced by.
* Whether the Python track can express anything the canonical compiler cannot (this decides the
  fail-closed envelope in B3).
* The full call path from `pallets/x3-kernel` into `mini_x3` (opened, not yet traced line by line).

## Commands used

```bash
git log --oneline -1
grep -n ... runtime/src/lib.rs runtime/Cargo.toml pallets/*/Cargo.toml
git ls-files / find ... | wc -l              # line counts for the three compiler tracks
bash scripts/mainnet/rc6_public_testnet_readiness.sh
```
