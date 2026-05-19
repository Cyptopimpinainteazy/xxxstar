#!/usr/bin/env python3
"""
ollama_recon.py — White-hat recon for publicly exposed Ollama endpoints.

Techniques:
  1. Shodan (free HTML search, no API key needed)
  2. Censys search (free HTML, no key)
  3. FOFA search (free HTML, no key)
  4. Direct port-scan style validation against candidate IPs
  5. Google dork URL construction (prints ready-to-open URLs)

Usage:
  python scripts/ollama_recon.py [--validate-only IP1,IP2,...] [--timeout 5]
"""

import argparse
import json
import os
import re
import time
import urllib.parse
from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass, field

import httpx

try:
    import shodan
    HAS_SHODAN = True
except ImportError:
    HAS_SHODAN = False

SHODAN_API_KEY = os.environ.get("SHODAN_API_KEY", "")

# ── Constants ──────────────────────────────────────────────────────────
OLLAMA_DEFAULT_PORT = 11434
OLLAMA_API_PATHS = [
    "/api/tags",       # list models (most reliable fingerprint)
    "/api/version",    # version info
    "/api/ps",         # running models
]

BANNER = """
╔═══════════════════════════════════════════════════════════╗
║  Ollama Endpoint Recon — White-Hat Security Scanner       ║
║  Finds publicly exposed Ollama instances without auth     ║
╚═══════════════════════════════════════════════════════════╝
"""

# Google dorks — user opens these manually or we print them
GOOGLE_DORKS = [
    'intitle:"Ollama" inurl:"/api/tags"',
    'inurl:"/api/tags" "name" "modified_at" "size"',
    'inurl:":11434" "ollama"',
    '"Ollama is running" inurl:11434',
    'inurl:"/api/generate" "models"',
    'inurl:"/v1/models" "ollama"',
    'inurl:"/api/show" "modelfile"',
]

SHODAN_DORKS = [
    'product:"Ollama" port:11434',
    'http.html:"Ollama is running"',
    '"Ollama is running" port:11434',
    'http.title:"Ollama" port:11434',
]

CENSYS_DORKS = [
    'services.port=11434 AND services.http.response.body:"Ollama"',
    'services.http.response.body:"Ollama is running"',
]

FOFA_DORKS = [
    'port="11434" && body="Ollama is running"',
    'port="11434" && body="api/tags"',
]


@dataclass
class OllamaEndpoint:
    ip: str
    port: int = OLLAMA_DEFAULT_PORT
    version: str | None = None
    models: list = field(default_factory=list)
    running_models: list = field(default_factory=list)
    response_time_ms: float = 0.0
    source: str = "unknown"


def print_section(title: str) -> None:
    print(f"\n{'─'*60}")
    print(f"  {title}")
    print(f"{'─'*60}")


# ── Shodan free HTML search (no API key) ───────────────────────────────
def shodan_free_search() -> list[str]:
    """Scrape Shodan free search for Ollama endpoints."""
    print_section("Shodan Free Search")
    ips = set()

    queries = SHODAN_DORKS

    headers = {
        "User-Agent": "Mozilla/5.0 (X11; Linux x86_64; rv:120.0) Gecko/20100101 Firefox/120.0",
    }

    for query in queries:
        url = f"https://www.shodan.io/search?query={urllib.parse.quote(query)}"
        print(f"  [*] Query: {query}")
        try:
            resp = httpx.get(url, headers=headers, timeout=15, follow_redirects=True)
            if resp.status_code == 200:
                found = re.findall(
                    r'(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})',
                    resp.text
                )
                found = [ip for ip in found if not ip.startswith(("0.", "127.", "10.", "172.", "192.168."))]
                ips.update(found)
                print(f"      Found {len(found)} candidate IPs")
            else:
                print(f"      HTTP {resp.status_code}")
        except Exception as e:
            print(f"      Error: {e}")
        time.sleep(1)

    print(f"  [+] Total unique IPs from Shodan Free: {len(ips)}")
    return list(ips)


def shodan_api_search(api_key: str) -> list[str]:
    """Use Shodan API for searching Ollama endpoints."""
    print_section("Shodan API Search")
    ips = set()

    if not HAS_SHODAN:
        print("  [!] Shodan library not installed: pip install shodan")
        return []

    api = shodan.Shodan(api_key)

    queries = SHODAN_DORKS

    for query in queries:
        print(f"  [*] Query: {query}")
        try:
            result = api.search(query)
            for match in result['matches']:
                ip = match['ip_str']
                port = match.get('port', OLLAMA_DEFAULT_PORT)
                if port == OLLAMA_DEFAULT_PORT:
                    ips.add(ip)
            print(f"      Found {len(result['matches'])} results")
        except shodan.APIError as e:
            print(f"      API Error: {e}")
        time.sleep(1)  # rate-limit

    print(f"  [+] Total unique IPs from Shodan API: {len(ips)}")
    return list(ips)


