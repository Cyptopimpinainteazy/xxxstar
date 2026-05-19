#!/usr/bin/env python3
"""
llm_recon.py — White-hat recon for publicly exposed LLM endpoints.

Scans for:
  - Ollama          (port 11434)
  - vLLM            (port 8000)
  - LM Studio       (port 1234)
  - LocalAI         (port 8080)
  - text-gen-webui  (port 7860/5000)
  - KoboldCpp       (port 5001)
  - llama.cpp       (port 8080)
  - Jan.ai          (port 1337)
  - OpenWebUI       (port 3000/8080)
  - ComfyUI         (port 8188)
  - Automatic1111   (port 7860)
  - FastChat         (port 8000/21001)
  - TabbyAPI        (port 5000)
  - TGI (HuggingFace) (port 8080)

Uses: FOFA, Shodan (free/API), Censys, Google dorks
"""

import argparse
import base64
import json
import os
import re
import time
import urllib.parse
from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass, field

import httpx

# Suppress browser-opening behavior
os.environ["BROWSER"] = "none"

# ── LLM Platform Definitions ──────────────────────────────────────────

@dataclass
class LLMPlatform:
    name: str
    default_port: int
    fingerprint_paths: list          # (path, expected_key_in_json_or_body_text)
    fofa_queries: list               # FOFA dork strings
    google_dorks: list               # Google dork strings
    shodan_queries: list             # Shodan dork strings
    icon: str = "🤖"


