#!/usr/bin/env python3
import json
import os
import re
import subprocess
from pathlib import Path

ROOT = Path(".")
OUT = ROOT / ".x3" / "graph"
OUT.mkdir(parents=True, exist_ok=True)
MAX_FILE_BYTES = 1_000_000

IGNORE_PARTS = {
    ".git",
    ".cache",
    ".reports",
    ".repomix",
    ".benchmark-logs",
    ".chopsticks-db",
    ".proof-results",
    ".srtool-reports",
    ".try-runtime-snapshots",
    "__pycache__",
    "archive",
    "bench-results",
    "ChatGPT_files",
    "out",
    "test-results",
    "target",
    "target_strict",
    "node_modules",
    "dist",
    "build",
    "coverage",
    ".venv",
}

IGNORE_FILENAMES = {
    "package-lock.json",
    "pnpm-lock.yaml",
    "yarn.lock",
}

IGNORE_PREFIXES = (
    Path(".x3/graph"),
    Path(".x3/dashboards"),
)

ALLOWED_SUFFIXES = {
    ".rs",
    ".sol",
    ".ts",
    ".tsx",
    ".js",
    ".py",
    ".toml",
    ".json",
    ".md",
    ".yaml",
    ".yml",
    ".sh",
}

PATTERNS = {
    "rust_fn": re.compile(r"\b(?:pub\s+)?(?:async\s+)?fn\s+([a-zA-Z0-9_]+)"),
    "rust_struct": re.compile(r"\b(?:pub\s+)?struct\s+([A-Za-z0-9_]+)"),
    "rust_enum": re.compile(r"\b(?:pub\s+)?enum\s+([A-Za-z0-9_]+)"),
    "sol_contract": re.compile(r"\bcontract\s+([A-Za-z0-9_]+)"),
    "sol_function": re.compile(r"\bfunction\s+([A-Za-z0-9_]+)"),
    "ts_function": re.compile(r"\b(?:export\s+)?(?:async\s+)?function\s+([A-Za-z0-9_]+)"),
    "py_function": re.compile(r"^\s*def\s+([A-Za-z0-9_]+)\s*\(", re.MULTILINE),
}

FEATURE_HINTS = {
    "canonical_supply": "Universal Asset Kernel",
    "asset": "Universal Asset Kernel",
    "supply": "Universal Asset Kernel",
    "kernel": "Universal Asset Kernel",
    "bridge": "Bridge / Router",
    "router": "Bridge / Router",
    "evm": "EVM Integration",
    "svm": "SVM Integration",
    "x3vm": "X3VM / Cross-VM",
    "cross-vm": "X3VM / Cross-VM",
    "cross_vm": "X3VM / Cross-VM",
    "vm": "X3VM / Cross-VM",
    "dex": "X3 DEX",
    "swap": "X3 DEX",
    "pool": "DEX Liquidity",
    "launchpad": "Launchpad",
    "launch": "Launchpad",
    "liquidity_lock": "Liquidity Locks",
    "liquidity-lock": "Liquidity Locks",
    "anti_rug": "Anti-Rug Mechanics",
    "anti-rug": "Anti-Rug Mechanics",
    "governance": "Governance",
    "genesis": "Genesis / Chain Spec",
    "chain_spec": "Genesis / Chain Spec",
    "chainspec": "Genesis / Chain Spec",
    "benchmark": "TPS Benchmark Suite",
    "tps": "TPS Benchmark Suite",
    "proof": "Proof System",
    "gpu": "GPU Validator Swarm",
    "validator": "Validator / LaunchOps",
}

INVARIANTS = {
    "Universal Asset Kernel": [
        "canonical_supply == native + evm + svm + external_locked + pending",
    ],
    "X3VM / Cross-VM": [
        "Either all VM legs commit or all VM legs roll back",
        "Replay and domain separation must hold across VM boundaries",
    ],
    "Bridge / Router": [
        "No message may execute twice",
        "No expired message may execute",
        "No message for a different chain/domain may execute",
    ],
    "X3 DEX": [
        "Pool reserves, LP supply, fees, and locked liquidity remain consistent",
    ],
    "Genesis / Chain Spec": [
        "No dev keys, unsafe defaults, or fake balances in production chain specs",
    ],
}

RISK_HINTS = {
    "unwrap(": "panic risk",
    "expect(": "panic risk",
    "panic!": "panic risk",
    "todo!": "stub risk",
    "unimplemented!": "stub risk",
    "TODO": "unfinished logic",
    "FIXME": "known issue",
    "unsafe": "unsafe code",
    "nonce": "replay/nonce risk",
    "replay": "replay risk",
    "bridge": "bridge risk",
    "rollback": "atomic rollback risk",
    "canonical_supply": "supply invariant risk",
    "localhost": "local-only config risk",
    "1704067200": "fake timestamp risk",
    "H256::from_low_u64_be": "mock hash risk",
}


