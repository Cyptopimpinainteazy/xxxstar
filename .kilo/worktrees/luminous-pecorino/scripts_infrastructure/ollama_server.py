#!/usr/bin/env python3
"""X3 Chain — Ollama HTTP Wrapper Service.

Provides a lightweight HTTP proxy in front of the native Ollama service with:
  - /health          → readiness + GPU summary
  - /status          → detailed GPU utilisation & loaded models
  - /v1/models       → OpenAI-compatible model list (proxy)
  - /v1/chat/completions  → OpenAI-compatible chat (proxy)
  - /api/*           → transparent proxy to native Ollama API

Designed to run as a systemd service alongside the native ``ollama.service``.
"""

from __future__ import annotations

import json
import logging
import os
import re
import shutil
import signal
import subprocess
import time
from http.server import BaseHTTPRequestHandler, HTTPServer
from typing import Any
from urllib.error import URLError
from urllib.parse import urlparse
from urllib.request import Request, urlopen

# ---------------------------------------------------------------------------
# Configuration (env-driven)
# ---------------------------------------------------------------------------
LISTEN_HOST = os.getenv("OLLAMA_WRAPPER_HOST", "0.0.0.0")
LISTEN_PORT = int(os.getenv("OLLAMA_WRAPPER_PORT", "11435"))
OLLAMA_URL = os.getenv("OLLAMA_URL", "http://127.0.0.1:11434")  # nosemgrep: py-no-localhost-endpoints
LOG_LEVEL = os.getenv("OLLAMA_WRAPPER_LOG_LEVEL", "INFO").upper()

# GPU env vars (mirror the ones already set in the user env / bashrc)
MAX_LOADED_MODELS = int(os.getenv("OLLAMA_MAX_LOADED_MODELS", "2"))
SCHED_SPREAD = os.getenv("OLLAMA_SCHED_SPREAD", "1") == "1"
NUM_PARALLEL = int(os.getenv("OLLAMA_NUM_PARALLEL", "4"))
KEEP_ALIVE = os.getenv("OLLAMA_KEEP_ALIVE", "30m")
FLASH_ATTENTION = os.getenv("OLLAMA_FLASH_ATTENTION", "1") == "1"

# ---------------------------------------------------------------------------
# Logging
# ---------------------------------------------------------------------------
logging.basicConfig(
    level=getattr(logging, LOG_LEVEL, logging.INFO),
    format="%(asctime)s [ollama-wrapper] %(levelname)s %(message)s",
    datefmt="%Y-%m-%d %H:%M:%S",
)
log = logging.getLogger("ollama-wrapper")

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------
_start_time = time.time()


def _proxy(path: str, method: str = "GET", body: bytes | None = None,
           headers: dict | None = None, timeout: int = 120) -> tuple[int, bytes, str]:
    """Forward a request to the native Ollama HTTP API.

    Returns (status_code, response_body_bytes, content_type).
    """
    url = OLLAMA_URL.rstrip("/") + path
    req = Request(url, data=body, method=method)
    req.add_header("Content-Type", "application/json")
    if headers:
        for k, v in headers.items():
            req.add_header(k, v)
    try:
        with urlopen(req, timeout=timeout) as resp:
            ct = resp.headers.get("Content-Type", "application/json")
            return resp.status, resp.read(), ct
    except URLError as exc:
        err = json.dumps({"error": f"upstream unavailable: {exc}"}).encode()
        return 502, err, "application/json"
    except Exception as exc:
        err = json.dumps({"error": str(exc)}).encode()
        return 500, err, "application/json"


def _gpu_info() -> list[dict[str, Any]]:
    """Return basic GPU info via nvidia-smi."""
    nvidia_smi = shutil.which("nvidia-smi")
    if not nvidia_smi:
        return []
    try:
        out = subprocess.check_output(
            [nvidia_smi,
             "--query-gpu=index,name,temperature.gpu,utilization.gpu,memory.used,memory.total,power.draw,power.limit",
             "--format=csv,noheader,nounits"],
            text=True, timeout=5,
        )
        gpus: list[dict[str, Any]] = []
        for line in out.strip().splitlines():
            parts = [p.strip() for p in line.split(",")]
            if len(parts) >= 8:
                gpus.append({
                    "index": int(parts[0]),
                    "name": parts[1],
                    "temp_c": int(parts[2]),
                    "util_pct": int(parts[3]),
                    "mem_used_mib": int(parts[4]),
                    "mem_total_mib": int(parts[5]),
                    "power_w": float(parts[6]),
                    "power_limit_w": float(parts[7]),
                })
        return gpus
    except Exception:
        return []


