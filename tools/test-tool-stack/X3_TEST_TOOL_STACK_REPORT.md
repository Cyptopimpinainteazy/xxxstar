# X3 Test Tool Stack — Installation & Verification Report

**Date**: 2026-06-09
**Rust**: 1.90.0 (x86_64-unknown-linux-gnu)
**Project**: xxxstar (X3 Chain)

---

## Installation Results

### ✅ Installed & Verified

| Tool | Version | Source | Status |
|------|---------|--------|--------|
| `cargo-audit` | 0.22.2 | `cargo install` | ✅ Verified |
| `cargo-deny` | 0.19.8 | `cargo install` | ✅ Verified |
| `cargo-mutants` | 27.1.0 | `cargo install` | ✅ Binary exists |
| `cargo-llvm-cov` | latest | `cargo install` | ✅ Binary exists |
| `cargo-fuzz` | 0.13.1 | `cargo install --locked` | ✅ Fresh install |
| `cargo-geiger` | 0.13.0 | `cargo install --locked` | ✅ Fresh install |
| `slither` | 0.11.5 | `pip3 install slither-analyzer` | ✅ Fresh install |
| `proptest` | 1.4 | workspace Cargo.toml | ✅ In deps |

### ❌ Not Installed / Failed

| Tool | Reason |
|------|--------|
| `cargo-nextest` | Requires rustc ≥1.91 (we have 1.90). Pin v0.9.128: `cargo install cargo-nextest --version 0.9.128 --locked` |
| `subwasm` | Not on crates.io. Install via: `cargo install subwasm-cli --locked` |
| `aderyn` | Build failed on rustc 1.90. Try: `cargo install aderyn --git https://github.com/Cyfrin/aderyn` |
| `echidna` | Binary download needed: see below |
| `forge` / `cast` / `anvil` | `curl -L https://foundry.paradigm.xyz | bash && foundryup` |
| `k6` | Binary download needed: `curl -L https://github.com/grafana/k6/releases/download/v0.54.0/k6-v0.54.0-linux-amd64.tar.gz` |
| `toxiproxy-cli` | Binary download needed: `curl -L https://github.com/shopify/toxiproxy/releases/download/v2.9.0/toxiproxy-cli-linux-amd64` |
| `kani` | `cargo install kani-verifier --locked` (may need rustup toolchain) |
| `loom` | Add as dev-dep: `loom = "0.7"` |
| `shuttle` | Add as dev-dep: `shuttle = "0.7"` |
| `litesvm` | Add as dev-dep: `litesvm = "0.1"` |

---

## Recommendation: Install Remaining Tools

Run these commands to complete the tool stack:

```bash
# 1. Install compatible cargo-nextest
cargo install cargo-nextest --version 0.9.128 --locked

# 2. Install subwasm (runtime WASM metadata tool)
cargo install subwasm-cli --locked

# 3. Install aderyn from git
cargo install aderyn --git https://github.com/Cyfrin/aderyn --locked

# 4. Kani Rust Verifier
cargo install kani-verifier --locked

# 5. Foundry
curl -L https://foundry.paradigm.xyz | bash
foundryup

# 6. Python tooling (echidna via binary)
wget https://github.com/crytic/echidna/releases/download/v2.2.4/echidna-2.2.4-Ubuntu-22.04.tar.gz
tar -xzf echidna-2.2.4-Ubuntu-22.04.tar.gz
cp echidna ~/.cargo/bin/

# 7. Infrastructure tools
# k6
wget https://github.com/grafana/k6/releases/download/v0.54.0/k6-v0.54.0-linux-amd64.tar.gz
tar -xzf k6-v0.54.0-linux-amd64.tar.gz
cp k6-v0.54.0-linux-amd64/k6 ~/.cargo/bin/

# Toxiproxy
wget https://github.com/shopify/toxiproxy/releases/download/v2.9.0/toxiproxy-server-linux-amd64 -O ~/.cargo/bin/toxiproxy-server
wget https://github.com/shopify/toxiproxy/releases/download/v2.9.0/toxiproxy-cli-linux-amd64 -O ~/.cargo/bin/toxiproxy-cli
chmod +x ~/.cargo/bin/toxiproxy-*

# 8. Zombienet + Chopsticks (npm)
npm install -g @zombienet/cli @polkadot/chopsticks
```

---

## Quick Smoke Tests

After installing, verify with:

```bash
# Security
cargo audit --no-fetch 2>&1 | head -10
cargo deny check 2>&1 | head -10

# Fuzzing
ls ~/.cargo/bin/cargo-fuzz && echo "fuzz ready"
# cargo fuzz init && cargo fuzz add bridge_message_decode

# Unsafe Rust audit
cargo geiger 2>&1 | head -20

# Solidity static analysis
slither X3-contracts/evm/ --print human-summary

# Coverage
cargo llvm-cov nextest --workspace --lcov --output-path lcov.info

# Makefile
make x3-tool-status
```

---

## Files Created

```
tools/test-tool-stack/
  README.md                    # Tool stack overview & install guide
  config.toml                  # Central configuration
  setup-x3-test-tools.sh       # Automated install script
  X3_TEST_TOOL_STACK_REPORT.md # This report
  fuzz-targets/
    bridge_message_decode.rs   # Fuzz harness template
  chaos-scenarios/
    rpc-spam.js                # k6 load test script
  zombienet/
    x3-local-7.toml            # 7-validator local network config
    x3-finality-smoke.zndsl    # Zombienet DSL smoke test

reports/
  security/                    # Security audit reports
  fuzzing/                     # Fuzz campaign results
  invariants/                  # Invariant test results
  substrate/                   # try-runtime, weights, zombienet
  provenance/                  # SLSA, signed artifacts
  benchmarks/                  # Performance benchmarks
  sbom/                        # Software Bill of Materials
```

## Makefile Integration

Added targets:

| Make Target | Purpose |
|-------------|---------|
| `make install-tools` | Install all test tools |
| `make install-rust-tools` | Install Rust cargo tools |
| `make install-python` | Install Python tools |
| `make install-chaos` | Install chaos/load tools |
| `make x3-audit` | Run cargo-audit + cargo-deny |
| `make x3-coverage` | Run cargo-llvm-cov |
| `make x3-mutants` | Run cargo-mutants on core crates |
| `make x3-fuzz` | Run fuzz targets |
| `make x3-nextest` | Run cargo nextest |
| `make x3-deny-check` | Run cargo-deny advisories + licenses |
| `make x3-geiger` | Run unsafe usage audit |
| `make x3-proptest` | Run property-based tests |
| `make x3-zombienet-checklist` | Show Zombienet test steps |
| `make x3-substrate-report` | Show Substrate tool status |
| `make x3-tool-status` | Show all tool status |

## CI Gate Mapping

| Gate | Tool | Purpose |
|------|------|---------|
| 0 | cargo check | Compile gate |
| 1 | cargo test / nextest | Unit test gate |
| 2 | cargo-llvm-cov | Coverage gate |
| 3 | cargo-audit + cargo-deny | Security gate |
| 4 | cargo-mutants | Mutation test gate |
| 5 | cargo-fuzz | Fuzz gate |
| 6 | try-runtime | Substrate migration gate |
| 7 | forge + slither | EVM contract gate |
| 8 | zombienet | Integration gate |
| 9 | srtool + subwasm | Release gate |