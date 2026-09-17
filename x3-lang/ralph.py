#!/usr/bin/env python3
"""
ralph.py — X3 PRD Execution Runner with DeepSeek Backend

Reads the X3 cross-VM expansion PRD prompt and feeds it to DeepSeek.
- If DEEPSEEK_API_KEY is set, uses DeepSeek cloud API (api.deepseek.com)
- Otherwise falls back to Ollama running deepseek-coder locally
- Streams responses and optionally auto-executes generated code

Usage:
  python3 ralph.py                  # Run PRD through DeepSeek
  python3 ralph.py --release-gate   # Run release gate prompt instead
  python3 ralph.py --dry-run        # Preview prompt without sending
  python3 ralph.py --model glm-4.7:cloud  # Use a different model

Env vars:
  DEEPSEEK_API_KEY     DeepSeek cloud API key (cloud mode)
  DEEPSEEK_BASE_URL    Override API base URL (default: https://api.deepseek.com)
  OLLAMA_HOST          Ollama host (default: http://localhost:11434)
"""

import argparse
import json
import os
import sys
import time
from pathlib import Path

ROOT = Path(__file__).parent
PROMPT_FILE = ROOT / "compiler" / "tests" / "x3_cross_vm_expansion_prompt.json"

# ---- DeepSeek Cloud Backend ----

def deepseek_cloud(prompt: str, api_key: str, base_url: str, model: str = "deepseek-chat") -> str:
    """Send prompt to DeepSeek cloud API and return full response."""
    import urllib.request
    import urllib.error

    url = f"{base_url.rstrip('/')}/v1/chat/completions"
    body = json.dumps({
        "model": model,
        "messages": [
            {"role": "system", "content": "You are a senior blockchain engineer operating in X3 Production Proof Mode. Execute the given PRD by producing real code, real tests, real proof commands, and honest status reports. Never produce fake stubs or docs-only completions."},
            {"role": "user", "content": prompt}
        ],
        "stream": True,
        "temperature": 0.3,
        "max_tokens": 65536,
    }).encode("utf-8")

    req = urllib.request.Request(url, data=body, headers={
        "Authorization": f"Bearer {api_key}",
        "Content-Type": "application/json",
        "Accept": "text/event-stream",
    })

    try:
        resp = urllib.request.urlopen(req, timeout=600)
        full = []
        for line_bytes in resp:
            line = line_bytes.decode("utf-8").strip()
            if not line or line.startswith(":"):
                continue
            if line == "[DONE]":
                break
            if line.startswith("data: "):
                chunk = json.loads(line[6:])
                delta = chunk.get("choices", [{}])[0].get("delta", {})
                content = delta.get("content", "")
                if content:
                    full.append(content)
                    sys.stdout.write(content)
                    sys.stdout.flush()
        sys.stdout.write("\n")
        return "".join(full)
    except urllib.error.HTTPError as e:
        body = e.read().decode()
        raise RuntimeError(f"DeepSeek API error {e.code}: {body}")

# ---- Ollama Backend ----

def ollama_stream(prompt: str, model: str, host: str) -> str:
    """Send prompt to Ollama and stream the response."""
    import urllib.request
    import urllib.error

    url = f"{host.rstrip('/')}/api/generate"
    body = json.dumps({
        "model": model,
        "prompt": prompt,
        "stream": True,
        "options": {
            "temperature": 0.3,
            "num_predict": 65536,
        }
    }).encode("utf-8")

    req = urllib.request.Request(url, data=body, headers={
        "Content-Type": "application/json",
    })

    try:
        resp = urllib.request.urlopen(req, timeout=600)
        full = []
        for line_bytes in resp:
            line = line_bytes.decode("utf-8").strip()
            if not line:
                continue
            try:
                chunk = json.loads(line)
            except json.JSONDecodeError:
                continue
            content = chunk.get("response", "")
            if content:
                full.append(content)
                sys.stdout.write(content)
                sys.stdout.flush()
            if chunk.get("done", False):
                break
        sys.stdout.write("\n")
        return "".join(full)
    except urllib.error.HTTPError as e:
        body = e.read().decode()
        raise RuntimeError(f"Ollama error {e.code}: {body}")

# ---- Main ----

