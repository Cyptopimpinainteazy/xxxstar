import json
import os
from typing import Any, Dict

DEFAULT_CONFIG_PATH = os.path.join(os.path.dirname(__file__), "..", "config", "social_agent.json")


def load_config(path: str | None = None) -> Dict[str, Any]:
    config_path = path or os.environ.get("SWARM_SOCIAL_CONFIG") or DEFAULT_CONFIG_PATH
    try:
        with open(config_path, "r", encoding="utf-8") as f:
            return json.load(f)
    except FileNotFoundError:
        return {}
    except Exception:
        # Fall back to empty config to avoid hard crash in runtime
        return {}


def get_config_value(config: Dict[str, Any], keys: list[str], default: Any) -> Any:
    current: Any = config
    for key in keys:
        if not isinstance(current, dict) or key not in current:
            return default
        current = current[key]
    return current
