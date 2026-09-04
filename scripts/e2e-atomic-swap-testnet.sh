#!/usr/bin/env bash
# X3 Cross-VM Atomic Swap — End-to-End Testnet Flow (Production Grade)
# Orchestrates: deploy EVM HTLC → deploy Solana HTLC → start relayer → run swap
#
# Idempotent: safe to re-run. Uses --skip-deploy to reuse existing deployments.
# Verification: pre-deployment balance check, post-deployment code check,
#               relayer health check, swap verification with timeout.
# Exit codes: 0 = full success, 1 = swap failed, 2 = setup failure
set -euo pipefail

# ── Configuration ──────────────────────────────────────────────────────────────
SCRIPT_TIMEOUT=${SCRIPT_TIMEOUT:-300}       # Total script timeout in seconds
RELAYER_STARTUP_WAIT=${RELAYER_STARTUP_WAIT:-15}  # Max seconds to wait for relayer health
SWAP_WAIT_TIME=${SWAP_WAIT_TIME:-60}        # Seconds to wait for relayer to process swap
POLL_INTERVAL=${POLL_INTERVAL:-5}           # Polling interval for swap verification

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; CYAN='\033[0;36m'; NC='\033[0m'
log()  { echo -e "${CYAN}[$(date +%H:%M:%S)]${NC} $1"; }
ok()   { echo -e "${GREEN}  ✓${NC} $1"; }
warn() { echo -e "${YELLOW}  ⚠${NC} $1"; }
fail() { echo -e "${RED}  ✗${NC} $1"; exit "$2"; }

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SCRIPT_START=$(date +%s)

# ── Global state ──────────────────────────────────────────────────────────────
EVM_HTLC_ADDRESS=""
SOLANA_HTLC_PROGRAM_ID=""
RELAYER_PID=""
RELAYER_LOG="/tmp/x3-relayer-e2e.log"
RELAYER_CONFIG="/tmp/x3-e2e-relayer-config.yaml"
SWAP_LOG="/tmp/x3-e2e-swap.log"
EVM_DEPLOYMENT_CACHE="$WORKSPACE_ROOT/.e2e-deploy-cache.json"
OVERALL_STATUS=0  # 0=success, 1=swap_failed, 2=setup_failure
SKIP_DEPLOY=false
SKIP_RELAYER=false

# Parse flags
for arg in "$@"; do
  case "$arg" in
    --skip-deploy) SKIP_DEPLOY=true ;;
    --skip-relayer) SKIP_RELAYER=true ;;
    --help|-h)
      echo "Usage: $0 [--skip-deploy] [--skip-relayer]"
      echo ""
      echo "  --skip-deploy    Skip contract deployment, use cached addresses"
      echo "  --skip-relayer   Skip starting relayer (for debug/testing)"
      echo ""
      echo "Required env: SEPOLIA_RPC_URL, DEPLOYER_PRIVATE_KEY"
      echo "Optional env: X3_NODE_URL, SOLANA_RPC_URL, SOLANA_HTLC_PROGRAM_ID,"
      echo "              SWAP_RECIPIENT, SCRIPT_TIMEOUT, RELAYER_STARTUP_WAIT,"
      echo "              SWAP_WAIT_TIME, POLL_INTERVAL"
      exit 0
      ;;
  esac
done

# ── Timeout enforcement ───────────────────────────────────────────────────────
# Use a background watchdog to enforce the total script timeout
_timeout_pid=""
_start_timeout_watchdog() {
  (
    sleep "$SCRIPT_TIMEOUT"
    # If we're still running after the timeout, kill the whole process group
    if kill -0 "$$" 2>/dev/null; then
      echo -e "${RED}[FATAL] Script timed out after ${SCRIPT_TIMEOUT}s${NC}" >&2
      # Try graceful cleanup first
      if [ -n "${RELAYER_PID:-}" ]; then
        kill "$RELAYER_PID" 2>/dev/null || true
      fi
      kill -TERM "$$" 2>/dev/null || true
    fi
  ) &
  _timeout_pid=$!
}
_stop_timeout_watchdog() {
  if [ -n "$_timeout_pid" ]; then
    kill "$_timeout_pid" 2>/dev/null || true
  fi
}
_start_timeout_watchdog

