import json
from intelligence.ast.ast_diff import AstDiffer
from intelligence.ast.heatmap import build_heatmap
from pathlib import Path
from datetime import datetime

AST_HEATMAP_FILE = Path('.md_supervisor/ast_heatmaps.json')


def analyze_file(before: str, after: str, file_path: str | None = None) -> dict:
    differ = AstDiffer()
    changes = differ.diff(before, after)
    heat = build_heatmap(changes)
    # Serialize changes with safe dicts (dataclasses may have non-serializable types)
    serialized_changes = []
    for c in changes:
        serialized_changes.append({
            'node_type': c.node_type,
            'lineno': c.lineno,
            'end_lineno': c.end_lineno,
            'col_offset': c.col_offset,
            'end_col_offset': c.end_col_offset,
            'snippet': c.snippet,
            'change_type': c.change_type,
            'depth': c.depth,
            'weight': c.weight,
            'hash_before': c.hash_before,
            'hash_after': c.hash_after,
        })

    payload = {
        'file': file_path or '<memory>',
        'time': datetime.utcnow().isoformat(),
        'changes': serialized_changes,
        'heatmap': heat,
        'total_heat': sum(heat.values())
    }

    return payload


def persist_analysis(analysis: dict):
    AST_HEATMAP_FILE.parent.mkdir(parents=True, exist_ok=True)
    content = []
    if AST_HEATMAP_FILE.exists():
        try:
            content = json.loads(AST_HEATMAP_FILE.read_text())
        except Exception:
            content = []
    content.append(analysis)
    # Write as an object with top-level `heatmaps` key so consumers (UI) can easily read it
    AST_HEATMAP_FILE.write_text(json.dumps({'heatmaps': content}, indent=2))


def analyze_and_persist(before: str, after: str, file_path: str | None = None) -> dict:
    a = analyze_file(before, after, file_path=file_path)
    persist_analysis(a)
    return a
