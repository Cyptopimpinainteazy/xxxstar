#!/usr/bin/env bash
set -euo pipefail

# ==========================================================================
#  Launch Ollama × Hunyuan3D 2.1
#
#  Usage:
#    ./scripts/run-hunyuan3d-ollama.sh                     # Gradio UI (default)
#    ./scripts/run-hunyuan3d-ollama.sh api                  # REST API server
#    ./scripts/run-hunyuan3d-ollama.sh chat                 # CLI chat mode
#    ./scripts/run-hunyuan3d-ollama.sh status               # Quick health check
#    ./scripts/run-hunyuan3d-ollama.sh install              # Install deps
#
#  Environment variables (all optional):
#    OLLAMA_URL        default http://127.0.0.1:11434
#    OLLAMA_MODEL      default qwen2.5-coder:14b
#    HUNYUAN_MODEL     default tencent/Hunyuan3D-2.1
#    DEVICE            default cuda
#    PORT              default 8083 (Gradio) / 8082 (API)
# ==========================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
H3D_DIR="$REPO_ROOT/third_party/Hunyuan3D-2.1"
if [[ ! -d "$H3D_DIR" ]]; then
    H3D_DIR="$REPO_ROOT/Hunyuan3D-2.1"
fi
if [[ ! -d "$H3D_DIR" ]]; then
    echo "Hunyuan3D-2.1 not found. Expected $REPO_ROOT/third_party/Hunyuan3D-2.1"
    exit 1
fi

OLLAMA_URL="${OLLAMA_URL:-http://127.0.0.1:11434}"
OLLAMA_MODEL="${OLLAMA_MODEL:-qwen2.5-coder:14b}"
HUNYUAN_MODEL="${HUNYUAN_MODEL:-tencent/Hunyuan3D-2.1}"
DEVICE="${DEVICE:-cuda}"
GRADIO_PORT="${PORT:-8083}"
API_PORT="${PORT:-8082}"
LOW_VRAM="${LOW_VRAM:-}"

# Activate venv if present
if [[ -f "$REPO_ROOT/.venv-2/bin/activate" ]]; then
    # shellcheck disable=SC1091
    source "$REPO_ROOT/.venv-2/bin/activate"
elif [[ -f "$H3D_DIR/.venv/bin/activate" ]]; then
    # shellcheck disable=SC1091
    source "$H3D_DIR/.venv/bin/activate"
fi

# Build extra flags
EXTRA_ARGS=()
if [[ -n "$LOW_VRAM" ]]; then
    EXTRA_ARGS+=("--low-vram")
fi

# ---- Commands -----------------------------------------------------------

cmd_install() {
    echo "=== Installing Hunyuan3D-2.1 + Ollama bridge dependencies ==="
    cd "$H3D_DIR"

    # Core PyTorch (CUDA 12.4)
    pip install torch==2.5.1 torchvision==0.20.1 torchaudio==2.5.1 \
        --index-url https://download.pytorch.org/whl/cu124

    # Hunyuan3D requirements
    pip install -r requirements.txt

    # Ollama bridge extra deps (requests is usually bundled but be safe)
    pip install requests

    # Custom rasterizer
    if [[ -d "hy3dpaint/custom_rasterizer" ]]; then
        echo "Building custom_rasterizer..."
        cd hy3dpaint/custom_rasterizer
        pip install -e .
        cd "$H3D_DIR"
    fi

    # DifferentiableRenderer
    if [[ -f "hy3dpaint/DifferentiableRenderer/compile_mesh_painter.sh" ]]; then
        echo "Building DifferentiableRenderer..."
        cd hy3dpaint/DifferentiableRenderer
        bash compile_mesh_painter.sh
        cd "$H3D_DIR"
    fi

    # RealESRGAN weights
    mkdir -p hy3dpaint/ckpt
    if [[ ! -f "hy3dpaint/ckpt/RealESRGAN_x4plus.pth" ]]; then
        echo "Downloading RealESRGAN weights..."
        wget -q https://github.com/xinntao/Real-ESRGAN/releases/download/v0.1.0/RealESRGAN_x4plus.pth \
            -P hy3dpaint/ckpt || echo "Warning: failed to download RealESRGAN weights"
    fi

    echo "=== Installation complete ==="
}

cmd_status() {
    cd "$H3D_DIR"
    python3 ollama_bridge.py \
        --ollama-url "$OLLAMA_URL" \
        --ollama-model "$OLLAMA_MODEL" \
        status
}

cmd_gradio() {
    echo "Starting Ollama × Hunyuan3D Gradio UI on port $GRADIO_PORT ..."
    cd "$H3D_DIR"
    python3 ollama_gradio_app.py \
        --host 0.0.0.0 \
        --port "$GRADIO_PORT" \
        --ollama-url "$OLLAMA_URL" \
        --ollama-model "$OLLAMA_MODEL" \
        --hunyuan-model "$HUNYUAN_MODEL" \
        --device "$DEVICE" \
        "${EXTRA_ARGS[@]}"
}

cmd_api() {
    echo "Starting Ollama × Hunyuan3D API server on port $API_PORT ..."
    cd "$H3D_DIR"
    python3 ollama_api_server.py \
        --host 0.0.0.0 \
        --port "$API_PORT" \
        --ollama-url "$OLLAMA_URL" \
        --ollama-model "$OLLAMA_MODEL" \
        --hunyuan-model "$HUNYUAN_MODEL" \
        --device "$DEVICE" \
        "${EXTRA_ARGS[@]}"
}

cmd_chat() {
    cd "$H3D_DIR"
    python3 ollama_bridge.py \
        --ollama-url "$OLLAMA_URL" \
        --ollama-model "$OLLAMA_MODEL" \
        --hunyuan-model "$HUNYUAN_MODEL" \
        --device "$DEVICE" \
        "${EXTRA_ARGS[@]}" \
        chat
}

# ---- Dispatch -----------------------------------------------------------

case "${1:-gradio}" in
    install)  cmd_install ;;
    status)   cmd_status ;;
    gradio)   cmd_gradio ;;
    api)      cmd_api ;;
    chat)     cmd_chat ;;
    *)
        echo "Usage: $0 {install|status|gradio|api|chat}"
        exit 1
        ;;
esac