PLATFORMS = [
    LLMPlatform(
        name="Ollama",
        default_port=11434,
        fingerprint_paths=[
            ("/api/tags", "models"),
            ("/api/version", "version"),
        ],
        fofa_queries=[
            'port="11434" && body="Ollama is running"',
            'port="11434" && body="api/tags"',
        ],
        google_dorks=[
            'intitle:"Ollama" inurl:"/api/tags"',
            'inurl:"/api/tags" "name" "modified_at" "size"',
            '"Ollama is running" inurl:11434',
        ],
        shodan_queries=[
            '"Ollama is running" port:11434',
            'http.title:"Ollama" port:11434',
        ],
        icon="🦙",
    ),
    LLMPlatform(
        name="vLLM",
        default_port=8000,
        fingerprint_paths=[
            ("/v1/models", "data"),
            ("/health", ""),
            ("/version", "version"),
        ],
        fofa_queries=[
            'port="8000" && body="v1/models"',
            'port="8000" && header="vllm"',
            'body="vllm" && body="v1/models"',
        ],
        google_dorks=[
            'inurl:":8000/v1/models" "vllm"',
            'inurl:"/v1/models" "data" "owned_by"',
        ],
        shodan_queries=[
            '"vllm" port:8000',
            'http.html:"v1/models" port:8000',
        ],
        icon="⚡",
    ),
    LLMPlatform(
        name="LM Studio",
        default_port=1234,
        fingerprint_paths=[
            ("/v1/models", "data"),
            ("/lmstudio/models/list", ""),
        ],
        fofa_queries=[
            'port="1234" && body="v1/models"',
            'port="1234" && body="lm-studio"',
            'body="lm-studio" && body="v1/models"',
        ],
        google_dorks=[
            'inurl:":1234/v1/models" "lm-studio"',
            '"lm-studio" inurl:"/v1/models"',
        ],
        shodan_queries=[
            '"lm-studio" port:1234',
            'http.html:"v1/models" port:1234',
        ],
        icon="🖥️",
    ),
    LLMPlatform(
        name="LocalAI",
        default_port=8080,
        fingerprint_paths=[
            ("/v1/models", "data"),
            ("/readyz", ""),
        ],
        fofa_queries=[
            'port="8080" && body="LocalAI"',
            'body="LocalAI" && body="v1/models"',
            'title="LocalAI"',
        ],
        google_dorks=[
            'intitle:"LocalAI" inurl:"/v1/models"',
            '"LocalAI" inurl:8080',
        ],
        shodan_queries=[
            '"LocalAI" port:8080',
            'http.title:"LocalAI"',
        ],
        icon="🏠",
    ),
    LLMPlatform(
        name="text-generation-webui (Oobabooga)",
        default_port=7860,
        fingerprint_paths=[
            ("/api/v1/model", "result"),
            ("/v1/models", "data"),
            ("/api/v1/generate", ""),
        ],
        fofa_queries=[
            'port="7860" && body="text-generation"',
            'port="5000" && body="text-generation-webui"',
            'title="Text generation" && body="gradio"',
        ],
        google_dorks=[
            'intitle:"Text generation" inurl:7860',
            '"text-generation-webui" inurl:"/api/v1/model"',
        ],
        shodan_queries=[
            '"text-generation" port:7860',
            'http.title:"Text generation" port:7860',
        ],
        icon="📝",
    ),
    LLMPlatform(
        name="KoboldCpp",
        default_port=5001,
        fingerprint_paths=[
            ("/api/v1/model", "result"),
            ("/api/v1/info/version", "result"),
            ("/v1/models", "data"),
        ],
        fofa_queries=[
            'port="5001" && body="KoboldCpp"',
            'body="KoboldAI" && body="api/v1/model"',
            'title="KoboldAI" || title="KoboldCpp"',
        ],
        google_dorks=[
            '"KoboldCpp" inurl:"/api/v1/model"',
            'intitle:"KoboldAI" inurl:5001',
        ],
        shodan_queries=[
            '"KoboldCpp" port:5001',
            '"KoboldAI" port:5001',
        ],
        icon="🐉",
    ),
    LLMPlatform(
        name="llama.cpp server",
        default_port=8080,
        fingerprint_paths=[
            ("/health", "status"),
            ("/v1/models", "data"),
            ("/slots", ""),
        ],
        fofa_queries=[
            'port="8080" && body="llama.cpp"',
            'body="llama.cpp" && body="completion"',
        ],
        google_dorks=[
            '"llama.cpp" inurl:"/health"',
            '"llama.cpp" inurl:"/completion"',
        ],
        shodan_queries=[
            '"llama.cpp" port:8080',
        ],
        icon="🦙",
    ),
    LLMPlatform(
        name="Jan.ai",
        default_port=1337,
        fingerprint_paths=[
            ("/v1/models", "data"),
        ],
        fofa_queries=[
            'port="1337" && body="v1/models"',
            'port="1337" && body="jan"',
        ],
        google_dorks=[
            '"jan" inurl:":1337/v1/models"',
        ],
        shodan_queries=[
            'port:1337 http.html:"models"',
        ],
        icon="🌟",
    ),
    LLMPlatform(
        name="OpenWebUI",
        default_port=3000,
        fingerprint_paths=[
            ("/api/version", "version"),
            ("/api/models", ""),
        ],
        fofa_queries=[
            'title="Open WebUI"',
            'body="Open WebUI" && body="api/models"',
            'icon_hash="" && body="open-webui"',
        ],
        google_dorks=[
            'intitle:"Open WebUI" inurl:"/api"',
            '"Open WebUI" "Sign in"',
        ],
        shodan_queries=[
            'http.title:"Open WebUI"',
        ],
        icon="🌐",
    ),
    LLMPlatform(
        name="ComfyUI",
        default_port=8188,
        fingerprint_paths=[
            ("/system_stats", "system"),
            ("/object_info", ""),
            ("/prompt", ""),
        ],
        fofa_queries=[
            'port="8188" && body="ComfyUI"',
            'port="8188" && body="system_stats"',
            'title="ComfyUI"',
        ],
        google_dorks=[
            'intitle:"ComfyUI" inurl:8188',
            '"ComfyUI" inurl:"/system_stats"',
        ],
        shodan_queries=[
            '"ComfyUI" port:8188',
            'http.title:"ComfyUI"',
        ],
        icon="🎨",
    ),
    LLMPlatform(
        name="Automatic1111 / Stable Diffusion WebUI",
        default_port=7860,
        fingerprint_paths=[
            ("/sdapi/v1/sd-models", ""),
            ("/sdapi/v1/options", "sd_model_checkpoint"),
            ("/info", ""),
        ],
        fofa_queries=[
            'port="7860" && body="Stable Diffusion"',
            'body="sdapi" && body="txt2img"',
            'title="Stable Diffusion"',
        ],
        google_dorks=[
            'intitle:"Stable Diffusion" inurl:7860',
            '"sdapi/v1" inurl:7860',
        ],
        shodan_queries=[
            '"Stable Diffusion" port:7860',
            'http.title:"Stable Diffusion"',
        ],
        icon="🎭",
    ),
    LLMPlatform(
        name="TGI (HuggingFace Text Gen Inference)",
        default_port=8080,
        fingerprint_paths=[
            ("/info", "model_id"),
            ("/health", ""),
            ("/v1/models", "data"),
        ],
        fofa_queries=[
            'body="text-generation-inference"',
            'header="text-generation-inference"',
            'body="model_id" && body="max_input_length"',
        ],
        google_dorks=[
            '"text-generation-inference" inurl:"/info"',
            '"model_id" "max_input_length" inurl:"/info"',
        ],
        shodan_queries=[
            '"text-generation-inference"',
        ],
        icon="🤗",
    ),
    LLMPlatform(
        name="TabbyAPI",
        default_port=5000,
        fingerprint_paths=[
            ("/v1/models", "data"),
            ("/v1/model", ""),
            ("/health", ""),
        ],
        fofa_queries=[
            'port="5000" && body="TabbyAPI"',
            'body="TabbyAPI" && body="v1/models"',
        ],
        google_dorks=[
            '"TabbyAPI" inurl:"/v1/models"',
        ],
        shodan_queries=[
            '"TabbyAPI" port:5000',
        ],
        icon="🐱",
    ),
]


