#!/usr/bin/env python3
"""
x3_router.py — X3 AI Command System Model Router

An intelligent proxy that sits between Cline (or any OpenAI-compatible client)
and Ollama, automatically routing requests to the right specialist model
based on the prompt content.

Usage:
  python3 x3_router.py                    # Start on port 11435
  python3 x3_router.py --port 11436      # Custom port
  python3 x3_router.py --ollama-host http://192.168.1.100:11434  # Remote Ollama

Architecture:
  Cline → x3_router (11435) → Ollama (11434)
              │
              ├── Classifies prompt
              ├── Routes to best specialist model
              └── Returns response

Cline Configuration:
  Provider: Ollama
  Base URL: http://localhost:11435
  Model: lojak/cryptomaster (router overrides per-request)
  Context Window: 32768
"""

import argparse
import json
import logging
import sys
import yaml
from http.server import HTTPServer, BaseHTTPRequestHandler
from http.client import HTTPConnection
from urllib.parse import urlparse
from pathlib import Path

# Import classifier from same directory
sys.path.insert(0, str(Path(__file__).parent))
from classifier import classify, load_registry

# Configuration defaults
DEFAULT_PORT = 11435
DEFAULT_OLLAMA_HOST = "http://localhost:11434"
DEFAULT_MODEL = "lojak/cryptomaster"
LOG_FORMAT = "%(asctime)s [%(levelname)s] %(message)s"


def load_config(config_path=None):
    """Load configuration from config.yaml if it exists."""
    if config_path is None:
        config_path = Path(__file__).parent / "config.yaml"
    
    if not Path(config_path).exists():
        return {}
    
    with open(config_path, 'r') as f:
        return yaml.safe_load(f) or {}


def get_config_value(config, key, default):
    """Get a config value with a fallback to default."""
    return config.get(key, default)

logging.basicConfig(level=logging.INFO, format=LOG_FORMAT)
logger = logging.getLogger("x3-router")


def proxy_to_ollama(method, ollama_host, path, body=None, headers=None):
    """Proxy a request to Ollama using http.client for proper handling."""
    parsed = urlparse(ollama_host)
    host = parsed.hostname
    port = parsed.port or 80

    conn = HTTPConnection(host, port, timeout=300)
    req_headers = dict(headers) if headers else {}
    req_headers["Content-Type"] = "application/json"

    try:
        conn.request(method, path, body=body, headers=req_headers)
        resp = conn.getresponse()
        status = resp.status
        resp_headers = dict(resp.getheaders())
        resp_body = resp.read()
        conn.close()
        return status, resp_headers, resp_body
    except Exception as e:
        conn.close()
        raise e


