#!/bin/bash
# Live block monitor - displays full block info with visualization
# Captures and displays: block number, hash, status, timestamp, and full log line

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOG_FILE="${1:-.x3-dev.log}"

# Colors
BOLD='\033[1m'
CYAN='\033[96m'
GREEN='\033[92m'
YELLOW='\033[93m'
RESET='\033[0m'

display_block_info() {
    local line="$1"
    local block_num="$2"
    
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
    echo -e "${GREEN}${line}${RESET}"
    echo ""
    python3 "$SCRIPT_DIR/block_display.py" "$block_num" 2>/dev/null || true
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
    echo ""
}

# If running with pipe input, read from stdin
if [ ! -t 0 ]; then
    while IFS= read -r line; do
        block_num=""
        
        # Try all known x3-chain log formats
        [[ $line =~ Block\ imported:\ \#([0-9]+) ]] && block_num="${BASH_REMATCH[1]}"
        [[ -z "$block_num" && $line =~ Imported\ \#([0-9]+) ]] && block_num="${BASH_REMATCH[1]}"
        [[ -z "$block_num" && $line =~ Block\ finalized:\ \#([0-9]+) ]] && block_num="${BASH_REMATCH[1]}"
        [[ -z "$block_num" && $line =~ finalized\ \#([0-9]+) ]] && block_num="${BASH_REMATCH[1]}"
        [[ -z "$block_num" && $line =~ proposing\ at\ ([0-9]+) ]] && block_num="${BASH_REMATCH[1]}"
        [[ -z "$block_num" && $line =~ block:\ \#([0-9]+) ]] && block_num="${BASH_REMATCH[1]}"
        
        if [[ ! -z "$block_num" ]]; then
            display_block_info "$line" "$block_num"
        fi
    done
else
    # Monitor the log file directly
    if [ -f "$LOG_FILE" ]; then
        echo -e "${YELLOW}Monitoring: $LOG_FILE${RESET}"
        echo "Press Ctrl+C to stop"
        echo ""
        
        tail -f "$LOG_FILE" 2>/dev/null | while IFS= read -r line; do
            block_num=""
            
            # Try all known x3-chain log formats
            [[ $line =~ Block\ imported:\ \#([0-9]+) ]] && block_num="${BASH_REMATCH[1]}"
            [[ -z "$block_num" && $line =~ Imported\ \#([0-9]+) ]] && block_num="${BASH_REMATCH[1]}"
            [[ -z "$block_num" && $line =~ Block\ finalized:\ \#([0-9]+) ]] && block_num="${BASH_REMATCH[1]}"
            [[ -z "$block_num" && $line =~ finalized\ \#([0-9]+) ]] && block_num="${BASH_REMATCH[1]}"
            [[ -z "$block_num" && $line =~ proposing\ at\ ([0-9]+) ]] && block_num="${BASH_REMATCH[1]}"
            [[ -z "$block_num" && $line =~ block:\ \#([0-9]+) ]] && block_num="${BASH_REMATCH[1]}"
            
            if [[ ! -z "$block_num" ]]; then
                display_block_info "$line" "$block_num"
            fi
        done
    else
        echo "Error: Log file not found: $LOG_FILE"
        echo "Usage: bash monitor_blocks.sh [LOG_FILE]"
        echo "Or pipe logs: tail -f .x3-dev.log | bash monitor_blocks.sh"
        exit 1
    fi
fi

