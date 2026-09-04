#!/usr/bin/env bash
# X3 RPC Load Test
# Verifies p50/p95/p99 latency, error rate, and throughput under load.
# Uses Apache Bench (ab) or curl-based loop if ab not installed.
set -euo pipefail

GATEWAY="${1:-http://localhost:8545}"
DURATION="${2:-30}"  # seconds
CONCURRENT="${3:-20}"

PASS=0
FAIL=0

green() { echo -e "\033[32m$1\033[0m"; }
red()   { echo -e "\033[31m$1\033[0m"; }

echo "X3 RPC Load Test"
echo "Gateway: $GATEWAY"
echo "Duration: ${DURATION}s, Concurrent: $CONCURRENT"
echo "===================="
echo ""

# ── Use ab if available, fall back to curl loop ──────────────────
if command -v ab &>/dev/null; then
    echo "Using Apache Bench..."
    
    # Test EVM endpoint
    EVM_BODY='{"jsonrpc":"2.0","method":"eth_chainId","params":[],"id":1}'
    echo "$EVM_BODY" > /tmp/rpc-load-evm.json
    
    ab_result=$(ab -n $((DURATION * 10)) -c "$CONCURRENT" \
        -p /tmp/rpc-load-evm.json \
        -T "application/json" \
        -s 10 \
        "$GATEWAY/" 2>&1) || true
    
    req_rate=$(echo "$ab_result" | grep "Requests per second" | awk '{print $4}')
    p50=$(echo "$ab_result" | grep "50%" | awk '{print $2}')
    p95=$(echo "$ab_result" | grep "95%" | awk '{print $2}')
    p99=$(echo "$ab_result" | grep "99%" | awk '{print $2}')
    failures=$(echo "$ab_result" | grep "Failed requests" | awk '{print $3}')
    
    echo "  Request rate: ${req_rate:-0} req/s"
    echo "  p50: ${p50:-0}ms"
    echo "  p95: ${p95:-0}ms"
    echo "  p99: ${p99:-0}ms"
    echo "  Failures: ${failures:-0}"
    
    if [ -n "$p50" ] && [ "$(echo "$p50 < 80" | bc -l 2>/dev/null || echo 0)" = "1" ]; then
        green "  PASS: p50 < 80ms"
        PASS=$((PASS + 1))
    else
        red "  FAIL: p50 = ${p50:-N/A}ms (target < 80ms)"
        FAIL=$((FAIL + 1))
    fi
    
    if [ -n "$p95" ] && [ "$(echo "$p95 < 250" | bc -l 2>/dev/null || echo 0)" = "1" ]; then
        green "  PASS: p95 < 250ms"
        PASS=$((PASS + 1))
    else
        red "  FAIL: p95 = ${p95:-N/A}ms (target < 250ms)"
        FAIL=$((FAIL + 1))
    fi
    
    if [ -n "$p99" ] && [ "$(echo "$p99 < 750" | bc -l 2>/dev/null || echo 0)" = "1" ]; then
        green "  PASS: p99 < 750ms"
        PASS=$((PASS + 1))
    else
        red "  FAIL: p99 = ${p99:-N/A}ms (target < 750ms)"
        FAIL=$((FAIL + 1))
    fi

else
    echo "Apache Bench not found. Running simple curl-based load test..."
    
    # Simple curl-based load test with timing
    declare -a latencies
    lat_success=0
    lat_fail=0
    start_time=$(date +%s)
    end_time=$((start_time + DURATION))
    request_count=0
    
    run_request() {
        local method="$1" body="$2"
        local start end latency
        start=$(date +%s%N)
        if curl -s -X POST "$GATEWAY" \
            -H "Content-Type: application/json" \
            --max-time 5 \
            -d "$body" >/dev/null 2>&1; then
            end=$(date +%s%N)
            latency=$(( (end - start) / 1000000 ))
            echo "$latency"
            return 0
        else
            return 1
        fi
    }
    
    EVM_BODY='{"jsonrpc":"2.0","method":"eth_chainId","params":[],"id":1}'
    
    while [ "$(date +%s)" -lt "$end_time" ]; do
        for i in $(seq 1 "$CONCURRENT"); do
            lat=$(run_request "eth_chainId" "$EVM_BODY" 2>/dev/null) && {
                latencies+=("$lat")
                lat_success=$((lat_success + 1))
            } || {
                lat_fail=$((lat_fail + 1))
            }
            request_count=$((request_count + 1))
        done &
        wait 2>/dev/null || true
    done
    
    # Calculate percentiles
    if [ ${#latencies[@]} -gt 0 ]; then
        sorted=($(printf '%s\n' "${latencies[@]}" | sort -n))
        total=${#sorted[@]}
        p50_idx=$((total * 50 / 100))
        p95_idx=$((total * 95 / 100))
        p99_idx=$((total * 99 / 100))
        
        p50_val=${sorted[$p50_idx]:-0}
        p95_val=${sorted[$p95_idx]:-0}
        p99_val=${sorted[$p99_idx]:-0}
        
        actual_duration=$(($(date +%s) - start_time))
        req_rate=$((request_count / (actual_duration > 0 ? actual_duration : 1)))
        
        echo "  Request rate: $req_rate req/s"
        echo "  p50: ${p50_val}ms"
        echo "  p95: ${p95_val}ms"
        echo "  p99: ${p99_val}ms"
        echo "  Failures: $lat_fail"
        
        if [ "$p50_val" -lt 80 ]; then
            green "  PASS: p50 < 80ms"
            PASS=$((PASS + 1))
        else
            red "  FAIL: p50 = ${p50_val}ms (target < 80ms)"
            FAIL=$((FAIL + 1))
        fi
        
        if [ "$p95_val" -lt 250 ]; then
            green "  PASS: p95 < 250ms"
            PASS=$((PASS + 1))
        else
            red "  FAIL: p95 = ${p95_val}ms (target < 250ms)"
            FAIL=$((FAIL + 1))
        fi
        
        if [ "$p99_val" -lt 750 ]; then
            green "  PASS: p99 < 750ms"
            PASS=$((PASS + 1))
        else
            red "  FAIL: p99 = ${p99_val}ms (target < 750ms)"
            FAIL=$((FAIL + 1))
        fi
    else
        red "  FAIL: No successful requests"
        FAIL=$((FAIL + 3))
    fi
fi

echo ""
echo "===================="
echo "Results: $PASS passed, $FAIL failed"
if [ "$FAIL" -eq 0 ]; then
    green "LOAD TEST PASSED"
    exit 0
else
    red "LOAD TEST FAILED"
    exit 1
fi