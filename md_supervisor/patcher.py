"""
Atomic patch engine — applies changes with full rollback support.
File-level staging, dry-run diff preview, apply → validate → commit.
Any failure triggers full rollback. No partial state.
"""
import difflib
import hashlib
import os
import shutil
from pathlib import Path
from typing import List, Optional, Tuple

from md_supervisor.schema import ChangeRequest, FileTarget, AuditEntry


class PatchError(Exception):
    """Raised when patch application fails."""


class PatchRollbackError(Exception):
    """Raised when rollback itself fails."""


def _backup_path(original: Path) -> Path:
    return original.with_suffix(original.suffix + ".bak.md_supervisor")


def create_diff(original: str, proposed: str) -> str:
    """Generate a unified diff between original and proposed content."""
    return "\n".join(difflib.unified_diff(
        original.splitlines(keepends=True),
        proposed.splitlines(keepends=True),
        fromfile="original", tofile="proposed",
    ))


def apply_change(req: ChangeRequest, dry_run: bool = False) -> List[AuditEntry]:
    """
    Apply a ChangeRequest atomically.
    1. Create backup
    2. Write new content
    3. Verify hash
    4. If any step fails → rollback all
    Returns list of audit entries for each file modified.
    """
    audit_entries: List[AuditEntry] = []
    applied: List[Tuple[Path, Path]] = []  # (original, backup)

    try:
        for ft in req.files:
            target = Path(ft.path)
            original_content = ""
            if target.exists():
                original_content = target.read_text(encoding="utf-8", errors="replace")
                original_hash = hashlib.sha256(original_content.encode()).hexdigest()
            else:
                original_hash = ""

            # Create backup
            if target.exists():
                bak = _backup_path(target)
                shutil.copy2(target, bak)
                applied.append((target, bak))

            # Write new content
            if not dry_run:
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_text(ft.proposed_content, encoding="utf-8")

            # Verify
            written = target.read_text(encoding="utf-8", errors="replace")
            written_hash = hashlib.sha256(written.encode()).hexdigest()
            if written_hash == ft.original_hash:
                raise PatchError(f"File {ft.path} was not modified (hash unchanged)")

            audit = AuditEntry(
                agent_id="md_supervisor",
                intent="patch",
                files=[ft.path],
                outcome="applied" if not dry_run else "dry_run",
                hash=written_hash,
            )
            audit_entries.append(audit)

        return audit_entries

    except Exception as e:
        # Rollback all applied changes
        for target, backup in reversed(applied):
            try:
                if backup.exists():
                    shutil.copy2(backup, target)
                    backup.unlink()
            except Exception as rb_e:
                raise PatchRollbackError(
                    f"Failed to rollback {target}: {rb_e}. Original error: {e}"
                ) from rb_e
        raise PatchError(f"Patch failed and rolled back: {e}") from e


def rollback(req: ChangeRequest) -> List[AuditEntry]:
    """Rollback a previously applied change by restoring backups."""
    audit_entries: List[AuditEntry] = []

    for ft in req.files:
        target = Path(ft.path)
        bak = _backup_path(target)

        if not bak.exists():
            audit_entries.append(AuditEntry(
                agent_id="md_supervisor",
                intent="rollback",
                files=[ft.path],
                outcome="no_backup_found",
            ))
            continue

        shutil.copy2(bak, target)
        bak.unlink()

        audit_entries.append(AuditEntry(
            agent_id="md_supervisor",
            intent="rollback",
            files=[ft.path],
            outcome="rolled_back",
        ))

    return audit_entries


def preview_diff(req: ChangeRequest) -> str:
    """Generate a human-readable diff preview for a change request."""
    lines = [f"=== Diff Preview: {req.id} ==="]
    for ft in req.files:
        target = Path(ft.path)
        original = target.read_text(encoding="utf-8", errors="replace") if target.exists() else ""
        diff = create_diff(original, ft.proposed_content)
        lines.append(f"File: {ft.path}")
        lines.append(diff)
    return "\n".join(lines)