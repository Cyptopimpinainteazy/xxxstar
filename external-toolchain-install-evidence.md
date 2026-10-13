# X3 External Toolchain — Install & Verification Evidence

**Date:** 2026-10-01
**Repo commit at start:** `ff57d15dd6e93ba4bcc19081d18bcd5632954941`
**Supersedes:** the `status` / `installed_on_host` fields in `external-tool-inventory.json` for every tool listed below. Phase-0 statuses there were measured before anything was installed.

This records what was installed, through which channel, and — critically — what each tool was **proved** to do against a known-good and a deliberately-broken fixture. An installed binary is not an implemented tool.

---

## 1. Environment facts that shaped the install

| Fact | Value | Consequence |
|---|---|---|
| sudo | **not available** (password required) | no `apt install` — everything went to `~/.cargo/bin`, `~/.local/bin`, or a venv |
| `python3-venv` | missing (`ensurepip` absent) | bootstrapped with `uv` instead |
| crates.io / GitHub / PyPI | reachable (only in an escalated shell; the default sandbox has no network) | all installs needed approval |
| rustc | 1.90.0 pinned by `rust-toolchain.toml` | `cargo-binstall` latest needs 1.91+ → installed the prebuilt binary |
| disk / CPU | 121 GB free, 8 cores, 31 GB RAM | no capacity constraint |
| `NO_COLOR=1` | set in the environment | breaks `subwasm` arg parsing (upstream bug); use `env -u NO_COLOR` |

---

## 2. Installed tools and versions

All of these were verified with `--version` after install.

| Tool | Version | Channel | Path |
|---|---|---|---|
| cargo-binstall | 1.24.0 | prebuilt release (bootstrap) | `~/.cargo/bin/cargo-binstall` |
| cargo-audit | 0.22.2 | cargo-binstall | `~/.cargo/bin/cargo-audit` |
| cargo-deny | 0.20.2 | cargo-binstall | `~/.cargo/bin/cargo-deny` |
| cargo-fuzz | 0.13.2 | cargo-binstall | `~/.cargo/bin/cargo-fuzz` |
| cargo-mutants | 27.1.0 | cargo-binstall | `~/.cargo/bin/cargo-mutants` |
| cargo-nextest | 0.9.146 | cargo-binstall | `~/.cargo/bin/cargo-nextest` |
| cargo-geiger | 0.13.0 | cargo-binstall | `~/.cargo/bin/cargo-geiger` |
| cargo-llvm-cov | 0.9.1 | cargo-binstall | `~/.cargo/bin/cargo-llvm-cov` |
| Kani | 0.68.0 (CBMC 6.11.0) | cargo-binstall + `cargo kani setup` | `~/.cargo/bin/kani`, `~/.kani/kani-0.68.0` |
| Miri | 0.1.0 (nightly 1.101.0) | rustup | nightly toolchain |
| rustup toolchains | `stable`, `1.90.0` (default), `nightly` 1.101.0, `nightly-2026-08-21` (Kani) | rustup | `~/.rustup` |
| Slither | 0.11.6 | uv venv | `~/.venvs/x3-tools/bin/slither` |
| Semgrep | 1.178.0 | uv venv | `~/.venvs/x3-tools/bin/semgrep` |
| Halmos | 0.3.3 | uv venv | `~/.venvs/x3-tools/bin/halmos` |
| pytest | 9.1.1 | uv venv | `~/.venvs/x3-tools/bin/pytest` |
| Ansible | core 2.21.4 | uv venv | `~/.venvs/x3-tools/bin/ansible` |
| solc-select | latest | uv venv | compiles 0.8.20 / 0.8.24 |
| uv | 0.12.21 | prebuilt release | `~/.local/bin/uv` |
| Foundry (forge/cast/anvil) | 1.8.3 | pre-existing | `~/.local/bin/` |
| Echidna | 2.3.3 | GitHub release | `~/.local/bin/echidna` |
| Medusa | 1.5.1 | GitHub release | `~/.local/bin/medusa` |
| k6 | 2.3.0 | GitHub release | `~/.local/bin/k6` |
| Trivy | 0.75.0 | GitHub release | `~/.local/bin/trivy` |
| Gitleaks | 8.30.1 | GitHub release | `~/.local/bin/gitleaks` |
| Syft | 1.54.0 | GitHub release | `~/.local/bin/syft` |
| Grype | 0.119.0 | GitHub release | `~/.local/bin/grype` |
| OSV-Scanner | 2.6.0 | GitHub release | `~/.local/bin/osv-scanner` |
| CodeQL CLI | 2.27.1 | GitHub bundle (661 MB) | `~/.local/bin/codeql` |
| Toxiproxy | 2.12.0 (server + cli) | GitHub release | `~/.local/bin/toxiproxy-*` |
| Prometheus | 3.15.0 | GitHub release | `~/.local/bin/prometheus` |
| Alertmanager | 0.34.1 | GitHub release | `~/.local/bin/alertmanager` |
| Loki | 3.7.8 | GitHub release | `~/.local/bin/loki` |
| Grafana | 13.2.3 | official tarball | `~/.local/bin/grafana` + `~/.local/share/grafana-13.2.3` |
| Zombienet | 1.3.10 | npm `@parity/zombienet` | nvm bin |
| subwasm | 0.21.3 | .deb extraction | `~/.local/bin/subwasm` |
| srtool | 0.13.2 | pre-existing | `~/.cargo/bin/srtool` |
| Solana / Agave | 4.3.0, `cargo-build-sbf` 4.4.0 | official tarball | `~/.local/share/solana/install/bin` + symlinks |
| Docker | 29.1.3 | pre-existing | `/usr/bin/docker` |

