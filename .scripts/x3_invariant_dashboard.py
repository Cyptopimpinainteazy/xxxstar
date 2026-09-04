#!/usr/bin/env python3
import json
from pathlib import Path

graph_path = Path(".x3/graph/nodes.json")
if not graph_path.exists():
    raise SystemExit("missing .x3/graph/nodes.json; run .scripts/x3_graph_builder.py first")

nodes = json.loads(graph_path.read_text())
out = Path(".x3/dashboards/INVARIANT_COVERAGE.md")
out.parent.mkdir(parents=True, exist_ok=True)

CODE_PREFIXES = (
    "apps/",
    "chain-specs/",
    "contracts/",
    "crates/",
    "deployment/",
    "infra-structure/",
    "launch-gates/",
    "node/",
    "packages/",
    "pallets/",
    "runtime/",
    "scripts/",
    "X3-contracts/",
    ".scripts/",
)


def is_code_surface(node):
    path = node.get("path", "")
    return path.startswith(CODE_PREFIXES)

critical = [
    "Universal Asset Kernel",
    "X3VM / Cross-VM",
    "Bridge / Router",
    "EVM Integration",
    "SVM Integration",
    "X3 DEX",
    "DEX Liquidity",
    "Launchpad",
    "Liquidity Locks",
    "Anti-Rug Mechanics",
    "Genesis / Chain Spec",
    "Proof System",
    "TPS Benchmark Suite",
    "GPU Validator Swarm",
    "Validator / LaunchOps",
]

lines = ["# X3 Invariant Coverage Dashboard", ""]

summary = []
for feature in critical:
    files = [
        n
        for n in nodes
        if n.get("type") == "file"
        and n.get("feature") == feature
        and is_code_surface(n)
    ]
    risky = [f for f in files if f.get("risks")]
    tests = [
        f
        for f in files
        if "test" in f.get("path", "").lower()
        or f.get("path", "").endswith("_tests.rs")
        or "/tests/" in f.get("path", "")
    ]

    if not files:
        status = "MISSING"
    elif tests and not risky:
        status = "PASS"
    elif tests:
        status = "NEEDS REVIEW"
    else:
        status = "NO TEST MAPPED"

    summary.append((feature, status, len(files), len(tests), len(risky)))

for feature, status, file_count, test_count, risk_count in summary:
    lines.append(f"## {feature}")
    lines.append(f"- Status: **{status}**")
    lines.append(f"- Files: {file_count}")
    lines.append(f"- Test files found: {test_count}")
    lines.append(f"- Risky files: {risk_count}")
    lines.append("")

    risky = [
        n
        for n in nodes
        if n.get("type") == "file"
        and n.get("feature") == feature
        and is_code_surface(n)
        and n.get("risks")
    ]
    for item in risky[:25]:
        lines.append(f"  - `{item['path']}` -> {', '.join(item['risks'])}")
    if risky:
        lines.append("")

out.write_text("\n".join(lines))
print(f"wrote {out}")
