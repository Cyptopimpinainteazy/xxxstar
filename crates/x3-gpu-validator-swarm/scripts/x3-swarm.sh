#!/bin/bash
# X3 GPU Validator Swarm - One-Command Onboarding Script
# 
# This script provides one-command install/run/join/bench flows
# for the X3 GPU Validator Swarm.

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
COMMAND=""
VALIDATOR_ID=""
ORCHESTRATOR_URL=""
GPU_ENABLED=true
CPU_ONLY=false
WALLET_ADDRESS=""
STAKE_AMOUNT=1000000

# Print colored message
print_msg() {
    local color=$1
    local msg=$2
    echo -e "${color}${msg}${NC}"
}

# Print usage
usage() {
    echo "X3 GPU Validator Swarm - Easy Onboarding"
    echo ""
    echo "Usage: $0 <command> [options]"
    echo ""
    echo "Commands:"
    echo "  install    - Install dependencies and build"
    echo "  run        - Run the validator"
    echo "  join       - Join the swarm as a provider"
    echo "  bench      - Run benchmarks"
    echo "  status     - Show swarm status"
    echo "  test       - Run tests"
    echo "  wallet-sync - Sync wallet with GPU (become a provider)"
    echo ""
    echo "Options:"
    echo "  --validator-id <id>   Set validator ID"
    echo "  --orchestrator <url>  Set orchestrator URL"
    echo "  --cpu-only           Run in CPU-only mode"
    echo "  --wallet <address>   Set wallet address for payments"
    echo "  --stake <amount>     Set stake amount (default: 1000000)"
    echo "  --help               Show this help"
    echo ""
    echo "Examples:"
    echo "  $0 install"
    echo "  $0 run --validator-id my-validator"
    echo "  $0 join --orchestrator https://orchestrator.x3.io --wallet 0x1234"
    echo "  $0 wallet-sync --wallet 0x1234 --stake 1000000"
    echo "  $0 bench"
}

# Install dependencies
cmd_install() {
    print_msg "$BLUE" "Installing X3 GPU Validator Swarm..."
    
    # Check Rust
    if ! command -v cargo &> /dev/null; then
        print_msg "$RED" "Error: Rust is not installed"
        print_msg "$YELLOW" "Please install Rust: https://rustup.rs/"
        exit 1
    fi
    
    print_msg "$GREEN" "✓ Rust found"
    
    # Check CUDA (optional)
    if command -v nvcc &> /dev/null; then
        print_msg "$GREEN" "✓ CUDA found"
        GPU_ENABLED=true
    else
        print_msg "$YELLOW" "⚠ CUDA not found - will run in CPU mode"
        GPU_ENABLED=false
    fi
    
    # Build the project
    print_msg "$BLUE" "Building X3 GPU Validator Swarm..."
    cd "$(dirname "$0")/.."
    
    if [ "$CPU_ONLY" = true ]; then
        cargo build --release --no-default-features
    else
        cargo build --release
    fi
    
    print_msg "$GREEN" "✓ Build complete"
    print_msg "$GREEN" "✓ Installation complete!"
    echo ""
    echo "Next steps:"
    echo "  $0 run          - Run the validator"
    echo "  $0 wallet-sync  - Become a provider (with wallet)"
    echo "  $0 bench        - Run benchmarks"
}

# Run the validator
cmd_run() {
    print_msg "$BLUE" "Starting X3 GPU Validator..."
    
    cd "$(dirname "$0")/.."
    
    # Set validator ID if not provided
    if [ -z "$VALIDATOR_ID" ]; then
        VALIDATOR_ID="validator-$(date +%s)"
    fi
    
    print_msg "$GREEN" "Validator ID: $VALIDATOR_ID"
    
    # Run validator
    cargo run --release --bin x3-validator -- run --validator-id "$VALIDATOR_ID"
}

# Join the swarm (as provider)
cmd_join() {
    print_msg "$BLUE" "Joining X3 GPU Validator Swarm as Provider..."
    
    cd "$(dirname "$0")/.."
    
    # Set validator ID if not provided
    if [ -z "$VALIDATOR_ID" ]; then
        VALIDATOR_ID="validator-$(date +%s)"
    fi
    
    print_msg "$GREEN" "Provider ID: $VALIDATOR_ID"
    
    # Check for wallet
    if [ -z "$WALLET_ADDRESS" ]; then
        print_msg "$YELLOW" "⚠ No wallet address provided"
        print_msg "$YELLOW" "Use --wallet <address> to set your wallet for payments"
        echo ""
        echo "To become a provider with payment:"
        echo "  $0 join --wallet 0xYourWalletAddress --stake 1000000"
        return
    fi
    
    print_msg "$GREEN" "Wallet: $WALLET_ADDRESS"
    print_msg "$GREEN" "Stake: $STAKE_AMOUNT X3"
    
    if [ -n "$ORCHESTRATOR_URL" ]; then
        print_msg "$GREEN" "Orchestrator URL: $ORCHESTRATOR_URL"
    fi
    
    # Run wallet sync first
    print_msg "$BLUE" "Running wallet sync..."
    WALLET_ADDRESS="$WALLET_ADDRESS" cargo run --release --bin wallet-sync -- register
    
    # Run validator in join mode
    cargo run --release --bin x3-validator -- run --validator-id "$VALIDATOR_ID" --orchestrator "$ORCHESTRATOR_URL"
}