Also installed into the EVM project (gitignored, from `foundry.lock`): **forge-std v1.16.1** and **OpenZeppelin v4.9.6** at `X3-contracts/evm/lib/`.

---

## 3. Proof runs — known-good vs deliberately-broken

Every row below is an executed command with an observed result, not a version check.

| Tool | Fixture | Result |
|---|---|---|
| **Miri** | throwaway crate with one clean test and one misaligned read | clean test **passes**; UB test **aborts** (misaligned pointer dereference), full run exit **1** |
| **Kani** | 1 correct proof + 1 deliberately wrong proof | `Complete - 1 successfully verified harnesses, 1 failures` → exit **1** |
| **cargo-fuzz** | X3 target `x3bc_engines` (X3-language bytecode), 60 s | **2,302,846** executions, cov **926**, corpus **61** entries, 0 crashes |
| **cargo-audit** | X3 `Cargo.lock` | **1 vulnerability + 20 allowed warnings**, exit 1 — real finding `RUSTSEC-2026-0255` (sized-chunks unsound) |
| **cargo-deny** | X3 workspace | `advisories FAILED` — gate is live |
| **Semgrep** | custom rule vs `unwrap()` file and safe file | fires on dirty, **silent on clean** |
| **Gitleaks** | planted synthetic GitHub PAT vs clean file | **leaks found: 1** (exit 1) vs none (exit 0) |
| **Slither** | `X3-contracts/evm` (after deps + build) | **53 contracts, 90 detectors, 13 results** — all in vendored OpenZeppelin |
| **Foundry** | `forge build` + `forge test` | build OK; foundry linter flags `arbitrary-send-eth` in `FoundryDisputeResolver.sol:320`; **168 tests passed, 0 failed** |
| **Echidna** | `tests_core/security/contracts/InvariantProperties.sol` | compiles + analyzes, but **deployment reverts** → the committed property fixture cannot run |
| **OSV-Scanner** | full repo | multiple advisories; incl. **next 16.3.4 → 9.5 critical** in `x3fronend` |
| **Trivy** | `trivy fs` vuln+secret | **CRITICAL** `next` GHSA-vcvr-r3jv-pc5j; **HIGH** `rustls-webpki`; more cargo-toml findings |
| **Toxiproxy** | local server | proxy created; **500 ms latency toxic injected** and read back |
| **Prometheus** | local instance + self-scrape | `/-/healthy` → 200; query API returns valid JSON |
| **k6** | 3-iteration script | executed, emitted metrics |
| **Syft** | repo scan | ran; full-repo scan is slow — needs a scoped target |
| **Grafana** | full distribution | started, migrations executed; health check not confirmed inside the wait window |
| **Loki** | minimal config | **failed to start** (`failed services`) — needs a validated config |

