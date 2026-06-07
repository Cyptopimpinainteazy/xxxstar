from __future__ import annotations

import json
import logging
import os
import time
import uuid
from dataclasses import dataclass
from typing import Any, Dict, List, Optional

from swarm.social.config import load_config, get_config_value
from swarm.social.keywords import expand_keywords
from swarm.social.open_notebook import OpenNotebookClient
from swarm.social.ollama import generate_text
# Lazy sqlite_store usage; import inside functions to avoid segfault on incompatible sqlite builds

logger = logging.getLogger(__name__)

_RATE_LIMIT_STATE: Dict[str, List[float]] = {}


@dataclass
class DraftResult:
    draft_id: str
    payload: Dict[str, Any]


def _load_approved_groups(path: str) -> List[str]:
    if not path:
        return []
    if not os.path.isfile(path):
        return []
    try:
        with open(path, "r", encoding="utf-8") as f:
            content = f.read().strip()
    except Exception:
        return []

    if not content:
        return []

    if path.endswith(".json"):
        try:
            data = json.loads(content)
            if isinstance(data, list):
                return [str(item) for item in data]
        except Exception:
            return []
    return [line.strip() for line in content.splitlines() if line.strip()]


def _rate_limit_ok(network: str, max_per_hour: int) -> bool:
    now = time.time()
    window_start = now - 3600
    timestamps = _RATE_LIMIT_STATE.get(network, [])
    timestamps = [ts for ts in timestamps if ts >= window_start]
    allowed = len(timestamps) < max_per_hour
    if allowed:
        timestamps.append(now)
    _RATE_LIMIT_STATE[network] = timestamps
    return allowed


def _extract_json(text: str) -> Dict[str, Any]:
    if not text:
        return {}
    try:
        return json.loads(text)
    except Exception:
        pass

    start = text.find("{")
    end = text.rfind("}")
    if start != -1 and end != -1 and end > start:
        try:
            return json.loads(text[start : end + 1])
        except Exception:
            return {}
    return {}