# ── Result Types ───────────────────────────────────────────────────────

@dataclass
class LLMEndpoint:
    ip: str
    port: int
    platform: str
    version: str | None = None
    models: list = field(default_factory=list)
    extra_info: dict = field(default_factory=dict)
    response_time_ms: float = 0.0
    source: str = "unknown"
    icon: str = "🤖"


BANNER = """
╔═══════════════════════════════════════════════════════════╗
║  LLM Endpoint Recon — Multi-Platform Security Scanner     ║
║  White-hat scanner for exposed AI/LLM services            ║
╠═══════════════════════════════════════════════════════════╣
║  Targets: Ollama, vLLM, LM Studio, LocalAI, Oobabooga,   ║
║  KoboldCpp, llama.cpp, Jan.ai, OpenWebUI, ComfyUI,       ║
║  Automatic1111, TGI, TabbyAPI                             ║
╚═══════════════════════════════════════════════════════════╝
"""


def print_section(title: str) -> None:
    print(f"\n{'─'*60}")
    print(f"  {title}")
    print(f"{'─'*60}")


# ── FOFA search (works best without API key) ──────────────────────────
def fofa_search(platforms: list[LLMPlatform]) -> dict[str, set[str]]:
    """Search FOFA for all platform dorks. Returns {platform_name: set(ips)}."""
    print_section("FOFA Search — All Platforms")
    results = {}
    headers = {
        "User-Agent": "Mozilla/5.0 (X11; Linux x86_64; rv:120.0) Gecko/20100101 Firefox/120.0",
    }

    for plat in platforms:
        results[plat.name] = set()
        for query in plat.fofa_queries:
            encoded = base64.b64encode(query.encode()).decode()
            url = f"https://fofa.info/result?qbase64={encoded}"
            print(f"  {plat.icon} [{plat.name}] {query}")
            try:
                resp = httpx.get(url, headers=headers, timeout=15, follow_redirects=True)
                if resp.status_code == 200:
                    found = re.findall(r'(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})', resp.text)
                    found = [ip for ip in found if not ip.startswith(("0.", "127.", "10.", "172.", "192.168."))]
                    results[plat.name].update(found)
                    if found:
                        print(f"      → {len(found)} IPs")
                    else:
                        print("      → 0 IPs")
                else:
                    print(f"      → HTTP {resp.status_code}")
            except httpx.ReadTimeout:
                print("      → timeout")
            except Exception as e:
                print(f"      → error: {e}")
            time.sleep(0.8)

    # Summary
    total = sum(len(v) for v in results.values())
    print(f"\n  [+] FOFA total: {total} unique IPs across all platforms")
    return results


