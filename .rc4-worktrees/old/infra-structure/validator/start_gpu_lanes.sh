#!/usr/bin/env bash
# Start Real GPU Acceleration Infrastructure
# Launches 3 GPU lanes (one per GTX 1070) + Real Toll Booth

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "🚀 Starting REAL GPU Acceleration Infrastructure"
echo "=================================================="
echo ""

# Check GPUs
echo "🔍 Detecting GPUs..."
if ! command -v nvidia-smi &> /dev/null; then
    echo "❌ nvidia-smi not found. Is NVIDIA driver installed?"
    exit 1
fi

GPU_COUNT=$(nvidia-smi --list-gpus | wc -l)
echo "✅ Found $GPU_COUNT GPUs:"
nvidia-smi --query-gpu=index,name,memory.total --format=csv,noheader | while read line; do
    echo "   $line"
done
echo ""

# Check Python environment
if [ ! -d ".venv" ]; then
    echo "❌ Virtual environment not found. Run from cross-chain-gpu-validator/"
    exit 1
fi

source .venv/bin/activate

# Check CuPy
echo "🔍 Checking CUDA libraries..."
if ! python3 -c "import cupy" 2>/dev/null; then
    echo "⚠️  CuPy not installed. Installing..."
    pip install -q cupy-cuda11x
fi
echo "✅ CuPy ready"
echo ""

# Kill old processes
echo "🧹 Cleaning up old GPU processes..."
pkill -f "gpu_lane_service.py" 2>/dev/null || true
pkill -f "mock_toll_booth.py" 2>/dev/null || true
sleep 2

# Create logs directory
mkdir -p logs

echo "🎯 Launching GPU Lanes..."
echo "=================================================="

# GPU 0: Primary Lane (Port 9001)
echo "🟢 Starting PRIMARY Lane (GPU 0, Port 9001)..."
python3 tests/inferstructor/gpu_lane_service.py primary 0 9001 > logs/gpu_lane_primary.log 2>&1 &
PRIMARY_PID=$!
echo "   PID: $PRIMARY_PID"
sleep 2

# GPU 1: Shadow Lane (Port 9002)
echo "🟡 Starting SHADOW Lane (GPU 1, Port 9002)..."
python3 tests/inferstructor/gpu_lane_service.py shadow 1 9002 > logs/gpu_lane_shadow.log 2>&1 &
SHADOW_PID=$!
echo "   PID: $SHADOW_PID"
sleep 2

# GPU 2: Tertiary Lane (Port 9003)
echo "🔵 Starting TERTIARY Lane (GPU 2, Port 9003)..."
python3 tests/inferstructor/gpu_lane_service.py tertiary 2 9003 > logs/gpu_lane_tertiary.log 2>&1 &
TERTIARY_PID=$!
echo "   PID: $TERTIARY_PID"
sleep 2

echo ""
echo "🔍 Verifying GPU lanes..."

# Check Primary
if curl -sf http://localhost:9001/health > /dev/null 2>&1; then
    echo "✅ Primary Lane healthy"
else
    echo "❌ Primary Lane failed to start"
    cat logs/gpu_lane_primary.log
    exit 1
fi

# Check Shadow
if curl -sf http://localhost:9002/health > /dev/null 2>&1; then
    echo "✅ Shadow Lane healthy"
else
    echo "⚠️  Shadow Lane may not be ready"
fi

# Check Tertiary
if curl -sf http://localhost:9003/health > /dev/null 2>&1; then
    echo "✅ Tertiary Lane healthy"
else
    echo "⚠️  Tertiary Lane may not be ready"
fi

# Save PIDs
cat > .gpu_lanes.pids <<EOF
$PRIMARY_PID
$SHADOW_PID
$TERTIARY_PID
EOF

echo ""
echo "=================================================="
echo "✅ REAL GPU INFRASTRUCTURE RUNNING!"
echo "=================================================="
echo ""
echo "📊 GPU Lanes:"
echo "   🟢 Primary:   http://localhost:9001 (GPU 0)"
echo "   🟡 Shadow:    http://localhost:9002 (GPU 1)"
echo "   🔵 Tertiary:  http://localhost:9003 (GPU 2)"
echo ""
echo "📈 Metrics:"
echo "   Primary:   http://localhost:9001/metrics"
echo "   Shadow:    http://localhost:9002/metrics"
echo "   Tertiary:  http://localhost:9003/metrics"
echo ""
echo "=================================================="
echo "🧪 TEST YOUR GPUs:"
echo "=================================================="
echo ""
echo "# Test Primary Lane (GPU 0)"
echo "curl -X POST http://localhost:9001/accelerate \\"
echo "  -H 'Content-Type: application/json' \\"
echo "  -d '{\"tx_hash\":\"test1\",\"tx_data\":\"48656c6c6f\",\"chain\":\"solana\",\"validator_id\":\"test\"}'"
echo ""
echo "# Check GPU utilization"
echo "nvidia-smi"
echo ""
echo "# Monitor logs"
echo "tail -f logs/gpu_lane_*.log"
echo ""
echo "=================================================="
echo "🛑 To stop all GPU lanes:"
echo "=================================================="
echo "   kill \$(cat .gpu_lanes.pids)"
echo ""