def generate_social_draft(payload: Dict[str, Any]) -> DraftResult:
    config = load_config()

    networks = get_config_value(config, ["networks"], [])
    if payload.get("network") not in networks:
        raise ValueError("network_not_allowed")

    guardrails = get_config_value(config, ["guardrails"], {})
    max_actions_per_hour = int(guardrails.get("max_actions_per_hour", 60))
    if not _rate_limit_ok(payload.get("network"), max_actions_per_hour):
        try:
            from swarm.storage import sqlite_store as _sql
            _sql.append_social_audit("rate_limit", {"network": payload.get("network")})
        except Exception:
            logger.warning("Sqlite unavailable: rate_limit audit stored in fallback")
            try:
                _audit_path = '/tmp/swarm_social_audit.json'
                _aud = {}
                if os.path.exists(_audit_path):
                    with open(_audit_path) as f:
                        _aud = json.load(f)
                _aud.setdefault('events', []).append({'type':'rate_limit','network':payload.get('network'),'timestamp':int(time.time())})
                with open(_audit_path,'w') as f:
                    json.dump(_aud,f)
            except Exception:
                pass
        raise ValueError("rate_limited")

    approved_groups_path = get_config_value(config, ["approved_groups_path"], "")
    approved_groups = _load_approved_groups(approved_groups_path)

    base_keywords = get_config_value(config, ["keywords", "base"], [])
    extra_keywords = payload.get("keywords") or []
    keywords = expand_keywords(base_keywords, extra_keywords, limit=100)

    open_notebook_cfg = get_config_value(config, ["open_notebook"], {})
    onb = OpenNotebookClient(open_notebook_cfg.get("base_url", "http://localhost:5055"))

    defaults = onb.get_default_models()
    strategy_model = open_notebook_cfg.get("strategy_model") or defaults.get("default_chat_model") or ""
    answer_model = open_notebook_cfg.get("answer_model") or defaults.get("default_chat_model") or ""
    final_answer_model = open_notebook_cfg.get("final_answer_model") or defaults.get("default_chat_model") or ""

    topic = payload.get("topic") or "X3 Chain"
    intent = payload.get("intent") or "growth"

    question = (
        "Using the Open Notebook knowledge base, provide key facts about X3 Chain (X3) "
        f"relevant to {topic}. Include approved communities or influencer angles if known. "
        f"Intent: {intent}."
    )

    grounding = None
    if strategy_model and answer_model and final_answer_model:
        grounding = onb.ask_simple(question, strategy_model, answer_model, final_answer_model)

    if not grounding:
        results = onb.search(topic, limit=5)
        grounding = "\n".join(str(r.get("content") or r.get("title") or r) for r in results)

    require_grounding = bool(open_notebook_cfg.get("require_grounding", True))
    if not grounding:
        if require_grounding:
            try:
                from swarm.storage import sqlite_store as _sql
                _sql.append_social_audit("grounding_missing", {"topic": topic})
            except Exception:
                logger.warning("Sqlite unavailable: grounding_missing audit stored in fallback")
            raise ValueError("grounding_unavailable")
        grounding = "Open Notebook grounding unavailable. Use existing approved content only."

    ollama_cfg = get_config_value(config, ["ollama"], {})
    ollama_host = ollama_cfg.get("host", "http://localhost:11434")
    ollama_model = ollama_cfg.get("model", "llama3")

    prompt = (
        "You are a social outreach agent for X3 Chain (X3). Generate a draft ONLY. "
        "Do NOT claim to have posted anything. Provide output as strict JSON with keys: "
        "title, body, tags, target_profiles, target_groups, cta, disclaimer, sources. "
        f"Network: {payload.get('network')}. Action: {payload.get('action', 'post')}. "
        f"Topic: {topic}. Intent: {intent}. "
        f"Keywords: {', '.join(keywords[:30])}. "
        f"Approved groups: {', '.join(approved_groups[:20])}. "
        f"Grounding: {grounding}"
    )

    draft_text = generate_text(ollama_host, ollama_model, prompt)
    parsed = _extract_json(draft_text)

    draft_id = str(uuid.uuid4())
    draft_payload = {
        "draft_id": draft_id,
        "network": payload.get("network"),
        "action": payload.get("action", "post"),
        "topic": topic,
        "intent": intent,
        "keywords": keywords,
        "approved_groups": approved_groups,
        "grounding": grounding,
        "raw_output": draft_text,
        "draft": parsed or {},
        "created_at": time.time(),
        "status": "draft",
    }

    # Prefer Postgres -> sqlite -> fallback
    try:
        from swarm.storage import pg_store as _pg
        _pg.init_social_tables()
        _pg.save_social_draft(draft_id, draft_payload)
        _pg.append_social_audit("draft_created", {"draft_id": draft_id, "network": payload.get("network")})
    except Exception:
        try:
            from swarm.storage import sqlite_store as _sql
            _sql.init_social_tables()
            _sql.save_social_draft(draft_id, draft_payload)
            _sql.append_social_audit("draft_created", {"draft_id": draft_id, "network": payload.get("network")})
        except Exception as e:
            logger.warning(f"No DB available, falling back to file store: {e}")
            try:
                _path = '/tmp/swarm_social_drafts.json'
                _data = {}
                if os.path.exists(_path):
                    with open(_path) as f:
                        _data = json.load(f)
                _data[draft_id] = draft_payload
                with open(_path, 'w') as f:
                    json.dump(_data, f)
                # Also record simple audit
                _audit_path = '/tmp/swarm_social_audit.json'
                _aud = {}
                if os.path.exists(_audit_path):
                    with open(_audit_path) as f:
                        _aud = json.load(f)
                _aud.setdefault('events', []).append({'type':'draft_created','draft_id':draft_id,'network':payload.get('network'),'timestamp':int(time.time())})
                with open(_audit_path,'w') as f:
                    json.dump(_aud,f)
            except Exception:
                logger.exception('Failed to write fallback draft')

    return DraftResult(draft_id=draft_id, payload=draft_payload)