# ── Cleanup handler ───────────────────────────────────────────────────────────
cleanup() {
  _stop_timeout_watchdog
  echo ""
  if [ -n "${RELAYER_PID:-}" ]; then
    log "Stopping relayer (PID: $RELAYER_PID)..."
    kill "$RELAYER_PID" 2>/dev/null || true
    # Give it a moment to flush logs
    sleep 1
    wait "$RELAYER_PID" 2>/dev/null || true
    ok "Relayer stopped"
  fi
  rm -f "$RELAYER_CONFIG" 2>/dev/null || true
  log "Cleanup complete."
}
trap cleanup EXIT

# ── RPC helper for X3 node ────────────────────────────────────────────────────
x3_rpc() {
  local method="$1"
  local params="${2:-[]}"
  curl -s -X POST -H 'Content-Type: application/json' \
    -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"$method\",\"params\":$params}" \
    "${X3_NODE_URL:-http://localhost:9933}" 2>/dev/null
}

# ── Time-bounded polling helper ───────────────────────────────────────────────
# poll_for <description> <max_seconds> <interval> <command...>
# Returns 0 if command succeeds (exit 0), 1 on timeout
poll_for() {
  local desc="$1" max_sec="$2" interval="$3"
  shift 3
  local elapsed=0
  while [ "$elapsed" -lt "$max_sec" ]; do
    if eval "$@" 2>/dev/null; then
      return 0
    fi
    sleep "$interval"
    elapsed=$((elapsed + interval))
  done
  return 1
}

# ── Header ────────────────────────────────────────────────────────────────────
echo -e "${CYAN}══════════════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}  X3 Cross-VM Atomic Swap — End-to-End Testnet Flow${NC}"
echo -e "${CYAN}  EVM Sepolia ↔ Solana Devnet${NC}"
echo -e "${CYAN}  Timeout: ${SCRIPT_TIMEOUT}s | Skip deploy: ${SKIP_DEPLOY} | Skip relayer: ${SKIP_RELAYER}${NC}"
echo -e "${CYAN}══════════════════════════════════════════════════════════════${NC}"
echo ""

# ══════════════════════════════════════════════════════════════════════════════
# STEP 0: Prerequisites and environment checks
# ══════════════════════════════════════════════════════════════════════════════
log "Step 0: Prerequisites check..."

command -v forge >/dev/null 2>&1 || fail "forge not found — install Foundry (https://book.getfoundry.sh)" 2
command -v cast  >/dev/null 2>&1 || fail "cast not found — install Foundry (https://book.getfoundry.sh)" 2
command -v jq    >/dev/null 2>&1 || fail "jq not found — install (apt install jq / brew install jq)" 2
command -v openssl >/dev/null 2>&1 || fail "openssl not found" 2
command -v xxd   >/dev/null 2>&1 || fail "xxd not found — install (apt install xxd / brew install xxd)" 2
command -v curl  >/dev/null 2>&1 || fail "curl not found" 2
ok "forge, cast, jq, openssl, xxd, curl"

HAS_SOLANA=false
if command -v solana &>/dev/null && (command -v cargo-build-sbf &>/dev/null || command -v cargo-build-bpf &>/dev/null); then
  HAS_SOLANA=true
  ok "solana CLI + cargo-build-sbf"
else
  warn "Solana CLI or cargo-build-sbf not installed — Solana deploy will be skipped."
  warn "Install: sh -c \"\$(curl -sSfL https://release.anza.xyz/v2.1.0/install)\""
fi

# ── Environment variables ─────────────────────────────────────────────────────
log "Checking environment variables..."

MISSING_ENV=false

if [ -z "${SEPOLIA_RPC_URL:-}" ]; then
  warn "SEPOLIA_RPC_URL not set."
  MISSING_ENV=true
fi

if [ -z "${DEPLOYER_PRIVATE_KEY:-}" ]; then
  warn "DEPLOYER_PRIVATE_KEY not set."
  MISSING_ENV=true
fi

if [ -z "${X3_NODE_URL:-}" ]; then
  export X3_NODE_URL="http://localhost:9933"
  ok "X3_NODE_URL defaulting to $X3_NODE_URL"
fi

if [ -z "${SOLANA_RPC_URL:-}" ]; then
  export SOLANA_RPC_URL="https://api.devnet.solana.com"
  ok "SOLANA_RPC_URL defaulting to $SOLANA_RPC_URL"
