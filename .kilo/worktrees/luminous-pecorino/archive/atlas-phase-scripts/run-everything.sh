#!/bin/bash
# ============================================================
# X3 Chain - Run Everything
# ============================================================
# Starts ALL services: Ollama/GPU, Blockchain, Swarm, X3OS,
# Explorer, Wallet, DEX, Solana DEX
# ============================================================

set -e

# Run mode
DETACH=0
# Strict mode: if set, fail fast on critical startup errors
STRICT=0
for a in "$@"; do
    if [ "$a" = "--strict" ]; then
        STRICT=1
    fi
done

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
NC='\033[0m' # No Color

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$SCRIPT_DIR"
cd "$PROJECT_ROOT"

# ============================================================
# Configuration
# ============================================================

# Infrastructure Ports
OLLAMA_PORT=11434
BLOCKCHAIN_PORT=9944
SWARM_API_PORT=8080
PROMETHEUS_PORT=9090
HTLC_COORDINATOR_PORT=8787  # HTLC Atomic Swap Coordinator

# Inferstructor & TPS Services
VALIDATOR_REGISTRY_PORT=7001
TPS_BRIDGE_PORT=9999
METRICS_DASHBOARD_PORT=8080
LLM_ROUTER_PORT=3000
LLM_METRICS_PORT=9091
TPS_TRACKER_INFLUX_PORT=8086
TPS_STREAMLIT_PORT=8501
INFERSTRUCTOR_DASHBOARD_PORT="${INFERSTRUCTOR_DASHBOARD_PORT:-${INFENSTRUCTIOR_DASHBOARD_PORT:-5174}}"

# Frontend Ports
X3OS_PORT=3001          # Explorer with X3OS at /x3os
WALLET_PORT=3002
DEX_PORT=3003
SOLANA_DEX_PORT=3006    # apps/next-solana-main (moved from 3000)
QUANTUM_DASHBOARD_PORT=3100  # Quantum Advisor Dashboard
X3_DESKTOP_PORT=5173     # X3 Desktop (Tauri) Vite dev server
VALIDATORS_PORT=3004        # Validators dashboard
X3_INTELLIGENCE_PORT=3005   # X3 Intelligence
POLKADEX_PORT=3007          # Polkadex DEX

# Service Ports
ANALYTICS_SERVICE_PORT=8081     # Analytics Service (Rust)
BLOCKCHAIN_ADAPTER_PORT=8082    # Blockchain Adapter (TypeScript)

# Cloudflare Tunnel & Placeholder
PLACEHOLDER_PORT=7000           # Subdomain placeholder server
CLOUDFLARE_TUNNEL_ID="6c118620-18cf-4795-80a8-6d44d37aecaa"
CLOUDFLARE_TUNNEL_NAME="x3-chain"
CLOUDFLARE_DOMAIN="x3star.net"

# URLs
BLOCKCHAIN_WS="ws://localhost:$BLOCKCHAIN_PORT"
SWARM_API_URL="http://localhost:$SWARM_API_PORT"

# Ollama binds are sometimes configured to docker0 (e.g. 172.17.0.1:11434).
# Allow override via env (OLLAMA_URL or OLLAMA_HOST), otherwise auto-detect.
OLLAMA_URL="${OLLAMA_URL:-${OLLAMA_HOST:-http://localhost:$OLLAMA_PORT}}"

# Timeouts
STARTUP_TIMEOUT=30
HEALTH_CHECK_INTERVAL=2

# PIDs file for cleanup
PIDS_FILE="$PROJECT_ROOT/.x3-pids"

# ============================================================
# Helper Functions
# ============================================================

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_header() {
    echo ""
    echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
    echo -e "${CYAN}  $1${NC}"
    echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
}

port_in_use() {
    local port=$1
    if command -v ss >/dev/null 2>&1; then
        ss -lntH "sport = :$port" 2>/dev/null | grep -q .
        return $?
    fi
    lsof -i:$port >/dev/null 2>&1
}

ollama_http_ok() {
    local base_url=$1
    curl -fsS --max-time 1 "$base_url/api/tags" >/dev/null 2>&1
}

