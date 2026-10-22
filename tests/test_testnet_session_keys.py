"""Exercise the deploy playbook's session-key staging and remote keystore sync.

The tasks under test are the real `ansible/playbooks/tasks/*-session-keys.yml`, run against local
directories; `keys insert` is a fake that writes one file per key the way the node does
(`<key type hex><public key hex>`). No node, seed handling or SSH is involved.

These are playbook integration tests, not proof that a physical deployment is safe.
"""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
TASKS = ROOT / "ansible/playbooks/tasks"
KEY_TYPE_HEX = {"aura": "61757261", "grandpa": "6772616e"}

FAKE_NODE = """#!/usr/bin/env python3
import hashlib, pathlib, sys
args = sys.argv[1:]
assert args[:2] == ["keys", "insert"], args
kind = args[args.index("--key-type") + 1]
seed = args[args.index("--seed") + 1]
path = pathlib.Path(args[args.index("--keystore-path") + 1])
prefix = {"aura": "61757261", "grandpa": "6772616e"}[kind]
public = hashlib.sha256((kind + seed).encode()).hexdigest()
(path / (prefix + public)).write_text('"' + seed + '"')
"""


def key_name(kind, seed):
    return KEY_TYPE_HEX[kind] + hashlib.sha256((kind + seed).encode()).hexdigest()


def run_playbook(directory, inventory, playbook_text):
    executable = shutil.which("ansible-playbook")
    assert executable, "ansible-playbook is required for playbook integration"
    playbook = directory / "play.yml"
    playbook.write_text(playbook_text)
    inv = directory / "inventory.json"
    inv.write_text(json.dumps(inventory))
    return subprocess.run([executable, "-i", str(inv), str(playbook)], cwd=ROOT,
                          env={**os.environ, "ANSIBLE_CONFIG": str(ROOT / "ansible/ansible.cfg")},
                          capture_output=True, text=True, timeout=120)


class StageSessionKeysTests(unittest.TestCase):
    def stage(self, directory, seeds, stale=()):
        keys_dir = directory / "keys"
        keys_dir.mkdir(exist_ok=True)
        for index, seed in seeds.items():
            (keys_dir / f"validator-{index}.suri").write_text(f"seed={seed}\n")
        node = directory / "x3-chain-node"
        node.write_text(FAKE_NODE)
        node.chmod(0o755)
        staged = directory / "staged"
        for index, name in stale:
            keystore = staged / f"validator-{index}/keystore"
            keystore.mkdir(parents=True, exist_ok=True)
            (keystore / name).write_text("stale")
        inventory = {"all": {"hosts": {"localhost": {
            "ansible_connection": "local", "ansible_python_interpreter": sys.executable}}}}
        result = run_playbook(directory, inventory, f"""- hosts: localhost
  gather_facts: false
  vars:
    staged: {staged}
    x3_validator_count: {len(seeds)}
    x3_binary_src: {node}
    x3_keys_dir: {keys_dir}
  tasks:
    - ansible.builtin.include_tasks: {TASKS / 'stage-session-keys.yml'}
""")
        return result, staged

    def test_each_validator_gets_exactly_its_two_keys(self):
        with tempfile.TemporaryDirectory() as d:
            seeds = {1: "0x" + "11" * 32, 2: "0x" + "22" * 32}
            result, staged = self.stage(Path(d), seeds)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            for index, seed in seeds.items():
                names = sorted(p.name for p in (staged / f"validator-{index}/keystore").iterdir())
                self.assertEqual(names, sorted(key_name(k, seed) for k in ("aura", "grandpa")))

    def test_keys_from_a_rotated_genesis_do_not_survive(self):
        # The previous genesis left validator 1's old Aura key behind.
        with tempfile.TemporaryDirectory() as d:
            seeds = {1: "0x" + "33" * 32}
            old = key_name("aura", "0x" + "99" * 32)
            result, staged = self.stage(Path(d), seeds, stale=[(1, old)])
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            names = {p.name for p in (staged / "validator-1/keystore").iterdir()}
            self.assertNotIn(old, names)
            self.assertEqual(len(names), 2)


class SyncSessionKeysTests(unittest.TestCase):
    def sync(self, directory, staged_files, remote_files):
        staged = directory / "staged"
        (staged / "validator-4/keystore").mkdir(parents=True)
        for name, content in staged_files.items():
            (staged / "validator-4/keystore" / name).write_text(content)
        home = directory / "home"
        remote = home / "data/validator-4/chains/x3_chain_testnet/keystore"
        # The modes a previous deploy leaves (0700 directory, 0600 keys), so "unchanged" means
        # unchanged content and names, not a permissions fix-up.
        remote.mkdir(parents=True, mode=0o700)
        remote.chmod(0o700)
        for name, content in remote_files.items():
            (remote / name).write_text(content)
            (remote / name).chmod(0o600)
        inventory = {"all": {"hosts": {"validator-host": {
            "ansible_connection": "local", "ansible_python_interpreter": sys.executable,
            "x3_validators": [{"index": 4}]}}}}
        result = run_playbook(directory, inventory, f"""- hosts: validator-host
  gather_facts: false
  vars:
    staged: {staged}
    x3_home: {home}
    x3_chain_id: x3_chain_testnet
  handlers:
    - name: Restart validators
      ansible.builtin.debug:
        msg: restart requested
  tasks:
    - ansible.builtin.include_tasks: {TASKS / 'sync-session-keys.yml'}
""")
        return result, remote

    STAGED = {key_name("aura", "s4"): "aura-4", key_name("grandpa", "s4"): "grandpa-4"}

    def test_a_key_left_on_the_host_is_removed(self):
        # Another validator's key from a reused data directory: the double-sign case.
        other = key_name("aura", "s5")
        with tempfile.TemporaryDirectory() as d:
            result, remote = self.sync(Path(d), self.STAGED, {other: "aura-5"})
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertEqual({p.name: p.read_text() for p in remote.iterdir()}, self.STAGED)
            self.assertIn("restart requested", result.stdout)

    def test_a_stale_copy_of_a_staged_key_is_replaced(self):
        name = key_name("grandpa", "s4")
        with tempfile.TemporaryDirectory() as d:
            result, remote = self.sync(Path(d), self.STAGED, {name: "stale"})
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertEqual((remote / name).read_text(), "grandpa-4")

    def test_an_already_exact_keystore_is_unchanged(self):
        with tempfile.TemporaryDirectory() as d:
            result, remote = self.sync(Path(d), self.STAGED, dict(self.STAGED))
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertNotIn("restart requested", result.stdout)

    def test_a_staged_set_that_is_not_two_keys_fails(self):
        staged = {**self.STAGED, key_name("aura", "extra"): "extra"}
        with tempfile.TemporaryDirectory() as d:
            result, _ = self.sync(Path(d), staged, {})
            self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