def load_prompt(gate_mode: bool, prd_file: str | None = None) -> str:
    """Load the PRD prompt from the generated JSON artifact or a custom PRD file."""
    # Custom PRD file takes priority
    if prd_file:
        path = Path(prd_file)
        if not path.exists():
            print(f"Error: PRD file not found at {path}", file=sys.stderr)
            sys.exit(1)
        with open(path, "r") as f:
            content = f.read()
        # If it's valid JSON with primary_prompt/release_gate_prompt keys, use those
        try:
            data = json.loads(content)
            if isinstance(data, dict):
                key = "release_gate_prompt" if gate_mode else "primary_prompt"
                if key in data:
                    return data[key]
                # If keys missing but has description/name, treat whole file as raw prompt
                if "description" in data or "name" in data:
                    return content
        except (json.JSONDecodeError, ValueError):
            pass
        # Return raw content as prompt (handles .md, .txt, etc.)
        return content

    if not PROMPT_FILE.exists():
        print(f"Error: Prompt file not found at {PROMPT_FILE}", file=sys.stderr)
        print("Run 'python3 compiler/tests/prd.json' first to generate it.", file=sys.stderr)
        sys.exit(1)

    with open(PROMPT_FILE, "r") as f:
        data = json.load(f)

    key = "release_gate_prompt" if gate_mode else "primary_prompt"
    return data.get(key, "")

def detect_backend():
    """Determine which backend to use based on available configuration."""
    api_key = os.environ.get("DEEPSEEK_API_KEY")
    if api_key:
        return "deepseek_cloud", api_key
    return "ollama", None

def test_ollama_connection(host: str, model: str) -> bool:
    """Quick check that Ollama is reachable and has the model."""
    import urllib.request
    try:
        req = urllib.request.Request(f"{host.rstrip('/')}/api/tags")
        resp = urllib.request.urlopen(req, timeout=10)
        data = json.loads(resp.read())
        models = [m["name"] for m in data.get("models", [])]
        if model in models:
            return True
        # Check if model starts with any known prefix match
        for m in models:
            if m == model or m.split(":")[0] == model.split(":")[0]:
                return True
        print(f"Warning: Model '{model}' not found in Ollama. Available: {', '.join(models[:10])}...", file=sys.stderr)
        return False
    except Exception as e:
        print(f"Warning: Cannot reach Ollama at {host}: {e}", file=sys.stderr)
        return False

def main():
    parser = argparse.ArgumentParser(
        description="ralph — X3 PRD Execution Runner (DeepSeek backend)",
        epilog="Set DEEPSEEK_API_KEY for cloud mode. Falls back to Ollama with deepseek-coder."
    )
    parser.add_argument("--dry-run", action="store_true", help="Print prompt without sending to backend")
    parser.add_argument("--release-gate", action="store_true", help="Run release gate prompt instead of primary")
    parser.add_argument("--model", type=str, default=None, help="Override the model used")
    parser.add_argument("--ollama-host", type=str, default=None, help="Ollama host URL")
    parser.add_argument("--no-stream", action="store_true", help="Disable streaming output")
    parser.add_argument("--output", "-o", type=str, help="Save response to file")
    parser.add_argument("--prd-file", type=str, default=None, help="Path to a custom PRD file (json, md, txt)")
    args = parser.parse_args()

    # Load prompt
    prompt = load_prompt(args.release_gate, args.prd_file)

    if args.dry_run:
        print(f"=== PRD Prompt ({len(prompt)} chars) ===")
        print(prompt[:2000])
        if len(prompt) > 2000:
            print(f"... ({len(prompt) - 2000} more chars)")
        print("=== End Preview (use without --dry-run to execute) ===")
        return

    # Determine backend
    backend, api_key = detect_backend()
    ollama_host = args.ollama_host or os.environ.get("OLLAMA_HOST", "http://localhost:11434")

    # Pick model
    if args.model:
        model = args.model
    elif backend == "deepseek_cloud":
        model = "deepseek-chat"
    else:
        # Try deepseek models available in Ollama
        preferred = ["deepseek-coder:1.3b", "lojak/cryptomaster", "qwen2.5-coder:14b"]
        model = preferred[0]
        for pref in preferred:
            if test_ollama_connection(ollama_host, pref):
                model = pref
                break

    # Generate system message for the prompt context
    mode_label = "Release Gate Check" if args.release_gate else "Cross-VM Expansion PRD"
    
    print(f"{'='*60}")
    print(f"  RALPH — X3 PRD Execution Runner")
    print(f"  Mode:     {mode_label}")
    print(f"  Backend:  {backend}")
    print(f"  Model:    {model}")
    print(f"  Prompt:   {len(prompt):,} chars")
    print(f"{'='*60}")
    print()
    print("--- Stream Start ---")

    start = time.time()
    try:
        if backend == "deepseek_cloud":
            base_url = os.environ.get("DEEPSEEK_BASE_URL", "https://api.deepseek.com")
            response = deepseek_cloud(prompt, api_key, base_url, model)
        else:
            response = ollama_stream(prompt, model, ollama_host)
    except RuntimeError as e:
        print(f"\nError: {e}", file=sys.stderr)
        sys.exit(1)
    elapsed = time.time() - start

    print()
    print(f"{'='*60}")
    print(f"  Response: {len(response):,} chars in {elapsed:.1f}s")
    print(f"{'='*60}")

    if args.output:
        out_path = Path(args.output)
        out_path.write_text(response, encoding="utf-8")
        print(f"Saved to {out_path}")

if __name__ == "__main__":
    main()