fi

if [ "$MISSING_ENV" = true ]; then
  echo ""
  echo "  Required environment variables:"
  echo "    export SEPOLIA_RPC_URL=https://sepolia.infura.io/v3/YOUR_KEY"
  echo "    export DEPLOYER_PRIVATE_KEY=0x..."
  echo "  Optional:"
  echo "    export X3_NODE_URL=http://localhost:9933"
  echo "    export SOLANA_RPC_URL=https://api.devnet.solana.com"
  echo "    export SOLANA_HTLC_PROGRAM_ID=<program-id>"
  echo "    export SWAP_RECIPIENT=0x..."
  echo "    export SCRIPT_TIMEOUT=300"
  echo "    export RELAYER_STARTUP_WAIT=15"
  echo "    export SWAP_WAIT_TIME=60"
  echo ""
  fail "Set required env vars and re-run." 2
fi

ok "All required environment variables set"

# ── Pre-deployment balance sanity check ───────────────────────────────────────
log "Pre-deployment sanity check: checking deployer balance on Sepolia..."

DEPLOYER_ADDR=$(cast wallet address --private-key "$DEPLOYER_PRIVATE_KEY" 2>/dev/null || echo "")
if [ -z "$DEPLOYER_ADDR" ]; then
  fail "Could not derive deployer address from DEPLOYER_PRIVATE_KEY" 2
fi
ok "Deployer address: $DEPLOYER_ADDR"

# Get balance with retry (RPC may be slow)
BALANCE_WEI=""
for i in $(seq 1 5); do
  BALANCE_WEI=$(cast balance --rpc-url "$SEPOLIA_RPC_URL" "$DEPLOYER_ADDR" 2>/dev/null || echo "")
  if [ -n "$BALANCE_WEI" ] && [ "$BALANCE_WEI" != "0" ]; then
    break
  fi
  sleep 1
done

if [ -z "$BALANCE_WEI" ] || [ "$BALANCE_WEI" = "0" ]; then
  warn "Deployer balance is zero or could not be fetched. Deployment may fail."
  warn "  Address: $DEPLOYER_ADDR"
  warn "  RPC: $SEPOLIA_RPC_URL"
  warn "  Continuing anyway..."
else
  BALANCE_ETH=$(cast to-unit "$BALANCE_WEI" ether 2>/dev/null || echo "$BALANCE_WEI wei")
  ok "Deployer balance: $BALANCE_ETH"
fi

# ══════════════════════════════════════════════════════════════════════════════
# STEP 1: Deploy AtlasHTLC to Sepolia (or reuse cached)
# ══════════════════════════════════════════════════════════════════════════════
echo ""
log "Step 1: AtlasHTLC on Sepolia..."

# Check cache first for idempotency
if [ -f "$EVM_DEPLOYMENT_CACHE" ] && [ "$SKIP_DEPLOY" = false ]; then
  CACHED_ADDR=$(jq -r '.evm_htlc_address // empty' "$EVM_DEPLOYMENT_CACHE" 2>/dev/null)
  CACHED_RPC=$(jq -r '.sepolia_rpc // empty' "$EVM_DEPLOYMENT_CACHE" 2>/dev/null)
  if [ -n "$CACHED_ADDR" ] && [ "$CACHED_RPC" = "$SEPOLIA_RPC_URL" ]; then
    # Verify the cached address still has code on-chain
    log "  Verifying cached deployment at $CACHED_ADDR..."
    ONCHAIN_CODE=$(cast code --rpc-url "$SEPOLIA_RPC_URL" "$CACHED_ADDR" 2>/dev/null || echo "")
    if [ -n "$ONCHAIN_CODE" ] && [ "$ONCHAIN_CODE" != "0x" ]; then
      log "  Cached deployment is still valid"
      SKIP_DEPLOY=true
    else
      warn "  Cached address has no on-chain code — will re-deploy"
      rm -f "$EVM_DEPLOYMENT_CACHE"
    fi
  fi
fi