def skip(path: Path) -> bool:
    if any(path == prefix or prefix in path.parents for prefix in IGNORE_PREFIXES):
        return True
    return any(part in IGNORE_PARTS for part in path.parts)


def iter_candidate_files(root: Path):
    try:
        result = subprocess.run(
            ["git", "ls-files", "-co", "--exclude-standard"],
            cwd=root,
            check=True,
            text=True,
            capture_output=True,
        )
        for line in result.stdout.splitlines():
            path = root / line
            if path.is_file() and not skip(path):
                yield path
        return
    except Exception:
        pass

    for dirpath, dirnames, filenames in os.walk(root):
        current = Path(dirpath)
        dirnames[:] = [
            name
            for name in dirnames
            if name not in IGNORE_PARTS and not skip(current / name)
        ]
        for filename in sorted(filenames):
            path = current / filename
            if skip(path):
                continue
            yield path


def node_id(kind: str, raw: str) -> str:
    safe = re.sub(r"[^A-Za-z0-9_.:/-]+", "_", raw)
    return f"{kind}:{safe}"


def feature_for(path: Path, text: str) -> str:
    hay = f"{path}\n{text[:5000]}".lower()
    for key, feature in FEATURE_HINTS.items():
        if key in hay:
            return feature
    return "Unclassified"


nodes = []
edges = []
feature_nodes = set()
invariant_nodes = set()
unreadable = []
skipped_large = []

for path in iter_candidate_files(ROOT):
    if not path.is_file():
        continue
    if path.suffix not in ALLOWED_SUFFIXES:
        continue
    if path.name in IGNORE_FILENAMES or path.name.endswith(".cdx.json"):
        continue
    try:
        size = path.stat().st_size
    except Exception as exc:
        unreadable.append({"path": str(path), "error": f"stat failed: {exc}"})
        continue
    if size > MAX_FILE_BYTES:
        skipped_large.append({"path": str(path), "bytes": size})
        continue

    try:
        text = path.read_text(errors="ignore")
    except Exception as exc:
        unreadable.append({"path": str(path), "error": str(exc)})
        continue

    file_id = node_id("file", str(path))
    feature = feature_for(path, text)
    feature_id = node_id("feature", feature)
    risks = sorted({risk for hint, risk in RISK_HINTS.items() if hint in text})

    nodes.append(
        {
            "id": file_id,
            "type": "file",
            "path": str(path),
            "feature": feature,
            "risks": risks,
            "loc": text.count("\n") + 1,
        }
    )

    if feature_id not in feature_nodes:
        feature_nodes.add(feature_id)
        nodes.append({"id": feature_id, "type": "feature", "name": feature})

    edges.append({"from": file_id, "to": feature_id, "relation": "belongs_to"})

    for invariant in INVARIANTS.get(feature, []):
        invariant_id = node_id("invariant", invariant)
        if invariant_id not in invariant_nodes:
            invariant_nodes.add(invariant_id)
            nodes.append({"id": invariant_id, "type": "invariant", "rule": invariant})
        edges.append({"from": feature_id, "to": invariant_id, "relation": "must_satisfy"})

    extractors = [
        ("function", PATTERNS["rust_fn"]),
        ("struct", PATTERNS["rust_struct"]),
        ("enum", PATTERNS["rust_enum"]),
        ("contract", PATTERNS["sol_contract"]),
        ("solidity_function", PATTERNS["sol_function"]),
        ("typescript_function", PATTERNS["ts_function"]),
        ("python_function", PATTERNS["py_function"]),
    ]
    for kind, pattern in extractors:
        for name in pattern.findall(text):
            child_id = node_id(kind, f"{path}:{name}")
            nodes.append({"id": child_id, "type": kind, "name": name, "file": str(path)})
            edges.append({"from": file_id, "to": child_id, "relation": "contains"})

seen = set()
deduped = []
for node in nodes:
    if node["id"] in seen:
        continue
    seen.add(node["id"])
    deduped.append(node)

(OUT / "nodes.json").write_text(json.dumps(deduped, indent=2))
(OUT / "edges.json").write_text(json.dumps(edges, indent=2))
(OUT / "unreadable.json").write_text(json.dumps(unreadable, indent=2))
(OUT / "skipped_large.json").write_text(json.dumps(skipped_large, indent=2))

index = ["# X3 Graph Index", ""]
for node in deduped:
    if node["type"] == "file":
        risks = ", ".join(node["risks"]) if node["risks"] else "none"
        index.append(
            f"- `{node['path']}` -> **{node['feature']}** | LOC: {node['loc']} | Risks: {risks}"
        )
(OUT / "index.md").write_text("\n".join(index) + "\n")

print(
    f"nodes={len(deduped)} edges={len(edges)} "
    f"unreadable={len(unreadable)} skipped_large={len(skipped_large)}"
)
print("wrote .x3/graph/nodes.json .x3/graph/edges.json .x3/graph/index.md")
