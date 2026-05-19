#!/bin/bash
#
# X3 Chain Testnet Management Script
#

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

SERVICES=("x3-bootnode" "x3-validator-01" "x3-validator-02" "x3-validator-03")

show_status() {
    echo -e "${BLUE}=== X3 Chain Testnet Status ===${NC}"
    echo ""
    for service in "${SERVICES[@]}"; do
        if sudo systemctl is-active --quiet "$service"; then
            status="${GREEN}RUNNING${NC}"
        else
            status="${RED}STOPPED${NC}"
        fi
        echo -e "$service: $status"
    done
    echo ""
    
    # Check block height
    if curl -s http://localhost:9944 > /dev/null 2>&1; then
        BLOCK=$(curl -s -H "Content-Type: application/json" \
            -d '{"id":1, "jsonrpc":"2.0", "method": "chain_getBlock"}' \
            http://localhost:9944 | jq -r '.result.block.header.number' 2>/dev/null)
        PEERS=$(curl -s -H "Content-Type: application/json" \
            -d '{"id":1, "jsonrpc":"2.0", "method": "system_health"}' \
            http://localhost:9944 | jq -r '.result.peers' 2>/dev/null)
        
        if [ -n "$BLOCK" ] && [ "$BLOCK" != "null" ]; then
            echo -e "Current Block: ${GREEN}$BLOCK${NC}"
            echo -e "Connected Peers: ${GREEN}$PEERS${NC}"
        fi
    fi
    echo ""
}

start_all() {
    echo -e "${YELLOW}Starting all services...${NC}"
    echo ""
    
    echo "Starting bootnode..."
    sudo systemctl start x3-bootnode
    sleep 3
    
    for i in 01 02 03; do
        echo "Starting validator-$i..."
        sudo systemctl start x3-validator-$i
        sleep 2
    done
    
    echo ""
    echo -e "${GREEN}All services started!${NC}"
    show_status
}

stop_all() {
    echo -e "${YELLOW}Stopping all services...${NC}"
    for service in "${SERVICES[@]}"; do
        sudo systemctl stop "$service"
    done
    echo -e "${GREEN}All services stopped!${NC}"
}

restart_all() {
    echo -e "${YELLOW}Restarting all services...${NC}"
    stop_all
    sleep 2
    start_all
}

show_logs() {
    service="${1:-x3-bootnode}"
    echo -e "${YELLOW}Showing logs for $service...${NC}"
    echo ""
    sudo journalctl -u "$service" -f
}

check_health() {
    echo -e "${BLUE}=== Health Check ===${NC}"
    echo ""
    
    # Check processes
    echo "Checking processes..."
    if pgrep -f x3-chain-node > /dev/null; then
        echo -e "${GREEN}✓ X3 nodes are running${NC}"
    else
        echo -e "${RED}✗ No X3 nodes found${NC}"
        return 1
    fi
    
    # Check RPC
    echo "Checking RPC endpoints..."
    for port in 9944 9945 9946 9947; do
        if curl -s http://localhost:$port > /dev/null 2>&1; then
            echo -e "${GREEN}✓ Port $port responding${NC}"
        else
            echo -e "${RED}✗ Port $port not responding${NC}"
        fi
    done
    
    # Check block production
    echo ""
    echo "Checking block production..."
    BLOCK1=$(curl -s -H "Content-Type: application/json" \
        -d '{"id":1, "jsonrpc":"2.0", "method": "chain_getBlock"}' \
        http://localhost:9944 | jq -r '.result.block.header.number' 2>/dev/null)
    
    sleep 15
    
    BLOCK2=$(curl -s -H "Content-Type: application/json" \
        -d '{"id":1, "jsonrpc":"2.0", "method": "chain_getBlock"}' \
        http://localhost:9944 | jq -r '.result.block.header.number' 2>/dev/null)
    
    if [ "$BLOCK2" -gt "$BLOCK1" ] 2>/dev/null; then
        echo -e "${GREEN}✓ Blocks being produced ($BLOCK1 → $BLOCK2)${NC}"
    else
        echo -e "${RED}✗ Block production stalled${NC}"
    fi
    
    echo ""
}

purge_data() {
    echo -e "${RED}WARNING: This will delete ALL blockchain data!${NC}"
    read -p "Are you sure? (type 'yes' to confirm): " confirm
    
    if [ "$confirm" != "yes" ]; then
        echo "Cancelled."
        return
    fi
    
    echo ""
    echo -e "${YELLOW}Stopping services...${NC}"
    stop_all
    
    echo -e "${YELLOW}Deleting data directories...${NC}"
    sudo rm -rf /var/lib/x3-chain/{bootnode,validator-01,validator-02,validator-03}/chains
    
    echo -e "${GREEN}Data purged!${NC}"
    echo "Run 'start' to begin fresh."
}

show_help() {
    echo "X3 Chain Testnet Management"
    echo ""
    echo "Usage: $0 [command]"
    echo ""
    echo "Commands:"
    echo "  status        Show service status and block height"
    echo "  start         Start all services"
    echo "  stop          Stop all services"
    echo "  restart       Restart all services"
    echo "  logs [name]   Show logs (default: bootnode)"
    echo "  health        Run health checks"
    echo "  purge         Delete all blockchain data (DANGEROUS!)"
    echo "  help          Show this help"
    echo ""
    echo "Examples:"
    echo "  $0 status"
    echo "  $0 logs x3-validator-01"
    echo "  $0 restart"
    echo ""
}

# Main
case "${1:-status}" in
    status)
        show_status
        ;;
    start)
        start_all
        ;;
    stop)
        stop_all
        ;;
    restart)
        restart_all
        ;;
    logs)
        show_logs "$2"
        ;;
    health)
        check_health
        ;;
    purge)
        purge_data
        ;;
    help|--help|-h)
        show_help
        ;;
    *)
        echo "Unknown command: $1"
        echo ""
        show_help
        exit 1
        ;;
esac