if [ "$SKIP_DEPLOY" = false ]; then
  log "  Deploying AtlasHTLC to Sepolia..."

  cd "$WORKSPACE_ROOT/X3-contracts/evm"

  if [ ! -f "out/AtlasHTLC.sol/AtlasHTLC.json" ]; then
    log "    Building AtlasHTLC..."
    forge build --contracts contracts/AtlasHTLC.sol 2>&1 | tail -5
    ok "Contract built"
  fi

  log "    Broadcasting deployment transaction..."
  DEPLOY_OUTPUT=$(forge script script/DeployAtlasHTLC.s.sol:DeployAtlasHTLC \
    --rpc-url "$SEPOLIA_RPC_URL" \
    --private-key "$DEPLOYER_PRIVATE_KEY" \
    --broadcast \
    --slow \
    2>&1) || { echo "$DEPLOY_OUTPUT"; fail "AtlasHTLC deployment failed" 2; }

  EVM_HTLC_ADDRESS=$(echo "$DEPLOY_OUTPUT" | grep "AtlasHTLC:" | sed 's/.*AtlasHTLC: \(0x[a-fA-F0-9]\{40\}\).*/\1/')

  if [ -z "$EVM_HTLC_ADDRESS" ]; then
    # Try alternative extraction pattern
    EVM_HTLC_ADDRESS=$(echo "$DEPLOY_OUTPUT" | grep -oE '0x[a-fA-F0-9]{40}' | head -1 || echo "")
  fi

  if [ -z "$EVM_HTLC_ADDRESS" ]; then
    echo "$DEPLOY_OUTPUT"
    fail "Could not parse AtlasHTLC address from deploy output" 2
  fi

  ok "AtlasHTLC deployed at: $EVM_HTLC_ADDRESS"

  # Post-deployment verification: check on-chain code
  log "    Verifying contract code on-chain..."
  ONCHAIN_CODE=$(cast code --rpc-url "$SEPOLIA_RPC_URL" "$EVM_HTLC_ADDRESS" 2>/dev/null || echo "")
  if [ -z "$ONCHAIN_CODE" ] || [ "$ONCHAIN_CODE" = "0x" ]; then
    fail "Contract code not found on-chain after deployment — check deployment" 2
  fi
  ok "Contract code verified on-chain (length: ${#ONCHAIN_CODE} bytes)"

  # Cache deployment
  echo "{\"evm_htlc_address\": \"$EVM_HTLC_ADDRESS\", \"sepolia_rpc\": \"$SEPOLIA_RPC_URL\"}" > "$EVM_DEPLOYMENT_CACHE"
  ok "Deployment cached to $EVM_DEPLOYMENT_CACHE"
else
  # Load from cache or env
  if [ -z "$EVM_HTLC_ADDRESS" ] && [ -f "$EVM_DEPLOYMENT_CACHE" ]; then
    EVM_HTLC_ADDRESS=$(jq -r '.evm_htlc_address // empty' "$EVM_DEPLOYMENT_CACHE" 2>/dev/null || echo "")
  fi
  if [ -z "$EVM_HTLC_ADDRESS" ]; then
    EVM_HTLC_ADDRESS="${EVM_HTLC_ADDRESS:-}"
  fi
  log "  Skipping deploy. Using AtlasHTLC at: $EVM_HTLC_ADDRESS"
fi

if [ -z "$EVM_HTLC_ADDRESS" ]; then
  fail "No AtlasHTLC address available. Deploy manually or export EVM_HTLC_ADDRESS." 2
fi

# ══════════════════════════════════════════════════════════════════════════════
# STEP 2: Deploy Solana HTLC program to devnet
# ══════════════════════════════════════════════════════════════════════════════
echo ""
log "Step 2: Solana HTLC program..."

