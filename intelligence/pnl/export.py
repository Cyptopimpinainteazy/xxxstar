from intelligence.pnl.ast_correlation import PnLCorrelator
from intelligence.ast.export import analyze_file
from intelligence.ast.ast_diff import AstNodeChange


def export_ast_pnl(before_src: str, after_src: str, pnl_logs: list) -> dict:
    ast_data = analyze_file(before_src, after_src)

    # rebuild AstNodeChange objects from serialized changes
    ast_changes = []
    for c in ast_data.get('changes', []):
        ast_changes.append(AstNodeChange(
            node_type=c.get('node_type'),
            lineno=c.get('lineno', -1),
            end_lineno=c.get('end_lineno'),
            col_offset=c.get('col_offset'),
            end_col_offset=c.get('end_col_offset'),
            snippet=c.get('snippet'),
            change_type=c.get('change_type'),
            depth=c.get('depth', 0),
            weight=c.get('weight', 1.0),
            hash_before=c.get('hash_before'),
            hash_after=c.get('hash_after'),
        ))

    correlator = PnLCorrelator(pnl_logs, ast_changes)
    line_pnl = correlator.correlate()

    return {
        'ast_changes': ast_data['changes'],
        'heatmap': ast_data['heatmap'],
        'pnl_correlation': line_pnl,
        'top_losses': correlator.top_losses()
    }