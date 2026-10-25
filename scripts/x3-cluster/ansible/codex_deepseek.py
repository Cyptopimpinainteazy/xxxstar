"""Point this node's Codex at DeepSeek with the control node's provider and model settings.

argv[1] is a 0600 copy of the control node's ~/.codex/config.toml; it is deleted after reading. Only the top-level model keys and the
[model_providers.deepseek] table are taken from it; everything else in this node's config (projects,
hooks, plugins) is kept. Prints CHANGED or UNCHANGED; never prints the token."""
import datetime as dt
import os
import re
import shutil
import sys

KEYS = ("model", "model_provider", "model_catalog_json", "model_reasoning_effort")
TABLE = re.compile(r"^\[model_providers\.deepseek\]\n(?:(?!\[).*\n?)*", re.M)


def top_keys(text):
    head = text.split("\n[", 1)[0]
    return {k: m.group(0) for k in KEYS for m in [re.search(rf"^{k} = .*$", head, re.M)] if m}


def main():
    staged = os.path.expanduser(sys.argv[1])
    try:
        source = open(staged).read()
    finally:
        if os.path.exists(staged):
            os.remove(staged)
    provider = TABLE.search(source)
    keys = top_keys(source)
    if not provider or keys.get("model_provider", "").split("=")[-1].strip() != '"deepseek"':
        sys.exit("control node's Codex config has no deepseek provider selected")
    path = os.path.expanduser("~/.codex/config.toml")
    os.makedirs(os.path.dirname(path), exist_ok=True)
    text = open(path).read() if os.path.exists(path) else ""
    body = TABLE.sub("", text)
    for k in KEYS:
        body = re.sub(rf"^{k} = .*\n?", "", body, flags=re.M)
    new = "\n".join(keys[k] for k in KEYS if k in keys) + "\n" + body.strip("\n") + "\n\n" + provider.group(0).rstrip("\n") + "\n"
    if new == text:
        print("UNCHANGED")
        return
    if text:
        shutil.copy2(path, f"{path}.bak-{dt.datetime.now().strftime('%Y%m%d-%H%M%S')}")
    with open(path, "w") as f:
        f.write(new)
    os.chmod(path, 0o600)
    print("CHANGED")


if __name__ == "__main__":
    main()
