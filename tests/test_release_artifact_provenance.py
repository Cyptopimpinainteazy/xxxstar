"""Test-only build commands exercise the real packager without claiming a node build."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
REL = Path('scripts/mainnet/build-release-artifacts.sh')
CODE = '0x0061736d01000000'


class ArtifactProvenance(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='x3-artifacts-')
        self.addCleanup(self.temp.cleanup)
        self.work = Path(self.temp.name)
        self.repo = self.work / 'repo with spaces'
        (self.repo / REL.parent).mkdir(parents=True)
        shutil.copy2(ROOT / REL, self.repo / REL)
        self.git('init', '-q')
        self.git('add', '.')
        self.git('-c', 'user.name=Test', '-c', 'user.email=test@example.invalid',
                 'commit', '-qm', 'fixture')
        self.spec = self.work / 'plain.json'
        self.spec.write_text(json.dumps({'id': 'test', 'name': 'Test',
            'chainType': 'Live', 'bootNodes': [],
            'genesis': {'runtimeGenesis': {'code': CODE, 'config': {}}}}))
        node = self.work / 'node-fixture'
        node.write_text('''#!/usr/bin/env python3
import json, os, sys
from pathlib import Path
chain = sys.argv[sys.argv.index('--chain') + 1]
if chain == 'local3':
    print(json.dumps({'genesis': {'runtimeGenesis': {'code': '0x0061736d01000000'}}}))
else:
    spec = json.loads(Path(chain).read_text())
    code = spec['genesis']['runtimeGenesis']['code']
    if os.environ.get('RC_TEST_BAD_RAW'):
        code = '0x00'
    spec['genesis'] = {'raw': {'top': {'0x3a636f6465': code}}}
    print(json.dumps(spec))
''')
        node.chmod(0o755)
        environment = self.work / 'environment.sh'
        environment.write_text('''cargo() {
  if [[ "$1" == --version ]]; then echo 'cargo test-fixture'; return; fi
  printf '%s\\n' "$*" > "$RC_TEST_ARGS"
  [[ "$*" == *--locked* && "$*" == *--no-default-features* ]] || return 19
  [[ -z "${SKIP_WASM_BUILD+x}" ]] || return 20
  [[ "${RC_TEST_BUILD_FAIL:-0}" == 0 ]] || return 21
  mkdir -p "$CARGO_TARGET_DIR/release"
  cp "$RC_TEST_NODE" "$CARGO_TARGET_DIR/release/x3-chain-node"
}
export -f cargo
''')
        # env starts a process and cannot call the exported function directly.
        bin_dir = self.work / 'bin'
        bin_dir.mkdir()
        cargo = bin_dir / 'cargo'
        cargo.write_text('#!/usr/bin/env bash\ncargo "$@"\n')
        cargo.chmod(0o755)
        self.env = {**os.environ, 'BASH_ENV': str(environment),
                    'PATH': str(bin_dir) + ':' + os.environ['PATH'],
                    'RC_TEST_NODE': str(node), 'RC_TEST_ARGS': str(self.work / 'args'),
                    'SKIP_WASM_BUILD': '1'}
        self.out = self.work / 'bundle'

    def git(self, *args):
        return subprocess.run(['git', '-C', str(self.repo), *args],
                              check=True, capture_output=True, text=True)

    def run_builder(self, *extra, sbom=False):
        return subprocess.run(['bash', str(self.repo / REL), 'candidate', '--chain',
            str(self.spec), '--features', 'cli,testnet', '--out', str(self.out),
            *(() if sbom else ('--skip-sbom',)), *extra], env=self.env, cwd=self.work,
            capture_output=True, text=True, timeout=30)

    def test_build_binds_and_checksums_all_required_files(self):
        result = self.run_builder()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        manifest = (self.out / 'MANIFEST.txt').read_text()
        self.assertIn(self.git('rev-parse', 'HEAD').stdout.strip(), manifest)
        self.assertIn('features:       cli,testnet', manifest)
        self.assertEqual((self.out / 'x3-runtime.wasm').read_bytes(), bytes.fromhex(CODE[2:]))
        for name in ('genesis.json', 'genesis-raw.json', 'x3-runtime.wasm',
                     'x3-chain-node', 'MANIFEST.txt', 'git-revision.txt'):
            with self.subTest(name=name):
                path = self.out / name
                original = path.read_bytes()
                path.write_bytes(original + b'x')
                result = subprocess.run(['sha256sum', '-c', 'SHA256SUMS'],
                    cwd=self.out, capture_output=True)
                self.assertNotEqual(result.returncode, 0)
                path.write_bytes(original)
        result = subprocess.run(['sha256sum', '-c', 'SHA256SUMS'],
            cwd=self.out, capture_output=True)
        self.assertEqual(result.returncode, 0)

    def test_dirty_tracked_and_untracked_inputs_refused(self):
        for path in (self.repo / REL, self.repo / 'untracked.rs'):
            with self.subTest(path=path.name):
                original = path.read_bytes() if path.exists() else None
                path.write_bytes((original or b'') + b'\n# dirty\n')
                result = self.run_builder()
                self.assertNotEqual(result.returncode, 0)
                self.assertIn('checkout must be clean', result.stderr)
                self.assertFalse(self.out.exists())
                if original is None:
                    path.unlink()
                else:
                    path.write_bytes(original)

    def test_prebuilt_binary_refused(self):
        result = self.run_builder('--binary', self.env['RC_TEST_NODE'])
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('prebuilt binaries', result.stderr)

    def test_mismatched_runtime_refused(self):
        spec = json.loads(self.spec.read_text())
        spec['genesis']['runtimeGenesis']['code'] = '0x00'
        self.spec.write_text(json.dumps(spec))
        result = self.run_builder()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('embedded runtime', result.stderr)
        self.assertFalse(self.out.exists())

    def test_raw_mismatch_refused(self):
        self.env['RC_TEST_BAD_RAW'] = '1'
        result = self.run_builder()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('raw spec runtime', result.stderr)
        self.assertFalse(self.out.exists())

    def test_build_failure_publishes_nothing(self):
        self.env['RC_TEST_BUILD_FAIL'] = '1'
        self.assertNotEqual(self.run_builder().returncode, 0)
        self.assertFalse(self.out.exists())

    def test_required_sbom_failure_publishes_nothing(self):
        result = self.run_builder(sbom=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(self.out.exists())

    def test_missing_spec_and_features_refused(self):
        for options in (('--features', ''), ('--chain', str(self.work / 'missing'))):
            with self.subTest(options=options):
                self.assertNotEqual(self.run_builder(*options).returncode, 0)
                self.assertFalse(self.out.exists())

    def test_existing_destination_is_preserved(self):
        self.out.mkdir()
        marker = self.out / 'keep'
        marker.write_text('user work')
        self.assertNotEqual(self.run_builder().returncode, 0)
        self.assertEqual(marker.read_text(), 'user work')


if __name__ == '__main__':
    unittest.main()