Tools installed but **not yet exercised**: cargo-nextest, cargo-geiger, cargo-llvm-cov, cargo-mutants (re-run), Halmos, Medusa, Ansible, CodeQL (CLI only, no DB build), Alertmanager, subwasm, Zombienet (CLI only), solana-test-validator, srtool, Trivy SBOM/Grype chain.

---

## 4. Defects found in the X3 tooling itself (not the tools)

These are actionable repo bugs, discovered by running the tools:

1. **`tests_core/security/echidna.config.yaml` is malformed** — `balanceAddr` / `balanceContract` are quoted strings; Echidna 2.3.3 rejects them (`parsing Integer failed … encountered String`). Also its paths (`tests/solidity_contracts/`, `tests/security/echidna-corpus/`) do not match where the contracts actually live (`tests_core/security/contracts/`).
2. **The Echidna fixture cannot deploy** — `InvariantProperties` reverts on deployment, so the five declared invariants never run.
3. **`X3-contracts/evm/lib` was empty** — `foundry.lock` pins forge-std v1.16.1 and OpenZeppelin v4.9.6, but nothing had installed them; `forge install` is a no-op on Foundry 1.8. Everything EVM-side was unbuildable before this.
4. **`crates/x3-integration/fuzz` breaks workspace-wide commands** — currently it fails `cargo check --workspace` on `libfuzzer-sys`, and its default musl target cannot link ASAN (`sanitizer is incompatible with statically linked libc`); `--target x86_64-unknown-linux-gnu` works.
5. **Gitleaks default allowlist** silently ignores the canonical AWS example keys — a "no leaks found" on documentation samples is not evidence of a clean scan.
6. **`subwasm` is unusable under `NO_COLOR=1`** (upstream arg-parsing bug).
7. **Slither's committed config excludes `reentrancy-no-eth`, `arbitrary-send-eth`, `timestamp`, `low-level-calls` and 8 more** — which is why all 13 current findings are naming/style issues in vendored OpenZeppelin rather than anything in X3's own contracts. Each exclusion needs a written justification.
8. **Real dependency findings** that should be triaged: `RUSTSEC-2026-0255` sized-chunks, `next 16.3.4` critical RCE, `rustls-webpki` DoS, `brace-expansion` / `fast-uri` in npm lockfiles.

---

## 5. Still blocked

| Item | Blocker |
|---|---|
| Ansible on physical hosts | no sudo for package install; playbooks not written |
| Zombienet end-to-end | needs a built X3 node binary (release build not yet produced here) |
| Loki / Grafana health | config still needs validation |
| try-runtime | standalone CLI no longer exists in the pinned SDK — needs a `frame-try-runtime` runner (code work, not an install) |
| cargo-mutants re-run on security crates | long-running; was interrupted |
| AFL++, honggfuzz, Pumba, Chaos Mesh, Podman, Tempo, wrk, Valgrind | deliberately deferred/rejected in `external-tool-gap-analysis.md` |

---

## 6. Workspace hygiene note

A **second agent is working in this repository concurrently** (it created `tools/tool-validation/`, `scripts/external-toolchain/*`, modified `Cargo.toml` and `scripts/local-ci.sh`, and is running its own fuzz campaign into `audit-artifacts/external-toolchain/`). Those changes were left untouched.

My interrupted `cargo-mutants` run rotated the committed `mutants.out/` evidence into `mutants.out.old/`. That was **restored from HEAD** with `git restore --source=HEAD --worktree -- mutants.out mutants.out.old`. The `crytic-export/` directory and crytic cache file my Echidna run produced were also removed/restored. No tracked change in this report's scope remains in the worktree.
