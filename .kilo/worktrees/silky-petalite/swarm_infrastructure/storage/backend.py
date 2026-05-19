"""Abstract storage backend for all AGI substrate layers.

Defines the ``StorageBackend`` protocol and concrete implementations:
- ``SqliteStorage``  — local dev & testing (zero external deps beyond stdlib)
- ``PostgresStorage`` — production (requires asyncpg or similar)

Every AGI layer (Self-Model, Goal Genome, World Sim, Self-Improvement)
uses this interface so persistence backends are swappable.
"""

from __future__ import annotations

import json
import logging
import sqlite3
from abc import ABC, abstractmethod
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

logger = logging.getLogger(__name__)


class StorageBackend(ABC):
    """Protocol every storage implementation must satisfy."""

    @abstractmethod
    def save(self, namespace: str, key: str, data: Dict[str, Any]) -> None:
        """Persist *data* under *namespace*/*key*."""

    @abstractmethod
    def load(self, namespace: str, key: str) -> Optional[Dict[str, Any]]:
        """Load the object stored at *namespace*/*key*, or ``None``."""

    @abstractmethod
    def delete(self, namespace: str, key: str) -> bool:
        """Delete *namespace*/*key*.  Return True if it existed."""

    @abstractmethod
    def list_keys(self, namespace: str) -> List[str]:
        """Return all keys in *namespace*."""

    @abstractmethod
    def save_many(
        self, namespace: str, items: List[Tuple[str, Dict[str, Any]]]
    ) -> None:
        """Batch-persist ``[(key, data), …]``."""

    @abstractmethod
    def load_many(
        self, namespace: str, keys: List[str]
    ) -> List[Optional[Dict[str, Any]]]:
        """Batch-load; returns list aligned with *keys* (``None`` for missing)."""

    @abstractmethod
    def query(
        self,
        namespace: str,
        filters: Optional[Dict[str, Any]] = None,
        order_by: Optional[str] = None,
        limit: Optional[int] = None,
    ) -> List[Dict[str, Any]]:
        """Simple predicate query against a namespace."""


# ---------------------------------------------------------------------------
# SQLite implementation
# ---------------------------------------------------------------------------


class SqliteStorage(StorageBackend):
    """SQLite-backed storage.  One file, zero external deps."""

    def __init__(self, db_path: str = ":memory:") -> None:
        self._db_path = db_path
        self._conn = sqlite3.connect(db_path)
        self._conn.execute("PRAGMA journal_mode=WAL")
        self._conn.execute(
            """
            CREATE TABLE IF NOT EXISTS kv (
                namespace TEXT NOT NULL,
                key       TEXT NOT NULL,
                data      TEXT NOT NULL,
                PRIMARY KEY (namespace, key)
            )
            """
        )
        self._conn.commit()

    def save(self, namespace: str, key: str, data: Dict[str, Any]) -> None:
        self._conn.execute(
            "INSERT OR REPLACE INTO kv (namespace, key, data) VALUES (?, ?, ?)",
            (namespace, key, json.dumps(data, default=str)),
        )
        self._conn.commit()

    def load(self, namespace: str, key: str) -> Optional[Dict[str, Any]]:
        row = self._conn.execute(
            "SELECT data FROM kv WHERE namespace = ? AND key = ?",
            (namespace, key),
        ).fetchone()
        if row is None:
            return None
        return json.loads(row[0])

    def delete(self, namespace: str, key: str) -> bool:
        cur = self._conn.execute(
            "DELETE FROM kv WHERE namespace = ? AND key = ?",
            (namespace, key),
        )
        self._conn.commit()
        return cur.rowcount > 0

    def list_keys(self, namespace: str) -> List[str]:
        rows = self._conn.execute(
            "SELECT key FROM kv WHERE namespace = ?", (namespace,)
        ).fetchall()
        return [r[0] for r in rows]

    def save_many(
        self, namespace: str, items: List[Tuple[str, Dict[str, Any]]]
    ) -> None:
        self._conn.executemany(
            "INSERT OR REPLACE INTO kv (namespace, key, data) VALUES (?, ?, ?)",
            [(namespace, k, json.dumps(v, default=str)) for k, v in items],
        )
        self._conn.commit()

    def load_many(
        self, namespace: str, keys: List[str]
    ) -> List[Optional[Dict[str, Any]]]:
        return [self.load(namespace, key) for key in keys]

    def query(
        self,
        namespace: str,
        filters: Optional[Dict[str, Any]] = None,
        order_by: Optional[str] = None,
        limit: Optional[int] = None,
    ) -> List[Dict[str, Any]]:
        rows = self._conn.execute(
            "SELECT data FROM kv WHERE namespace = ?", (namespace,)
        ).fetchall()

        results: List[Dict[str, Any]] = []
        for (raw,) in rows:
            obj = json.loads(raw)
            if filters:
                if not all(_deep_get(obj, k) == v for k, v in filters.items()):
                    continue
            results.append(obj)

        if order_by:
            results.sort(key=lambda o: _deep_get(o, order_by) or "")

        if limit is not None:
            results = results[:limit]

        return results

    def close(self) -> None:
        self._conn.close()


# ---------------------------------------------------------------------------
# PostgreSQL stub
# ---------------------------------------------------------------------------


class PostgresStorage(StorageBackend):
    """Placeholder for a PostgreSQL-backed storage adapter.

    This class intentionally fails fast instead of silently using in-memory
    SQLite, which would discard data and mask production misconfiguration.
    Use `swarm.storage.pg_store` for the concrete Postgres integration.
    """

    def __init__(self, dsn: str = "") -> None:
        raise NotImplementedError(
            "PostgresStorage in swarm.storage.backend is not implemented. "
            "Use swarm.storage.pg_store or a concrete StorageBackend instead."
        )

    def save(self, namespace: str, key: str, data: Dict[str, Any]) -> None:
        raise NotImplementedError

    def load(self, namespace: str, key: str) -> Optional[Dict[str, Any]]:
        raise NotImplementedError

    def delete(self, namespace: str, key: str) -> bool:
        raise NotImplementedError

    def list_keys(self, namespace: str) -> List[str]:
        raise NotImplementedError

    def save_many(
        self, namespace: str, items: List[Tuple[str, Dict[str, Any]]]
    ) -> None:
        raise NotImplementedError

    def load_many(
        self, namespace: str, keys: List[str]
    ) -> List[Optional[Dict[str, Any]]]:
        raise NotImplementedError

    def query(
        self,
        namespace: str,
        filters: Optional[Dict[str, Any]] = None,
        order_by: Optional[str] = None,
        limit: Optional[int] = None,
    ) -> List[Dict[str, Any]]:
        raise NotImplementedError


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _deep_get(obj: Any, path: str) -> Any:
    """Dot-notation access into nested dicts."""
    parts = path.split(".")
    current = obj
    for part in parts:
        if isinstance(current, dict):
            current = current.get(part)
        else:
            return None
    return current