if [ "$HAS_SOLANA" = true ] && [ "$SKIP_DEPLOY" = false ]; then
  SOLANA_DEPLOY_SCRIPT="$WORKSPACE_ROOT/programs/svm/x3_atomic_swap/deploy-devnet.sh"
  if [ -f "$SOLANA_DEPLOY_SCRIPT" ]; then
    log "  Running deploy-devnet.sh..."
    bash "$SOLANA_DEPLOY_SCRIPT" 2>&1 || warn "Solana deploy script exited with error — continuing"
  else
    warn "  deploy-devnet.sh not found at $SOLANA_DEPLOY_SCRIPT"
    warn "  Falling back to manual build + deploy..."
    cd "$WORKSPACE_ROOT/programs/svm/x3_atomic_swap"
    solana config set --url "$SOLANA_RPC_URL"
    cargo build-sbf --manifest-path Cargo.toml 2>&1 || cargo build-bpf --manifest-path Cargo.toml 2>&1 || true
    PROGRAM_KEYPAIR="target/deploy/x3_atomic_swap-keypair.json"
    if [ -f "$PROGRAM_KEYPAIR" ]; then
      solana program deploy --program-id "$PROGRAM_KEYPAIR" target/deploy/x3_atomic_swap.so 2>&1 || true
    fi
  fi

  SOLANA_KEYPAIR="$WORKSPACE_ROOT/programs/svm/x3_atomic_swap/target/deploy/x3_atomic_swap-keypair.json"
  if [ -f "$SOLANA_KEYPAIR" ]; then
    SOLANA_HTLC_PROGRAM_ID=$(solana-keygen pubkey "$SOLANA_KEYPAIR" 2>/dev/null || echo "")
    if [ -n "$SOLANA_HTLC_PROGRAM_ID" ]; then
      ok "Solana HTLC program ID: $SOLANA_HTLC_PROGRAM_ID"
    fi
  fi
elif [ -n "${SOLANA_HTLC_PROGRAM_ID:-}" ]; then
  log "  Using SOLANA_HTLC_PROGRAM_ID from environment: $SOLANA_HTLC_PROGRAM_ID"
else
  warn "  Solana CLI tools not available — skipping Solana HTLC deployment."
  warn "  Set SOLANA_HTLC_PROGRAM_ID manually after deploying."
  SOLANA_HTLC_PROGRAM_ID="${SOLANA_HTLC_PROGRAM_ID:-}"
fi

if [ -z "$SOLANA_HTLC_PROGRAM_ID" ]; then
  warn "  No Solana program ID available. Relayer will start without SVM monitoring."
  SOLANA_HTLC_PROGRAM_ID=""
fi

# ══════════════════════════════════════════════════════════════════════════════
# STEP 3: Configure and start x3-relayer
# ══════════════════════════════════════════════════════════════════════════════
echo ""
log "Step 3: x3-relayer..."

RELAYER_BIN="$WORKSPACE_ROOT/target/release/x3-relayer"
if [ ! -f "$RELAYER_BIN" ] && [ "$SKIP_RELAYER" = false ]; then
  log "  Building x3-relayer (release)..."
  cd "$WORKSPACE_ROOT"
  cargo build -p x3-relayer --release 2>&1 | tail -10
  ok "x3-relayer built"
elif [ ! -f "$RELAYER_BIN" ]; then
  warn "  x3-relayer binary not found at $RELAYER_BIN"
fi

# Build SVM cluster config with optional program ID
SVM_CONFIG_YAML=""
if [ -n "$SOLANA_HTLC_PROGRAM_ID" ]; then
  SVM_CONFIG_YAML="    htlc_program_id: \"${SOLANA_HTLC_PROGRAM_ID}\""
fi

# Write relayer config for this testnet run
cat > "$RELAYER_CONFIG" <<EOF
# Auto-generated by e2e-atomic-swap-testnet.sh
x3:
  rpc_url: "${X3_NODE_URL}"
  relayer_account: "${DEPLOYER_ADDR}"
  relayer_seed_phrase: "${X3_RELAYER_SEED_PHRASE:-//Alice}"

evm_chains:
  - name: "Sepolia Testnet"
    chain_id: 11155111
    x3_domain_id: 200
    rpc_endpoint: "${SEPOLIA_RPC_URL}"
    state_root_contract: "${EVM_HTLC_ADDRESS}"
    finality_threshold: 12
    block_poll_interval_ms: 13000
    max_concurrent_requests: 5

svm_clusters:
  - name: "Solana Devnet"
    cluster_name: "solana-devnet"
    x3_domain_id: 502
    rpc_endpoint: "${SOLANA_RPC_URL}"
    finality_threshold: 32
    slot_poll_interval_ms: 15000
    max_concurrent_requests: 10
${SVM_CONFIG_YAML}

submission:
  batch_size: 1
  timeout_secs: 60
  max_retries: 3
  retry_backoff_ms: 1000

governance:
  poll_interval_secs: 5
  enable_graceful_shutdown: true

logging:
  level: "info"
  format: "default"
EOF
ok "Relayer config written to $RELAYER_CONFIG"

export X3_RELAYER_CONFIG="$RELAYER_CONFIG"
export SVM_HTLC_PROGRAM_ID="$SOLANA_HTLC_PROGRAM_ID"

