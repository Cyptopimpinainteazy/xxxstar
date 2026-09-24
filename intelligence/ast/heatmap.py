from collections import defaultdict
from intelligence.ast.ast_diff import AstNodeChange
from typing import Dict


def build_heatmap(changes: list[AstNodeChange]) -> Dict[int, float]:
    """Returns mapping: line -> accumulated heat score."""
    heat = defaultdict(float)
    for c in changes:
        if c.lineno and c.lineno > 0:
            heat[c.lineno] += c.weight
    return dict(heat)
