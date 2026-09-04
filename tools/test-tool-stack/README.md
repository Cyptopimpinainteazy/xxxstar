# X3 Test Tool Stack

## Overview

X3's comprehensive testing and security verification tool stack. This is not a demo — these tools are integrated into the build, test, and release pipelines.

## Tool Tiers

### Tier 1: Rust/Substrate Core (Must-Have)

| Tool | Status | Purpose | X3 Target |
|------|--------|---------|-----------|
| `cargo-audit` | ✅ v0.22.2 | Dependency advisory scanning | CI gate |
| `cargo-deny` | ✅ v0.19.8 | License + advisory + supply-chain policy | CI gate |
| `cargo-mutants` | ✅ v27.1.0 | Mutation testing — test suite quality | asset-kernel, atomic-trade, bridge |
| `cargo-llvm-cov` | ✅ installed | LLVM coverage reporting | Workspace-wide |
| `proptest` | ✅ in Cargo.toml | Property-based testing | supply invariants, swap math, fees |
| `cargo-fuzz` | ⬜ needs install | LibFuzzer-based coverage-guided fuzzing | SCALE decode, bridge proofs, extrinsics |
| `cargo-nextest` | ⬜ needs install | Next-gen Rust test runner | Faster CI test execution |
| `cargo-geiger` | ⬜ needs install | Unsafe Rust usage auditing | GPU accelerator, SVM adapter |
| `loom` | ⬜ add dev-dep | Concurrency interleaving tests | mempool, reservation, nonce cache |
| `shuttle` | ⬜ add dev-dep | Randomized async concurrency tests | gossip, validator workers |
| `kani` | ⬜ needs install | Bounded model checking | overflow, impossible states, accounting |

### Tier 2: Security/Audit (Should-Have)

| Tool | Status | Purpose | X3 Target |
|------|--------|---------|-----------|
| `cargo-audit` | ✅ | CVE scanning | All deps |
| `cargo-deny` | ✅ | Policy enforcement | License compliance |
| `cargo-mutants` | ✅ | Test suite quality audit | Core pallets |

### Tier 3: Substrate/Chain

| Tool | Status | Purpose | X3 Target |
|------|--------|---------|-----------|
| `try-runtime` | ✅ in deps | Migration/replay safety | All runtime upgrades |
| `frame-benchmarking` | ✅ in deps | Pallet weight generation | All production pallets |
| `subwasm` | ⬜ needs install | Runtime WASM metadata diff | Release artifact validation |
| `srtool` | ⬜ Docker | Deterministic runtime WASM | Release builds |
| `Zombienet` | ⬜ npm install | Multi-node test network | Validator behavior, finality |
| `Chopsticks` | ⬜ npm install | Fork/replay/mutate chain state | Block replay, storage mutation |

### Tier 4: EVM/Solidity

| Tool | Status | Purpose | X3 Target |
|------|--------|---------|-----------|
| `Foundry (forge)` | ⬜ needs install | Solidity test + fuzz + invariant | Bridge contracts, vaults, DEX |
| `Foundry (anvil)` | ⬜ needs install | Local EVM test node | Fork testing |
| `Foundry (cast)` | ⬜ needs install | EVM RPC interaction | Contract interaction |
| `Slither` | ⬜ pip install | Solidity static analysis | Contract security audit |
| `Echidna` | ⬜ binary install | Stateful contract fuzzing | Multi-tx exploit paths |
| `Aderyn` | ⬜ cargo install | Solidity static analysis (Rust) | Contract security audit |

### Tier 5: SVM/Solana

| Tool | Status | Purpose | X3 Target |
|------|--------|---------|-----------|
| `LiteSVM` | ⬜ add dep | Fast in-process SVM testing | SVM adapter, account ownership |
| `solana-program-test` | ⬜ add dep | BanksClient SBF program tests | Deeper SVM execution behavior |

### Tier 6: Chaos/Load

| Tool | Status | Purpose | X3 Target |
|------|--------|---------|-----------|
| `Toxiproxy` | ⬜ binary install | Network failure simulation | RPC rotator, gossip, bridge relayer |
| `k6` | ⬜ binary install | RPC/API load testing | Transaction spam, quote endpoint stress |
| `Chaos Mesh` | ⬜ optional | K8s chaos platform | Validator cluster failure drills |

## Quick Install

```bash
# Rust tools (cargo-install)
cargo install cargo-fuzz cargo-nextest cargo-geiger subwasm aderyn --locked

# Kani Rust Verifier
cargo install kani-verifier --locked

# Foundry
curl -L https://foundry.paradigm.xyz | bash
foundryup

# Python tooling
pip3 install slither-analyzer

# Echidna (binary)
curl -L https://github.com/crytic/echidna/releases/download/v2.2.4/echidna-2.2.4-Ubuntu-22.04.tar.gz | tar -xz
cp echidna ~/.cargo/bin/

# Load testing
curl -L https://github.com/grafana/k6/releases/download/v0.54.0/k6-v0.54.0-linux-amd64.tar.gz | tar -xz
cp k6-v0.54.0-linux-amd64/k6 ~/.cargo/bin/

# Toxiproxy
curl -L https://github.com/shopify/toxiproxy/releases/download/v2.9.0/toxiproxy-server-linux-amd64 -o ~/.cargo/bin/toxiproxy-server
curl -L https://github.com/shopify/toxiproxy/releases/download/v2.9.0/toxiproxy-cli-linux-amd64 -o ~/.cargo/bin/toxiproxy-cli
chmod +x ~/.cargo/bin/toxiproxy-*

# Zombienet + Chopsticks (npm)
npm install -g @zombienet/cli @polkadot/chopsticks

# Dev-dependencies for loom/shuttle/LiteSVM (add to Cargo.toml when needed)
## loom, shuttle, litesvm (crates.io)
```

## Makefile Integration

All tools integrated via `Makefile` targets:

```bash
make install-tools      # Install all test tools
make install-rust-tools # Install only Rust tooling
make install-python     # Install Python tooling
make audit              # cargo audit + deny
make coverage           # cargo llvm-cov
make mutants            # cargo mutants on core crates
make fuzz               # Run fuzz targets
make substrate-check    # try-runtime + zombienet
make evm-check          # forge test + slither
make chaos-check        # k6 + toxiproxy scenarios
```

## Proof Report Directory

Reports go in `reports/`:

```
reports/security/       # Audit, static analysis, SBOM
reports/fuzzing/        # Fuzz campaign results
reports/invariants/     # Invariant test results
reports/substrate/      # try-runtime, weights, zombienet
reports/provenance/     # SLSA, signed artifacts
reports/benchmarks/     # Performance benchmarks
reports/sbom/           # Software Bill of Materials
```

## Validation Gates (CI)

Each tier maps to a CI gate:

1. **Gate 0: Compile** — `cargo check --workspace`
2. **Gate 1: Unit** — `cargo nextest run`
3. **Gate 2: Coverage** — `cargo llvm-cov`
4. **Gate 3: Security** — `cargo audit + cargo deny`
5. **Gate 4: Mutations** — `cargo mutants` on core crates
6. **Gate 5: Fuzz** — `cargo fuzz` for critical paths
7. **Gate 6: Substrate** — `try-runtime`, `frame-benchmarking`
8. **Gate 7: EVM** — `forge test`, `slither`, `echidna`
9. **Gate 8: Integration** — `Zombienet`, `Chopsticks`
10. **Gate 9: Release** — `srtool`, `subwasm`, signed artifacts

See `docs/X3_PROOF_LEDGER.md` for current gate status.