class X3RouterHandler(BaseHTTPRequestHandler):
    """HTTP request handler that routes to the right X3 specialist model."""

    # Class-level config (set by main())
    ollama_host = DEFAULT_OLLAMA_HOST
    default_model = DEFAULT_MODEL
    routing_enabled = True
    log_routing = True

    def log_message(self, format, *args):
        """Override to use our logger."""
        logger.info(format % args)

    def _route_request(self, body):
        """Classify a request body and return the target model name.

        Routing priority:
        1. X-X3-Model header → forced model (no classification)
        2. Model already a lojak/x3-* model → passthrough (Roo Code / direct spec)
        3. Keyword classification → auto-route based on prompt content
        4. Fallback → default model (lojak/cryptomaster)
        """
        if not body or not self.routing_enabled:
            return None

        try:
            data = json.loads(body)

            # Priority 1: Check for forced model via custom header
            forced_model = self.headers.get("X-X3-Model")
            if forced_model:
                if self.log_routing:
                    logger.info(f"Forced: X-X3-Model header → {forced_model}")
                return forced_model

            # Priority 2: If model is already an X3 specialist, pass through
            requested_model = data.get("model", "")
            if requested_model and requested_model.startswith("lojak/x3-"):
                if self.log_routing:
                    logger.info(f"Passthrough: model={requested_model} (already specialist)")
                return requested_model

            # Priority 3: Keyword classification
            prompt_text = self._extract_prompt(data)
            if prompt_text:
                key, model, reviewer, score = classify(prompt_text)
                if self.log_routing:
                    logger.info(f"Routed: '{prompt_text[:80]}' → {model} (score={score})")
                return model

        except (json.JSONDecodeError, Exception) as e:
            logger.warning(f"Error classifying request: {e}")

        return None

    def _extract_prompt(self, data):
        """Extract the user prompt from a request body."""
        # OpenAI chat format
        if "messages" in data:
            messages = data["messages"]
            for msg in reversed(messages):
                if msg.get("role") == "user":
                    content = msg.get("content", "")
                    if isinstance(content, list):
                        parts = []
                        for part in content:
                            if isinstance(part, dict) and part.get("type") == "text":
                                parts.append(part.get("text", ""))
                        return " ".join(parts)
                    return str(content)

        # Ollama generate format
        if "prompt" in data:
            return str(data["prompt"])

        # Ollama chat format
        if "content" in data:
            return str(data["content"])

        return ""

    def _handle_request(self, method):
        """Handle a request: classify, rewrite model, proxy to Ollama."""
        content_length = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(content_length) if content_length > 0 else None

        # Route and rewrite model
        routed_model = None
        if body:
            routed_model = self._route_request(body)
            if routed_model:
                try:
                    data = json.loads(body)
                    data["model"] = routed_model
                    body = json.dumps(data).encode()
                except (json.JSONDecodeError, Exception) as e:
                    logger.warning(f"Error rewriting model: {e}")

        # Proxy to Ollama
        try:
            # Forward original headers (except Host and Content-Length which we handle)
            forward_headers = {}
            for key in ["Authorization", "Accept", "User-Agent"]:
                val = self.headers.get(key)
                if val:
                    forward_headers[key] = val

            status, resp_headers, resp_body = proxy_to_ollama(
                method, self.ollama_host, self.path, body, forward_headers
            )

            # Send response
            self.send_response(status)

            # Forward relevant response headers
            for key, value in resp_headers.items():
                if key.lower() not in ("transfer-encoding", "connection", "content-length"):
                    self.send_header(key, value)

            # Add routing info header
            if routed_model:
                self.send_header("X-X3-Model-Routed-To", routed_model)

            # CORS headers
            self.send_header("Access-Control-Allow-Origin", "*")
            self.send_header("Access-Control-Expose-Headers", "X-X3-Model-Routed-To")

            self.send_header("Content-Length", str(len(resp_body)))
            self.end_headers()
            self.wfile.write(resp_body)

            # Log
            if routed_model:
                logger.info(f"{method} {self.path} → {routed_model} (status={status})")
            else:
                logger.info(f"{method} {self.path} (passthrough, status={status})")

        except Exception as e:
            logger.error(f"Error proxying to Ollama: {e}")
            self.send_response(502)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            error_msg = json.dumps({"error": f"Failed to connect to Ollama: {e}"}).encode()
            self.wfile.write(error_msg)

    def do_GET(self):
        """Handle GET requests."""
        # Roo Code / OpenAI-compatible models endpoint
        if self.path == "/v1/models":
            self._handle_models_list()
            return
        # All other GETs proxy to Ollama
        self._handle_request("GET")

    def do_POST(self):
        """Handle POST requests — route model based on prompt content."""
        self._handle_request("POST")

    def do_OPTIONS(self):
        """Handle CORS preflight requests."""
        self.send_response(200)
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "Content-Type, X-X3-Model, Authorization")
        self.send_header("Access-Control-Expose-Headers", "X-X3-Model-Routed-To")
        self.send_header("Access-Control-Max-Age", "86400")
        self.end_headers()

    def _handle_models_list(self):
        """Return OpenAI-compatible /v1/models list for Roo Code compatibility."""
        try:
            # Fetch models from Ollama and convert to OpenAI format
            status, headers, body = proxy_to_ollama("GET", self.ollama_host, "/api/tags")
            if status != 200:
                self.send_response(status)
                self.send_header("Content-Type", "application/json")
                self.end_headers()
                self.wfile.write(body)
                return

            ollama_data = json.loads(body)
            models = []
            for m in ollama_data.get("models", []):
                name = m.get("name", "")
                if name.startswith("lojak/"):
                    models.append({
                        "id": name,
                        "object": "model",
                        "created": 0,
                        "owned_by": "lojak"
                    })

            # Always include the default model
            default = self.default_model
            if not any(m["id"] == default for m in models):
                models.insert(0, {
                    "id": default,
                    "object": "model",
                    "created": 0,
                    "owned_by": "lojak"
                })

            response = {
                "object": "list",
                "data": sorted(models, key=lambda m: m["id"])
            }

            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Access-Control-Allow-Origin", "*")
            self.end_headers()
            self.wfile.write(json.dumps(response).encode())
            logger.info(f"GET /v1/models → {len(models)} models listed")

        except Exception as e:
            logger.error(f"Error listing models: {e}")
            self.send_response(502)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps({"error": f"Failed to list models: {e}"}).encode())


