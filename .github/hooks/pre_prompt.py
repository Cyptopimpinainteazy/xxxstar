"""Pre-prompt hook - executes before prompt rendering."""
from pathlib import Path

def run(context: dict) -> dict:
    """Modify context or inject instructions before prompt renders."""
    # Load mode from workspace state if available
    state_file = Path(".claude/state.json")
    if state_file.exists():
        import json
        with open(state_file) as f:
            state = json.load(f)
        mode = state.get("mode", "implement")
        context["mode"] = mode
    
    return {"continue": True, "context": context}