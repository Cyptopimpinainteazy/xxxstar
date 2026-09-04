"""Post-task hook - executes after task completion."""
import json
from pathlib import Path

def run(context: dict) -> dict:
    """Log completion and update workspace state."""
    log_dir = Path(".claude/hooks/logs")
    log_dir.mkdir(parents=True, exist_ok=True)
    
    log_file = log_dir / "post_task.log"
    entry = {
        "task": context.get("task", "unknown"),
        "status": context.get("status", "complete"),
        "timestamp": context.get("timestamp", "")
    }
    
    with open(log_file, "a") as f:
        f.write(json.dumps(entry) + "\n")
    
    return {"continue": True, "context": context}