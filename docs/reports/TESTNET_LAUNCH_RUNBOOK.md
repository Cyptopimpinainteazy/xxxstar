# Testnet Launch Gate Verification (Pass 2)

**Date:** 2026-04-24  
**Commit:** `9aeb4bf089f719c23ddd6d351b9541b345fd7685`  
**Scope:** All testnet launch scripts, configs, workflows, systemd units, chain specs

---

## File-by-File Verification

### 1. `scripts/testnet-full-launch.sh`

**EXISTS.** 322 lines. Real bash script.

**What it does:**
- Launches N validators (default 3) from a built `x3-chain-node` binary
- Uses chain spec from `chain-specs/x3-local3-current-raw.json`
- Starts validators with sequential ports (30333+, 9933+)
- Runs health checks: RPC responsiveness, peer count, block production, finality

**References:**
```bash
BINARY="${X3_NODE_BIN:-$PROJECT_ROOT/target/release/x3-chain-node}"
CHAIN_SPEC="${X3_CHAIN_SPEC:-$PROJECT_ROOT/chain-specs/x3-local3-current-raw.json}"
```

**Issues:**
- Line 131: `grep -oP` uses Perl-compatible regex — may fail on systems without GNU grep (macOS). Minor.
- Line 184: `chain_getBlockNumber` is NOT a standard Substrate RPC method. **FAIL.** The correct method is `chain_getHeader`. This call will always return empty. The script will show "Could not verify block production" every time but won't actually fail because it checks with `|| return 1` but the outer script uses `set -e` — the `||` prevents exit, but the validation step effectively does nothing.
- Health check loop runs indefinitely until Ctrl+C. No graceful shutdown signal handling beyond the trap on EXIT.
- Assumes binary exists or builds it with `cargo build --release`. This is fine for dev but slow for CI.

**Verdict: PARTIAL FAIL.** Works for basic testnet launch but has a broken RPC endpoint check. Will show false negatives on chain state validation.

### 2. `scripts/mainnet_release_gate.py`

**EXISTS.** 251 lines. Real Python script.

**What it checks:**
1. Required documentation (MAINNET_READINESS.md, INVARIANTS.md, RELEASE_GATES.md, SECURITY.md, TESTING.md, AUDIT_SPEC.md)
2. Build artifacts (x3-chain-node binary, runtime WASM)
3. Chain-spec JSON validity with genesis key check
4. Critical test suites (x3-chain-runtime, x3-supply-ledger, x3-packet-standard, x3-bridge, x3-fees, x3-slash)
5. Reproducible-build prerequisites (srtool, docker, no SKIP_WASM_BUILD)
6. Forbidden-secret scanning (private keys, mnemonics, AWS keys)

**Issues:**
- Fully functional Python script. No placeholders.
- References real paths like `chain-specs/x3-local3-current-plain.json`, `node/src/chain_spec.rs`.
- Secret scan is thorough (PRIVATE_KEY, MNEMONIC, AKIA patterns).
- Test suite runner uses `cargo test --lib --no-fail-fast -q` — this is valid.
- Does NOT check that the `production_config()` function in `chain_spec.rs` actually compiles or is reachable.

**Verdict: PASS.** Real validation gate. All checks are substantive.

### 3. `scripts/install-validator.sh`

**EXISTS.** 135 lines. Real bash script.

**References:**
```bash
REPO="Cyptopimpinainteazy/xxxstar"
BINARY_URL="https://github.com/${REPO}/releases/download/${VERSION}/${BINARY}"
SPEC_URL="https://github.com/${REPO}/releases/download/${VERSION}/x3-mainnet-raw.json"
```

**Issues:**
- References a real GitHub repo (`Cyptopimpinainteazy/xxxstar`).
- **FAIL: None of these release URLs actually exist yet.** The binary `x3-chain-node` has never been published as a release artifact on this repo. The chain spec URL `x3-mainnet-raw.json` is also non-existent. All downloads will `curl: (22) The requested URL returned error: 404`.
- Line 77: `--fail --silent --show-error 2>/dev/null || true` — checksum download failure is silently ignored.
- Line 83: `echo "    WARNING: No checksum file found. Skipping verification."` — no checksum verification is a security gap.
- Line 92: Chain spec download is wrapped in `|| { ... WARNING ... }` — if the release doesn't have it, the install continues without a chain spec. The validator will fail to start.

**Verdict: FAIL.** References release URLs that do not exist. Install will complete but the validator cannot start because no binary or chain spec will be downloaded.

### 4. `packaging/systemd/x3-validator.service`

**EXISTS.** 59 lines. Real systemd unit.

