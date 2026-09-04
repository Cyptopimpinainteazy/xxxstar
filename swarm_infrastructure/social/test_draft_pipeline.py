import json
import os
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

from swarm.social import draft_pipeline


def test_generate_social_draft(monkeypatch, tmp_path):
    db_path = tmp_path / "agents.db"
    os.environ["AGENT_DB_PATH"] = str(db_path)

    config_path = tmp_path / "social_agent.json"
    config_path.write_text(
        json.dumps(
            {
                "version": 1,
                "mode": {"draft_only": True, "live_actions_enabled": False},
                "networks": ["facebook"],
                "actions": ["post"],
                "keywords": {"base": ["blockchain"]},
                "open_notebook": {
                    "base_url": "http://localhost:5055",
                    "strategy_model": "m1",
                    "answer_model": "m1",
                    "final_answer_model": "m1",
                    "require_grounding": True,
                },
                "ollama": {"host": "http://localhost:11434", "model": "llama3"},
                "guardrails": {"max_actions_per_hour": 10, "max_targets_per_task": 5},
            }
        )
    )
    os.environ["SWARM_SOCIAL_CONFIG"] = str(config_path)

    def fake_ask_simple(self, question, strategy_model, answer_model, final_answer_model):
        return "X3 Chain is a dual-VM chain."

    def fake_generate_text(host, model, prompt, options=None, timeout=60):
        return json.dumps(
            {
                "title": "Hello", 
                "body": "Draft body", 
                "tags": ["#x3"],
                "target_profiles": [],
                "target_groups": [],
                "cta": "Learn more",
                "disclaimer": "Draft only",
                "sources": ["open-notebook"]
            }
        )

    monkeypatch.setattr(draft_pipeline.OpenNotebookClient, "ask_simple", fake_ask_simple)
    monkeypatch.setattr(draft_pipeline, "generate_text", fake_generate_text)

    result = draft_pipeline.generate_social_draft(
        {"network": "facebook", "action": "post", "topic": "X3 Chain"}
    )

    assert result.draft_id
    assert result.payload["network"] == "facebook"
    assert result.payload["draft"]["title"] == "Hello"