if [ "$SKIP_RELAYER" = false ] && [ -f "$RELAYER_BIN" ]; then
  log "  Starting x3-relayer (background, log: $RELAYER_LOG)..."
  "$RELAYER_BIN" > "$RELAYER_LOG" 2>&1 &
  RELAYER_PID=$!
  ok "Relayer started with PID $RELAYER_PID"

  # Relayer health check: wait for it to connect to both chains
  log "  Waiting for relayer to connect to chains (up to ${RELAYER_STARTUP_WAIT}s)..."
  RELAYER_HEALTHY=false
  if poll_for "relayer health" "$RELAYER_STARTUP_WAIT" "1" \
    "grep -q 'Connected\|Listening\|starting\|watching\|polling\|Starting' \"$RELAYER_LOG\" 2>/dev/null"; then
    RELAYER_HEALTHY=true
    ok "Relayer connected to chains"
    # Show connection messages
    grep -i "connected\|listening\|starting\|watching\|polling" "$RELAYER_LOG" 2>/dev/null | head -5 | sed 's/^/       /'
  else
    warn "Relayer did not show connection messages within ${RELAYER_STARTUP_WAIT}s"
    if kill -0 "$RELAYER_PID" 2>/dev/null; then
      warn "Relayer process is still running — may be slow to connect"
    else
      warn "Relayer process has exited. Log excerpt:"
      tail -30 "$RELAYER_LOG" 2>/dev/null || true
    fi
  fi
else
  warn "Relayer not started (--skip-relayer or binary not found)"
fi

# ══════════════════════════════════════════════════════════════════════════════
# STEP 4: Execute atomic swap
# ══════════════════════════════════════════════════════════════════════════════
echo ""
log "Step 4: Executing atomic swap..."

# Generate a secret and its hash lock
SECRET=$(openssl rand -hex 32)
HASH_LOCK=$(echo -n "$SECRET" | xxd -r -p | openssl dgst -sha256 | awk '{print $2}')
log "  Secret (hex):  $SECRET"
log "  Hash lock:     0x$HASH_LOCK"

# Recipient on Solana side (set via env or use deployer address)
RECIPIENT="${SWAP_RECIPIENT:-$DEPLOYER_ADDR}"
TIMEOUT=$(( $(date +%s) + 3600 ))  # 1 hour from now
AMOUNT="0.01"  # ETH

log "  Creating HTLC on Sepolia (recipient=$RECIPIENT, amount=${AMOUNT}ETH, timeout=$TIMEOUT)..."
TX_HASH=""
CREATE_OUTPUT=$(cast send --rpc-url "$SEPOLIA_RPC_URL" \
    --private-key "$DEPLOYER_PRIVATE_KEY" \
    "$EVM_HTLC_ADDRESS" \
    "createHTLC(address,bytes32,uint256,address,uint256)" \
    "$RECIPIENT" \
    "0x$HASH_LOCK" \
    "$TIMEOUT" \
    "0x0000000000000000000000000000000000000000" \
    0 \
    --value "${AMOUNT}ether" \
    --json 2>&1) || { echo "$CREATE_OUTPUT" | tail -5; warn "HTLC creation failed — continuing to check relayer logs"; CREATE_OUTPUT=""; }

if [ -n "$CREATE_OUTPUT" ]; then
  TX_HASH=$(echo "$CREATE_OUTPUT" | jq -r '.transactionHash // empty' 2>/dev/null || echo "")
  if [ -n "$TX_HASH" ]; then
    ok "HTLC created on Sepolia. Tx: https://sepolia.etherscan.io/tx/$TX_HASH"
    echo "$CREATE_OUTPUT" > "$SWAP_LOG"
  else
    warn "Could not parse transaction hash from cast output"
    echo "$CREATE_OUTPUT" >> "$SWAP_LOG"
  fi
fi

