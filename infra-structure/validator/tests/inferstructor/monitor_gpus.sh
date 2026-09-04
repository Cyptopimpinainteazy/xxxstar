#!/bin/bash
# Real-time GPU monitoring during load test

echo "🔍 GPU ACCELERATION MONITOR"
echo "Monitoring all 3 GPUs + Lane Services"
echo "Press Ctrl+C to stop"
echo ""

while true; do
    clear
    echo "═══════════════════════════════════════════════════════════════════"
    echo "   🎮 GPU HARDWARE STATUS - $(date '+%H:%M:%S')"
    echo "═══════════════════════════════════════════════════════════════════"
    nvidia-smi --query-gpu=index,name,utilization.gpu,utilization.memory,memory.used,memory.total,temperature.gpu,power.draw --format=csv,noheader,nounits | \
        awk -F', ' '{printf "GPU %s (%s):\n  Compute: %s%% | Memory: %s%% (%s/%s MB) | Temp: %s°C | Power: %sW\n\n", $1, $2, $3, $4, $5, $6, $7, $8}'
    
    echo "═══════════════════════════════════════════════════════════════════"
    echo "   ⚡ GPU LANE SERVICES"
    echo "═══════════════════════════════════════════════════════════════════"
    
    for port in 9001 9002 9003; do
        lane_name=$([ $port -eq 9001 ] && echo "Primary  (GPU 0)" || ([ $port -eq 9002 ] && echo "Shadow   (GPU 1)" || echo "Tertiary (GPU 2)"))
        
        response=$(curl -s http://localhost:$port/health 2>/dev/null)
        if [ $? -eq 0 ]; then
            status=$(echo "$response" | python3 -c "import sys, json; d=json.load(sys.stdin); print(d.get('status', 'unknown'))" 2>/dev/null)
            requests=$(echo "$response" | python3 -c "import sys, json; d=json.load(sys.stdin); print(d['stats'].get('total_requests', 0))" 2>/dev/null)
            success=$(echo "$response" | python3 -c "import sys, json; d=json.load(sys.stdin); print(d['stats'].get('total_success', 0))" 2>/dev/null)
            failed=$(echo "$response" | python3 -c "import sys, json; d=json.load(sys.stdin); print(d['stats'].get('total_failed', 0))" 2>/dev/null)
            success_rate=$(echo "$response" | python3 -c "import sys, json; d=json.load(sys.stdin); print(f\"{d['stats'].get('success_rate', 0)*100:.1f}\")" 2>/dev/null)
            
            status_icon="✅"
            [ "$status" = "degraded" ] && status_icon="⚠️ "
            [ "$status" = "unhealthy" ] && status_icon="❌"
            
            echo "$lane_name (port $port): $status_icon $status"
            echo "  Requests: $requests | Success: $success | Failed: $failed | Rate: ${success_rate}%"
        else
            echo "$lane_name (port $port): ❌ OFFLINE"
        fi
        echo ""
    done
    
    echo "═══════════════════════════════════════════════════════════════════"
    echo "   🌉 TPS BRIDGE"
    echo "═══════════════════════════════════════════════════════════════════"
    bridge_response=$(curl -s http://localhost:9999/stats 2>/dev/null)
    if [ $? -eq 0 ]; then
        echo "$bridge_response" | python3 -c "
import sys, json
try:
    d = json.load(sys.stdin)
    print(f\"  Status: ✅ Online\")
    print(f\"  Validators: {d.get('total_validators', '?')}\")
    print(f\"  Total TPS: {d.get('total_tps', '?'):,}\")
except:
    print('  Status: ⚠️  Parse error')
" 2>/dev/null || echo "  Status: ⚠️  Error parsing response"
    else
        echo "  Status: ❌ OFFLINE"
    fi
    
    echo ""
    echo "Refreshing every 2 seconds... (Ctrl+C to stop)"
    sleep 2
done
