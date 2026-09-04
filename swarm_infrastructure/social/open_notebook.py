from __future__ import annotations

import logging
from typing import Any, Dict, List, Optional

import requests

logger = logging.getLogger(__name__)


class OpenNotebookClient:
    def __init__(self, base_url: str) -> None:
        self.base_url = base_url.rstrip("/")
        self.session = requests.Session()

    def _get(self, path: str, params: Optional[Dict[str, Any]] = None, timeout: int = 10) -> Dict[str, Any]:
        url = f"{self.base_url}{path}"
        resp = self.session.get(url, params=params, timeout=timeout)
        resp.raise_for_status()
        return resp.json()

    def _post(self, path: str, payload: Dict[str, Any], timeout: int = 20) -> Dict[str, Any]:
        url = f"{self.base_url}{path}"
        resp = self.session.post(url, json=payload, timeout=timeout)
        resp.raise_for_status()
        return resp.json()

    def get_default_models(self) -> Dict[str, Any]:
        try:
            return self._get("/api/models/defaults")
        except Exception as exc:
            logger.warning("Open Notebook defaults lookup failed: %s", exc)
            return {}

    def ask_simple(
        self,
        question: str,
        strategy_model: str,
        answer_model: str,
        final_answer_model: str,
    ) -> Optional[str]:
        payload = {
            "question": question,
            "strategy_model": strategy_model,
            "answer_model": answer_model,
            "final_answer_model": final_answer_model,
        }
        try:
            response = self._post("/api/search/ask/simple", payload)
            return response.get("answer")
        except Exception as exc:
            logger.warning("Open Notebook ask/simple failed: %s", exc)
            return None

    def search(self, query: str, limit: int = 5) -> List[Dict[str, Any]]:
        payload = {
            "query": query,
            "type": "text",
            "limit": limit,
            "search_sources": True,
            "search_notes": True,
        }
        try:
            response = self._post("/api/search", payload)
            return response.get("results") or []
        except Exception as exc:
            logger.warning("Open Notebook search failed: %s", exc)
            return []
