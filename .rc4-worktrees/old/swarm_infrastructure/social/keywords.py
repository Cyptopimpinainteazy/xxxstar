from __future__ import annotations

from typing import Iterable, List


def normalize_keyword(term: str) -> str:
    return " ".join(term.strip().lower().split())


def expand_keywords(base: Iterable[str], extra: Iterable[str] | None = None, limit: int = 100) -> List[str]:
    merged = []
    seen = set()

    for term in list(base) + list(extra or []):
        normalized = normalize_keyword(term)
        if not normalized:
            continue
        if normalized in seen:
            continue
        seen.add(normalized)
        merged.append(normalized)
        if len(merged) >= limit:
            break

    return merged
