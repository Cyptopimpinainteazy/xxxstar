from __future__ import annotations

import json
import logging
from typing import Any, Dict

import requests

logger = logging.getLogger(__name__)


def generate_text(host: str, model: str, prompt: str, options: Dict[str, Any] | None = None, timeout: int = 60) -> str:
    url = host.rstrip("/") + "/api/generate"
    payload: Dict[str, Any] = {
        "model": model,
        "prompt": prompt,
        "stream": False,
    }
    if options:
        payload["options"] = options

    resp = requests.post(url, json=payload, timeout=timeout)
    resp.raise_for_status()
    data = resp.json()
    if isinstance(data, dict):
        return data.get("response") or ""
    return json.dumps(data)
