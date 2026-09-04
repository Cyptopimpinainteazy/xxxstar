# Skills integration (Ollama)

This module implements a minimal loader and Ollama client so local services can use skills from the `third_party/agent-skills` collection.

Quick usage

- Install dependencies:

```
pip install -r swarm/requirements.txt
```

- Run a quick test (requires Ollama running locally at `http://localhost:11434`):

```
python swarm/skills_adapter.py react-best-practices
```

Integration notes

- The adapter exposes `load_skills()`, `ask_skill(skill_name, user_input, skills_dir=None, model='llama2')`.
- To integrate with `swarm/api_server.py`, import `swarm.skills_adapter` and call `ask_skill(...)` from the request handler. Provide `host` or `model` overrides via environment or config if needed.
- Ollama API compatibility: this adapter posts to `/api/generate`. If your Ollama install uses a different API surface, adjust `ollama_generate()` accordingly.

Security

- Avoid sending sensitive data to models. Treat outputs as untrusted unless verified.
