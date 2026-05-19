#!/bin/bash
set -euo pipefail

OUTPUT_PATH="${1:-benchmark_report.json}"

python -m cross_chain_gpu_validator.cli benchmark --output "${OUTPUT_PATH}"

echo "Benchmark report written to ${OUTPUT_PATH}"
