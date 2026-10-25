"""Merge the control node's VS Code settings into this node's.

argv[1] is the VS Code CLI on this node (code-insiders or code), which picks the settings file;
argv[2] is the control node's settings.json, base64. The control node's values win; keys only this
node has are kept. A settings file that is not plain JSON (comments, trailing commas) is never
rewritten: it is reported as SKIPPED so nothing hand-written is lost. Prints CHANGED, UNCHANGED or
SKIPPED."""
import base64
import datetime as dt
import json
import os
import shutil
import sys

DIRS = {"code-insiders": "~/.config/Code - Insiders/User", "code": "~/.config/Code/User"}


def main():
    source = json.loads(base64.b64decode(sys.argv[2]).decode())
    path = os.path.join(os.path.expanduser(DIRS[sys.argv[1]]), "settings.json")
    current = {}
    if os.path.exists(path):
        try:
            current = json.load(open(path))
        except ValueError:
            print(f"SKIPPED {path} is not plain JSON")
            return
    merged = {**current, **source}
    if merged == current:
        print("UNCHANGED")
        return
    os.makedirs(os.path.dirname(path), exist_ok=True)
    if os.path.exists(path):
        shutil.copy2(path, f"{path}.bak-{dt.datetime.now().strftime('%Y%m%d-%H%M%S')}")
    with open(path, "w") as f:
        json.dump(merged, f, indent=4)
        f.write("\n")
    print(f"CHANGED {len([k for k in source if current.get(k) != source[k]])} keys")


if __name__ == "__main__":
    main()