# Wallet GPU Sync (become provider)
cmd_wallet_sync() {
    print_msg "$BLUE" "Starting Wallet GPU Sync..."
    echo ""
    echo "╔════════════════════════════════════════════════════╗"
    echo "║     X3 GPU Provider Registration Flow             ║"
    echo "╚════════════════════════════════════════════════════╝"
    echo ""
    
    cd "$(dirname "$0")/.."
    
    # Step 1: Detect GPU
    print_msg "$BLUE" "Step 1: Detecting GPU..."
    cargo run --release --bin wallet-sync -- detect
    
    if [ $? -ne 0 ]; then
        print_msg "$RED" "GPU detection failed"
        return 1
    fi
    
    # Step 2: Run benchmark
    print_msg "$BLUE" "\nStep 2: Running benchmark..."
    cargo run --release --bin wallet-sync -- benchmark
    
    # Step 3: Register
    print_msg "$BLUE" "\nStep 3: Registering as provider..."
    
    # Check for wallet
    if [ -z "$WALLET_ADDRESS" ]; then
        print_msg "$YELLOW" "Enter your wallet address:"
        read -p "> " WALLET_ADDRESS
    fi
    
    export WALLET_ADDRESS
    export STAKE_AMOUNT
    
    cargo run --release --bin wallet-sync -- register
    
    # Step 4: Start node
    print_msg "$BLUE" "\nStep 4: Starting validator node..."
    print_msg "$GREEN" "✓ You are now a provider!"
    echo ""
    echo "Your GPU will now participate in the swarm."
    echo "Rewards will be paid to: $WALLET_ADDRESS"
    echo ""
    echo "View your status:"
    echo "  cargo run --release --bin wallet-sync -- status"
}

# Run benchmarks
cmd_bench() {
    print_msg "$BLUE" "Running X3 GPU Validator Benchmarks..."
    
    cd "$(dirname "$0")/.."
    
    cargo run --release --bin x3-bench -- run
    
    print_msg "$GREEN" "✓ Benchmarks complete!"
    echo ""
    echo "Results saved to: benchmark-results.json"
    echo ""
    cargo run --release --bin x3-bench -- report
}

# Show status
cmd_status() {
    print_msg "$BLUE" "X3 GPU Validator Swarm Status"
    echo "================================"
    echo ""
    
    cd "$(dirname "$0")/.."
    
    print_msg "$BLUE" "Validator Status:"
    cargo run --release --bin x3-validator -- status || true
    
    echo ""
    print_msg "$BLUE" "Payment Status:"
    cargo run --release --bin wallet-sync -- status || true
}

# Run tests
cmd_test() {
    print_msg "$BLUE" "Running X3 GPU Validator Tests..."
    
    cd "$(dirname "$0")/.."
    
    cargo test --release
    
    print_msg "$GREEN" "✓ All tests passed!"
}

# Parse arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            install)
                COMMAND="install"
                shift
                ;;
            run)
                COMMAND="run"
                shift
                ;;
            join)
                COMMAND="join"
                shift
                ;;
            wallet-sync)
                COMMAND="wallet-sync"
                shift
                ;;
            bench)
                COMMAND="bench"
                shift
                ;;
            status)
                COMMAND="status"
                shift
                ;;
            test)
                COMMAND="test"
                shift
                ;;
            --validator-id)
                VALIDATOR_ID="$2"
                shift 2
                ;;
            --orchestrator)
                ORCHESTRATOR_URL="$2"
                shift 2
                ;;
            --cpu-only)
                CPU_ONLY=true
                shift
                ;;
            --wallet)
                WALLET_ADDRESS="$2"
                shift 2
                ;;
            --stake)
                STAKE_AMOUNT="$2"
                shift 2
                ;;
            --help|-h)
                usage
                exit 0
                ;;
            *)
                print_msg "$RED" "Unknown option: $1"
                usage
                exit 1
                ;;
        esac
    done
}

# Main
main() {
    if [ $# -eq 0 ]; then
        usage
        exit 1
    fi
    
    parse_args "$@"
    
    case $COMMAND in
        install)
            cmd_install
            ;;
        run)
            cmd_run
            ;;
        join)
            cmd_join
            ;;
        wallet-sync)
            cmd_wallet_sync
            ;;
        bench)
            cmd_bench
            ;;
        status)
            cmd_status
            ;;
        test)
            cmd_test
            ;;
        *)
            print_msg "$RED" "Unknown command: $COMMAND"
            usage
            exit 1
            ;;
    esac
}

main "$@"