def main():
    # Load config.yaml for default values
    config = load_config()
    
    # Use config values as defaults, CLI args override config
    default_port = get_config_value(config, 'router_port', DEFAULT_PORT)
    default_ollama = get_config_value(config, 'ollama_host', DEFAULT_OLLAMA_HOST)
    default_model = get_config_value(config, 'default_model', DEFAULT_MODEL)
    
    parser = argparse.ArgumentParser(description="X3 AI Command System Model Router")
    parser.add_argument("--port", type=int, default=default_port, help=f"Port to listen on (default: {default_port})")
    parser.add_argument("--ollama-host", type=str, default=default_ollama, help=f"Ollama host URL (default: {default_ollama})")
    parser.add_argument("--default-model", type=str, default=default_model, help=f"Default model when no match (default: {default_model})")
    parser.add_argument("--no-routing", action="store_true", help="Disable routing, pass all requests through unchanged")
    parser.add_argument("--quiet", action="store_true", help="Reduce logging output")
    args = parser.parse_args()

    if args.quiet:
        logging.getLogger("x3-router").setLevel(logging.WARNING)

    # Configure handler with values from config or CLI
    X3RouterHandler.ollama_host = args.ollama_host
    X3RouterHandler.default_model = args.default_model
    X3RouterHandler.routing_enabled = not args.no_routing
    X3RouterHandler.log_routing = not args.quiet
    
    # Log config source
    if config:
        logger.info(f"Loaded config from config.yaml (config values are defaults, CLI args override)")

    # Verify Ollama is reachable
    try:
        status, headers, body = proxy_to_ollama("GET", args.ollama_host, "/api/tags")
        models = json.loads(body)
        model_count = len(models.get("models", []))
        logger.info(f"Connected to Ollama at {args.ollama_host} ({model_count} models available)")
    except Exception as e:
        logger.error(f"Cannot reach Ollama at {args.ollama_host}: {e}")
        logger.error("Make sure Ollama is running: ollama serve")
        sys.exit(1)

    # Load registry
    registry = load_registry()
    model_count = len(registry.get("models", {}))
    logger.info(f"Loaded registry with {model_count} specialist models")

    # Start server
    server = HTTPServer(("0.0.0.0", args.port), X3RouterHandler)
    logger.info(f"X3 Router listening on port {args.port}")
    logger.info(f"Routing to Ollama at {args.ollama_host}")
    logger.info(f"Default model: {args.default_model}")
    logger.info(f"Routing {'enabled' if X3RouterHandler.routing_enabled else 'disabled'}")
    logger.info("")
    logger.info("Configure Cline with:")
    logger.info(f"  Provider: Ollama")
    logger.info(f"  Base URL: http://localhost:{args.port}")
    logger.info(f"  Model: {args.default_model}")
    logger.info(f"  Context Window: 32768")
    logger.info("")
    logger.info("Configure Roo Code with:")
    logger.info(f"  Provider: OpenAI Compatible")
    logger.info(f"  Base URL: http://localhost:{args.port}/v1")
    logger.info(f"  Model: {args.default_model}")
    logger.info(f"  Context Window: 32768")
    logger.info("")
    logger.info("Custom headers:")
    logger.info(f"  X-X3-Model: lojak/x3-auditor  (force a specific model)")
    logger.info("")
    logger.info("Model passthrough: lojak/x3-* models bypass classification")
    logger.info("")
    logger.info("Press Ctrl+C to stop")

    try:
        server.serve_forever()
    except KeyboardInterrupt:
        logger.info("Shutting down...")
        server.server_close()


if __name__ == "__main__":
    main()