def shodan_free_search(platforms: list[LLMPlatform]) -> dict[str, set[str]]:
    """Scrape Shodan free search for all platforms. Returns {platform_name: set(ips)}."""
    print_section("Shodan Free Search — All Platforms")
    results = {}
    headers = {
        "User-Agent": "Mozilla/5.0 (X11; Linux x86_64; rv:120.0) Gecko/20100101 Firefox/120.0",
    }

    for plat in platforms:
        results[plat.name] = set()
        for query in plat.shodan_queries:
            url = f"https://www.shodan.io/search?query={urllib.parse.quote(query)}"
            print(f"  {plat.icon} [{plat.name}] {query}")
            try:
                resp = httpx.get(url, headers=headers, timeout=15, follow_redirects=True)
                if resp.status_code == 200:
                    found = re.findall(r'(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})', resp.text)
                    found = [ip for ip in found if not ip.startswith(("0.", "127.", "10.", "172.", "192.168."))]
                    results[plat.name].update(found)
                    if found:
                        print(f"      → {len(found)} IPs")
                    else:
                        print("      → 0 IPs")
                else:
                    print(f"      → HTTP {resp.status_code}")
            except httpx.ReadTimeout:
                print("      → timeout")
            except Exception as e:
                print(f"      → error: {e}")
            time.sleep(0.8)

    # Summary
    total = sum(len(v) for v in results.values())
    print(f"\n  [+] Shodan Free total: {total} unique IPs across all platforms")
    return results


# ── Shodan API search ─────────────────────────────────────────────────
def shodan_api_search(api_key: str, platforms: list[LLMPlatform]) -> dict[str, set[str]]:
    """Search Shodan API for all platforms."""
    print_section("Shodan API Search — All Platforms")
    results = {}

    for plat in platforms:
        results[plat.name] = set()
        for query in plat.shodan_queries:
            print(f"  {plat.icon} [{plat.name}] {query}")
            try:
                resp = httpx.get(
                    "https://api.shodan.io/shodan/host/search",
                    params={"key": api_key, "query": query, "minify": "true"},
                    timeout=30,
                )
                if resp.status_code == 200:
                    data = resp.json()
                    matches = data.get("matches", [])
                    for m in matches:
                        ip = m.get("ip_str", "")
                        if ip:
                            results[plat.name].add(ip)
                    print(f"      → {data.get('total', 0)} total, {len(matches)} returned")
                elif resp.status_code == 401:
                    print("      → Invalid API key")
                    return results
                elif resp.status_code == 403:
                    print("      → Needs paid plan for filters")
                else:
                    print(f"      → HTTP {resp.status_code}")
            except Exception as e:
                print(f"      → error: {e}")
            time.sleep(1)

    total = sum(len(v) for v in results.values())
    print(f"\n  [+] Shodan total: {total} unique IPs")
    return results