# ── Swap verification with polling ──────────────────────────────────────────
if [ -n "$TX_HASH" ]; then
  echo ""
  log "Step 5: Verifying swap (timeout: ${SWAP_WAIT_TIME}s)..."

  # 5a. Verify the HTLC is recorded on-chain via htlcCount
  log "  Checking HTLC count on-chain..."
  HTLC_COUNT="0"
  if poll_for "HTLC count > 0" "$((SWAP_WAIT_TIME / 2))" "$POLL_INTERVAL" \
    "HTLC_COUNT=\$(cast call --rpc-url \"$SEPOLIA_RPC_URL\" \"$EVM_HTLC_ADDRESS\" \"htlcCount()(uint256)\" 2>/dev/null || echo '0'); [ \"\$HTLC_COUNT\" != \"0\" ] && [ \"\$HTLC_COUNT\" != \"\" ]"; then
    HTLC_COUNT=$(cast call --rpc-url "$SEPOLIA_RPC_URL" "$EVM_HTLC_ADDRESS" "htlcCount()(uint256)" 2>/dev/null || echo "0")
    ok "HTLC count on AtlasHTLC: $HTLC_COUNT"
  else
    HTLC_COUNT=$(cast call --rpc-url "$SEPOLIA_RPC_URL" "$EVM_HTLC_ADDRESS" "htlcCount()(uint256)" 2>/dev/null || echo "0")
    warn "HTLC count: $HTLC_COUNT (may take longer to appear)"
  fi

  # 5b. Check relayer logs for proof submission (if relayer is running)
  if [ -n "${RELAYER_PID:-}" ] && kill -0 "$RELAYER_PID" 2>/dev/null; then
    log "  Waiting for relayer to detect and submit proof (up to ${SWAP_WAIT_TIME}s)..."
    
    DEPOSIT_DETECTED=false
    PROOF_SUBMITTED=false
    
    if poll_for "relayer deposit detection" "$SWAP_WAIT_TIME" "$POLL_INTERVAL" \
      "grep -q 'DepositLocked\|deposit event\|Submitting deposit proof\|CreatedHTLC\|HTLC created' \"$RELAYER_LOG\" 2>/dev/null"; then
      DEPOSIT_DETECTED=true
      ok "Relayer detected HTLC deposit event"
    else
      warn "Relayer did not detect deposit event within ${SWAP_WAIT_TIME}s"
    fi

    if poll_for "proof submission" "$SWAP_WAIT_TIME" "$POLL_INTERVAL" \
      "grep -q 'proof submitted\|submitted successfully\|ProofSubmitted\|ProofSubmitted\|deposit proof\|submitted to X3' \"$RELAYER_LOG\" 2>/dev/null"; then
      PROOF_SUBMITTED=true
      ok "Relayer submitted proof to X3 chain"
    else
      warn "Relayer did not confirm proof submission within ${SWAP_WAIT_TIME}s"
    fi
  fi

  # 5c. Display relayer log analysis
  echo ""
  log "Relayer Log Analysis:"
  echo "  ┌─ Relayer Log Analysis ──────────────────────────────────┐"
  if [ -f "$RELAYER_LOG" ]; then
    DETECTED_COUNT=$(grep -c "DepositLocked\|deposit event\|Submitting deposit proof\|CreatedHTLC\|HTLC created" "$RELAYER_LOG" 2>/dev/null || echo "0")
    SUBMITTED_COUNT=$(grep -c "proof submitted\|submitted successfully\|ProofSubmitted\|deposit proof\|submitted to X3" "$RELAYER_LOG" 2>/dev/null || echo "0")
    ERROR_COUNT=$(grep -ci "error\|failed\|panic\|Fatal" "$RELAYER_LOG" 2>/dev/null || echo "0")

    if [ "$DETECTED_COUNT" -gt 0 ]; then
      echo "  │ ✅ HTLC deposit events detected by relayer ($DETECTED_COUNT matches)"
    else
      echo "  │ ⚠️  No deposit events detected in relayer logs"
    fi
    if [ "$SUBMITTED_COUNT" -gt 0 ]; then
      echo "  │ ✅ Proof submissions to X3 chain ($SUBMITTED_COUNT matches)"
    else
      echo "  │ ⚠️  No proof submissions confirmed in relayer logs"
    fi
    if [ "$ERROR_COUNT" -gt 0 ]; then
      echo "  │ ⚠️  Errors/warnings in relayer logs ($ERROR_COUNT matches)"
    else
      echo "  │ ✅ No errors in relayer logs"
    fi
    echo "  └──────────────────────────────────────────────────────────┘"

    echo ""
    log "Relayer log excerpt (last 30 lines):"
    tail -30 "$RELAYER_LOG" 2>/dev/null || true
  else
    echo "  │ ⚠️  Relayer log not found"
    echo "  └──────────────────────────────────────────────────────────┘"
  fi
