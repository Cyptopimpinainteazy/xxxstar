#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# X3 external toolchain — command sheet
#
# Everything the audit tooling needs on this box, as copy-pasteable commands.
# Written 2026-10-01; companion to external-toolchain-install-evidence.md.
#
# Usage:
#   ./external-toolchain-commands.sh path       # print the PATH line for ~/.bashrc
#   ./external-toolchain-commands.sh verify     # check every tool resolves
#   ./external-toolchain-commands.sh gates      # dependency + policy gates
#   ./external-toolchain-commands.sh fuzz       # fuzz the X3 language target
#   ./external-toolchain-commands.sh evm        # Foundry build/test + Slither
#   ./external-toolchain-commands.sh proofs     # scanners on this repo
#   ./external-toolchain-commands.sh echidna    # invariant fuzz (workaround)
#   ./external-toolchain-commands.sh zombienet  # multi-validator (needs node)
#   ./external-toolchain-commands.sh local-ci   # the gate of record
#   ./external-toolchain-commands.sh all        # verify + gates + evm + proofs
#
# Nothing here deletes anything. `echidna` writes a config to /tmp.
# ─────────────────────────────────────────────────────────────────────────────
set -uo pipefail

X3_ROOT="${X3_ROOT:-$HOME/Desktop/xxxstar-master}"
VENV_BIN="$HOME/.venvs/x3-tools/bin"
export PATH="$VENV_BIN:$HOME/.local/bin:$HOME/.cargo/bin:$PATH"

c_ok()   { printf '\033[32m✓\033[0m %s\n' "$*"; }
c_bad()  { printf '\033[31m✗\033[0m %s\n' "$*"; }
c_info() { printf '\033[36m»\033[0m %s\n' "$*"; }

# ── 1. PATH ──────────────────────────────────────────────────────────────────
cmd_path() {
  cat <<'EOF'
Add this line to ~/.bashrc (venv FIRST so the working pytest/slither win):

export PATH="$HOME/.venvs/x3-tools/bin:$HOME/.local/bin:$HOME/.cargo/bin:$PATH"
EOF
}

# ── 2. verify ────────────────────────────────────────────────────────────────
cmd_verify() {
  local missing=0
  # NOTE: Miri has no standalone binary — it is `cargo +nightly miri`,
  # provided by the cargo-miri shim, so that is what gets checked here.
  for t in cargo-binstall cargo-audit cargo-deny cargo-fuzz cargo-mutants \
           cargo-nextest cargo-geiger cargo-llvm-cov cargo-miri kani slither semgrep \
           halmos pytest ansible solc-select uv forge cast anvil echidna medusa \
           k6 trivy gitleaks syft grype osv-scanner codeql toxiproxy-server \
           toxiproxy-cli prometheus alertmanager loki grafana zombienet subwasm \
           srtool solana solana-test-validator cargo-build-sbf docker; do
    if command -v "$t" >/dev/null 2>&1; then
      printf '  %-22s %s\n' "$t" "$(command -v "$t")"
    else
      c_bad "MISSING: $t"; missing=$((missing+1))
    fi
  done
  if [ "$missing" -eq 0 ]; then c_ok "all tools resolve"; else c_bad "$missing missing"; fi
}

# ── 3. dependency / policy gates ─────────────────────────────────────────────
cmd_gates() {
  cd "$X3_ROOT" || return 1
  # Fixed 2026-10-01: wasmtime RUSTSEC-2026-0316 was the sole vulnerability and
  # is resolved by the 36.0.16 bump. sized-chunks/im remain warnings only.
  c_info "cargo audit"
  cargo audit --file Cargo.lock || true
  c_info "cargo deny advisories"
  cargo deny check advisories || true
  c_info "cargo nextest"
  cargo nextest run -p x3-order-window || true
}

