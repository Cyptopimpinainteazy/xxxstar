#!/usr/bin/env python3
"""Real archive/filesystem checks for the validator restore command."""
import io
import importlib.util
import os
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import unittest
from unittest.mock import patch

SCRIPT = Path(__file__).resolve().parents[1] / "scripts/snapshot-restore.sh"
spec = importlib.util.spec_from_file_location('archive_restore', SCRIPT.with_name('snapshot-archive-restore.py'))
archive_restore = importlib.util.module_from_spec(spec)
spec.loader.exec_module(archive_restore)


class RestoreTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.archive = self.root / "snapshot.tar.gz"
        self.target = self.root / "new-validator"

    def archive_entries(self, entries):
        with tarfile.open(self.archive, "w:gz") as archive:
            for name, content, kind in entries:
                member = tarfile.TarInfo(name)
                member.mode = 0o600
                if kind in ('dir', 'readonly-dir'):
                    member.type = tarfile.DIRTYPE
                    member.mode = 0o500 if kind == 'readonly-dir' else 0o700
                    archive.addfile(member)
                elif kind == "link":
                    member.type = tarfile.SYMTYPE
                    member.linkname = str(self.root / "neighbor")
                    archive.addfile(member)
                else:
                    data = content.encode()
                    member.size = len(data)
                    archive.addfile(member, io.BytesIO(data))

    def run_restore(self):
        env = {k: v for k, v in os.environ.items() if not k.startswith("X3_SNAPSHOT_")}
        return subprocess.run(["bash", str(SCRIPT), "restore", str(self.archive), str(self.target)],
                              env=env, text=True, capture_output=True, timeout=10)

    def test_restores_to_requested_name_without_touching_original_directory(self):
        original = self.root / "old-validator"
        original.mkdir()
        (original / "state").write_text("preserve")
        self.archive_entries([("old-validator/chains/state", "restored", "file")])
        result = self.run_restore()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual((self.target / "chains/state").read_text(), "restored")
        self.assertEqual((original / "state").read_text(), "preserve")
        self.assertFalse((original / "chains").exists())

    def test_unsafe_or_ambiguous_archives_leave_target_and_neighbors_untouched(self):
        neighbor = self.root / "neighbor"
        neighbor.write_text("preserve")
        for entries in ([('old/../../neighbor', 'overwrite', 'file')],
                        [('old/link', '', 'link'), ('old/link/state', 'overwrite', 'file')],
                        [('old/state', 'a', 'file'), ('other/state', 'b', 'file')]):
            with self.subTest(entries=entries):
                self.archive_entries(entries)
                self.assertNotEqual(self.run_restore().returncode, 0)
                self.assertEqual(neighbor.read_text(), "preserve")
                self.assertFalse(self.target.exists())
                self.assertEqual(list(self.root.glob('.x3-restore-*')), [])

    def test_nonempty_target_is_preserved(self):
        self.target.mkdir()
        (self.target / "state").write_text("preserve")
        self.archive_entries([("old/state", "overwrite", "file")])
        self.assertEqual(self.run_restore().returncode, 3)
        self.assertEqual((self.target / "state").read_text(), "preserve")

    def test_corrupt_archive_does_not_create_target(self):
        self.archive.write_bytes(b"invalid gzip")
        self.assertEqual(self.run_restore().returncode, 4)
        self.assertFalse(self.target.exists())

    def test_target_symlink_is_preserved(self):
        neighbor = self.root / 'neighbor-dir'
        neighbor.mkdir()
        self.target.symlink_to(neighbor, target_is_directory=True)
        self.archive_entries([('old/state', 'overwrite', 'file')])
        self.assertEqual(self.run_restore().returncode, 3)
        self.assertTrue(self.target.is_symlink())
        self.assertEqual(list(neighbor.iterdir()), [])

    def test_duplicate_file_is_corrupt_archive_and_preserves_empty_target(self):
        self.target.mkdir()
        self.archive_entries([('old/state', 'first', 'file'), ('old/state', 'second', 'file')])
        self.assertEqual(self.run_restore().returncode, 4)
        self.assertEqual(list(self.target.iterdir()), [])
        self.assertEqual(list(self.root.glob('.x3-restore-*')), [])

    def test_directory_and_file_permissions_are_preserved(self):
        self.archive_entries([('old/private', '', 'dir'), ('old/private/state', 'data', 'file')])
        self.assertEqual(self.run_restore().returncode, 0)
        self.assertEqual((self.target / 'private').stat().st_mode & 0o777, 0o700)
        self.assertEqual((self.target / 'private/state').stat().st_mode & 0o777, 0o600)

    def test_existing_validator_owner_and_root_mode_are_preserved(self):
        self.target.mkdir(mode=0o750)
        before = self.target.stat()
        self.archive_entries([('old/private', '', 'dir'), ('old/private/state', 'data', 'file')])
        with patch.object(archive_restore.os, 'chown', wraps=os.chown) as ownership:
            archive_restore.restore(self.archive, self.target)
        self.assertTrue(ownership.call_args_list)
        self.assertTrue(all(call.args[1:] == (before.st_uid, before.st_gid)
                            for call in ownership.call_args_list))
        after = self.target.stat()
        self.assertEqual((after.st_uid, after.st_gid, after.st_mode & 0o777),
                         (before.st_uid, before.st_gid, 0o750))

    def test_file_directory_collisions_are_corrupt_archives(self):
        for entries in ([('old/a', 'file', 'file'), ('old/a/sub', 'child', 'file')],
                        [('old/a/sub', 'child', 'file'), ('old/a', 'file', 'file')]):
            with self.subTest(entries=entries):
                self.archive_entries(entries)
                self.assertEqual(self.run_restore().returncode, 4)
                self.assertFalse(self.target.exists())
                self.assertEqual(list(self.root.glob('.x3-restore-*')), [])

    def test_failed_install_cleans_readonly_directories(self):
        self.archive_entries([('old/private', '', 'readonly-dir'), ('old/private/state', 'data', 'file')])
        with patch.object(archive_restore.os, 'chmod', wraps=os.chmod) as modes:
            with patch.object(archive_restore.Path, 'rename', side_effect=OSError('install failed')):
                with self.assertRaisesRegex(OSError, 'install failed'):
                    archive_restore.restore(self.archive, self.target)
        self.assertFalse(self.target.exists())
        self.assertEqual(list(self.root.glob('.x3-restore-*')), [])
        self.assertTrue(modes.call_args_list)

    def test_backup_dot_roundtrips_to_requested_target(self):
        source = self.root / 'source'
        source.mkdir()
        (source / 'state').write_text('data')
        snapshots = self.root / 'snapshots'
        result = subprocess.run(['bash', str(SCRIPT), 'backup', '.'], cwd=source,
            env={**os.environ, 'X3_SNAPSHOT_DIR': str(snapshots)},
            text=True, capture_output=True, timeout=10)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.archive = next(snapshots.glob('*.tar.gz'))
        self.assertEqual(self.run_restore().returncode, 0)
        self.assertEqual((self.target / 'state').read_text(), 'data')

    def running_target_is_refused(self, option, executable_name='x3-chain-node'):
        process = subprocess.Popen(
            [executable_name, '-c', 'import time; print("ready", flush=True); time.sleep(30)',
             option, str(self.target)], executable=sys.executable,
            stdout=subprocess.PIPE, text=True)
        try:
            self.assertEqual(process.stdout.readline().strip(), 'ready')
            self.archive_entries([('old/state', 'restore', 'file')])
            self.assertEqual(self.run_restore().returncode, 2)
            self.target = self.root / 'another-validator'
            self.assertEqual(self.run_restore().returncode, 0)
        finally:
            process.kill()
            process.wait(timeout=5)
            process.stdout.close()

    def test_running_node_for_exact_target_is_refused(self):
        self.running_target_is_refused('--base-path')

    def test_short_option_and_renamed_node_are_refused(self):
        self.running_target_is_refused('-d', 'renamed-validator')


if __name__ == "__main__":
    unittest.main()
