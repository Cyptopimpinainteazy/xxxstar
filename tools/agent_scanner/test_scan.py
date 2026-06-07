#!/usr/bin/env python3
"""
Tests for tools/agent_scanner/scan.py
Run with: python3 -m pytest tools/agent_scanner/test_scan.py -v
or simply: python3 tools/agent_scanner/test_scan.py
"""
import hashlib
import json
import os
import sys
import tempfile
import unittest

# Make the scanner importable
sys.path.insert(0, os.path.dirname(__file__))
import scan


class TestDetectHalfDone(unittest.TestCase):
    def test_rust_unimplemented(self):
        signals = scan.detect_half_done('foo.rs', 'fn do_thing() { unimplemented!() }')
        self.assertTrue(any('unimplemented' in s for s in signals), signals)

    def test_rust_todo_macro(self):
        signals = scan.detect_half_done('bar.rs', 'fn x() { todo!() }')
        self.assertTrue(any('todo' in s.lower() for s in signals), signals)

    def test_python_not_implemented(self):
        signals = scan.detect_half_done('util.py', 'raise NotImplementedError("nope")')
        self.assertTrue(any('NotImplementedError' in s for s in signals), signals)

    def test_ts_stub(self):
        signals = scan.detect_half_done('api.ts', 'throw new Error("not implemented")')
        self.assertTrue(any('not implemented' in s.lower() for s in signals), signals)

    def test_no_signal_clean_file(self):
        signals = scan.detect_half_done('clean.rs', 'fn hello() { println!("hi"); }')
        self.assertEqual(signals, [], signals)

    def test_unknown_extension_no_crash(self):
        signals = scan.detect_half_done('file.xyz', 'some content')
        self.assertEqual(signals, [])


class TestIsTextFile(unittest.TestCase):
    def test_text_file(self):
        with tempfile.NamedTemporaryFile(delete=False, suffix='.rs', mode='wb') as f:
            f.write(b'fn main() {}')
            fname = f.name
        try:
            self.assertTrue(scan.is_text_file(fname))
        finally:
            os.unlink(fname)

    def test_binary_file(self):
        with tempfile.NamedTemporaryFile(delete=False, mode='wb') as f:
            f.write(bytes(range(256)))
            fname = f.name
        try:
            self.assertFalse(scan.is_text_file(fname))
        finally:
            os.unlink(fname)


class TestSha256(unittest.TestCase):
    def test_consistent_hash(self):
        with tempfile.NamedTemporaryFile(delete=False, mode='wb') as f:
            f.write(b'hello world')
            fname = f.name
        try:
            expected = hashlib.sha256(b'hello world').hexdigest()
            self.assertEqual(scan.sha256_of_file(fname), expected)
        finally:
            os.unlink(fname)


class TestGatherEndToEnd(unittest.TestCase):
    def setUp(self):
        self.tmpdir = tempfile.mkdtemp()

    def tearDown(self):
        import shutil
        shutil.rmtree(self.tmpdir)

    def _write(self, relpath, content):
        path = os.path.join(self.tmpdir, relpath)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, 'w', encoding='utf-8') as f:
            f.write(content)
        return path

    def test_detects_todo(self):
        self._write('src/main.rs', 'fn foo() { todo!() }')
        checklist = scan.gather(self.tmpdir, stale_days=36500, artifact_days=36500)
        types = [it['type'] for it in checklist]
        # should detect both todo-marker and half-done
        self.assertIn('todo-marker', types)
        self.assertIn('half-done', types)

    def test_detects_duplicates(self):
        self._write('a/file.txt', 'same content')
        self._write('b/file.txt', 'same content')
        checklist = scan.gather(self.tmpdir, stale_days=36500, artifact_days=36500)
        dups = [it for it in checklist if it['type'] == 'duplicate']
        self.assertTrue(len(dups) >= 1, dups)

    def test_no_false_duplicate_for_unique(self):
        self._write('a/file.txt', 'unique content A')
        self._write('b/file.txt', 'unique content B')
        checklist = scan.gather(self.tmpdir, stale_days=36500, artifact_days=36500)
        dups = [it for it in checklist if it['type'] == 'duplicate']
        self.assertEqual(dups, [])

    def test_write_outputs(self):
        self._write('src/main.py', 'raise NotImplementedError()')
        checklist = scan.gather(self.tmpdir, stale_days=36500, artifact_days=36500)
        out_json = os.path.join(self.tmpdir, 'out.json')
        out_md = os.path.join(self.tmpdir, 'out.md')
        scan.write_outputs(checklist, out_json, out_md, self.tmpdir)
        self.assertTrue(os.path.exists(out_json))
        self.assertTrue(os.path.exists(out_md))
        with open(out_json) as f:
            data = json.load(f)
        self.assertIn('items', data)
        self.assertIn('generated', data)


if __name__ == '__main__':
    unittest.main(verbosity=2)