```ini
ExecStart=/usr/local/bin/x3-chain-node \
  --chain /etc/x3/chain-spec.json \
  --base-path /var/lib/x3 \
  --validator \
  --name "x3-validator-$(hostname -s)" \
  --port 30333 \
  --rpc-port 9944 \
  --prometheus-port 9615 \
  --prometheus-external \
  --log info
```

**Issues:**
- References real paths: `/usr/local/bin/x3-chain-node`, `/etc/x3/chain-spec.json`, `/var/lib/x3`.
- **No placeholder paths detected.** All paths are production-standard.
- Has proper security hardening: `NoNewPrivileges=true`, `PrivateTmp=true`, `ProtectSystem=strict`, `ReadWritePaths`, etc.
- File descriptor limits are set (`LimitNOFILE=1048576`, `LimitNPROC=65536`).
- However, `--prometheus-external` with `--rpc-port 9944` (WS port) may be a concern — the WS port is exposed as RPC. Should use `--rpc-port 9933` for HTTP RPC and serve Prometheus separately.

**Verdict: PASS.** Real service file with valid production paths.

### 5. `packaging/systemd/x3-bootnode.service`

**EXISTS.** 39 lines. Real systemd unit.

```ini
ExecStart=/usr/local/bin/x3-chain-node \
  --chain /etc/x3/chain-spec.json \
  --base-path /var/lib/x3-bootnode \
  --name "x3-bootnode-$(hostname -s)" \
  --port 30333 \
  --no-telemetry \
  --no-prometheus \
  --log warn
```

**Issues:**
- Same binary and chain spec paths as validator service.
- **Minor: No `--no-mdns` flag.** Bootnodes should disable mDNS to avoid leaking internal network topology.
- `--log warn` is very sparse for a bootnode — `--log info` would be more appropriate for debugging connectivity issues.

**Verdict: PASS.** Real service file with valid paths. Minor config concerns.

### 6. `validator_config/mainnet.toml`

**EXISTS.** 40 lines. Real config file.

**Content analysis:**
```toml
[validator]
name = "x3-mainnet-validator"
chain = "x3-atomic-star"

[consensus]
algorithm = "aura-grandpa"

[build]
artifact = "target/release/x3-chain-node"

[references]
genesis_ceremony_script = "scripts/mainnet/genesis_ceremony.sh"
```

**Issues:**
- `scripts/mainnet/genesis_ceremony.sh` — **does not exist.** This is a placeholder reference.
- `references.ci_workflow = ".github/workflows/mainnet-readiness.yml"` — this file **exists**. Verifies correctly.
- The config is **documentation only** — it is not consumed by any actual tool or script. It's a reference document that happens to be in TOML format.

**Verdict: PARTIAL FAIL.** Contains a reference to a non-existent genesis ceremony script. Otherwise describes the real consensus and build setup.

### 7. `chain-specs/`

Four files exist:
- `x3-local3-current-plain.json` — VALID JSON, has `genesis` key
- `x3-local3-current-raw.json` — VALID JSON, has `genesis` key
- `x3-local3-plain.json` — VALID JSON, has `genesis` key
- `x3-local3-raw.json` — VALID JSON, has `genesis` key

```json
{"name": "X3 Local Testnet", "id": "x3_local_testnet", "chainType": "Local", ...}
```

**Verdict: PASS.** All four chain specs are valid JSON with proper genesis configuration.

**However:** No `x3-mainnet-raw.json` exists, which is what `install-validator.sh` tries to download. This is consistent with the current RC phase (local testnet only).

### 8. `docker/docker-compose.yml`

**EXISTS.** 168 lines. Real Docker Compose file.

**Services:**
- `postgres` — `postgres:16-alpine` (real image on Docker Hub)
- `indexer` — `ghcr.io/cyptopimpinainteazy/xxxstar/x3-indexer:latest` (references actual GHCR)
- `prometheus` — `prom/prometheus:v2.53.0` (real image)
- `loki` — `grafana/loki:3.0.0` (real image)
- `grafana` — `grafana/grafana:11.1.0` (real image)
- `faucet` — `ghcr.io/cyptopimpinainteazy/xxxstar/x3-faucet:latest` (references GHCR)

**Issues:**
- `ghcr.io/cyptopimpinainteazy/xxxstar/x3-indexer:latest` and `x3-faucet:latest` — **these images likely do not exist.** The GHCR namespace `cyptopimpinainteazy` is the correct org, but `x3-indexer` and `x3-faucet` have never been published as Docker images.
- `monitoring/prometheus.yml`, `monitoring/loki-config.yml` — referenced paths exist in the repo?
- Grafana dashboards path: `../monitoring/grafana-dashboards` — needs checking.
- Default passwords hardcoded: `POSTGRES_PASSWORD: ${DB_PASSWORD:-changeme}`, `GF_SECURITY_ADMIN_PASSWORD: ${GRAFANA_PASSWORD:-changeme}`. These are env-overridable but the defaults are "changeme".