def _ollama_alive() -> bool:
    """Quick liveness check against Ollama."""
    try:
        status, _, _ = _proxy("/api/tags", timeout=3)
        return status == 200
    except Exception:
        return False


def _models() -> list[dict[str, Any]]:
    """Fetch model list from Ollama."""
    try:
        status, body, _ = _proxy("/api/tags", timeout=5)
        if status == 200:
            data = json.loads(body)
            return data.get("models", [])
    except Exception:
        pass
    return []


def _parse_embedding_alias_map() -> dict[str, str]:
    """Parse embedding alias configuration from env.

    Supports:
      - JSON object: {"qwen":"qwen-embed-large"}
      - CSV map: qwen:qwen-embed-large,mistral:nomic-embed-text
    """
    raw = (os.getenv("OLLAMA_EMBEDDING_ALIAS_MAP", "") or "").strip()
    if not raw:
        return {}

    try:
        parsed = json.loads(raw)
        if isinstance(parsed, dict):
            return {
                str(k).strip().lower(): str(v).strip()
                for k, v in parsed.items()
                if str(k).strip() and str(v).strip()
            }
    except Exception:
        pass

    mapping: dict[str, str] = {}
    for pair in raw.split(","):
        if ":" not in pair:
            continue
        key, value = pair.split(":", 1)
        key = key.strip().lower()
        value = value.strip()
        if key and value:
            mapping[key] = value
    return mapping


def rewrite_embedding_payload(payload: dict[str, Any]) -> dict[str, Any]:
    """Rewrite embedding payload model aliases for tool compatibility."""
    if not isinstance(payload, dict):
        return payload

    model = payload.get("model")
    if not isinstance(model, str) or not model.strip():
        return payload

    lowered = model.strip().lower()
    alias_map = _parse_embedding_alias_map()
    for prefix, alias in alias_map.items():
        if lowered.startswith(prefix):
            updated = payload.copy()
            updated["model"] = alias
            return updated

    if lowered.startswith("qwen"):
        fallback_model = (os.getenv("OLLAMA_EMBEDDING_FALLBACK_MODEL", "") or "").strip() or "mxbai-embed-large"
        updated = payload.copy()
        updated["model"] = fallback_model
        return updated

    return payload


def _strip_json_code_fence(text: str) -> str:
    stripped = text.strip()
    if stripped.startswith("```"):
        stripped = re.sub(r"^```(?:json)?\s*", "", stripped, flags=re.IGNORECASE)
        stripped = re.sub(r"\s*```$", "", stripped)
    return stripped.strip()


def _try_parse_json(text: str) -> Any:
    candidate = _strip_json_code_fence(text)
    try:
        return json.loads(candidate)
    except Exception:
        pass

    # Fallback for assistant output with surrounding prose.
    match = re.search(r"(\{[\s\S]*\}|\[[\s\S]*\])", candidate)
    if not match:
        return None
    try:
        return json.loads(match.group(1))
    except Exception:
        return None


def _serialize_tool_arguments(arguments: Any) -> str:
    if isinstance(arguments, str):
        return arguments
    if arguments is None:
        return "{}"
    try:
        return json.dumps(arguments)
    except Exception:
        return "{}"


def _candidate_to_tool_call(candidate: dict[str, Any], index: int) -> dict[str, Any] | None:
    if not isinstance(candidate, dict):
        return None

    name = None
    arguments = {}

    if isinstance(candidate.get("function"), dict):
        fn = candidate["function"]
        name = fn.get("name")
        arguments = fn.get("arguments", {})
    elif isinstance(candidate.get("name"), str):
        name = candidate.get("name")
        arguments = candidate.get("arguments", {})
    elif isinstance(candidate.get("tool_name"), str):
        name = candidate.get("tool_name")
        arguments = candidate.get("arguments", {})

    if not isinstance(name, str) or not name.strip():
        return None

    return {
        "id": f"call_auto_{index}",
        "type": "function",
        "function": {
            "name": name.strip(),
            "arguments": _serialize_tool_arguments(arguments),
        },
    }


