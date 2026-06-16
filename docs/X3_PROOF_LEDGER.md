# X3 Proof Ledger

## Purpose
This ledger tracks every verified claim about the X3 Atomic Star system. Each claim has an evidence trail: source files, test results, CI gates, and runtime wiring.

## Recent: Verification Comment Remediation (2026-06-15)

Six security/correctness review comments remediated across 10 files.

| Comment | What | Files | Proof |
|---|---|---|---|
| 1 | Quorum dedup in SecurityReview | `policy.rs` | Regression test `security_review_rejects_duplicate_signer_replay` |
| 2 | GPU validator type alignment | `orchestrator.rs`, `evm_validator.rs`, `svm_validator.rs` | `cargo check -p cross-chain-gpu-validator` passes |
| 3 | Stubbed validation loop removal | `lib.rs` | Removed dead stubs; validators usable as building blocks |
| 4 | JIT compile() cfg fix | `jit_compiler.rs` | Prod returns unsupported error; test uses mock |
| 5 | SVM proof real signatures | `submitter.rs`, `types.rs` | BLAKE2b-256 + Ed25519 wired; `ValidatorSignature` struct |
| 6 | Automation oracle wiring | `lib.rs`, `mock.rs`, `tests.rs` | `type Oracle: OracleProvider` in Config; PriceThreshold tests |

## Claims

### CLAIM-001: Internal cross-VM routing works (Native ↔ Evm ↔ Svm)
| Field | Value |
|---|---|
| **Status** | ✅ PROVEN |
| **Source** | `pallets/pallet-x3-cross-vm-router/src/lib.rs` |
| **Tests** | 74+ unit tests + fuzz tests in CI |
| **Wiring** | Wired into all 6 runtime variants |
| **Proof** | `cargo test -p pallet-x3-cross-vm-router --features mainnet-rc1` |
| **Last verified** | 2026-06-09 |

### CLAIM-002: Supply invariant is enforced
| Field | Value |
|---|---|
| **Status** | ✅ PROVEN |
| **Source** | `pallets/pallet-x3-supply-ledger/src/lib.rs` |
| **Tests** | Supply invariant tests + fuzzing |
| **Wiring** | Active in all runtime variants |
| **Proof** | `cargo test -p pallet-x3-supply-ledger` |
| **Last verified** | 2026-06-09 |

### CLAIM-003: Settlement engine with OCW finalization exists
| Field | Value |
|---|---|
| **Status** | ✅ PROVEN |
| **Source** | `pallets/pallet-x3-settlement-engine/src/` |
| **Tests** | OCW tests, on_idle tests |
| **Wiring** | offchain_worker + on_idle + sidecar |
| **Proof** | `cargo test -p pallet-x3-settlement-engine` |
| **Last verified** | 2026-06-09 |

### CLAIM-004: Aura + GRANDPA consensus is real
| Field | Value |
|---|---|
| **Status** | ✅ PROVEN |
| **Source** | `runtime/src/lib.rs` (construct_runtime includes Aura + Grandpa) |
| **Tests** | 3-validator local testnet runs |
| **Wiring** | Node binary builds with both consensus engines |
| **Proof** | `cargo build --release -p x3-chain-node` |
| **Last verified** | 2026-06-09 |

### CLAIM-005: External bridges disabled at genesis
| Field | Value |
|---|---|
| **Status** | ✅ PROVEN |
| **Source** | Feature gate `mainnet-rc1` in `pallets/pallet-x3-cross-vm-router/Cargo.toml` |
| **Tests** | Gate 1 in Dockerfile.mainnet-check validates compile-time exclusion |
| **Wiring** | Faulty features cause compile-time #[cfg] error |
| **Proof** | `cargo check -p pallet-x3-cross-vm-router --features mainnet-rc1 --no-default-features` |
| **Last verified** | 2026-06-09 |

### CLAIM-006: No committed secrets remain in repo
| Field | Value |
|---|---|
| **Status** | ✅ PROVEN |
| **Source** | `deployment/keys/bootnode-keys.json` and `bootnode-node-key` replaced with placeholders |
| **Tests** | Manual inspection, CI secret scanning gate |
| **Wiring** | Files gitignored; CI blocks future commits |
| **Proof** | `grep -r "secret_key" deployment/keys/` returns only `REPLACED_` |
| **Last verified** | 2026-06-09 |

### CLAIM-007: Dockerfile.validator uses production-safe defaults
| Field | Value |
|---|---|
| **Status** | ✅ PROVEN |
| **Source** | `Dockerfile.validator` |
| **Tests** | N/A (build-time config change) |
| **Wiring** | CMD uses `--rpc-methods Safe`, no `--tmp`, no `--unsafe-rpc-external` |
| **Proof** | `grep -c "Unsafe\|--tmp" Dockerfile.validator` returns 0 |
| **Last verified** | 2026-06-09 |

### CLAIM-008: Toolchain is consistent across all build paths
| Field | Value |
|---|---|
| **Status** | ✅ PROVEN |
| **Source** | `rust-toolchain.toml`, `Dockerfile.validator`, `Dockerfile.indexer`, `Dockerfile.mainnet-check` |
| **Tests** | N/A (config alignment) |
| **Wiring** | All Rust build images use Rust 1.90.0 |
| **Proof** | `grep "RUST_VERSION\|rust:" Dockerfile.* rust-toolchain.toml` shows 1.90.0 everywhere |
| **Last verified** | 2026-06-09 |

### CLAIM-009: deny.toml exists and is valid
| Field | Value |
|---|---|
| **Status** | ✅ PROVEN |
| **Source** | `deny.toml` at repo root (copied by Dockerfile.mainnet-check) |
| **Tests** | Gate 5 in Dockerfile.mainnet-check |
| **Wiring** | Sourced in mainnet-check and CI pipelines |
| **Proof** | `cargo deny check` |
| **Last verified** | 2026-06-09 |

## Unproven Claims (future milestones)

### CLAIM-010: External EVM bridge is production-ready
**Status**: ❌ NOT PROVEN — governance-gated, disabled in RC-1

### CLAIM-011: Solana external bridge is production-ready
**Status**: ❌ NOT PROVEN — governance-gated, disabled in RC-1

### CLAIM-012: Public testnet has run 6+ weeks without critical incident
**Status**: ❌ NOT YET — testnet not yet launched publicly

### CLAIM-013: Mainnet genesis is frozen and signed
**Status**: ❌ NOT YET — pre-audit / pre-freeze

## Stub Detections
| File | Issue | Status |
|---|---|---|
| `deployment/keys/bootnode-keys.json` | Had real `secret_key` values | ✅ RESOLVED — replaced with placeholders |
| `deployment/keys/bootnode-node-key` | Had raw hex key | ✅ RESOLVED — replaced with placeholder |
| `Dockerfile.validator` | Used `--unsafe-rpc-external`, `--rpc-methods Unsafe`, `--tmp` | ✅ RESOLVED — switched to production-safe flags |
| `Dockerfile.mainnet-check` | Toolchain mismatch (1.82 vs 1.90) | ✅ RESOLVED — all on 1.90.0 |
| `Dockerfile.indexer` | Toolchain mismatch (1.80 vs 1.90) | ✅ RESOLVED — all on 1.90.0 |
| `deny.toml` | Missing file at repo root | ✅ RESOLVED — created |