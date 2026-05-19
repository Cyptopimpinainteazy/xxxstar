#!/usr/bin/env bash
# Measures RPC latency across listed endpoints.

if [ -z "$RPC_LIST" ]; then
    echo "Error: RPC_LIST env var not set"
    exit 1
fi

IFS=',' read -ra ADDR <<< "$RPC_LIST"
for url in "${ADDR[@]}"; do
    echo -n "Benchmarking $url... "
    start=$(date +%s%N)
    # Corrected curl check
    HTTP_CODE=$(curl -s -X POST -H "Content-Type: application/json" \
        --data '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' \
        -w "%{http_code}" -o /dev/null "$url")
    
    end=$(date +%s%N)
    
    if [ "$HTTP_CODE" -eq 200 ]; then
        elapsed=$(( (end - start) / 1000000 ))
        echo "Latency: ${elapsed}ms"
    else
        echo "Failed (HTTP $HTTP_CODE)"
    fi
done