def extract_tool_calls_from_content(content: str) -> list[dict[str, Any]] | None:
    parsed = _try_parse_json(content)
    if parsed is None:
        return None

    candidates: list[dict[str, Any]] = []
    if isinstance(parsed, dict):
        if isinstance(parsed.get("tool_calls"), list):
            for item in parsed["tool_calls"]:
                if isinstance(item, dict):
                    candidates.append(item)
        else:
            candidates.append(parsed)
    elif isinstance(parsed, list):
        for item in parsed:
            if isinstance(item, dict):
                candidates.append(item)

    if not candidates:
        return None

    tool_calls: list[dict[str, Any]] = []
    for idx, candidate in enumerate(candidates):
        normalized = _candidate_to_tool_call(candidate, idx)
        if normalized:
            tool_calls.append(normalized)

    return tool_calls or None


def normalize_chat_completion_payload(payload: dict[str, Any]) -> tuple[dict[str, Any], bool]:
    if not isinstance(payload, dict):
        return payload, False

    choices = payload.get("choices")
    if not isinstance(choices, list):
        return payload, False

    changed = False
    for choice in choices:
        if not isinstance(choice, dict):
            continue
        message = choice.get("message")
        if not isinstance(message, dict):
            continue
        if message.get("tool_calls"):
            continue
        content = message.get("content")
        if not isinstance(content, str) or not content.strip():
            continue

        tool_calls = extract_tool_calls_from_content(content)
        if not tool_calls:
            continue

        message["tool_calls"] = tool_calls
        message["content"] = None
        if choice.get("finish_reason") in (None, "", "stop"):
            choice["finish_reason"] = "tool_calls"
        changed = True

    return payload, changed


def normalize_chat_completion_response_body(body: bytes) -> tuple[bytes, bool]:
    try:
        payload = json.loads(body.decode())
    except Exception:
        return body, False

    normalized, changed = normalize_chat_completion_payload(payload)
    if not changed:
        return body, False
    return json.dumps(normalized).encode(), True


