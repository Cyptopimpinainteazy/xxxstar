import os
import json
from typing import Dict, Optional

try:
    import requests
except Exception:  # pragma: no cover - requests may be installed separately
        try:
            import shutil, subprocess

            if shutil.which("ollama") is None:
                raise RuntimeError("No Ollama HTTP endpoint and `ollama` CLI not found on PATH")

            # Ollama CLI usage: `ollama run MODEL PROMPT --format json`
            cmd = ["ollama", "run", model, prompt, "--format", "json", "--think=false", "--hidethinking"]
            proc = subprocess.run(cmd, check=True, capture_output=True, text=True, timeout=120)
            out = proc.stdout.strip()

            # Parse potentially streaming/concatenated JSON objects
            parsed = _parse_streaming_json(out)
            parts = []
            if parsed:
                for obj in parsed:
                    if isinstance(obj, dict):
                        # common keys observed: 'response', 'text', or single-key maps
                        if 'response' in obj:
                            parts.append(obj.get('response') or '')
                        elif 'text' in obj:
                            parts.append(obj.get('text') or '')
                        elif len(obj) == 1:
                            parts.append(str(list(obj.values())[0] or ''))
                        else:
                            parts.append(json.dumps(obj))
                    else:
                        parts.append(str(obj))

                # Join parts into a single string; preserve order
                joined = ''.join(str(p) for p in parts).strip()
                if joined:
                    return joined

            # Fallback: if we couldn't parse JSON pieces, try a raw JSON load
            try:
                j = json.loads(out)
                if isinstance(j, dict) and len(j) == 1:
                    return str(list(j.values())[0])
                return json.dumps(j)
            except Exception:
                return out
        except Exception as e:
            raise RuntimeError(f"Failed to generate with Ollama: {e}")
        try:
            obj = json.loads(candidate)
            objs.append(obj)
        except Exception:
            # ignore parse errors for this candidate and continue
            pass

    return objs


DEFAULT_SKILLS_DIR = os.path.join(os.path.dirname(__file__), "..", "third_party", "agent-skills", "skills")


def load_skills(skills_dir: Optional[str] = None) -> Dict[str, str]:
    """Load skills from a skills directory. Returns mapping skill_name -> content (SKILL.md).

    By default loads from `third_party/agent-skills/skills` relative to repository root.
    """
    if skills_dir is None:
        skills_dir = os.path.abspath(DEFAULT_SKILLS_DIR)
    skills = {}
    if not os.path.isdir(skills_dir):
        return skills
    for entry in os.listdir(skills_dir):
        path = os.path.join(skills_dir, entry)
        if os.path.isdir(path):
            skill_md = os.path.join(path, "SKILL.md")
            if os.path.isfile(skill_md):
                try:
                    with open(skill_md, "r", encoding="utf-8") as f:
                        skills[entry] = f.read()
                except Exception:
                    continue
    return skills


def ollama_generate(prompt: str, model: str = "llama2", host: str = "http://localhost:11434", max_tokens: int = 512) -> str:
    """Send a simple generate request to a local Ollama HTTP API.

    This function uses the common Ollama endpoint `/api/generate`.
    It returns the raw text response when available.
    """
    # Try HTTP API first (some Ollama installs expose an HTTP server)
    url = host.rstrip("/") + "/api/generate"
    if requests is not None:
        try:
            payload = {"model": model, "prompt": prompt, "max_tokens": max_tokens}
            resp = requests.post(url, json=payload, timeout=60)
            # If endpoint exists and succeeded, return content
            if resp.status_code == 200:
                try:
                    j = resp.json()
                    if isinstance(j, dict) and "text" in j:
                        return j.get("text") or ""
                    return json.dumps(j)
                except Exception:
                    return resp.text
            # If 404 or other error, fall through to CLI fallback
        except Exception:
            # ignore and try CLI fallback
            pass

    # Fallback: use local `ollama` CLI if available
    try:
        import shutil, subprocess

        if shutil.which("ollama") is None:
            raise RuntimeError("No Ollama HTTP endpoint and `ollama` CLI not found on PATH")

        # Ollama CLI usage: `ollama run MODEL PROMPT --format json`
        # Use flags to avoid streaming/thinking mode so the CLI returns a completed result
        cmd = ["ollama", "run", model, prompt, "--format", "json", "--think=false", "--hidethinking"]
        proc = subprocess.run(cmd, check=True, capture_output=True, text=True, timeout=120)
        out = proc.stdout.strip()
        try:
            j = json.loads(out)
            # Common CLI JSON shapes vary; if it's a dict of one key, take its value
            if isinstance(j, dict):
                # If the dict has a single key that's the generated text, return its value
                if len(j) == 1:
                    return list(j.values())[0]
                # Otherwise return the JSON string
                return json.dumps(j)
            return str(j)
        except Exception:
            return out
    except Exception as e:
        raise RuntimeError(f"Failed to generate with Ollama: {e}")


def ask_skill(skill_name: str, user_input: str, skills_dir: Optional[str] = None, model: str = "llama2") -> str:
    """Generate a response combining the `SKILL.md` content and the user input.

    Returns the generated text from Ollama.
    """
    skills = load_skills(skills_dir)
    skill = skills.get(skill_name)
    if not skill:
        raise KeyError(f"Skill not found: {skill_name}")
    prompt = f"Skill instructions:\n{skill}\n\nUser request:\n{user_input}\n\nResponse:" 
    return ollama_generate(prompt=prompt, model=model)


if __name__ == "__main__":
    import argparse

    p = argparse.ArgumentParser()
    p.add_argument("skill")
    p.add_argument("--model", default="llama2")
    p.add_argument("--host", default="http://localhost:11434")
    p.add_argument("--skills-dir", default=None)
    args = p.parse_args()
    try:
        out = ask_skill(args.skill, "Run a quick test using the skill.", skills_dir=args.skills_dir, model=args.model)
        print(out)
    except Exception as e:
        print("ERROR:", e)
