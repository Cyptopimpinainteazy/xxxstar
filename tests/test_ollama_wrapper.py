import importlib
import os


def reload_ollama_module_with_env(env: dict):
    """Set environment variables and reload scripts.ollama_server for testing."""
    for k, v in env.items():
        if v is None:
            os.environ.pop(k, None)
        else:
            os.environ[k] = v
    # import lazily and reload so module-level parsing picks up env changes
    import scripts.ollama_server as mod
    importlib.reload(mod)
    return mod


def test_rewrite_qwen_uses_default_fallback():
    mod = reload_ollama_module_with_env({"OLLAMA_EMBEDDING_ALIAS_MAP": "", "OLLAMA_EMBEDDING_FALLBACK_MODEL": ""})
    payload = {"model": "qwen2.5-coder:14b", "input": "hello"}
    out = mod.rewrite_embedding_payload(payload.copy())
    assert out["model"] == "mxbai-embed-large"


def test_rewrite_preserves_explicit_embedding_model():
    mod = reload_ollama_module_with_env({"OLLAMA_EMBEDDING_ALIAS_MAP": ""})
    payload = {"model": "mxbai-embed-large", "input": "x"}
    out = mod.rewrite_embedding_payload(payload.copy())
    assert out["model"] == "mxbai-embed-large"


def test_no_rewrite_for_non_qwen_model():
    mod = reload_ollama_module_with_env({})
    payload = {"model": "gpt-4o", "input": "x"}
    out = mod.rewrite_embedding_payload(payload.copy())
    assert out["model"] == "gpt-4o"


def test_alias_map_overrides_qwen():
    mod = reload_ollama_module_with_env({"OLLAMA_EMBEDDING_ALIAS_MAP": "qwen:qwen-embed-large"})
    payload = {"model": "qwen2.5-coder:14b"}
    out = mod.rewrite_embedding_payload(payload.copy())
    assert out["model"] == "qwen-embed-large"


def test_alias_map_json_parsing():
    mod = reload_ollama_module_with_env({"OLLAMA_EMBEDDING_ALIAS_MAP": '{"qwen":"qwen-embed-json"}'})
    payload = {"model": "qwen2.5"}
    out = mod.rewrite_embedding_payload(payload.copy())
    assert out["model"] == "qwen-embed-json"
