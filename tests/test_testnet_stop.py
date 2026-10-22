"""Exercise 90-stop.yml: stopping nothing fails, an already-removed unit is already stopped.

The real playbook runs against local hosts with a fake `systemctl` first on PATH, so no user
manager is touched (the box running this may host a live validator unit).

These are playbook integration tests, not proof that a physical deployment is safe.
"""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]

FAKE_SYSTEMCTL = """#!/bin/sh
here=$(dirname "$0")
echo "$*" >> "$here/calls.log"
mode=$(cat "$here/mode")
unit=$(echo "$*" | grep -o 'x3-validator-[0-9]*\\.service' | head -1)
case "$*" in
  *show*)
    if [ "$mode" = missing ]; then
      printf 'Id=%s\\nLoadState=not-found\\nActiveState=inactive\\nSubState=dead\\n' "$unit"
    else
      printf 'Id=%s\\nLoadState=loaded\\nActiveState=active\\nSubState=running\\nUnitFileState=enabled\\n' "$unit"
    fi ;;
  *is-enabled*) [ "$mode" = missing ] && { echo not-found; exit 4; }; echo enabled ;;
  *" stop "*|*" disable "*) [ "$mode" = broken ] && { echo "Failed to stop $unit" >&2; exit 1; } ;;
esac
exit 0
"""


class StopPlaybookTests(unittest.TestCase):
    def stop(self, *extra, mode="missing"):
        executable = shutil.which("ansible-playbook")
        self.assertIsNotNone(executable, "ansible-playbook is required for playbook integration")
        with tempfile.TemporaryDirectory() as directory:
            directory = Path(directory)
            fake = directory / "bin"
            fake.mkdir()
            (fake / "systemctl").write_text(FAKE_SYSTEMCTL)
            (fake / "systemctl").chmod(0o755)
            (fake / "mode").write_text(mode)
            host = {"ansible_connection": "local", "ansible_python_interpreter": sys.executable,
                    "x3_home": str(directory / "home")}
            inventory = directory / "inventory.json"
            inventory.write_text(json.dumps({"all": {"children": {"validators": {"hosts": {
                "old": {**host, "x3_validators": [{"index": 4}]},
                "new": {**host, "x3_validators": [{"index": 5}]},
            }}}}}))
            result = subprocess.run(
                [executable, "-i", str(inventory), *extra, str(ROOT / "ansible/playbooks/90-stop.yml")],
                cwd=ROOT, capture_output=True, text=True, timeout=180,
                env={**os.environ, "PATH": f"{fake}:{os.environ['PATH']}",
                     "ANSIBLE_CONFIG": str(ROOT / "ansible/ansible.cfg")})
            calls = (fake / "calls.log").read_text() if (fake / "calls.log").exists() else ""
            return result, calls

    def test_stopping_an_already_removed_unit_succeeds(self):
        result, calls = self.stop("-l", "old", "-e", "x3_only=4")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("x3-validator-4.service", calls)
        self.assertNotIn("x3-validator-5.service", calls)

    def test_a_validator_that_is_not_on_the_host_is_refused(self):
        result, calls = self.stop("-l", "old", "-e", "x3_only=5")
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("nothing to stop: validator 5 is not placed on old", result.stdout)
        # Fact gathering asks `systemctl --version`; no validator unit may be touched.
        self.assertNotIn("x3-validator-", calls, "nothing may be stopped when the selection is wrong")

    def test_a_malformed_index_is_refused(self):
        result, _ = self.stop("-e", "x3_only=four")
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_a_unit_that_fails_to_stop_still_fails(self):
        result, _ = self.stop("-l", "old", "-e", "x3_only=4", mode="broken")
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