# ── Validate endpoint against a specific platform ─────────────────────
def validate_platform(ip: str, platform: LLMPlatform, timeout: float = 5.0,
                       source: str = "recon") -> LLMEndpoint | None:
    """Check if ip:port matches a specific LLM platform fingerprint."""
    port = platform.default_port
    base_url = f"http://{ip}:{port}"

    for path, expected_key in platform.fingerprint_paths:
        try:
            start = time.time()
            resp = httpx.get(f"{base_url}{path}", timeout=timeout)
            elapsed = (time.time() - start) * 1000

            if resp.status_code != 200:
                continue

            # JSON fingerprinting
            try:
                data = resp.json()
            except Exception:
                data = None


            # Check for expected key in JSON
            if expected_key and data and isinstance(data, dict) and expected_key not in data:
                continue

            ep = LLMEndpoint(
                ip=ip, port=port, platform=platform.name,
                response_time_ms=elapsed, source=source, icon=platform.icon,
            )

            # Extract models based on platform
            if platform.name == "Ollama":
                ep = _enrich_ollama(ep, base_url, data, timeout)
            elif platform.name in ("vLLM", "LM Studio", "LocalAI", "Jan.ai",
                                    "llama.cpp server", "TabbyAPI",
                                    "TGI (HuggingFace Text Gen Inference)",
                                    "FastChat"):
                ep = _enrich_openai_compat(ep, base_url, data, timeout, platform.name)
            elif platform.name == "text-generation-webui (Oobabooga)":
                ep = _enrich_oobabooga(ep, base_url, data, timeout)
            elif platform.name == "KoboldCpp":
                ep = _enrich_kobold(ep, base_url, data, timeout)
            elif platform.name == "ComfyUI":
                ep = _enrich_comfyui(ep, base_url, data, timeout)
            elif platform.name == "Automatic1111 / Stable Diffusion WebUI":
                ep = _enrich_a1111(ep, base_url, data, timeout)
            elif platform.name == "OpenWebUI":
                ep = _enrich_openwebui(ep, base_url, data, timeout)
            else:
                if data and isinstance(data, dict):
                    ep.extra_info = {k: str(v)[:100] for k, v in list(data.items())[:5]}

            return ep

        except (httpx.ConnectError, httpx.ConnectTimeout, httpx.ReadTimeout):
            return None
        except Exception:
            continue

    return None


# ── Platform-specific enrichment ──────────────────────────────────────

def _enrich_ollama(ep: LLMEndpoint, base_url: str, data: dict, timeout: float) -> LLMEndpoint:
    if data and "models" in data:
        ep.models = [
            {"name": m.get("name", "?"),
             "size_gb": round(m.get("size", 0) / 1e9, 2),
             "family": m.get("details", {}).get("family", "?"),
             "params": m.get("details", {}).get("parameter_size", "?")}
            for m in data.get("models", [])
        ]
    try:
        vr = httpx.get(f"{base_url}/api/version", timeout=timeout)
        if vr.status_code == 200:
            ep.version = vr.json().get("version")
    except Exception:
        pass
    try:
        pr = httpx.get(f"{base_url}/api/ps", timeout=timeout)
        if pr.status_code == 200:
            running = [m.get("name", "?") for m in pr.json().get("models", [])]
            if running:
                ep.extra_info["running"] = ", ".join(running)
    except Exception:
        pass
    return ep


def _enrich_openai_compat(ep: LLMEndpoint, base_url: str, data: dict,
                           timeout: float, platform_name: str) -> LLMEndpoint:
    """Enrich for OpenAI-compatible APIs (vLLM, LM Studio, LocalAI, etc.)."""
    if data and "data" in data:
        for m in data["data"]:
            ep.models.append({
                "name": m.get("id", "?"),
                "owned_by": m.get("owned_by", "?"),
            })
    # TGI /info endpoint
    if platform_name == "TGI (HuggingFace Text Gen Inference)":
        try:
            ir = httpx.get(f"{base_url}/info", timeout=timeout)
            if ir.status_code == 200:
                info = ir.json()
                ep.extra_info["model_id"] = info.get("model_id", "?")
                ep.extra_info["max_input"] = info.get("max_input_length", "?")
                ep.extra_info["max_tokens"] = info.get("max_total_tokens", "?")
                if not ep.models:
                    ep.models.append({"name": info.get("model_id", "?")})
        except Exception:
            pass
    return ep