# ── Censys free HTML search ────────────────────────────────────────────
def censys_free_search() -> list[str]:
    """Scrape Censys free search for Ollama endpoints."""
    print_section("Censys Free Search")
    ips = set()

    queries = [
        "Ollama port:11434",
        '"Ollama is running"',
    ]

    headers = {
        "User-Agent": "Mozilla/5.0 (X11; Linux x86_64; rv:120.0) Gecko/20100101 Firefox/120.0",
    }

    for query in queries:
        url = f"https://search.censys.io/search?resource=hosts&q={urllib.parse.quote(query)}"
        print(f"  [*] Query: {query}")
        try:
            resp = httpx.get(url, headers=headers, timeout=15, follow_redirects=True)
            if resp.status_code == 200:
                found = re.findall(
                    r'(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})',
                    resp.text
                )
                # Filter out common non-target IPs
                found = [ip for ip in found if not ip.startswith(("0.", "127.", "10.", "172.", "192.168."))]
                ips.update(found)
                print(f"      Found {len(found)} candidate IPs")
            else:
                print(f"      HTTP {resp.status_code}")
        except Exception as e:
            print(f"      Error: {e}")
        time.sleep(1)

    print(f"  [+] Total unique IPs from Censys: {len(ips)}")
    return list(ips)


# ── FOFA free search ──────────────────────────────────────────────────
def fofa_free_search() -> list[str]:
    """Scrape FOFA free search for Ollama endpoints."""
    print_section("FOFA Free Search")
    ips = set()

    import base64
    for query in FOFA_DORKS:
        encoded = base64.b64encode(query.encode()).decode()
        url = f"https://fofa.info/result?qbase64={encoded}"
        print(f"  [*] Query: {query}")
        try:
            headers = {
                "User-Agent": "Mozilla/5.0 (X11; Linux x86_64; rv:120.0) Gecko/20100101 Firefox/120.0",
            }
            resp = httpx.get(url, headers=headers, timeout=15, follow_redirects=True)
            if resp.status_code == 200:
                found = re.findall(
                    r'(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})',
                    resp.text
                )
                found = [ip for ip in found if not ip.startswith(("0.", "127.", "10.", "172.", "192.168."))]
                ips.update(found)
                print(f"      Found {len(found)} candidate IPs")
            else:
                print(f"      HTTP {resp.status_code}")
        except Exception as e:
            print(f"      Error: {e}")
        time.sleep(1)

    print(f"  [+] Total unique IPs from FOFA: {len(ips)}")
    return list(ips)


# ── Validate a single Ollama endpoint ─────────────────────────────────
def validate_endpoint(ip: str, port: int = OLLAMA_DEFAULT_PORT,
                       timeout: float = 5.0, source: str = "scan") -> OllamaEndpoint | None:
    """Check if an IP:port is a live, unauthenticated Ollama instance."""
    base_url = f"http://{ip}:{port}"
    endpoint = OllamaEndpoint(ip=ip, port=port, source=source)

    try:
        # 1. Check /api/tags (most reliable)
        start = time.time()
        resp = httpx.get(f"{base_url}/api/tags", timeout=timeout)
        endpoint.response_time_ms = (time.time() - start) * 1000

        if resp.status_code == 200:
            data = resp.json()
            if "models" in data:
                endpoint.models = [
                    {
                        "name": m.get("name", "?"),
                        "size_gb": round(m.get("size", 0) / 1e9, 2),
                        "family": m.get("details", {}).get("family", "?"),
                        "params": m.get("details", {}).get("parameter_size", "?"),
                    }
                    for m in data.get("models", [])
                ]
        else:
            return None

        # 2. Check version
        try:
            vresp = httpx.get(f"{base_url}/api/version", timeout=timeout)
            if vresp.status_code == 200:
                endpoint.version = vresp.json().get("version", "?")
        except Exception:
            pass

        # 3. Check running models
        try:
            presp = httpx.get(f"{base_url}/api/ps", timeout=timeout)
            if presp.status_code == 200:
                pdata = presp.json()
                endpoint.running_models = [
                    m.get("name", "?") for m in pdata.get("models", [])
                ]
        except Exception:
            pass

        return endpoint

    except (httpx.ConnectError, httpx.ConnectTimeout, httpx.ReadTimeout):
        return None
    except Exception:
        return None


# ── Print results ─────────────────────────────────────────────────────
def print_endpoint(ep: OllamaEndpoint, idx: int) -> None:
    print(f"\n  ┌─ Endpoint #{idx} {'─'*45}")
    print(f"  │ Host:          {ep.ip}:{ep.port}")
    print(f"  │ Version:       {ep.version or 'unknown'}")
    print(f"  │ Response:      {ep.response_time_ms:.0f}ms")
    print(f"  │ Source:        {ep.source}")
    print(f"  │ Models ({len(ep.models)}):")
    for m in ep.models:
        print(f"  │   • {m['name']}  ({m['params']}, {m['size_gb']}GB, {m['family']})")
    if ep.running_models:
        print(f"  │ Running:       {', '.join(ep.running_models)}")
    print(f"  │ URL:           http://{ep.ip}:{ep.port}/api/tags")
    print(f"  └{'─'*55}")