# ---------------------------------------------------------------------------
# HTTP Handler
# ---------------------------------------------------------------------------
class OllamaWrapperHandler(BaseHTTPRequestHandler):
    """Simple request handler that provides health/status + proxies to Ollama."""

    server_version = "AtlasSphere-OllamaWrapper/1.0"

    def log_message(self, fmt, *args) -> None:
        log.debug(fmt, *args)

    # ── GET routes ────────────────────────────────────────────────────────
    def do_GET(self):
        path = urlparse(self.path).path

        if path == "/health":
            return self._handle_health()
        if path == "/status":
            return self._handle_status()

        # Proxy everything else to Ollama
        return self._proxy_get(self.path)

    # ── POST routes ───────────────────────────────────────────────────────
    def do_POST(self):
        content_len = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(content_len) if content_len else None
        return self._proxy_post(self.path, body)

    # ── Route implementations ────────────────────────────────────────────
    def _handle_health(self) -> None:
        alive = _ollama_alive()
        gpus = _gpu_info()
        payload = {
            "status": "healthy" if alive else "degraded",
            "ollama_reachable": alive,
            "uptime_s": round(time.time() - _start_time, 1),
            "gpu_count": len(gpus),
            "config": {
                "ollama_url": OLLAMA_URL,
                "max_loaded_models": MAX_LOADED_MODELS,
                "sched_spread": SCHED_SPREAD,
                "num_parallel": NUM_PARALLEL,
                "keep_alive": KEEP_ALIVE,
                "flash_attention": FLASH_ATTENTION,
            },
        }
        code = 200 if alive else 503
        self._json(code, payload)

    def _handle_status(self) -> None:
        alive = _ollama_alive()
        gpus = _gpu_info()
        models = _models()

        model_names = [m.get("name", "?") for m in models]
        total_vram = sum(g.get("mem_total_mib", 0) for g in gpus)
        used_vram = sum(g.get("mem_used_mib", 0) for g in gpus)

        payload = {
            "status": "healthy" if alive else "degraded",
            "ollama_reachable": alive,
            "uptime_s": round(time.time() - _start_time, 1),
            "gpus": gpus,
            "gpu_summary": {
                "count": len(gpus),
                "total_vram_mib": total_vram,
                "used_vram_mib": used_vram,
                "free_vram_mib": total_vram - used_vram,
            },
            "models": model_names,
            "model_count": len(model_names),
            "config": {
                "ollama_url": OLLAMA_URL,
                "max_loaded_models": MAX_LOADED_MODELS,
                "sched_spread": SCHED_SPREAD,
                "num_parallel": NUM_PARALLEL,
                "keep_alive": KEEP_ALIVE,
                "flash_attention": FLASH_ATTENTION,
                # Embedding fallback used when a Qwen LLM model is supplied to the
                # embeddings endpoint (see wrapper rewrite behaviour).
                "embedding_fallback_model": os.getenv("OLLAMA_EMBEDDING_FALLBACK_MODEL", "mxbai-embed-large"),
            },
        }
        self._json(200, payload)

    # ── Proxy helpers ────────────────────────────────────────────────────
    def _proxy_get(self, path: str) -> None:
        status, body, ct = _proxy(path)
        self.send_response(status)
        self.send_header("Content-Type", ct)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def _proxy_post(self, path: str, body: bytes | None) -> None:
        hdrs = {}
        if self.headers.get("Content-Type"):
            hdrs["Content-Type"] = self.headers["Content-Type"]

        request_payload: dict[str, Any] | None = None
        if body:
            try:
                maybe_payload = json.loads(body.decode())
                if isinstance(maybe_payload, dict):
                    request_payload = maybe_payload
            except Exception:
                request_payload = None

        # --- Special-case: embedding requests from tools that pass an LLM model name
        # (e.g. 'qwen2.5-coder:14b'). Ollama may not expose embeddings for those
        # LLM models — rewrite to a configured embedding-capable model so
        # external tools (RooCode, etc.) that send a Qwen model name still get
        # embeddings.
        if path.startswith("/api/embeddings") or path.startswith("/v1/embeddings"):
            payload = request_payload or {}

            if isinstance(payload, dict):
                new_payload = rewrite_embedding_payload(payload)
                # If the payload was mutated or model changed, re-encode body for forwarding
                if new_payload is not payload or payload.get("model") != new_payload.get("model"):
                    body = json.dumps(new_payload).encode()
                    request_payload = new_payload

        status, resp_body, ct = _proxy(path, method="POST", body=body, headers=hdrs)

        # Some local models return tool calls as plain JSON content instead of
        # OpenAI `tool_calls`. Normalize these responses for clients like Roo.
        if (
            path.startswith("/v1/chat/completions")
            and isinstance(request_payload, dict)
            and request_payload.get("tools")
            and "application/json" in (ct or "").lower()
        ):
            resp_body, rewritten = normalize_chat_completion_response_body(resp_body)
            if rewritten:
                log.info("Normalized text-form tool call response into OpenAI tool_calls.")

        self.send_response(status)
        self.send_header("Content-Type", ct)
        self.send_header("Content-Length", str(len(resp_body)))
        self.end_headers()
        self.wfile.write(resp_body)

    # ── JSON response helper ─────────────────────────────────────────────
    def _json(self, code: int, data: Any) -> None:
        payload = json.dumps(data, indent=2).encode()
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
def main() -> None:
    server = HTTPServer((LISTEN_HOST, LISTEN_PORT), OllamaWrapperHandler)

    def _shutdown(signum, frame) -> None:
        log.info("Received signal %s — shutting down", signum)
        server.shutdown()

    signal.signal(signal.SIGTERM, _shutdown)
    signal.signal(signal.SIGINT, _shutdown)

    log.info("Ollama wrapper listening on %s:%d  (upstream: %s)", LISTEN_HOST, LISTEN_PORT, OLLAMA_URL)
    log.info("GPU config: spread=%s  max_models=%d  parallel=%d  keep_alive=%s  flash_attn=%s",
             SCHED_SPREAD, MAX_LOADED_MODELS, NUM_PARALLEL, KEEP_ALIVE, FLASH_ATTENTION)

    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()
        log.info("Wrapper stopped.")


if __name__ == "__main__":
    main()
