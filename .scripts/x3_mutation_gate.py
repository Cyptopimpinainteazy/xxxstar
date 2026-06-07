#!/usr/bin/env python3
from pathlib import Path
import subprocess
import sys

DANGER_PREFIXES = (
    "runtime/",
    "pallets/",
    "crates/",
    "contracts/",
    "X3-contracts/",
    "genesis/",
    "chain-spec/",
    "chain-specs/",
    "deployment/",
    "launch-gates/",
    "node/",
)


def lines_for(cmd):
    return subprocess.check_output(cmd, shell=True, text=True).splitlines()


changed = set(lines_for("git diff --name-only || true"))
staged = set(lines_for("git diff --cached --name-only || true"))
untracked = set(lines_for("git ls-files --others --exclude-standard || true"))
paths = sorted(changed | staged | untracked)
danger = [p for p in paths if p.startswith(DANGER_PREFIXES)]

report = Path(".x3/reports/MUTATION_GATE.md")
report.parent.mkdir(parents=True, exist_ok=True)

out = ["# X3 Mutation Gate", ""]
out.append(f"- Changed/staged/untracked paths: {len(paths)}")
out.append(f"- Danger-zone paths: {len(danger)}")
out.append("")

if danger:
    out.append("## DANGER ZONE MODIFIED")
    out.extend(f"- {p}" for p in danger[:300])
    if len(danger) > 300:
        out.append(f"- ... {len(danger) - 300} more")
    out.append("")
    out.append("Required before merge:")
    out.append("- tests")
    out.append("- risk register update")
    out.append("- rollback plan")
    out.append("- audit note")
    report.write_text("\n".join(out) + "\n")
    print(report.read_text())
    sys.exit(1)

out.append("PASS: no danger-zone changes detected.")
report.write_text("\n".join(out) + "\n")
print(report.read_text())