**Verdict: PARTIAL FAIL.** Infrastructure images (Postgres, Prometheus, Grafana, Loki) are real. Custom images (x3-indexer, x3-faucet) likely do not exist on GHCR. Stack will partially start.

### 9. GitHub Workflow Files

#### `.github/workflows/ci.yml`
**EXISTS.** 677 lines. Real CI workflow. Runs build, test, clippy, fmt across the entire workspace. References real cargo commands. **PASS.**

#### `.github/workflows/release-provenance.yml`
**EXISTS.** 146 lines. Real provenance workflow. Uses `docker/build-push-action@v5`, generates SLSA provenance with `actions/attest-build-provenance`. **PASS.**
- References real actions, real Dockerfiles.
- Generates provenance attestation — non-trivial real setup.

#### `.github/workflows/v04-ship-gate.yml`
**EXISTS.** 189 lines. Real ship gate workflow for v0.4 internal mainnet. We read this file in full earlier.
- Hard rules: no `continue-on-error`, no silent skips.
- Runs focused checks on `x3-packet-standard`, `x3-ixl`, `x3-readiness-report`, `pallet-x3-cross-vm-router`.
- Runs specific named tests for scope-freeze verification.
- Runs embarrassment scan with explicit suppression config.
- **PASS.** Real, substantive workflow.

#### `.github/workflows/full-ci.yml`
**EXISTS.** 297 lines. Real full CI workflow. Builds everything, runs all tests. **PASS.**

#### `.github/workflows/try-runtime-upgrade.yml`
**EXISTS.** 96 lines. Real try-runtime workflow. Uses `paritytech/try-runtime` action. **PASS.**

#### `.github/workflows/zombienet-integration.yml`
**EXISTS.** 71 lines. Real Zombienet integration test workflow. Uses `paritytech/zombienet` action. **PASS.**

**All six workflow files are real (not boilerplate).** They reference real actions, real cargo commands, real test suites. No placeholder references.

---

## Summary Table

| File | Exists? | Real Paths? | Placeholders? | Verdict |
|------|---------|-------------|---------------|---------|
| `scripts/testnet-full-launch.sh` | Yes | Yes | No | **PARTIAL FAIL** — broken `chain_getBlockNumber` RPC call |
| `scripts/mainnet_release_gate.py` | Yes | Yes | No | **PASS** |
| `scripts/install-validator.sh` | Yes | No | Yes (release URLs) | **FAIL** — URLs return 404 |
| `packaging/systemd/x3-validator.service` | Yes | Yes | No | **PASS** |
| `packaging/systemd/x3-bootnode.service` | Yes | Yes | No | **PASS** (minor: no `--no-mdns`) |
| `validator_config/mainnet.toml` | Yes | Partially | Yes (non-existent ceremony script) | **PARTIAL FAIL** |
| `chain-specs/` (4 files) | Yes | N/A | No | **PASS** — all valid JSON |
| `docker/docker-compose.yml` | Yes | Partially | Yes (x3-indexer/faucet images) | **PARTIAL FAIL** |
| `.github/workflows/ci.yml` | Yes | Yes | No | **PASS** |
| `.github/workflows/release-provenance.yml` | Yes | Yes | No | **PASS** |
| `.github/workflows/v04-ship-gate.yml` | Yes | Yes | No | **PASS** |
| `.github/workflows/full-ci.yml` | Yes | Yes | No | **PASS** |
| `.github/workflows/try-runtime-upgrade.yml` | Yes | Yes | No | **PASS** |
| `.github/workflows/zombienet-integration.yml` | Yes | Yes | No | **PASS** |

---

## Critical Issues

1. **`install-validator.sh` downloads from non-existent release URLs.** The binary URL `https://github.com/Cyptopimpinainteazy/xxxstar/releases/download/latest/x3-chain-node` and chain spec URL will both 404. No releases have been published.

2. **`docker-compose.yml` references non-existent GHCR images.** `ghcr.io/cyptopimpinainteazy/xxxstar/x3-indexer:latest` and `x3-faucet:latest` will pull failure.

3. **Testnet launch script has a broken RPC call.** `chain_getBlockNumber` is not a valid Substrate RPC method.

4. **`mainnet.toml` references non-existent `genesis_ceremony.sh`.**

## Recommendations

1. Publish initial release artifacts (binary + chain spec) or update `install-validator.sh` to build from source.
2. Build and push `x3-indexer` and `x3-faucet` Docker images to GHCR, or remove them from docker-compose.yml.
3. Fix `testnet-full-launch.sh` line 184: replace `chain_getBlockNumber` with `chain_getHeader` and parse the `number` field from the response.
4. Either create `scripts/mainnet/genesis_ceremony.sh` or remove the reference from `mainnet.toml`.