resolve_ollama_url() {
    if [[ "$OLLAMA_URL" != http://* && "$OLLAMA_URL" != https://* ]]; then
        OLLAMA_URL="http://$OLLAMA_URL"
    fi

    # If the current value works, keep it.
    if ollama_http_ok "$OLLAMA_URL"; then
        return 0
    fi

    local candidates=()

    # systemd may have OLLAMA_HOST configured (commonly http://172.17.0.1:11434)
    local systemd_env
    systemd_env=$(systemctl show -p Environment ollama 2>/dev/null | sed -e 's/^Environment=//')
    local systemd_host
    systemd_host=$(echo "$systemd_env" | tr ' ' '\n' | sed -n 's/^\"\?OLLAMA_HOST=\(.*\)\"\?$/\1/p' | head -n 1)
    if [ -n "$systemd_host" ]; then
        if [[ "$systemd_host" != http://* && "$systemd_host" != https://* ]]; then
            systemd_host="http://$systemd_host"
        fi
        candidates+=("$systemd_host")
    fi

    # Common fallbacks
    candidates+=("http://localhost:$OLLAMA_PORT")
    candidates+=("http://127.0.0.1:$OLLAMA_PORT")
    candidates+=("http://172.17.0.1:$OLLAMA_PORT")

    local candidate
    for candidate in "${candidates[@]}"; do
        if [[ "$candidate" != http://* && "$candidate" != https://* ]]; then
            candidate="http://$candidate"
        fi
        if ollama_http_ok "$candidate"; then
            OLLAMA_URL="$candidate"
            return 0
        fi
    done

    return 1
}

wait_for_port() {
    local port=$1
    local name=$2
    local timeout=${3:-$STARTUP_TIMEOUT}
    local elapsed=0

    echo -n "  Waiting for $name on port $port..."
    while ! port_in_use $port && [ $elapsed -lt $timeout ]; do
        sleep $HEALTH_CHECK_INTERVAL
        elapsed=$((elapsed + HEALTH_CHECK_INTERVAL))
        echo -n "."
    done

    if port_in_use $port; then
        echo -e " ${GREEN}✓${NC}"
        return 0
    else
        echo -e " ${RED}✗${NC}"
        return 1
    fi
}

# Wait for HTTP JSON-RPC readiness for Substrate node
wait_for_rpc() {
    local host=${1:-"http://localhost:$BLOCKCHAIN_PORT"}
    local timeout=${2:-60}
    local elapsed=0

    echo -n "  Waiting for Blockchain RPC at $host..."
    while [ $elapsed -lt $timeout ]; do
        # system_health is a lightweight JSON-RPC method
        if curl -sf -X POST -H 'Content-Type: application/json' --data '{"jsonrpc":"2.0","method":"system_health","params":[],"id":1}' "$host" >/dev/null 2>&1; then
            echo -e " ${GREEN}✓${NC}"
            return 0
        fi
        sleep $HEALTH_CHECK_INTERVAL
        elapsed=$((elapsed + HEALTH_CHECK_INTERVAL))
        echo -n "."
    done
    echo -e " ${RED}✗${NC}"
    return 1
}

wait_for_health() {
    local url=$1
    local name=$2
    local timeout=${3:-$STARTUP_TIMEOUT}
    local elapsed=0

    echo -n "  Health check for $name at $url..."
    while [ $elapsed -lt $timeout ]; do
        if curl -sf "$url" >/dev/null 2>&1; then
            echo -e " ${GREEN}✓${NC}"
            return 0
        fi
        sleep $HEALTH_CHECK_INTERVAL
        elapsed=$((elapsed + HEALTH_CHECK_INTERVAL))
        echo -n "."
    done

    echo -e " ${RED}✗${NC}"
    return 1
}

save_pid() {
    echo "$1:$2" >> "$PIDS_FILE"
}

cleanup_pids() {
    if [ -f "$PIDS_FILE" ]; then
        while IFS=: read -r name pid; do
            if kill -0 "$pid" 2>/dev/null; then
                log_info "Stopping $name (PID: $pid) - sending SIGTERM"
                kill -TERM "$pid" 2>/dev/null || true
                # Wait up to 5 seconds for graceful stop
                local waited=0
                while kill -0 "$pid" 2>/dev/null && [ $waited -lt 5 ]; do
                    sleep 1
                    waited=$((waited + 1))
                done
                if kill -0 "$pid" 2>/dev/null; then
                    log_info "$name did not stop gracefully; sending SIGKILL"
                    kill -KILL "$pid" 2>/dev/null || true
                fi
            fi
        done < "$PIDS_FILE"
        rm -f "$PIDS_FILE"
    fi
}

kill_port() {
    local port=$1
    if port_in_use $port; then
        log_info "Killing process on port $port"
        fuser -k $port/tcp 2>/dev/null || true
        sleep 1
    fi
}

# ============================================================
# Service Starters
# ============================================================

start_ollama() {
    log_header "Starting Ollama (Multi-GPU AI)"

    # If already running, require the HTTP endpoint to respond.
    if port_in_use $OLLAMA_PORT; then
        resolve_ollama_url || true
        log_info "Ollama URL: $OLLAMA_URL"

        if ollama_http_ok "$OLLAMA_URL"; then
            log_success "Ollama is ready"
            return 0
        fi

        log_warn "Ollama is listening but HTTP is not reachable yet"

        # If managed by systemd, try a restart to recover quickly.
        if systemctl is-active --quiet ollama 2>/dev/null; then
            log_info "Restarting Ollama via systemd..."
            sudo systemctl restart ollama 2>/dev/null || true
            sleep 2
        fi
    fi

    if ! command -v ollama &> /dev/null; then
        log_warn "Ollama not installed. Skipping AI services..."
        return 1
    fi

    log_info "Starting Ollama service..."

    # Try systemctl first (preferred for multi-GPU setup)
    if systemctl is-active --quiet ollama 2>/dev/null; then
        log_success "Ollama systemd service already active"
    elif sudo systemctl start ollama 2>/dev/null; then
        sleep 2
        log_success "Ollama started via systemd"
    else
        # Fallback to direct start
        ollama serve > "$PROJECT_ROOT/logs/ollama.log" 2>&1 &
        local pid=$!
        save_pid "ollama" $pid
    fi

    # Wait for the HTTP API (bind address might not be localhost).
    local timeout=30
    local elapsed=0
    echo -n "  Waiting for Ollama HTTP API..."
    while [ $elapsed -lt $timeout ]; do
        resolve_ollama_url || true
        if ollama_http_ok "$OLLAMA_URL"; then
            echo -e " ${GREEN}✓${NC}"
            log_info "Ollama URL: $OLLAMA_URL"

            local models
            models=$(curl -sS --max-time 2 "$OLLAMA_URL/api/tags" | jq -r '.models[].name' 2>/dev/null | head -5)
            if [ -n "$models" ]; then
                log_info "Available models: $(echo $models | tr '\n' ' ')"
            fi
            return 0
        fi

        sleep 2
        elapsed=$((elapsed + 2))
        echo -n "."
    done
    echo -e " ${RED}✗${NC}"

    log_warn "Ollama not responding (showing recent logs)"
    sudo journalctl -u ollama --no-pager -n 30 2>/dev/null || true
    return 1
}

start_blockchain() {
    log_header "Starting Blockchain Node"

    if port_in_use $BLOCKCHAIN_PORT; then
        log_warn "Blockchain already running on port $BLOCKCHAIN_PORT"
        return 0
    fi

    # Find node binary (support cargo placing artifacts in target/<triple>/release)
    local node_binary=""
    if [ -f "$PROJECT_ROOT/target/release/x3-chain-node" ]; then
        node_binary="$PROJECT_ROOT/target/release/x3-chain-node"
    elif [ -f "$PROJECT_ROOT/target/x86_64-unknown-linux-gnu/release/x3-chain-node" ]; then
        node_binary="$PROJECT_ROOT/target/x86_64-unknown-linux-gnu/release/x3-chain-node"
    elif [ -f "$PROJECT_ROOT/target/release/node" ]; then
        node_binary="$PROJECT_ROOT/target/release/node"
    elif [ -f "$PROJECT_ROOT/node/target/release/x3-chain-node" ]; then
        node_binary="$PROJECT_ROOT/node/target/release/x3-chain-node"
    else
        # Last-resort discovery: pick the first matching executable under target/**/release
        node_binary=$(find "$PROJECT_ROOT/target" -path '*/release/x3-chain-node' -type f -executable 2>/dev/null | head -n 1)
    fi

    if [ -z "$node_binary" ] || [ ! -f "$node_binary" ]; then
        log_warn "Node binary not found. Build with: cd node && cargo build --release"
        return 1
    fi

    log_info "Starting blockchain node from: $node_binary"
    $node_binary --dev --rpc-cors all --rpc-methods Unsafe \
        --rpc-port $BLOCKCHAIN_PORT \
        > "$PROJECT_ROOT/logs/blockchain.log" 2>&1 &

    local pid=$!
    save_pid "blockchain" $pid

    if wait_for_port $BLOCKCHAIN_PORT "Blockchain Node" 60; then
        # Verify JSON-RPC responds to a simple system_health call
        if wait_for_rpc "http://localhost:$BLOCKCHAIN_PORT" 60; then
            log_success "Blockchain node started and RPC responsive (PID: $pid)"
            return 0
        else
            log_error "Blockchain node port is open but RPC did not respond"
            return 1
        fi
    else
        log_error "Blockchain node failed to start (port not listening)"
        return 1
    fi
}

start_swarm_server() {
    log_header "Starting Swarm Server"

    if port_in_use $SWARM_API_PORT; then
        log_warn "Swarm server already running on port $SWARM_API_PORT"
        return 0
    fi

    # Activate Python venv
    if [ -f "$PROJECT_ROOT/.venv/bin/activate" ]; then
        source "$PROJECT_ROOT/.venv/bin/activate"
    fi

    # Set environment variables
    export SWARM_HOST="0.0.0.0"
    export SWARM_PORT="$SWARM_API_PORT"
    export BLOCKCHAIN_WS_URL="$BLOCKCHAIN_WS"
    export OLLAMA_URL="$OLLAMA_URL"
    export TOTAL_GPUS="100"
    export LOG_LEVEL="INFO"
    export PYTHONPATH="$PROJECT_ROOT:$PYTHONPATH"

    log_info "Starting unified swarm server..."

    # Try unified server first, then api_server
    if [ -f "$PROJECT_ROOT/swarm/unified_server.py" ]; then
        python3 "$PROJECT_ROOT/swarm/unified_server.py" \
            > "$PROJECT_ROOT/logs/swarm.log" 2>&1 &
    elif [ -f "$PROJECT_ROOT/swarm/api_server.py" ]; then
        python3 -m swarm.api_server \
            > "$PROJECT_ROOT/logs/swarm.log" 2>&1 &
    else
        log_warn "No swarm server found"
        return 1
    fi

    local pid=$!
    save_pid "swarm" $pid

    # Prefer readiness probe which includes dependent checks (blockchain_connected, gpu_manager_ready)
    if wait_for_health "$SWARM_API_URL/ready" "Swarm Server (readiness)" 60; then
        log_success "Swarm server started and ready (PID: $pid)"
        return 0
    else
        log_error "Swarm server failed readiness check or is not connected to dependencies"
        return 1
    fi
}

start_nextjs_app() {
    local app_name=$1
    local app_dir=$2
    local port=$3

    if port_in_use $port; then
        log_warn "$app_name already running on port $port"
        return 0
    fi

    if [ ! -d "$app_dir" ] || [ ! -f "$app_dir/package.json" ]; then
        log_warn "$app_name not found at $app_dir"
        return 1
    fi

    # Check for app directory
    if [ ! -d "$app_dir/app" ] && [ ! -d "$app_dir/pages" ] && [ ! -d "$app_dir/src/app" ] && [ ! -d "$app_dir/src" ]; then
        # Try to detect a Next.js app inside a monorepo packages/ subfolder
        for candidate in "$app_dir"/packages/*; do
            if [ -d "$candidate" ] && [ -f "$candidate/package.json" ]; then
                if [ -d "$candidate/app" ] || [ -d "$candidate/pages" ] || [ -d "$candidate/src/app" ] || [ -d "$candidate/src" ]; then
                    log_info "Detected Next.js app for $app_name under $candidate"
                    app_dir="$candidate"
                    break
                fi
            fi
        done
    fi

    if [ ! -d "$app_dir/app" ] && [ ! -d "$app_dir/pages" ] && [ ! -d "$app_dir/src/app" ] && [ ! -d "$app_dir/src" ]; then
        log_warn "$app_name has no Next.js app structure, skipping..."
        return 1
    fi

    log_info "Starting $app_name on port $port..."

    cd "$app_dir"

    # Install deps if needed
    if [ ! -d "node_modules" ]; then
        log_info "Installing dependencies for $app_name..."
        npm install --silent 2>/dev/null || true
    fi

    # Set environment for swarm/blockchain integration
    export NEXT_PUBLIC_SWARM_API_URL="$SWARM_API_URL"
    export NEXT_PUBLIC_SWARM_WS_URL="ws://localhost:$SWARM_API_PORT/ws"
    export NEXT_PUBLIC_GPU_WS_URL="ws://localhost:$SWARM_API_PORT/ws/gpu"
    export NEXT_PUBLIC_BLOCKCHAIN_WS_URL="$BLOCKCHAIN_WS"
    export NEXT_PUBLIC_OLLAMA_URL="$OLLAMA_URL"

    npx next dev --port $port > "$PROJECT_ROOT/logs/$app_name.log" 2>&1 &

    local pid=$!
    save_pid "$app_name" $pid

    cd "$PROJECT_ROOT"

    if wait_for_port $port "$app_name" 60; then
        log_success "$app_name started (PID: $pid)"
        return 0
    else
        log_warn "$app_name may still be compiling..."
        return 0
    fi
}

start_x3_desktop() {
    log_header "Starting X3 Desktop (Tauri)"

    local app_dir="$PROJECT_ROOT/apps/x3-desktop"

    if [ ! -d "$app_dir" ] || [ ! -f "$app_dir/package.json" ]; then
        log_warn "X3 Desktop not found at $app_dir"
        return 1
    fi

    # Check for pre-built Tauri binary first
    local tauri_bin=""
    local tauri_candidates=(
        "$app_dir/src-tauri/target/release/x3-desktop"
        "$app_dir/src-tauri/target/release/x3_desktop_backend"
        "$app_dir/src-tauri/target/*/release/x3-desktop"
        "$app_dir/src-tauri/target/*/release/x3_desktop_backend"
    )

    for c in "${tauri_candidates[@]}"; do
        for f in $c; do
            if [ -x "$f" ] && [ -f "$f" ]; then
                tauri_bin="$f"
                break 2
            fi
        done
    done

    cd "$app_dir"

    # Install deps if needed
    if [ ! -d "node_modules" ]; then
        log_info "Installing dependencies for X3 Desktop..."
        npm install --silent 2>/dev/null || true
    fi

    # Set local-dev environment so Vite connects to the running node
    export VITE_DOMAIN="localhost"
    export VITE_APP_URL="http://localhost:$X3_DESKTOP_PORT"
    export VITE_RPC_HTTP="http://127.0.0.1:$BLOCKCHAIN_PORT"
    export VITE_RPC_WS="ws://127.0.0.1:$BLOCKCHAIN_PORT"
    export VITE_RPC_HTTP_LOCAL="http://127.0.0.1:$BLOCKCHAIN_PORT"
    export VITE_RPC_WS_LOCAL="ws://127.0.0.1:$BLOCKCHAIN_PORT"
    export VITE_API_URL="http://127.0.0.1:$SWARM_API_PORT"
    export VITE_EXPLORER_URL="http://127.0.0.1:$X3OS_PORT"

    if [ -n "$tauri_bin" ]; then
        log_info "Launching X3 Desktop from pre-built binary: $tauri_bin"
        "$tauri_bin" > "$PROJECT_ROOT/logs/x3-desktop.log" 2>&1 &
        local pid=$!
        save_pid "x3-desktop" $pid
        sleep 2
        if kill -0 "$pid" 2>/dev/null; then
            log_success "X3 Desktop started (PID: $pid)"
            cd "$PROJECT_ROOT"
            return 0
        fi
    fi

    # Fallback: run in Tauri dev mode (starts Vite + Tauri together)
    if [ -f "$app_dir/src-tauri/tauri.conf.json" ]; then
        log_info "Starting X3 Desktop in Tauri dev mode..."
        npm run tauri:dev > "$PROJECT_ROOT/logs/x3-desktop.log" 2>&1 &
        local pid=$!
        save_pid "x3-desktop" $pid

        cd "$PROJECT_ROOT"

        if wait_for_port $X3_DESKTOP_PORT "X3 Desktop" 90; then
            log_success "X3 Desktop Tauri started (PID: $pid)"
            return 0
        else
            log_warn "X3 Desktop may still be compiling (Tauri + Vite)..."
            return 0
        fi
    fi

    # Final fallback: Vite dev only (no Tauri shell)
    log_info "Starting X3 Desktop (Vite only) on port $X3_DESKTOP_PORT..."
    npm run dev > "$PROJECT_ROOT/logs/x3-desktop.log" 2>&1 &
    local pid=$!
    save_pid "x3-desktop" $pid

    cd "$PROJECT_ROOT"

    if wait_for_port $X3_DESKTOP_PORT "X3 Desktop" 60; then
        log_success "X3 Desktop Vite started (PID: $pid)"
        return 0
    else
        log_warn "X3 Desktop may still be compiling..."
        return 0
    fi
}

start_x3os_tauri() {
    log_header "Starting X3OS Tauri Desktop"

    # Common release binary locations
    local candidates=(
        "$PROJECT_ROOT/apps/x3os/src-tauri/target/release/x3os"
        "$PROJECT_ROOT/apps/x3os/src-tauri/target/*/release/x3os"
        "$PROJECT_ROOT/apps/x3os/target/release/x3os"
        "$PROJECT_ROOT/target/release/x3os"
        "$PROJECT_ROOT/target/*/release/x3os"
    )

    local bin=""
    for c in "${candidates[@]}"; do
        # expand globs
        for f in $c; do
            if [ -x "$f" ] && [ -f "$f" ]; then
                bin="$f"
                break 2
            fi
        done
    done

    if [ -z "$bin" ]; then
        log_warn "X3OS Tauri binary not found. Build with: cd apps/x3os/src-tauri && cargo build --release"
        return 1
    fi

    # Attempt to start the desktop (will open a GUI on the host)
    log_info "Launching X3OS desktop from: $bin"
    "$bin" > "$PROJECT_ROOT/logs/x3os-desktop.log" 2>&1 &
    local pid=$!
    save_pid "x3os-desktop" $pid

    # We don't wait on a specific port; just give the process a moment
    sleep 2
    if kill -0 "$pid" 2>/dev/null; then
        log_success "X3OS desktop started (PID: $pid)"
        return 0
    else
        log_error "Failed to start X3OS desktop; check $PROJECT_ROOT/logs/x3os-desktop.log"
        return 1
    fi
}

start_vite_app() {
    local app_name=$1
    local app_path=$2
    local port=$3
    
    log_header "Starting $app_name"
    
    if [ ! -d "$app_path" ]; then
        log_warn "$app_name not found at $app_path"
        return 1
    fi
    
    if port_in_use $port; then
        log_success "$app_name already running on port $port"
        return 0
    fi
    
    cd "$app_path" || return 1
    
    if [ ! -d "node_modules" ]; then
        log_info "Installing dependencies for $app_name..."
        npm install
    fi
    
    log_info "Starting $app_name on port $port..."
    PORT=$port npm run dev > "$PROJECT_ROOT/logs/$app_name.log" 2>&1 &
    local pid=$!
    echo "$pid $app_name" >> "$PIDS_FILE"
    
    sleep 3
    
    if ps -p $pid > /dev/null; then
        log_success "$app_name started (PID: $pid) on http://localhost:$port"
        cd "$PROJECT_ROOT"
        return 0
    else
        log_error "$app_name failed to start. Check logs/$app_name.log"
        cd "$PROJECT_ROOT"
        return 1
    fi
}

start_analytics_service() {
    log_header "Starting Analytics Service"
    
    local service_path="$PROJECT_ROOT/apps/analytics/analytics-service"
    
    if [ ! -d "$service_path" ]; then
        log_warn "Analytics service not found"
        return 1
    fi
    
    if port_in_use $ANALYTICS_SERVICE_PORT; then
        log_success "Analytics service already running"
        return 0
    fi
    
    cd "$service_path" || return 1
    
    log_info "Starting analytics service on port $ANALYTICS_SERVICE_PORT..."
    PORT=$ANALYTICS_SERVICE_PORT cargo run --release > "$PROJECT_ROOT/logs/analytics-service.log" 2>&1 &
    local pid=$!
    echo "$pid analytics-service" >> "$PIDS_FILE"
    
    sleep 3
    
    if ps -p $pid > /dev/null; then
        log_success "Analytics service started (PID: $pid)"
        cd "$PROJECT_ROOT"
        return 0
    else
        log_warn "Analytics service failed to start"
        cd "$PROJECT_ROOT"
        return 1
    fi
}

start_blockchain_adapter() {
    log_header "Starting Blockchain Adapter"
    
    local adapter_path="$PROJECT_ROOT/apps/blockchain-adapter"
    
    if [ ! -d "$adapter_path" ]; then
        log_warn "Blockchain adapter not found"
        return 1
    fi
    
    if port_in_use $BLOCKCHAIN_ADAPTER_PORT; then
        log_success "Blockchain adapter already running"
        return 0
    fi
    
    cd "$adapter_path" || return 1
    
    if [ ! -d "node_modules" ]; then
        log_info "Installing adapter dependencies..."
        npm install
    fi
    
    log_info "Starting blockchain adapter on port $BLOCKCHAIN_ADAPTER_PORT..."
    PORT=$BLOCKCHAIN_ADAPTER_PORT npx ts-node src/index.ts > "$PROJECT_ROOT/logs/blockchain-adapter.log" 2>&1 &
    local pid=$!
    echo "$pid blockchain-adapter" >> "$PIDS_FILE"
    
    sleep 2
    
    if ps -p $pid > /dev/null; then
        log_success "Blockchain adapter started (PID: $pid)"
        cd "$PROJECT_ROOT"
        return 0
    else
        log_warn "Blockchain adapter failed to start"
        cd "$PROJECT_ROOT"
        return 1
    fi
}

start_quantum_apps/dash-legacy-2-legacy-2board() {
    log_header "Starting Quantum Dashboard (Tauri)"

    local app_dir="$PROJECT_ROOT/apps/quantum-apps/dash-legacy-2-legacy-2board"

    if [ ! -d "$app_dir" ] || [ ! -f "$app_dir/package.json" ]; then
        log_warn "Quantum Dashboard not found at $app_dir"
        return 1
    fi

    # Check if Next.js dev server port is already in use
    if port_in_use $QUANTUM_DASHBOARD_PORT; then
        log_warn "Quantum Dashboard already running on port $QUANTUM_DASHBOARD_PORT"
        return 0
    fi

    cd "$app_dir"

    # Install deps if needed
    if [ ! -d "node_modules" ]; then
        log_info "Installing dependencies for Quantum Dashboard..."
        npm install --silent 2>/dev/null || true
    fi

    # Set environment for swarm/blockchain integration
    export NEXT_PUBLIC_SWARM_API_URL="$SWARM_API_URL"
    export NEXT_PUBLIC_SWARM_WS_URL="ws://localhost:$SWARM_API_PORT/ws"
    export NEXT_PUBLIC_GPU_WS_URL="ws://localhost:$SWARM_API_PORT/ws/gpu"
    export NEXT_PUBLIC_BLOCKCHAIN_WS_URL="$BLOCKCHAIN_WS"
    export NEXT_PUBLIC_OLLAMA_URL="$OLLAMA_URL"

    # Try to find pre-built Tauri binary first
    local tauri_bin=""
    local tauri_candidates=(
        "$app_dir/src-tauri/target/release/x3-quantum-apps/dash-legacy-2-legacy-2board"
        "$app_dir/src-tauri/target/release/X3 Quantum Dashboard"
        "$PROJECT_ROOT/target/release/x3-quantum-apps/dash-legacy-2-legacy-2board"
    )

    for c in "${tauri_candidates[@]}"; do
        if [ -x "$c" ] && [ -f "$c" ]; then
            tauri_bin="$c"
            break
        fi
    done

    if [ -n "$tauri_bin" ]; then
        log_info "Launching Quantum Dashboard Tauri app from: $tauri_bin"
        "$tauri_bin" > "$PROJECT_ROOT/logs/quantum-apps/dash-legacy-2-legacy-2board.log" 2>&1 &
        local pid=$!
        save_pid "quantum-apps/dash-legacy-2-legacy-2board" $pid
        sleep 2
        if kill -0 "$pid" 2>/dev/null; then
            log_success "Quantum Dashboard Tauri started (PID: $pid)"
            cd "$PROJECT_ROOT"
            return 0
        fi
    fi

    # Fallback: run tauri dev mode (starts Next.js + Tauri together)
    if [ -f "$app_dir/src-tauri/tauri.conf.json" ]; then
        log_info "Starting Quantum Dashboard in Tauri dev mode..."
        npm run tauri:dev > "$PROJECT_ROOT/logs/quantum-apps/dash-legacy-2-legacy-2board.log" 2>&1 &
        local pid=$!
        save_pid "quantum-apps/dash-legacy-2-legacy-2board" $pid

        cd "$PROJECT_ROOT"

        if wait_for_port $QUANTUM_DASHBOARD_PORT "Quantum Dashboard" 60; then
            log_success "Quantum Dashboard started (PID: $pid)"
            return 0
        else
            log_warn "Quantum Dashboard may still be compiling..."
            return 0
        fi
    fi

    # Final fallback: just run Next.js dev server
    log_info "Starting Quantum Dashboard (Next.js only) on port $QUANTUM_DASHBOARD_PORT..."
    npm run dev > "$PROJECT_ROOT/logs/quantum-apps/dash-legacy-2-legacy-2board.log" 2>&1 &
    local pid=$!
    save_pid "quantum-apps/dash-legacy-2-legacy-2board" $pid

    cd "$PROJECT_ROOT"

    if wait_for_port $QUANTUM_DASHBOARD_PORT "Quantum Dashboard" 60; then
        log_success "Quantum Dashboard started (PID: $pid)"
        return 0
    else
        log_warn "Quantum Dashboard may still be compiling..."
        return 0
    fi
}

start_llm_router() {
    log_header "Starting LLM Router with Metrics"

    if port_in_use $LLM_ROUTER_PORT; then
        log_success "LLM Router already running on port $LLM_ROUTER_PORT"
        return 0
    fi

    local llm_service_dir="$PROJECT_ROOT/llm-service"
    if [ ! -d "$llm_service_dir" ] || [ ! -f "$llm_service_dir/start-with-metrics.js" ]; then
        log_warn "LLM service not found at $llm_service_dir"
        return 1
    fi

    log_info "Starting LLM Router on ports $LLM_ROUTER_PORT (router) and $LLM_METRICS_PORT (metrics)..."

    cd "$llm_service_dir"
    PORT=$LLM_ROUTER_PORT METRICS_PORT=$LLM_METRICS_PORT node start-with-metrics.js --config=../llm-config.json \
        > "$PROJECT_ROOT/logs/llm-router.log" 2>&1 &
    local pid=$!
    save_pid "llm-router" $pid

    cd "$PROJECT_ROOT"

    if wait_for_health "http://localhost:$LLM_ROUTER_PORT/health" "LLM Router" 30; then
        log_success "LLM Router started (PID: $pid)"
        return 0
    else
        log_warn "LLM Router may still be starting..."
        return 1
    fi
}

start_validator_registry() {
    log_header "Starting Validator Registry"

    if port_in_use $VALIDATOR_REGISTRY_PORT; then
        log_success "Validator Registry already running on port $VALIDATOR_REGISTRY_PORT"
        return 0
    fi

    local registry_dir="$PROJECT_ROOT/cross-chain-gpu-validator/tests/inferstructor"
    if [ ! -d "$registry_dir" ] || [ ! -f "$registry_dir/validator_registry.py" ]; then
        log_warn "Validator Registry not found"
        return 1
    fi

    log_info "Starting Validator Registry on port $VALIDATOR_REGISTRY_PORT..."

    # Activate Python venv if available
    if [ -f "$PROJECT_ROOT/cross-chain-gpu-validator/.venv/bin/activate" ]; then
        source "$PROJECT_ROOT/cross-chain-gpu-validator/.venv/bin/activate"
    fi

    cd "$registry_dir"
    mkdir -p logs
    python3 validator_registry.py > "$PROJECT_ROOT/logs/validator-registry.log" 2>&1 &
    local pid=$!
    save_pid "validator-registry" $pid

    cd "$PROJECT_ROOT"

    if wait_for_health "http://localhost:$VALIDATOR_REGISTRY_PORT/health" "Validator Registry" 30; then
        log_success "Validator Registry started (PID: $pid)"
        return 0
    else
        log_warn "Validator Registry may still be starting..."
        return 1
    fi
}

start_tps_bridge() {
    log_header "Starting TPS Bridge"

    if port_in_use $TPS_BRIDGE_PORT; then
        log_success "TPS Bridge already running on port $TPS_BRIDGE_PORT"
        return 0
    fi

    local bridge_dir="$PROJECT_ROOT/cross-chain-gpu-validator/tests/inferstructor"
    if [ ! -d "$bridge_dir" ] || [ ! -f "$bridge_dir/tps_bridge.py" ]; then
        log_warn "TPS Bridge not found"
        return 1
    fi

    log_info "Starting TPS Bridge on port $TPS_BRIDGE_PORT..."

    cd "$bridge_dir"
    python3 tps_bridge.py > "$PROJECT_ROOT/logs/tps-bridge.log" 2>&1 &
    local pid=$!
    save_pid "tps-bridge" $pid

    cd "$PROJECT_ROOT"

    if wait_for_health "http://localhost:$TPS_BRIDGE_PORT/health" "TPS Bridge" 30; then
        log_success "TPS Bridge started (PID: $pid)"
        return 0
    else
        log_warn "TPS Bridge may still be starting..."
        return 1
    fi
}

start_lane_orchestrator() {
    log_header "Starting Lane Orchestrator"

    local orchestrator_dir="$PROJECT_ROOT/cross-chain-gpu-validator/tests/inferstructor"
    if [ ! -d "$orchestrator_dir" ] || [ ! -f "$orchestrator_dir/lane_orchestrator.py" ]; then
        log_warn "Lane Orchestrator not found"
        return 1
    fi

    log_info "Starting Lane Orchestrator (background service)..."

    cd "$orchestrator_dir"
    python3 lane_orchestrator.py > "$PROJECT_ROOT/logs/lane-orchestrator.log" 2>&1 &
    local pid=$!
    save_pid "lane-orchestrator" $pid

    cd "$PROJECT_ROOT"

    sleep 2
    if kill -0 "$pid" 2>/dev/null; then
        log_success "Lane Orchestrator started (PID: $pid)"
        return 0
    else
        log_warn "Lane Orchestrator failed to start"
        return 1
    fi
}

start_inferstructor_dashboard() {
    log_header "Starting Inferstructor Dashboard"

    if port_in_use $INFERSTRUCTOR_DASHBOARD_PORT; then
        log_success "Inferstructor Dashboard already running on port $INFERSTRUCTOR_DASHBOARD_PORT"
        return 0
    fi

    local dashboard_dir="$PROJECT_ROOT/apps/inferstructor-dashboard"
    if [ ! -d "$dashboard_dir" ]; then
        log_warn "Inferstructor Dashboard not found at $dashboard_dir"
        return 1
    fi

    cd "$dashboard_dir"

    if [ ! -d "node_modules" ]; then
        log_info "Installing Inferstructor Dashboard dependencies..."
        npm install --silent 2>/dev/null || true
    fi

    log_info "Starting Inferstructor Dashboard on port $INFERSTRUCTOR_DASHBOARD_PORT..."
    PORT=$INFERSTRUCTOR_DASHBOARD_PORT npm run dev > "$PROJECT_ROOT/logs/inferstructor-dashboard.log" 2>&1 &
    local pid=$!
    save_pid "inferstructor-dashboard" $pid

    cd "$PROJECT_ROOT"

    if wait_for_port $INFERSTRUCTOR_DASHBOARD_PORT "Inferstructor Dashboard" 60; then
        log_success "Inferstructor Dashboard started (PID: $pid)"
        return 0
    else
        log_warn "Inferstructor Dashboard may still be compiling..."
        return 0
    fi
}

start_htlc_coordinator() {
    log_header "Starting HTLC Atomic Swap Coordinator"

    if port_in_use $HTLC_COORDINATOR_PORT; then
        log_warn "HTLC Coordinator already running on port $HTLC_COORDINATOR_PORT"
        return 0
    fi

    local coordinator_dir="$PROJECT_ROOT/scripts/coordinator"

    if [ ! -d "$coordinator_dir" ] || [ ! -f "$coordinator_dir/package.json" ]; then
        log_warn "HTLC Coordinator not found at $coordinator_dir"
        return 1
    fi

    log_info "Starting HTLC Coordinator..."

    cd "$coordinator_dir"

    # Install deps if needed
    if [ ! -d "node_modules" ]; then
        log_info "Installing coordinator dependencies..."
        npm install --silent 2>/dev/null || true
    fi

    # Set environment variables for RPC endpoints
    export BTC_RPC_URL="${BTC_RPC_URL:-http://localhost:18332}"
    export EVM_RPC_URL="${EVM_RPC_URL:-http://localhost:8545}"
    export SOLANA_RPC_URL="${SOLANA_RPC_URL:-http://localhost:8899}"
    export X3_RPC_ENDPOINT="${X3_RPC_ENDPOINT:-http://127.0.0.1:9944}"
    export PORT=$HTLC_COORDINATOR_PORT

    log_info "Coordinator environment:"
    log_info "  BTC_RPC_URL: $BTC_RPC_URL"
    log_info "  EVM_RPC_URL: $EVM_RPC_URL"
    log_info "  SOLANA_RPC_URL: $SOLANA_RPC_URL"
    log_info "  X3_RPC_ENDPOINT: $X3_RPC_ENDPOINT"
    log_info "  PORT: $PORT"

    npm run serve > "$PROJECT_ROOT/logs/htlc-coordinator.log" 2>&1 &

    local pid=$!
    save_pid "htlc-coordinator" $pid

    cd "$PROJECT_ROOT"

    if wait_for_health "http://localhost:$HTLC_COORDINATOR_PORT/health" "HTLC Coordinator" 30; then
        log_success "HTLC Coordinator started (PID: $pid)"
        return 0
    else
        log_warn "HTLC Coordinator may still be starting..."
        return 0
    fi
}

# ============================================================
# Cloudflare Tunnel & Placeholder (x3star.net)
# ============================================================

start_placeholder_server() {
    log_header "Starting Placeholder Server (x3star.net subdomains)"

    local server_dir="$PROJECT_ROOT/infra/cloudflare-tunnel/placeholder"

    if [ ! -f "$server_dir/server.js" ]; then
        log_warn "Placeholder server not found at $server_dir/server.js"
        return 1
    fi

    if port_in_use $PLACEHOLDER_PORT; then
        log_success "Placeholder server already running on port $PLACEHOLDER_PORT"
        return 0
    fi

    cd "$server_dir"

    # Install deps if needed (currently none, but future-proof)
    if [ -f "package.json" ] && [ ! -d "node_modules" ]; then
        npm install --silent 2>/dev/null || true
    fi

    log_info "Starting placeholder server on port $PLACEHOLDER_PORT..."
    node server.js > "$PROJECT_ROOT/logs/placeholder-server.log" 2>&1 &
    local pid=$!
    save_pid "placeholder-server" $pid

    cd "$PROJECT_ROOT"

    if wait_for_port $PLACEHOLDER_PORT "Placeholder Server" 15; then
        log_success "Placeholder server started (PID: $pid)"
        return 0
    else
        log_warn "Placeholder server may still be starting..."
        return 0
    fi
}

start_cloudflare_tunnel() {
    log_header "Starting Cloudflare Tunnel ($CLOUDFLARE_DOMAIN)"

    if ! command -v cloudflared &> /dev/null; then
        log_warn "cloudflared not installed. Skipping tunnel..."
        log_info "Install: https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/downloads/"
        return 1
    fi

    # Check if tunnel is already connected
    if pgrep -f "cloudflared tunnel run" >/dev/null 2>&1; then
        log_success "Cloudflare Tunnel already running"
        return 0
    fi

    # Verify credentials exist
    local creds_file="$HOME/.cloudflared/${CLOUDFLARE_TUNNEL_ID}.json"
    if [ ! -f "$creds_file" ]; then
        log_error "Tunnel credentials not found: $creds_file"
        log_info "Run: cloudflared tunnel login"
        return 1
    fi

    log_info "Starting Cloudflare Tunnel: $CLOUDFLARE_TUNNEL_NAME..."
    cloudflared tunnel run "$CLOUDFLARE_TUNNEL_NAME" > "$PROJECT_ROOT/logs/cloudflare-tunnel.log" 2>&1 &
    local pid=$!
    save_pid "cloudflare-tunnel" $pid

    # Wait for tunnel to register (check log for connection confirmation)
    local timeout=30
    local elapsed=0
    echo -n "  Waiting for tunnel connections..."
    while [ $elapsed -lt $timeout ]; do
        if grep -q "Registered tunnel connection" "$PROJECT_ROOT/logs/cloudflare-tunnel.log" 2>/dev/null; then
            echo -e " ${GREEN}✓${NC}"
            local conns
            conns=$(grep -c "Registered tunnel connection" "$PROJECT_ROOT/logs/cloudflare-tunnel.log" 2>/dev/null || echo 0)
            log_success "Cloudflare Tunnel connected ($conns connections, PID: $pid)"
            log_info "Domain live: https://$CLOUDFLARE_DOMAIN"
            return 0
        fi
        sleep $HEALTH_CHECK_INTERVAL
        elapsed=$((elapsed + HEALTH_CHECK_INTERVAL))
        echo -n "."
    done

    echo -e " ${YELLOW}?${NC}"
    log_warn "Tunnel may still be connecting (PID: $pid). Check logs/cloudflare-tunnel.log"
    return 0
}

# ============================================================
# Status Display
# ============================================================

show_status() {
    resolve_ollama_url || true
    echo ""
    echo -e "${CYAN}╔══════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║                    🩺 SERVICE STATUS                         ║${NC}"
    echo -e "${CYAN}╠══════════════════════════════════════════════════════════════╣${NC}"

    # Ollama
    if ollama_http_ok "$OLLAMA_URL"; then
        echo -e "${CYAN}║${NC}  ${GREEN}✓${NC} Ollama (GPU/AI)     ${OLLAMA_URL}              ${CYAN}║${NC}"
    elif port_in_use $OLLAMA_PORT || systemctl is-active --quiet ollama 2>/dev/null; then
        echo -e "${CYAN}║${NC}  ${YELLOW}○${NC} Ollama (GPU/AI)     LISTENING/STARTING               ${CYAN}║${NC}"
    else
        echo -e "${CYAN}║${NC}  ${RED}✗${NC} Ollama (GPU/AI)     NOT RUNNING                        ${CYAN}║${NC}"
    fi

    # Blockchain
    if port_in_use $BLOCKCHAIN_PORT; then
        echo -e "${CYAN}║${NC}  ${GREEN}✓${NC} Blockchain Node     ws://localhost:$BLOCKCHAIN_PORT                ${CYAN}║${NC}"
    else
        echo -e "${CYAN}║${NC}  ${RED}✗${NC} Blockchain Node     NOT RUNNING                        ${CYAN}║${NC}"
    fi

    # Swarm
    if port_in_use $SWARM_API_PORT; then
        echo -e "${CYAN}║${NC}  ${GREEN}✓${NC} Swarm API Server    http://localhost:$SWARM_API_PORT                  ${CYAN}║${NC}"
    else
        echo -e "${CYAN}║${NC}  ${RED}✗${NC} Swarm API Server    NOT RUNNING                        ${CYAN}║${NC}"
    fi

    # HTLC Coordinator
    if port_in_use $HTLC_COORDINATOR_PORT; then
        echo -e "${CYAN}║${NC}  ${GREEN}✓${NC} HTLC Coordinator    http://localhost:$HTLC_COORDINATOR_PORT                ${CYAN}║${NC}"
    else
        echo -e "${CYAN}║${NC}  ${RED}✗${NC} HTLC Coordinator    NOT RUNNING                        ${CYAN}║${NC}"
    fi

    # LLM Router
    if port_in_use $LLM_ROUTER_PORT; then
        echo -e "${CYAN}║${NC}  ${GREEN}✓${NC} LLM Router          http://localhost:$LLM_ROUTER_PORT                  ${CYAN}║${NC}"
    else
        echo -e "${CYAN}║${NC}  ${RED}✗${NC} LLM Router          NOT RUNNING                        ${CYAN}║${NC}"
    fi

    # Validator Registry
    if port_in_use $VALIDATOR_REGISTRY_PORT; then
        echo -e "${CYAN}║${NC}  ${GREEN}✓${NC} Validator Registry  http://localhost:$VALIDATOR_REGISTRY_PORT                ${CYAN}║${NC}"
    else
        echo -e "${CYAN}║${NC}  ${RED}✗${NC} Validator Registry  NOT RUNNING                        ${CYAN}║${NC}"
    fi

    # TPS Bridge
    if port_in_use $TPS_BRIDGE_PORT; then
        echo -e "${CYAN}║${NC}  ${GREEN}✓${NC} TPS Bridge          http://localhost:$TPS_BRIDGE_PORT                ${CYAN}║${NC}"
    else
        echo -e "${CYAN}║${NC}  ${RED}✗${NC} TPS Bridge          NOT RUNNING                        ${CYAN}║${NC}"
    fi

    # Inferstructor Dashboard
    if port_in_use $INFERSTRUCTOR_DASHBOARD_PORT; then
        echo -e "${CYAN}║${NC}  ${GREEN}✓${NC} Inferstructor      http://localhost:$INFERSTRUCTOR_DASHBOARD_PORT                ${CYAN}║${NC}"
    else
        echo -e "${CYAN}║${NC}  ${RED}✗${NC} Inferstructor      NOT RUNNING                        ${CYAN}║${NC}"
    fi

    # Solana DEX (apps/next-solana-main with X3 Exchange)
    if port_in_use $SOLANA_DEX_PORT; then
        echo -e "${CYAN}║${NC}  ${GREEN}✓${NC} Solana DEX          http://localhost:$SOLANA_DEX_PORT                  ${CYAN}║${NC}"
    else
        echo -e "${CYAN}║${NC}  ${RED}✗${NC} Solana DEX          NOT RUNNING                        ${CYAN}║${NC}"
    fi

    # Explorer (with X3OS)
    if port_in_use $X3OS_PORT; then
        echo -e "${CYAN}║${NC}  ${GREEN}✓${NC} Explorer + X3OS     http://localhost:$X3OS_PORT                  ${CYAN}║${NC}"
    else
        echo -e "${CYAN}║${NC}  ${RED}✗${NC} Explorer + X3OS     NOT RUNNING                        ${CYAN}║${NC}"
    fi

    # Wallet
    if port_in_use $WALLET_PORT; then
        echo -e "${CYAN}║${NC}  ${GREEN}✓${NC} Wallet              http://localhost:$WALLET_PORT                  ${CYAN}║${NC}"
    else
        echo -e "${CYAN}║${NC}  ${RED}✗${NC} Wallet              NOT RUNNING                        ${CYAN}║${NC}"
    fi

    # DEX
    if port_in_use $DEX_PORT; then
        echo -e "${CYAN}║${NC}  ${GREEN}✓${NC} DEX                 http://localhost:$DEX_PORT                  ${CYAN}║${NC}"
    else
        echo -e "${CYAN}║${NC}  ${RED}✗${NC} DEX                 NOT RUNNING                        ${CYAN}║${NC}"
    fi

    # Quantum Dashboard
    if port_in_use $QUANTUM_DASHBOARD_PORT; then
        echo -e "${CYAN}║${NC}  ${GREEN}✓${NC} Quantum Dashboard   http://localhost:$QUANTUM_DASHBOARD_PORT                ${CYAN}║${NC}"
    else
        echo -e "${CYAN}║${NC}  ${RED}✗${NC} Quantum Dashboard   NOT RUNNING                        ${CYAN}║${NC}"
    fi

    # X3 Desktop
    if port_in_use $X3_DESKTOP_PORT; then
        echo -e "${CYAN}║${NC}  ${GREEN}✓${NC} X3 Desktop       http://localhost:$X3_DESKTOP_PORT                ${CYAN}║${NC}"
    else
        echo -e "${CYAN}║${NC}  ${RED}✗${NC} X3 Desktop       NOT RUNNING                        ${CYAN}║${NC}"
    fi

    # Placeholder Server
    if port_in_use $PLACEHOLDER_PORT; then
        echo -e "${CYAN}║${NC}  ${GREEN}✓${NC} Placeholder Server  http://localhost:$PLACEHOLDER_PORT                ${CYAN}║${NC}"
    else
        echo -e "${CYAN}║${NC}  ${RED}✗${NC} Placeholder Server  NOT RUNNING                        ${CYAN}║${NC}"
    fi

    # Cloudflare Tunnel
    if pgrep -f "cloudflared tunnel run" >/dev/null 2>&1; then
        echo -e "${CYAN}║${NC}  ${GREEN}✓${NC} Cloudflare Tunnel   https://$CLOUDFLARE_DOMAIN              ${CYAN}║${NC}"
    else
        echo -e "${CYAN}║${NC}  ${RED}✗${NC} Cloudflare Tunnel   NOT RUNNING                        ${CYAN}║${NC}"
    fi

    echo -e "${CYAN}╚══════════════════════════════════════════════════════════════╝${NC}"

    # GPU Status if nvidia-smi available
    if command -v nvidia-smi &> /dev/null; then
        echo ""
        echo -e "${MAGENTA}GPU Status:${NC}"
        nvidia-smi --query-gpu=index,name,memory.used,memory.free,utilization.gpu --format=csv,noheader 2>/dev/null || true
    fi

    # Swarm health if running
    if port_in_use $SWARM_API_PORT; then
        echo ""
        echo -e "${MAGENTA}Swarm Health:${NC}"
        curl -s "$SWARM_API_URL/health" 2>/dev/null | jq . 2>/dev/null || echo "  Unable to fetch health"
    fi
}

# ============================================================
# Main Script
# ============================================================

main() {
    echo ""
    echo -e "${MAGENTA}╔══════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${MAGENTA}║       🚀 X3 CHAIN - RUN EVERYTHING 🚀                    ║${NC}"
    echo -e "${MAGENTA}╠══════════════════════════════════════════════════════════════╣${NC}"
    echo -e "${MAGENTA}║  ${CYAN}GPU/AI:${NC}       ${OLLAMA_URL} (Ollama)                ║${NC}"
    echo -e "${MAGENTA}║  ${CYAN}Blockchain:${NC}   ws://localhost:$BLOCKCHAIN_PORT                          ║${NC}"
    echo -e "${MAGENTA}║  ${CYAN}Swarm API:${NC}    http://localhost:$SWARM_API_PORT                           ║${NC}"
    echo -e "${MAGENTA}║  ${CYAN}LLM Router:${NC}   http://localhost:$LLM_ROUTER_PORT                           ║${NC}"
    echo -e "${MAGENTA}║  ${CYAN}Validator:${NC}    http://localhost:$VALIDATOR_REGISTRY_PORT (Registry)       ║${NC}"
    echo -e "${MAGENTA}║  ${CYAN}TPS Bridge:${NC}   http://localhost:$TPS_BRIDGE_PORT (Inferstructor)        ║${NC}"
    echo -e "${MAGENTA}║  ${CYAN}Inferstructor:${NC} http://localhost:$INFERSTRUCTOR_DASHBOARD_PORT (Dashboard) ║${NC}"
    echo -e "${MAGENTA}║  ${CYAN}HTLC Coord:${NC}   http://localhost:$HTLC_COORDINATOR_PORT (Atomic Swaps)     ║${NC}"
    echo -e "${MAGENTA}║  ${CYAN}Solana DEX:${NC}   http://localhost:$SOLANA_DEX_PORT (X3 Exchange)             ║${NC}"
    echo -e "${MAGENTA}║  ${CYAN}X3OS:${NC}         http://localhost:$X3OS_PORT/x3os                      ║${NC}"
    echo -e "${MAGENTA}║  ${CYAN}Explorer:${NC}     http://localhost:$X3OS_PORT                           ║${NC}"
    echo -e "${MAGENTA}║  ${CYAN}Wallet:${NC}       http://localhost:$WALLET_PORT                           ║${NC}"
    echo -e "${MAGENTA}║  ${CYAN}DEX:${NC}          http://localhost:$DEX_PORT                           ║${NC}"
    echo -e "${MAGENTA}║  ${CYAN}X3 Desktop:${NC}http://localhost:$X3_DESKTOP_PORT (Tauri)              ║${NC}"
    echo -e "${MAGENTA}║  ${CYAN}Quantum:${NC}      http://localhost:$QUANTUM_DASHBOARD_PORT                          ║${NC}"
    echo -e "${MAGENTA}╚══════════════════════════════════════════════════════════════╝${NC}"
    echo ""

    # Create logs directory
    mkdir -p "$PROJECT_ROOT/logs"

    # Resolve Ollama URL early so downstream services get the correct endpoint.
    resolve_ollama_url || true

    # Cleanup old PIDs
    cleanup_pids

    # Cleanup existing processes on our ports
    log_header "Cleaning Up Existing Processes"
    for port in $X3OS_PORT $WALLET_PORT $DEX_PORT $SWARM_API_PORT $SOLANA_DEX_PORT $QUANTUM_DASHBOARD_PORT $X3_DESKTOP_PORT $LLM_ROUTER_PORT $VALIDATOR_REGISTRY_PORT $TPS_BRIDGE_PORT $INFERSTRUCTOR_DASHBOARD_PORT $PLACEHOLDER_PORT; do
        kill_port $port
    done

    # ============================================================
    # Layer 0: GPU/AI Infrastructure
    # ============================================================
    if ! start_ollama; then
        if [ "$STRICT" -eq 1 ]; then
            log_error "Ollama failed to start and --strict is set; aborting startup"
            exit 1
        else
            log_warn "Continuing without Ollama..."
        fi
    fi

    # ============================================================
    # Layer 1: Blockchain
    # ============================================================
    if ! start_blockchain; then
        if [ "$STRICT" -eq 1 ]; then
            log_error "Blockchain failed to start and --strict is set; aborting startup"
            exit 1
        else
            log_warn "Blockchain not started, continuing..."
        fi
    fi

    # ============================================================
    # Layer 2: Backend Services & APIs
    # ============================================================
    
    # Start LLM Router (needed by many services)
    start_llm_router || log_warn "LLM Router not started, continuing..."
    sleep 1
    
    # Start Validator Registry (needed by TPS Bridge)
    start_validator_registry || log_warn "Validator Registry not started, continuing..."
    sleep 1
    
    # Start TPS Bridge
    start_tps_bridge || log_warn "TPS Bridge not started, continuing..."
    sleep 1
    
    # Start Lane Orchestrator
    start_lane_orchestrator || log_warn "Lane Orchestrator not started, continuing..."
    sleep 1
    
    # Start Swarm Server
    if ! start_swarm_server; then
        if [ "$STRICT" -eq 1 ]; then
            log_error "Swarm server failed readiness check and --strict is set; aborting startup"
            exit 1
        else
            log_warn "Swarm server not started, continuing..."
        fi
    fi

    # ============================================================
    # Layer 2.5: HTLC Coordinator (Atomic Swaps)
    # ============================================================
    start_htlc_coordinator || log_warn "HTLC Coordinator not started, continuing..."

    # ============================================================
    # Layer 3: Frontend Applications
    # ============================================================
    log_header "Starting Frontend Applications"
    
    # Inferstructor Dashboard
    start_inferstructor_dashboard || log_warn "Inferstructor Dashboard not started"
    sleep 2

    # Solana DEX (apps/next-solana-main) - Main trading platform with X3 Exchange (TEMPORARILY DISABLED)
    # start_nextjs_app "solana-dex" "$PROJECT_ROOT/apps/next-solana-main" $SOLANA_DEX_PORT
    # sleep 2

    # Explorer with X3OS
    start_nextjs_app "explorer" "$PROJECT_ROOT/apps/explorer" $X3OS_PORT
    sleep 2

    # Wallet
    start_nextjs_app "wallet" "$PROJECT_ROOT/apps/wallet" $WALLET_PORT
    sleep 2

    # DEX
    start_nextjs_app "dex" "$PROJECT_ROOT/apps/dex" $DEX_PORT
    sleep 1

    # ============================================================
    # Layer 4: Desktop (Tauri)
    # ============================================================
    start_x3_desktop || log_warn "X3 Desktop not started"
    start_x3os_tauri || log_warn "X3OS Tauri desktop not started"

    # Quantum Dashboard (Tauri)
    start_quantum_apps/dash-legacy-2-legacy-2board || log_warn "Quantum Dashboard not started"

    # ============================================================
    # Layer 5: Networking / Ingress (Cloudflare Tunnel)
    # ============================================================
    log_header "Starting Networking & Ingress"

    # Placeholder server handles all undeployed subdomains on :7000
    start_placeholder_server || log_warn "Placeholder server not started"

    # Cloudflare Tunnel exposes local services to x3star.net
    start_cloudflare_tunnel || log_warn "Cloudflare Tunnel not started (local-only mode)"

    # ============================================================
    # Final Status
    # ============================================================
    log_header "Startup Complete"

    echo ""
    echo -e "${GREEN}🎉 All services are starting! Access points:${NC}"
    echo ""
    echo -e "  🖥️  ${CYAN}X3 Desktop:${NC}       http://localhost:$X3_DESKTOP_PORT (Tauri)"
    echo -e "  🖥️  ${CYAN}X3OS Desktop:${NC}        http://localhost:$X3OS_PORT/x3os"
    echo -e "  📊 ${CYAN}Explorer Dashboard:${NC}  http://localhost:$X3OS_PORT"
    echo -e "  💱 ${CYAN}Solana DEX (X3):${NC}     http://localhost:$SOLANA_DEX_PORT"
    echo -e "  💰 ${CYAN}Wallet:${NC}              http://localhost:$WALLET_PORT"
    echo -e "  🔄 ${CYAN}DEX:${NC}                 http://localhost:$DEX_PORT"
    echo -e "  🔄 ${CYAN}DEX + Atomic Swaps:${NC}   http://localhost:$DEX_PORT (with HTLC)"
    echo -e "  ✨ ${CYAN}Quantum Dashboard:${NC}   http://localhost:$QUANTUM_DASHBOARD_PORT"
    echo -e "  🐝 ${CYAN}Swarm API:${NC}           http://localhost:$SWARM_API_PORT"
    echo -e "  🔐 ${CYAN}HTLC Coordinator:${NC}    http://localhost:$HTLC_COORDINATOR_PORT"
    echo -e "  🤖 ${CYAN}LLM Router:${NC}          http://localhost:$LLM_ROUTER_PORT"
    echo -e "  🔒 ${CYAN}Validator Registry:${NC}  http://localhost:$VALIDATOR_REGISTRY_PORT"
    echo -e "  🌉 ${CYAN}TPS Bridge:${NC}          http://localhost:$TPS_BRIDGE_PORT"
    echo -e "  📊 ${CYAN}Inferstructor:${NC}      http://localhost:$INFERSTRUCTOR_DASHBOARD_PORT"
    echo -e "  🤖 ${CYAN}Ollama AI:${NC}           ${OLLAMA_URL}"
    echo -e "  ⛓️  ${CYAN}Blockchain:${NC}          ws://localhost:$BLOCKCHAIN_PORT"
    echo -e "  🌐 ${CYAN}Cloudflare Tunnel:${NC}   https://$CLOUDFLARE_DOMAIN"
    echo -e "  📡 ${CYAN}Placeholder:${NC}         http://localhost:$PLACEHOLDER_PORT (subdomains)"
    echo ""
    echo -e "${YELLOW}Quick Links:${NC}"
    echo -e "  • X3 Desktop:       http://localhost:$X3_DESKTOP_PORT (Tauri app)"
    echo -e "  • X3OS:                http://localhost:$X3OS_PORT/x3os"
    echo -e "  • Swarm Dashboard:     http://localhost:$X3OS_PORT/x3/swarm"
    echo -e "  • GPU Contributors:    http://localhost:$X3OS_PORT/x3/swarm/gpu"
    echo -e "  • AI Swarm:            http://localhost:$X3OS_PORT/ai-swarm"
    echo -e "  • Quantum Advisor:     http://localhost:$QUANTUM_DASHBOARD_PORT"
    echo -e "  • Metrics:             http://localhost:$SWARM_API_PORT/metrics"
    echo -e "  • x3star.net:          https://$CLOUDFLARE_DOMAIN (Cloudflare Tunnel)"
    echo ""
    echo -e "${YELLOW}Logs:${NC} $PROJECT_ROOT/logs/"
    echo ""
    echo -e "Press ${RED}Ctrl+C${NC} to stop all services."
    echo ""

    if [ "$DETACH" -eq 1 ]; then
        echo -e "${GREEN}[SUCCESS]${NC} Detach mode enabled: launcher exiting, services remain running."
        echo -e "${YELLOW}Tip:${NC} Use '$0 --status' to check and '$0 --stop' to stop."
        return 0
    fi

    # Handle cleanup on exit (interactive mode)
    trap cleanup_pids EXIT INT TERM

    # Keep script running
    wait
}

# ============================================================
# Command Line Options
# ============================================================

case "${1:-}" in
    --detach|--daemon)
        DETACH=1
        main
        ;;
    --stop)
        log_info "Stopping all X3 Chain services..."
        cleanup_pids
        for port in $OLLAMA_PORT $BLOCKCHAIN_PORT $SWARM_API_PORT $X3OS_PORT $WALLET_PORT $DEX_PORT $SOLANA_DEX_PORT $X3_DESKTOP_PORT $LLM_ROUTER_PORT $VALIDATOR_REGISTRY_PORT $TPS_BRIDGE_PORT $INFERSTRUCTOR_DASHBOARD_PORT $HTLC_COORDINATOR_PORT; do
            kill_port $port
        done
        # Stop Ollama systemd if running
        sudo systemctl stop ollama 2>/dev/null || true
        log_success "All services stopped"
        ;;
    --status)
        show_status
        ;;
    --restart)
        log_info "Restarting all services..."
        $0 --stop
        sleep 2
        $0
        ;;
    --help|-h)
        echo ""
        echo -e "${CYAN}X3 Chain - Run Everything${NC}"
        echo ""
        echo "Usage: $0 [OPTION]"
        echo ""
        echo "Options:"
        echo "  (none)      Start all services"
        echo "  --stop      Stop all running services"
        echo "  --status    Show status of all services"
        echo "  --restart   Restart all services"
        echo "  --detach    Start services and exit (do not wait)"
        echo "  --strict    Fail fast if critical services (blockchain, swarm) fail to become ready"
        echo "  --help      Show this help message"
        echo ""
        echo "Services Started:"
        echo "  • Ollama (GPU/AI)            - Port $OLLAMA_PORT"
        echo "  • Blockchain Node            - Port $BLOCKCHAIN_PORT"
        echo "  • Swarm API Server           - Port $SWARM_API_PORT"
        echo "  • LLM Router                 - Port $LLM_ROUTER_PORT"
        echo "  • Validator Registry         - Port $VALIDATOR_REGISTRY_PORT"
        echo "  • TPS Bridge                 - Port $TPS_BRIDGE_PORT"
        echo "  • Inferstructor Dashboard   - Port $INFERSTRUCTOR_DASHBOARD_PORT"
        echo "  • HTLC Coordinator           - Port $HTLC_COORDINATOR_PORT"
        echo "  • Solana DEX                 - Port $SOLANA_DEX_PORT"
        echo "  • Explorer + X3OS            - Port $X3OS_PORT"
        echo "  • Wallet                     - Port $WALLET_PORT"
        echo "  • DEX                        - Port $DEX_PORT"
        echo "  • Quantum Dashboard          - Port $QUANTUM_DASHBOARD_PORT"
        echo "  • X3 Desktop              - Port $X3_DESKTOP_PORT"
        echo ""
        ;;
    *)
        main
        ;;
esac