# ── 4. fuzzing ───────────────────────────────────────────────────────────────
cmd_fuzz() {
  # musl cannot link ASAN on this toolchain — the gnu target is required.
  cd "$X3_ROOT/crates/x3-integration/fuzz" || return 1
  c_info "fuzz x3bc_engines for 60s (gnu target, nightly)"
  cargo +nightly fuzz run --target x86_64-unknown-linux-gnu x3bc_engines \
    -- -max_total_time=60 -rss_limit_mb=4096 -print_final_stats=1
}

# ── 5. EVM / Foundry / Slither ───────────────────────────────────────────────
cmd_evm() {
  cd "$X3_ROOT/X3-contracts/evm" || return 1
  if [ ! -d lib/forge-std ] || [ ! -d lib/openzeppelin-contracts ]; then
    c_info "installing EVM deps at the revisions in foundry.lock"
    [ -d lib/forge-std ] || {
      git clone https://github.com/foundry-rs/forge-std lib/forge-std
      git -C lib/forge-std checkout 620536fa5277db4e3fd46772d5cbc1ea0696fb43
    }
    [ -d lib/openzeppelin-contracts ] || {
      git clone https://github.com/OpenZeppelin/openzeppelin-contracts lib/openzeppelin-contracts
      git -C lib/openzeppelin-contracts checkout dc44c9f1a4c3b10af99492eed84f83ed244203f6
    }
  fi
  c_info "forge build + test"
  forge build
  forge test --no-match-path 'test/parity/*'
  c_info "slither"
  slither . --config-file slither.config.json
}

# ── 6. repo-wide scanners ────────────────────────────────────────────────────
cmd_proofs() {
  cd "$X3_ROOT" || return 1
  c_info "gitleaks"
  gitleaks dir . --no-banner --redact || true
  c_info "semgrep (X3 rules)"
  semgrep --config tests/security/semgrep/x3-security-rules.yml --quiet . || true
  c_info "osv-scanner"
  osv-scanner scan source --recursive --no-ignore . || true
  c_info "trivy fs (HIGH/CRITICAL)"
  trivy fs --scanners vuln,secret --severity HIGH,CRITICAL --no-progress . || true
}

# ── 7. Echidna ───────────────────────────────────────────────────────────────
cmd_echidna() {
  cd "$X3_ROOT" || return 1
  # The committed tests_core/security/echidna.config.yaml is malformed:
  # balanceAddr/balanceContract are quoted strings and Echidna rejects them.
  local cfg=/tmp/echidna-fixed.yml
  cat > "$cfg" <<'YML'
testMode: property
testLimit: 5000
seqLen: 20
shrinkLimit: 100
balanceAddr: 1000000000000000000000
balanceContract: 1000000000000000000000
workers: 2
YML
  solc-select use 0.8.24
  c_info "echidna — compiles + analyzes; the fixture itself currently reverts on deploy"
  echidna tests_core/security/contracts/InvariantProperties.sol \
    --contract InvariantProperties --config "$cfg"
}

# ── 8. Zombienet ─────────────────────────────────────────────────────────────
cmd_zombienet() {
  cd "$X3_ROOT" || return 1
  c_info "building the node binary Zombienet will spawn"
  cargo build --release -p x3-chain-node || return 1
  c_info "running the finality smoke spec"
  zombienet test tools/test-tool-stack/zombienet/x3-finality-smoke.zndsl
}

# ── 9. the gate of record ────────────────────────────────────────────────────
cmd_local_ci() {
  cd "$X3_ROOT" || return 1
  bash scripts/local-ci.sh "$@"
}

case "${1:-help}" in
  path)      cmd_path ;;
  verify)    cmd_verify ;;
  gates)     cmd_gates ;;
  fuzz)      cmd_fuzz ;;
  evm)       cmd_evm ;;
  proofs)    cmd_proofs ;;
  echidna)   cmd_echidna ;;
  zombienet) cmd_zombienet ;;
  local-ci)  shift; cmd_local_ci "$@" ;;
  all)       cmd_verify; cmd_gates; cmd_evm; cmd_proofs ;;
  *)
    sed -n '2,24p' "$0"
    ;;
esac
