#!/usr/bin/env python3
from pathlib import Path
import subprocess

OUT = Path(".x3/reports/DRIFT_REPORT.md")
OUT.parent.mkdir(parents=True, exist_ok=True)


def sh(cmd):
    try:
        return subprocess.check_output(
            cmd, shell=True, text=True, stderr=subprocess.STDOUT
        )
    except subprocess.CalledProcessError as exc:
        return exc.output


changed = sh("git diff --name-only || true").splitlines()
staged = sh("git diff --cached --name-only || true").splitlines()
untracked = sh("git ls-files --others --exclude-standard || true").splitlines()
all_paths = sorted(set(changed + staged + untracked))

docs = [p for p in all_paths if p.endswith((".md", ".txt", ".rst"))]
code = [p for p in all_paths if p.endswith((".rs", ".sol", ".ts", ".tsx", ".js", ".py"))]
tests = [
    p
    for p in all_paths
    if "test" in p.lower() or "spec" in p.lower() or "/tests/" in p
]
danger = [
    p
    for p in all_paths
    if p.startswith(
        (
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
    )
]

lines = ["# X3 Drift Report", ""]
lines.append(f"- Changed/staged/untracked paths: {len(all_paths)}")
lines.append(f"- Docs: {len(docs)}")
lines.append(f"- Code: {len(code)}")
lines.append(f"- Tests: {len(tests)}")
lines.append(f"- Danger-zone paths: {len(danger)}")
lines.append("")

if docs and not code:
    lines.append("## WARNING: Docs changed but code did not")
    lines.extend(f"- {p}" for p in docs[:200])
    lines.append("")

if code and not tests:
    lines.append("## WARNING: Code changed but tests did not")
    lines.extend(f"- {p}" for p in code[:200])
    lines.append("")

if danger:
    lines.append("## DANGER ZONE CHANGES")
    lines.extend(f"- {p}" for p in danger[:300])
    if len(danger) > 300:
        lines.append(f"- ... {len(danger) - 300} more")
    lines.append("")
    lines.append("Required: audit note, tests, rollback plan, and risk register update.")
    lines.append("")

if not all_paths:
    lines.append("No git changes detected.")

OUT.write_text("\n".join(lines) + "\n")
print(f"wrote {OUT}")
