"""
Change deduplication engine — hash-based and semantic diffing.
Ensures only the latest, most relevant changes are applied.
"""
import hashlib
from typing import List, Dict, Optional
from datetime import datetime

from md_supervisor.schema import ChangeRequest


def deduplicate(requests: List[ChangeRequest]) -> List[ChangeRequest]:
    """
    Main deduplication pipeline:
    1. Remove exact duplicates (content_hash match)
    2. Merge semantic duplicates (semantic_hash match, keep latest)
    3. Resolve supersedes relations
    4. Sort by priority then recency
    """
    if not requests:
        return []

    # Phase 1: Exact dedup - content_hash
    exact_dedup: Dict[str, ChangeRequest] = {}
    for req in requests:
        key = req.content_hash
        if key in exact_dedup:
            # Keep the later one
            if req.timestamp > exact_dedup[key].timestamp:
                exact_dedup[key] = req
        else:
            exact_dedup[key] = req

    # Phase 2: Semantic dedup - merge semantically identical
    semantic_groups: Dict[str, List[ChangeRequest]] = {}
    for req in exact_dedup.values():
        key = req.semantic_hash
        semantic_groups.setdefault(key, []).append(req)

    merged: List[ChangeRequest] = []
    for key, group in semantic_groups.items():
        if len(group) == 1:
            merged.append(group[0])
        else:
            # Keep latest, merge file lists
            latest = max(group, key=lambda r: r.timestamp)
            latest.files = _merge_file_targets([r.files[0] for r in group if r.files])
            latest.supersedes = [r.id for r in group if r.id != latest.id]
            merged.append(latest)

    # Phase 3: Resolve supersedes chain
    resolved = _resolve_supersedes(merged)

    # Phase 4: Sort by priority (desc) then recency (desc)
    resolved.sort(key=lambda r: (-r.priority, r.timestamp), reverse=True)

    return resolved


def _merge_file_targets(targets: list) -> list:
    """Merge file targets for semantically identical changes, keeping latest content."""
    seen: Dict[str, object] = {}
    for ft in targets:
        seen[ft.path] = ft  # last writer wins
    return list(seen.values())


def _resolve_supersedes(requests: List[ChangeRequest]) -> List[ChangeRequest]:
    """Remove changes that are explicitly superseded by newer ones."""
    superseded_ids = set()
    for req in requests:
        for sid in req.supersedes:
            superseded_ids.add(sid)
        if req.superseded_by:
            superseded_ids.add(req.id)

    return [r for r in requests if r.id not in superseded_ids]


def is_conflict(a: ChangeRequest, b: ChangeRequest) -> bool:
    """Check if two changes conflict (modify same file/symbol)."""
    a_paths = {ft.path for ft in a.files}
    b_paths = {ft.path for ft in b.files}
    return bool(a_paths & b_paths)


def prioritize_conflicts(conflicts: List[List[ChangeRequest]]) -> List[ChangeRequest]:
    """Resolve conflicts by priority + timestamp. Returns winning changes."""
    winners: List[ChangeRequest] = []
    for group in conflicts:
        winner = max(group, key=lambda r: (r.priority, r.timestamp))
        winner.supersedes = [r.id for r in group if r.id != winner.id]
        winners.append(winner)
    return winners