def _enrich_oobabooga(ep: LLMEndpoint, base_url: str, data: dict, timeout: float) -> LLMEndpoint:
    if data and "result" in data:
        ep.models.append({"name": data["result"]})
    try:
        mr = httpx.get(f"{base_url}/v1/models", timeout=timeout)
        if mr.status_code == 200:
            for m in mr.json().get("data", []):
                ep.models.append({"name": m.get("id", "?")})
    except Exception:
        pass
    return ep


def _enrich_kobold(ep: LLMEndpoint, base_url: str, data: dict, timeout: float) -> LLMEndpoint:
    if data and "result" in data:
        ep.models.append({"name": data["result"]})
    try:
        vr = httpx.get(f"{base_url}/api/v1/info/version", timeout=timeout)
        if vr.status_code == 200:
            ep.version = vr.json().get("result", "?")
    except Exception:
        pass
    try:
        cr = httpx.get(f"{base_url}/api/extra/true_max_context_length", timeout=timeout)
        if cr.status_code == 200:
            ep.extra_info["max_context"] = cr.json().get("value", "?")
    except Exception:
        pass
    return ep


def _enrich_comfyui(ep: LLMEndpoint, base_url: str, data: dict, timeout: float) -> LLMEndpoint:
    if data and "system" in data:
        sys_info = data["system"]
        ep.extra_info["os"] = sys_info.get("os", "?")
        ep.extra_info["python"] = sys_info.get("python_version", "?")
        ep.extra_info["comfyui_version"] = sys_info.get("comfyui_version", "?")
        devices = data.get("devices", [])
        for d in devices:
            ep.extra_info[f"gpu_{d.get('name', '?')}"] = f"{d.get('vram_total', 0) / 1e9:.1f}GB"
    return ep


def _enrich_a1111(ep: LLMEndpoint, base_url: str, data: dict, timeout: float) -> LLMEndpoint:
    try:
        mr = httpx.get(f"{base_url}/sdapi/v1/sd-models", timeout=timeout)
        if mr.status_code == 200:
            for m in mr.json():
                ep.models.append({
                    "name": m.get("model_name", m.get("title", "?")),
                })
    except Exception:
        pass
    try:
        opt = httpx.get(f"{base_url}/sdapi/v1/options", timeout=timeout)
        if opt.status_code == 200:
            od = opt.json()
            ep.extra_info["active_model"] = od.get("sd_model_checkpoint", "?")
            ep.extra_info["sd_vae"] = od.get("sd_vae", "?")
    except Exception:
        pass
    return ep


def _enrich_openwebui(ep: LLMEndpoint, base_url: str, data: dict, timeout: float) -> LLMEndpoint:
    if data and "version" in data:
        ep.version = data.get("version", "?")
    try:
        mr = httpx.get(f"{base_url}/api/models", timeout=timeout)
        if mr.status_code == 200:
            md = mr.json()
            if isinstance(md, list):
                for m in md:
                    ep.models.append({"name": m.get("name", m.get("id", "?"))})
            elif isinstance(md, dict) and "data" in md:
                for m in md["data"]:
                    ep.models.append({"name": m.get("id", "?")})
    except Exception:
        pass
    return ep


# ── Print helpers ─────────────────────────────────────────────────────

