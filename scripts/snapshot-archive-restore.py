#!/usr/bin/env python3
"""Stage a validator archive and install its contents at the requested target."""
import os
from pathlib import Path, PurePosixPath
import shutil
import sys
import tarfile
import tempfile


class TargetOccupied(Exception):
    """The requested target, rather than the archive, prevents restoration."""


def check_target(target):
    if target.is_symlink() or (target.exists() and
            (not target.is_dir() or any(target.iterdir()))):
        raise TargetOccupied(f"restore target must be an empty directory: {target}")


def restore(archive, target):
    target = Path(os.path.abspath(target))
    check_target(target)
    target_stat = target.stat() if target.exists() else None
    with tarfile.open(archive, "r:gz") as source:
        members = source.getmembers()
        roots = set()
        entries = []
        seen = set()
        for member in members:
            path = PurePosixPath(member.name)
            if path.is_absolute() or ".." in path.parts or not path.parts:
                raise ValueError(f"unsafe archive path: {member.name}")
            if not (member.isdir() or member.isfile()):
                raise ValueError(f"unsupported archive entry: {member.name}")
            roots.add(path.parts[0])
            relative = Path(*path.parts[1:])
            if not path.parts[1:] and not member.isdir():
                raise ValueError("archive must contain one top-level directory")
            if relative in seen:
                raise ValueError(f"duplicate archive path: {member.name}")
            seen.add(relative)
            entries.append((member, relative))
        if len(roots) != 1:
            raise ValueError("archive must contain one top-level directory")
        target.parent.mkdir(parents=True, exist_ok=True)
        staging = Path(tempfile.mkdtemp(prefix=".x3-restore-", dir=target.parent))
        root_member = next((member for member, relative in entries if not relative.parts), None)
        owner = ((target_stat.st_uid, target_stat.st_gid) if target_stat else
                 (root_member.uid, root_member.gid) if root_member else (os.getuid(), os.getgid()))
        root_mode = (target_stat.st_mode & 0o777 if target_stat else
                     root_member.mode & 0o777 if root_member else 0o700)
        try:
            for member, relative in entries:
                destination = staging / relative
                if member.isdir():
                    destination.mkdir(parents=True, exist_ok=True)
                else:
                    destination.parent.mkdir(parents=True, exist_ok=True)
                    with source.extractfile(member) as reader, destination.open("xb") as writer:
                        shutil.copyfileobj(reader, writer)
                    os.chmod(destination, member.mode & 0o777)
                os.chown(destination, *owner)
            # Implicit parent directories also belong to the selected validator.
            for directory, dirs, _ in os.walk(staging):
                os.chown(directory, *owner)
                for name in dirs:
                    os.chown(Path(directory) / name, *owner)
            # Apply directory modes after extraction, children before parents,
            # so read-only archive directories cannot block their own files.
            for member, relative in sorted(entries, key=lambda entry: len(entry[1].parts), reverse=True):
                if member.isdir():
                    os.chmod(staging / relative, member.mode & 0o777)
            os.chmod(staging, root_mode)
            check_target(target)
            if target.exists():
                target.rmdir()
            staging.rename(target)
        finally:
            if staging.exists():
                # Restore traversal/write permissions before cleanup, including
                # when archive directories were read-only or inaccessible.
                os.chmod(staging, 0o700)
                for directory, dirs, _ in os.walk(staging):
                    os.chmod(directory, 0o700)
                    for name in dirs:
                        os.chmod(Path(directory) / name, 0o700)
                shutil.rmtree(staging)


def main():
    try:
        restore(sys.argv[1], sys.argv[2])
    except TargetOccupied as exc:
        print(f"Restore refused: {exc}", file=sys.stderr)
        return 3
    except (OSError, ValueError, tarfile.TarError, EOFError) as exc:
        print(f"Restore failed: {exc}", file=sys.stderr)
        return 4
    return 0


if __name__ == "__main__":
    sys.exit(main())