def print_google_dorks() -> None:
    print_section("Google Dorks (open in browser)")
    for i, dork in enumerate(GOOGLE_DORKS, 1):
        url = f"https://www.google.com/search?q={urllib.parse.quote(dork)}"
        print(f"  [{i}] {dork}")
        print(f"      → {url}")
    print()


def print_shodan_dorks() -> None:
    print_section("Shodan Dorks (open in browser)")
    for i, dork in enumerate(SHODAN_DORKS, 1):
        url = f"https://www.shodan.io/search?query={urllib.parse.quote(dork)}"
        print(f"  [{i}] {dork}")
        print(f"      → {url}")
    print()


# ── Main ──────────────────────────────────────────────────────────────
def main() -> None:
    parser = argparse.ArgumentParser(description="White-hat Ollama endpoint recon")
    parser.add_argument("--validate-only", type=str, default="",
                        help="Comma-separated IPs to validate directly")
    parser.add_argument("--timeout", type=float, default=5.0,
                        help="Connection timeout in seconds (default: 5)")
    parser.add_argument("--threads", type=int, default=20,
                        help="Max concurrent validation threads (default: 20)")
    parser.add_argument("--skip-search", action="store_true",
                        help="Skip web scraping, only print dorks + validate IPs")
    parser.add_argument("--port", type=int, default=OLLAMA_DEFAULT_PORT,
                        help=f"Port to check (default: {OLLAMA_DEFAULT_PORT})")
    args = parser.parse_args()

    print(BANNER)

    all_ips = set()

    # If user passes specific IPs, validate those
    if args.validate_only:
        manual_ips = [ip.strip() for ip in args.validate_only.split(",") if ip.strip()]
        all_ips.update(manual_ips)
        print(f"  [*] Manual targets: {len(manual_ips)} IPs")

    # Print dork URLs for manual browser use
    print_google_dorks()
    print_shodan_dorks()

    if not args.skip_search and not args.validate_only:
        # Run automated searches
        try:
            shodan_ips = shodan_api_search("0WZgZSLvyPThoB2AcWVCFoWdRdec3tSr")
            all_ips.update(shodan_ips)
        except Exception as e:
            print(f"  [!] Shodan API search failed: {e}")

        try:
            shodan_free = shodan_free_search()
            all_ips.update(shodan_free)
        except Exception as e:
            print(f"  [!] Shodan free search failed: {e}")

        try:
            censys_ips = censys_free_search()
            all_ips.update(censys_ips)
        except Exception as e:
            print(f"  [!] Censys search failed: {e}")

        try:
            fofa_ips = fofa_free_search()
            all_ips.update(fofa_ips)
        except Exception as e:
            print(f"  [!] FOFA search failed: {e}")

    # Validate all discovered IPs
    if all_ips:
        print_section(f"Validating {len(all_ips)} candidate endpoints (timeout={args.timeout}s)")

        confirmed = []
        with ThreadPoolExecutor(max_workers=args.threads) as pool:
            futures = {
                pool.submit(validate_endpoint, ip, args.port, args.timeout, "recon"): ip
                for ip in all_ips
            }
            for future in as_completed(futures):
                ip = futures[future]
                try:
                    result = future.result()
                    if result:
                        confirmed.append(result)
                        print(f"  [✓] CONFIRMED: {ip}:{args.port}")
                    else:
                        print(f"  [✗] Not Ollama: {ip}:{args.port}")
                except Exception as e:
                    print(f"  [✗] Error {ip}: {e}")

        # Summary
        print_section(f"RESULTS — {len(confirmed)} Open Ollama Endpoints Found")
        if confirmed:
            total_models = 0
            for i, ep in enumerate(confirmed, 1):
                print_endpoint(ep, i)
                total_models += len(ep.models)

            print(f"\n  Summary: {len(confirmed)} endpoints, {total_models} total models exposed")

            # Export to JSON
            outfile = "/home/lojak/Desktop/x3-chain-master/ollama_recon_results.json"
            export = [
                {
                    "ip": ep.ip,
                    "port": ep.port,
                    "version": ep.version,
                    "models": ep.models,
                    "running_models": ep.running_models,
                    "response_time_ms": ep.response_time_ms,
                    "source": ep.source,
                }
                for ep in confirmed
            ]
            with open(outfile, "w") as f:
                json.dump(export, f, indent=2)
            print(f"\n  [+] Results saved to: {outfile}")
        else:
            print("  No confirmed open Ollama instances from scraped IPs.")
            print("  Try the Google/Shodan dork URLs above in a browser for more results.")
    else:
        print_section("No IPs to validate")
        print("  Use the dork URLs above in your browser to find candidates,")
        print("  then run: python scripts/ollama_recon.py --validate-only IP1,IP2,IP3")

    print(f"\n{'═'*60}")
    print("  Scan complete. Stay ethical. 🛡️")
    print(f"{'═'*60}\n")


if __name__ == "__main__":
    main()