def print_endpoint(ep: LLMEndpoint, idx: int) -> None:
    print(f"\n  ┌─ {ep.icon} #{idx} {'─'*50}")
    print(f"  │ Platform:      {ep.platform}")
    print(f"  │ Host:          {ep.ip}:{ep.port}")
    if ep.version:
        print(f"  │ Version:       {ep.version}")
    print(f"  │ Response:      {ep.response_time_ms:.0f}ms")
    print(f"  │ Source:        {ep.source}")
    if ep.models:
        print(f"  │ Models ({len(ep.models)}):")
        for m in ep.models[:25]:
            detail = "  ".join(f"{v}" for k, v in m.items() if k != "name")
            print(f"  │   • {m.get('name', '?')}  {detail}")
        if len(ep.models) > 25:
            print(f"  │   ... and {len(ep.models) - 25} more")
    if ep.extra_info:
        print("  │ Extra:")
        for k, v in ep.extra_info.items():
            print(f"  │   {k}: {v}")
    print(f"  │ URL:           http://{ep.ip}:{ep.port}")
    print(f"  └{'─'*55}")


def print_all_dorks(platforms: list[LLMPlatform]) -> None:
    print_section("Google Dorks — All Platforms")
    for plat in platforms:
        if plat.google_dorks:
            print(f"\n  {plat.icon} {plat.name}:")
            for d in plat.google_dorks:
                url = f"https://www.google.com/search?q={urllib.parse.quote(d)}"
                print(f"    • {d}")
                print(f"      → {url}")


# ── Main ──────────────────────────────────────────────────────────────

