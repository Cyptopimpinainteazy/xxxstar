"""Hook runtime for x3star - wires hooks into Claude Code's hook system."""
import json
import os
from pathlib import Path
from typing import Any

HOOKS_DIR = Path(__file__).parent.parent.parent / ".github" / "hooks"

def load_hooks_config() -> dict[str, Any]:
    """Load hooks configuration from .claude/settings.json."""
    settings_path = Path(__file__).parent.parent / "settings.json"
    if settings_path.exists():
        with open(settings_path) as f:
            return json.load(f)
    return {}

def run_hook(hook_name: str, context: dict[str, Any]) -> dict[str, Any]:
    """Execute a named hook with given context."""
    config = load_hooks_config()
    hooks_config = config.get("hooks", {})
    
    if not hooks_config.get("enabled", False):
        return {"continue": True, "output": ""}
    
    hook_files = hooks_config.get(hook_name, [])
    output_parts = []
    
    for hook_file in hook_files:
        hook_path = HOOKS_DIR / Path(hook_file).name
        if hook_path.exists():
            with open(hook_path) as f:
                hook_code = f.read()
            # Hooks can modify context or return instructions
            output_parts.append(f"[{hook_name}:{hook_path.name}]")
    
    return {"continue": True, "output": "\n".join(output_parts)}

def pre_prompt_hook(context: dict[str, Any]) -> dict[str, Any]:
    """Called before prompt is rendered."""
    return run_hook("pre_prompt", context)

def post_task_hook(context: dict[str, Any]) -> dict[str, Any]:
    """Called after task completion."""
    return run_hook("post_task", context)