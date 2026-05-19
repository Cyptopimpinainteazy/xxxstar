#!/usr/bin/env bash
# No-P0-blockers hard invariant: .launchops/blockers.json must remain empty.
# The LaunchOps scanner emits this list for launch-critical blockers; if it
# ever becomes non-empty, mainnet cannot ship. This gate turns that into a
# hard CI failure with no baseline escape hatch — fixing a blocker means
# resolving it, not accepting it.
set -euo pipefail

blockers=${BLOCKERS_FILE:-.launchops/blockers.json}

if [[ ! -f "$blockers" ]]; then
  echo "::error::missing $blockers; run 'cargo run -p launchops -- scan' first"
  exit 1
fi

python3 - "$blockers" <<'PY'
import json
import sys

blockers_path = sys.argv[1]

with open(blockers_path, encoding="utf-8") as handle:
    data = json.load(handle)

if isinstance(data, dict):
    items = data.get("blockers") or data.get("items") or []
elif isinstance(data, list):
    items = data
else:
    print(f"::error::unexpected shape for {blockers_path}: {type(data).__name__}")
    sys.exit(1)

if not items:
    print("LaunchOps blockers: OK (0 launch-critical blockers)")
    sys.exit(0)

print(f"::error::LaunchOps reports {len(items)} launch-critical blocker(s); mainnet cannot ship while any blocker is open")
for i, item in enumerate(items[:20], 1):
    if isinstance(item, dict):
        sev = item.get("severity", "?")
        title = (
            item.get("title")
            or item.get("reason")
            or item.get("message")
            or item.get("description")
            or "<no title>"
        )
        loc = ""
        if item.get("file"):
            loc = f" @ {item['file']}"
            if item.get("line"):
                loc += f":{item['line']}"
        print(f"  {i}. [{sev}] {title}{loc}")
    else:
        print(f"  {i}. {item}")
if len(items) > 20:
    print(f"  ... and {len(items) - 20} more")
sys.exit(1)
PY