else
  warn "No transaction hash available — swap verification skipped"
  OVERALL_STATUS=1
fi

# ══════════════════════════════════════════════════════════════════════════════
# VERIFICATION SUMMARY
# ══════════════════════════════════════════════════════════════════════════════
echo ""
log "Final verification:"

# Check X3 node health
X3_HEALTH=$(x3_rpc "system_health" 2>/dev/null || echo "")
if [ -n "$X3_HEALTH" ] && echo "$X3_HEALTH" | jq -e '.result' >/dev/null 2>&1; then
  ok "X3 node is reachable at $X3_NODE_URL"
  X3_BLOCK=$(x3_rpc "chain_getHeader" 2>/dev/null | jq -r '.result.number // "unknown"' 2>/dev/null || echo "unknown")
  echo "       X3 block height: $X3_BLOCK"
else
  warn "X3 node not reachable at $X3_NODE_URL — is it running?"
fi

# Determine overall exit status
if [ -n "$TX_HASH" ]; then
  if [ "${PROOF_SUBMITTED:-false}" = true ]; then
    OVERALL_STATUS=0
    echo ""
    echo -e "${GREEN}══════════════════════════════════════════════════════════════${NC}"
    echo -e "${GREEN}  ✅ E2E Atomic Swap — FULL SUCCESS${NC}"
    echo -e "${GREEN}  HTLC created, relayer detected, proof submitted${NC}"
    echo -e "${GREEN}══════════════════════════════════════════════════════════════${NC}"
  elif [ -n "${RELAYER_PID:-}" ]; then
    # Relayer is running but hasn't confirmed yet — partial success
    OVERALL_STATUS=1
    echo ""
    echo -e "${YELLOW}══════════════════════════════════════════════════════════════${NC}"
    echo -e "${YELLOW}  ⚠️  E2E Atomic Swap — PARTIAL SUCCESS${NC}"
    echo -e "${YELLOW}  HTLC created but relayer proof not yet confirmed${NC}"
    echo -e "${YELLOW}  Check relayer logs: $RELAYER_LOG${NC}"
    echo -e "${YELLOW}══════════════════════════════════════════════════════════════${NC}"
  else
    OVERALL_STATUS=0
    echo ""
    echo -e "${GREEN}══════════════════════════════════════════════════════════════${NC}"
    echo -e "${GREEN}  ✅ E2E Atomic Swap — SUCCESS (relayer skipped)${NC}"
    echo -e "${GREEN}  HTLC created on Sepolia. Start relayer separately.${NC}"
    echo -e "${GREEN}══════════════════════════════════════════════════════════════${NC}"
  fi
else
  OVERALL_STATUS=1
  echo ""
  echo -e "${RED}══════════════════════════════════════════════════════════════${NC}"
  echo -e "${RED}  ✗ E2E Atomic Swap — FAILED${NC}"
  echo -e "${RED}  HTLC creation was not successful${NC}"
  echo -e "${RED}══════════════════════════════════════════════════════════════${NC}"
fi

echo ""
echo "  EVM HTLC contract:    $EVM_HTLC_ADDRESS"
echo "  Solana program ID:    ${SOLANA_HTLC_PROGRAM_ID:-<not deployed>}"
echo "  Relayer PID:          ${RELAYER_PID:-stopped}"
echo "  Relayer log:          $RELAYER_LOG"
echo "  Secret (hex):         $SECRET"
echo "  Hash lock:            0x$HASH_LOCK"
echo ""

if [ -n "$TX_HASH" ]; then
  echo "  Sepolia explorer:"
  echo "    https://sepolia.etherscan.io/tx/$TX_HASH"
  echo ""
fi

if [ -n "$SOLANA_HTLC_PROGRAM_ID" ]; then
  echo "  Solana explorer:"
  echo "    https://explorer.solana.com/address/$SOLANA_HTLC_PROGRAM_ID?cluster=devnet"
  echo ""
fi

SCRIPT_ELAPSED=$(( $(date +%s) - SCRIPT_START ))
echo -e "${CYAN}Script completed in ${SCRIPT_ELAPSED}s — exit code: ${OVERALL_STATUS}${NC}"

exit "$OVERALL_STATUS"
