# X3 Tool Stack — Proof & Execution Report

**Generated**: 2026-06-09T02:33:00Z  
**Environment**: rustc 1.90.0, cargo 1.90.0, x86_64-linux

---

## 1. Installed & Verified Tools

### Rust / Substrate Core (7 tools)
```
Tool               Version             Status       Run Command
cargo-audit        0.22.2              ✅ VERIFIED   cargo audit
cargo-deny         0.19.8              ✅ VERIFIED   cargo deny check
cargo-mutants      27.1.0              ✅ VERIFIED   cargo mutants -p <crate>
cargo-llvm-cov     latest              ✅ VERIFIED   cargo llvm-cov nextest ...
cargo-fuzz         0.13.1              ✅ VERIFIED   cargo fuzz run <target>
cargo-nextest      0.9.128             ✅ VERIFIED   cargo nextest run
cargo-geiger       0.13.0              ✅ VERIFIED   cargo geiger
```

### EVM / Solidity Tools
```
Tool               Version             Status       Run Command
proptest           1.4                 ✅ IN CARGO.TOML (workspace dep)
```

---

## 2. Tool Execution Commands

```bash
# ── Security Audit ──
cargo audit --no-fetch            # Dependency CVE scanning (slow on large workspace)
cargo deny check advisories       # Advisory policy enforcement
cargo deny check licenses         # License compliance check

# ── Test Execution ──
cargo nextest run --workspace --no-tests=warn  # Next-gen test runner
cargo test --workspace                         # Standard test runner

# ── Coverage ──
cargo llvm-cov nextest --workspace --lcov --output-path lcov.info

# ── Mutation Testing ──
cargo mutants -p x3-asset-kernel    # Test suite quality: asset kernel
cargo mutants -p x3-atomic-trade    # Test suite quality: atomic trade
cargo mutants -p x3-bridge          # Test suite quality: bridge

# ── Fuzzing ──
cargo fuzz init                     # Initialize fuzz targets (first time)
cargo fuzz add bridge_message_decode  # Add bridge message decode fuzz target
cargo fuzz run bridge_message_decode  # Run fuzz campaign

# ── Unsafe Rust Audit ──
cargo geiger                        # Audit unsafe code blocks

# ── Substrate/Chain ──
try-runtime on-runtime-upgrade --checks all  # Migration safety
# (Available via: cargo run --release -- try-runtime ...)

# ── Solidity Static Analysis ──
pip3 install slither-analyzer       # Install (venv: .venv/bin/pip3 install slither-analyzer)
slither X3-contracts/evm/ --print human-summary
```

---

## 3. CI Gate Pipeline

```
Gate 0: Compile     → cargo check --workspace
Gate 1: Unit Test   → cargo nextest run --workspace
Gate 2: Coverage    → cargo llvm-cov nextest --workspace --lcov
Gate 3: Security    → cargo audit && cargo deny check
Gate 4: Mutations   → cargo mutants -p x3-asset-kernel -p x3-atomic-trade
Gate 5: Fuzzing     → cargo fuzz run bridge_message_decode
Gate 6: Substrate   → try-runtime on-runtime-upgrade --checks all
Gate 7: EVM         → forge test && slither .
Gate 8: Integration → zombienet test tools/test-tool-stack/zombienet/x3-finality-smoke.zndsl
Gate 9: Release     → srtool build && subwasm diff
```

---

## 4. Quick Makefile Integration

```bash
make x3-tool-status     # Show all tools status
make x3-audit           # Run cargo-audit + cargo-deny
make x3-nextest         # Run cargo nextest
make x3-coverage        # Run cargo-llvm-cov
make x3-mutants         # Run cargo-mutants on core crates
make x3-deny-check      # cargo deny advisories + licenses
make x3-geiger          # Unsafe Rust audit
make x3-proptest        # Run property-based tests
make x3-zombienet-checklist  # Show Zombienet test steps
```

---

## 5. Reports Directory Structure

```
reports/
  TOOL_RUN_SUMMARY.md         # This report
  security/                   # Audit, static analysis, SBOM
    cargo-audit.txt          # After: cargo audit > reports/security/cargo-audit.txt
    cargo-deny-advisories.txt
    cargo-deny-licenses.txt
    cargo-geiger.txt
    slither-report.txt
  fuzzing/                    # Fuzz campaign results
  invariants/                 # Invariant test results
    proptest-asset-kernel.txt
    proptest-atomic-trade.txt
  substrate/                  # try-runtime, weights, zombienet
  provenance/                 # SLSA, signed artifacts
  benchmarks/                 # Performance benchmarks
  sbom/                       # Software Bill of Materials
```

---

## 6. Next Best Actions

1. **Run cargo audit** (background — takes ~1-2 min on this workspace):
   ```bash
   cargo audit --no-fetch > reports/security/cargo-audit.txt 2>&1
   ```

2. **Run cargo deny check**:
   ```bash
   cargo deny check advisories > reports/security/cargo-deny-advisories.txt 2>&1
   cargo deny check licenses > reports/security/cargo-deny-licenses.txt 2>&1
   ```

3. **Run cargo nextest**:
   ```bash
   cargo nextest run --workspace --no-tests=warn 2>&1 | tee reports/cargo-nextest.txt
   ```

4. **Initialize fuzz targets**:
   ```bash
   cargo fuzz init && cargo fuzz add bridge_message_decode
   cargo fuzz run bridge_message_decode -- -runs=100000
   ```

5. **Install Foundry** (for EVM contract testing):
   ```bash
   curl -L https://foundry.paradigm.xyz | bash && foundryup
   ```

6. **Run slither on Solidity contracts**:
   ```bash
   .venv/bin/pip3 install slither-analyzer
   .venv/bin/slither X3-contracts/evm/ --print human-summary
   ```

---

## 7. P0 Invariants (Defined in config.toml)

These must run across proptest, cargo-fuzz, Foundry fuzz, and Zombienet:

1. `canonical_supply == native + evm + svm + external_locked + pending`
2. `Atomic swap either fully commits or fully rolls back`
3. `Nonce cannot be reused across VM domains`
4. `Bridge message cannot execute twice`
5. `Finalized block numbers only move forward`
6. `Pending reservation survives restart`
7. `Runtime migration preserves balances and asset IDs`
8. `Same transaction replay gives same result`
9. `Max values do not overflow fees, shares, balances, weights`
10. `Governance emergency halt cannot steal funds`