def main() -> None:
    parser = argparse.ArgumentParser(description="Multi-platform LLM endpoint recon")
    parser.add_argument("--platforms", type=str, default="all",
                        help="Comma-separated platform names to scan (default: all)")
    parser.add_argument("--validate-only", type=str, default="",
                        help="Comma-separated IPs to validate directly")
    parser.add_argument("--timeout", type=float, default=5.0,
                        help="Connection timeout in seconds (default: 5)")
    parser.add_argument("--threads", type=int, default=30,
                        help="Max concurrent threads (default: 30)")
    parser.add_argument("--shodan-key", type=str,
                        default=os.environ.get("SHODAN_API_KEY", ""),
                        help="Shodan API key")
    parser.add_argument("--skip-search", action="store_true",
                        help="Skip FOFA/Shodan, only print dorks")
    parser.add_argument("--dorks-only", action="store_true",
                        help="Only print dork URLs, don't scan")
    parser.add_argument("--search-all", action="store_true",
                        help="Search all platforms (default behavior)")
    parser.add_argument("--local-only", action="store_true",
                        help="Skip external searches (FOFA/Shodan), only validate local/manual IPs")
    parser.add_argument("--quiet", action="store_true",
                        help="Suppress detailed output, only show results")
    args = parser.parse_args()

    print(BANNER)

    # Select platforms
    if args.platforms == "all":
        active_platforms = PLATFORMS
    else:
        names = [n.strip().lower() for n in args.platforms.split(",")]
        active_platforms = [p for p in PLATFORMS if p.name.lower() in names
                            or any(n in p.name.lower() for n in names)]
        if not active_platforms:
            print("  [!] No matching platforms. Available:")
            for p in PLATFORMS:
                print(f"      {p.icon} {p.name}")
            return

    print(f"  Scanning {len(active_platforms)} platforms:")
    for p in active_platforms:
        print(f"    {p.icon} {p.name} (port {p.default_port})")

    # Print all dorks
    print_all_dorks(active_platforms)

    if args.dorks_only:
        return

    # Gather candidate IPs per platform
    platform_ips: dict[str, set[str]] = {p.name: set() for p in active_platforms}

    # Manual IPs
    if args.validate_only:
        manual = [ip.strip() for ip in args.validate_only.split(",") if ip.strip()]
        for p in active_platforms:
            platform_ips[p.name].update(manual)
        print(f"\n  [*] Manual targets: {len(manual)} IPs × {len(active_platforms)} platforms")

    # Skip external searches if --local-only or --skip-search is set
    if not args.skip_search and not args.validate_only and not args.local_only:
        try:
            # FOFA
            fofa_results = fofa_search(active_platforms)
            for pname, ips in fofa_results.items():
                platform_ips[pname].update(ips)
        except Exception as e:
            print(f"  [!] FOFA search failed: {e}")

        try:
            # Shodan Free
            shodan_free_results = shodan_free_search(active_platforms)
            for pname, ips in shodan_free_results.items():
                platform_ips[pname].update(ips)
        except Exception as e:
            print(f"  [!] Shodan free search failed: {e}")

        try:
            # Shodan API
            if args.shodan_key:
                shodan_results = shodan_api_search(args.shodan_key, active_platforms)
                for pname, ips in shodan_results.items():
                    platform_ips[pname].update(ips)
        except Exception as e:
            print(f"  [!] Shodan API search failed: {e}")
    elif args.local_only:
        print_section("Local-Only Mode — Skipping External Searches")
        print("  Use --validate-only to specify IPs to check")

    # Validate
    total_candidates = sum(len(v) for v in platform_ips.values())
    if total_candidates == 0:
        print_section("No candidates found")
        print("  Use the dork URLs above in a browser, then:")
        print("  python scripts/llm_recon.py --validate-only IP1,IP2,IP3")
        return

    print_section(f"Validating {total_candidates} candidates across {len(active_platforms)} platforms")

    all_confirmed: list[LLMEndpoint] = []

    with ThreadPoolExecutor(max_workers=args.threads) as pool:
        futures = {}
        for plat in active_platforms:
            for ip in platform_ips[plat.name]:
                f = pool.submit(validate_platform, ip, plat, args.timeout, "recon")
                futures[f] = (ip, plat)

        for future in as_completed(futures):
            ip, plat = futures[future]
            try:
                result = future.result()
                if result:
                    all_confirmed.append(result)
                    print(f"  {plat.icon} [✓] {plat.name}: {ip}:{plat.default_port}")
                else:
                    print(f"  [✗] {plat.name}: {ip}:{plat.default_port}")
            except Exception as e:
                print(f"  [✗] {plat.name}: {ip} — {e}")

    # Results
    print_section(f"RESULTS — {len(all_confirmed)} Open LLM Endpoints Found")

    if not all_confirmed:
        print("  No confirmed open LLM instances.")
        return

    # Group by platform
    by_platform: dict[str, list[LLMEndpoint]] = {}
    for ep in all_confirmed:
        by_platform.setdefault(ep.platform, []).append(ep)

    idx = 1
    total_models = 0
    for pname, eps in sorted(by_platform.items()):
        print(f"\n  {'='*55}")
        print(f"  {eps[0].icon} {pname} — {len(eps)} endpoints")
        print(f"  {'='*55}")
        for ep in eps:
            print_endpoint(ep, idx)
            total_models += len(ep.models)
            idx += 1

    print(f"\n  ╔{'═'*55}╗")
    print(f"  ║  TOTAL: {len(all_confirmed)} endpoints, {total_models} models exposed")
    by_plat_str = ", ".join(f"{k}: {len(v)}" for k, v in sorted(by_platform.items()))
    print(f"  ║  {by_plat_str}")
    print(f"  ╚{'═'*55}╝")

    # Export
    outfile = os.path.join(os.path.dirname(os.path.abspath(__file__)),
                            "..", "llm_recon_results.json")
    outfile = os.path.abspath(outfile)
    export = [
        {
            "ip": ep.ip, "port": ep.port, "platform": ep.platform,
            "version": ep.version, "models": ep.models,
            "extra_info": ep.extra_info,
            "response_time_ms": ep.response_time_ms, "source": ep.source,
        }
        for ep in all_confirmed
    ]
    with open(outfile, "w") as f:
        json.dump(export, f, indent=2)
    print(f"\n  [+] Results saved to: {outfile}")
    print(f"\n{'═'*60}")
    print("  Scan complete. Stay ethical. 🛡️")
    print(f"{'═'*60}\n")


if __name__ == "__main__":
    main()
