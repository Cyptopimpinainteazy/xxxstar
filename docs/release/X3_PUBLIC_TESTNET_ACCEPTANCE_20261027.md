# X3 public testnet: one executable profile and exact-head proof contract

**Current verdict: NO-GO.** Candidate PR #703 is a **staged integration branch** based on
the exact #694 execution tree, not a ready-to-merge replacement for `master`.
A green workflow on #682, #683, #694 or any other commit is **not** evidence for
a different candidate SHA. The upstream security fixes merged into `master`
through `754ad4687f8b227e776a7a639064d1499dd06682` have not been reconciled
with the integration branch's independently changed lockfiles and dependencies.

## Canonical runtime decision

The **only admission target here** is `x3-chain-node --features mainnet-rc1`
plus the corresponding reproducibly built `x3-chain-runtime/mainnet-rc1`.
Its selected `construct_runtime!` activates AtomicTradeEngine, X3AtomicKernel,
X3CrossVmRouter, X3SettlementEngine, X3SupplyLedger, SvmRuntime and the
consensus/economics pallets. It **does not activate** experimental `X3Dex`,
`X3Automation`, `pallet-evm` or `pallet-ethereum`.

Frontier is a **different**, post-RC1 runtime configuration.
`mainnet-rc1,frontier` together is not an admissible testnet build because the
runtime deliberately has no `construct_runtime!` variant for that combination.
A compile-only Frontier check is not a demonstration of persistent stateful EVM.
Do not claim public EVM execution on an RC1 network; enabling that capability
requires a separately defined, independently audited release scope.

The source provenance contract in
`scripts/ci/canonical-candidate-manifest.json` tracks all 36 imported blobs.
`scripts/ci/verify_canonical_candidate.py` refuses a changed blob,
mismatched candidate head, unexpected RC1 pallet roster, unsafe bridge feature,
or absent latest master security ancestor. This is an **identity check**, not a
substitute for runtime execution tests.

## Release admission matrix

| Domain | Present code / source of truth | Mandatory evidence before a public claim | Status |
| --- | --- | --- | --- |
| Canonical production path | `runtime/src/lib.rs` RC1 variant, node feature mapping, X3Lang production unification | One frozen SHA; merged security updates and locked dependencies; root + nested compiler, format, Clippy, test, WASM and signed reproducible-runtime checks green on **that SHA** | BLOCKED |
| Atomic kernel, router, settlement | `pallets/x3-atomic-kernel`, `x3-cross-vm-router`, `x3-settlement-engine`, `x3-x3-integration` | Real runtime state transitions for all three VM leg types, rollback, conservation, authorization, replay denial, deterministic gas/state root equivalence | UNVERIFIED |
| DEX / liquidity | `pallets/atomic-trade-engine`; PR #694 removes synthetic pool fallback | Registered pool reserves tested through the actual runtime/extrinsic path, insufficient-pool refusal, fees, slippage and atomic real-reserve settlement; full `X3Dex` is **not** part of RC1 | UNVERIFIED |
| Stateful EVM | Frontier crates, optional `frontier` node/runtime feature, existing live Anvil HTLC lifecycle | Persistent state, Ethereum origin and chain-ID policy, gas/weight, deployed contract behavior and rollback on an actual node; separately approved post-RC1 profile | NOT IN RC1 |
| SVM execution | `SvmRuntime` pallet and SVM integration; existing local solana-test-validator lifecycle harness | Actual SBF program deployed and executed, RPC-observed account state, signer restrictions, rent/compute accounting, X3↔SVM atomic finality/refund; evidence on candidate head | UNVERIFIED |
| External bridges | RC1 scope lock excludes `external-gateway`; stored bridge enablement must remain false | Live proof that routes are disabled; future activation only after finality verifier, signer membership/quorum, replay/expiry/refunds, adversarial tests and independent custody audit | DISABLED BY POLICY; DYNAMIC PROOF PENDING |
| Validator economics | `pallet-x3-consensus`, X3 coin/governance, `scripts/mainnet/x3_economic_model_gate.sh` | Actual accrued block rewards/payouts, staking distribution, slashing triggers, treasury/fee conservation and projected security budget under sustained adversarial load | UNVERIFIED |
| Independent security | RustSec/npm scanners, internal security gates and code review | Critical/high findings disposition with expiry, independent audit of consensus, runtime, custody, bridges, and deployed contracts; secret/validator-key handling and replay threat model | NOT SIGNED OFF |
| Operations and incident response | Existing testnet deployment, health, snapshot, release, monitoring and fault-drill scripts | Seven **independent on-chain authority keys** and peers, common genesis/finality; clean upgrades, rollbacks, snapshot restore, partition/recovery, rate-limited public RPC, faucet controls, alerting, backups and operator ownership | NOT PROVED ON PHYSICAL FLEET |

A passing `cargo test` in a crate confirms only those tests. For example,
`tests/atomic/cross_vm_rollback_test.rs` simulates a failure with in-memory
booleans and **does not prove rollback of real chain state**. The
`runtime/tests/dex_atomic_rollback.rs` test is guarded with
`#[cfg(not(feature = "mainnet-rc1"))]`, so its green result is not evidence of
the narrowed launch runtime. No reports may relabel these as deployed proofs.

## Exact-head workflow

The dedicated `X3 Canonical Public Testnet Candidate` workflow in PR #703
independently attempts three gate families against its **full immutable SHA**:

1. Frozen source hashes, narrowed runtime roster, Python Guardian/queue tests
   and the required newer upstream security commit.
2. Atomic kernel, router, settlement, real liquidity, validator consensus,
   X3Lang trading receipt replay, nested X3Lang compiler and full-workspace formatting.
3. RC1 node compilation and the **separate** optional Frontier profile check,
   EVM/SVM integration unit tests, gateway proof-summary tests and genesis/economics policy.

These gates cannot prove physical validator or live EVM/SVM program execution.
The relevant existing isolated live harnesses include
`.github/workflows/x3vm-evm-live-lifecycle.yml` and
`.github/workflows/x3vm-svm-live-lifecycle.yml`. Their code requires local
Anvil/Solana nodes, fixed ports, tools and cleanup, so a trusted operator must
schedule them on dedicated lab capacity and attach **exact-head** evidence,
rather than launch them opportunistically on a shared validator host.

## Physical network / operational signoff

- `tests/testnet/check_seven_validators.py` is read-only. It requires
  distinct peer IDs and finality progress, but authority-set membership
  must be established separately from trusted on-chain session state.
- `scripts/testnet/validator-failure-drill.sh` kills test validators and
  **removes its BASE_DIR**; run only in an isolated, disposable environment,
  never against live production data or a reused chain state directory.
- `scripts/mainnet/verify-snapshot-restore.sh` tests state recovery using
  ephemeral local nodes; restoring a snapshot is **not** proof a validator can
  rejoin a running chain at that height.
- Collect an independently signed operator/security approval that cites
  exact candidate SHA, chain spec and genesis hash, binary and WASM digests,
  GitHub run/job IDs, observed block heights and restored-state evidence.

All operator assertions remain review inputs, not cryptographically validated
proofs, in Guardian v0.1. No automatic merge, public endpoint exposure,
external bridge enablement or deployment is permitted.

**Promotion rule:** every mandatory exact-head gate must pass, unresolved
security ancestry must be reconciled, independent audit findings must be
cleared, and the seven-validator live fleet must have dated evidence. Only a
separate human-approved GO decision can authorize public testnet exposure.
