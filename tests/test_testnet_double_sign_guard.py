"""Exercise the deploy playbook's double-sign guard against controlled `systemctl` output.

Each inventory host gets its own fake `systemctl` on PATH (through `x3_remote_environment`, the
same per-host environment an operator uses for XDG_RUNTIME_DIR), so the guard's real tasks run
unchanged while the unit states are chosen by the test. No real user manager is queried: the box
running this may host a live validator unit, and the test must never see or touch it.

These are guard integration tests, not proof that a physical deployment is safe.
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
GUARD = ROOT / "ansible/playbooks/tasks/double-sign-guard.yml"

FAKE_SYSTEMCTL = """#!/bin/sh
here=$(dirname "$0")
case "$*" in
  *list-unit-files*) kind=files ;;
  *list-units*) kind=units ;;
  *) echo "unexpected systemctl call: $*" >&2; exit 2 ;;
esac
cat "$here/$kind.txt"
exit "$(cat "$here/$kind.rc")"
"""

# A user manager always lists some units of its own; the guard must ignore them.
NOISE_UNITS = "dbus.service loaded active running D-Bus User Message Bus\n"
NOISE_FILES = "dbus.service static -\n"


def unit_line(index, active, sub):
    return f"x3-validator-{index}.service loaded {active} {sub} X3 testnet validator {index}\n"


def file_line(index, state):
    return f"x3-validator-{index}.service {state} enabled\n"


class DoubleSignGuardTests(unittest.TestCase):
    def run_guard(self, old_host_units="", old_host_files="", units_rc=0, files_rc=0,
                  validator_moves=True):
        """Validator 4 is placed on host `new`; host `old` reports the given unit state."""
        executable = shutil.which("ansible-playbook")
        self.assertIsNotNone(executable, "ansible-playbook is required for guard integration")
        with tempfile.TemporaryDirectory() as directory:
            directory = Path(directory)
            hosts = {}
            for name, units, files, u_rc, f_rc, placed in (
                ("old", NOISE_UNITS + old_host_units, NOISE_FILES + old_host_files, units_rc,
                 files_rc, [] if validator_moves else [{"index": 4}]),
                ("new", NOISE_UNITS, NOISE_FILES, 0, 0, [{"index": 4}] if validator_moves else []),
            ):
                bin_dir = directory / f"bin-{name}"
                bin_dir.mkdir()
                (bin_dir / "systemctl").write_text(FAKE_SYSTEMCTL)
                (bin_dir / "systemctl").chmod(0o755)
                (bin_dir / "units.txt").write_text(units)
                (bin_dir / "files.txt").write_text(files)
                (bin_dir / "units.rc").write_text(str(u_rc))
                (bin_dir / "files.rc").write_text(str(f_rc))
                hosts[name] = {
                    "ansible_connection": "local",
                    "ansible_python_interpreter": sys.executable,
                    "x3_validators": placed,
                    "x3_remote_environment": {"PATH": f"{bin_dir}:{os.environ['PATH']}"},
                }
            inventory = directory / "inventory.json"
            inventory.write_text(json.dumps({"all": {"children": {"validators": {"hosts": hosts}}}}))
            playbook = directory / "guard.yml"
            playbook.write_text(
                "- hosts: localhost\n  connection: local\n  gather_facts: false\n  tasks:\n"
                f"    - ansible.builtin.include_tasks: {GUARD}\n")
            return subprocess.run(
                [executable, "-i", str(inventory), str(playbook)], cwd=ROOT,
                env={**os.environ, "ANSIBLE_CONFIG": str(ROOT / "ansible/ansible.cfg")},
                capture_output=True, text=True, timeout=120)

    def assert_refused(self, result, *fragments):
        output = result.stdout + result.stderr
        self.assertNotEqual(result.returncode, 0, output)
        for fragment in fragments:
            self.assertIn(fragment, output)

    def assert_allowed(self, result):
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_a_running_validator_cannot_be_placed_elsewhere(self):
        self.assert_refused(self.run_guard(unit_line(4, "active", "running"), file_line(4, "enabled")),
                            "validator 4 can still sign on old", "placed on new")

    def test_an_enabled_but_failed_validator_cannot_be_placed_elsewhere(self):
        # The P0: start-limit hit, unit `failed`, still enabled - it starts again at boot.
        self.assert_refused(self.run_guard(unit_line(4, "failed", "failed"), file_line(4, "enabled")),
                            "validator 4 can still sign on old", "unit file enabled")

    def test_an_enabled_inactive_validator_cannot_be_placed_elsewhere(self):
        self.assert_refused(self.run_guard(unit_line(4, "inactive", "dead"), file_line(4, "enabled")),
                            "unit file enabled")

    def test_an_enabled_unit_that_is_not_loaded_still_counts(self):
        # Never started since boot, so `list-units` does not show it; the unit file does.
        self.assert_refused(self.run_guard("", file_line(4, "enabled")), "unit file enabled")

    def test_an_auto_restarting_validator_counts_even_when_disabled(self):
        self.assert_refused(self.run_guard(unit_line(4, "activating", "auto-restart"),
                                           file_line(4, "disabled")), "activating (auto-restart)")

    def test_stopped_and_disabled_releases_the_validator(self):
        # 90-stop's end state, from a running unit and from a failed one.
        self.assert_allowed(self.run_guard(unit_line(4, "inactive", "dead"), file_line(4, "disabled")))
        self.assert_allowed(self.run_guard(unit_line(4, "failed", "failed"), file_line(4, "disabled")))

    def test_a_masked_unit_releases_the_validator(self):
        self.assert_allowed(self.run_guard(unit_line(4, "inactive", "dead"), file_line(4, "masked")))

    def test_a_host_with_no_validator_unit_releases_it(self):
        self.assert_allowed(self.run_guard())

    def test_a_validator_redeployed_on_its_own_host_is_allowed(self):
        self.assert_allowed(self.run_guard(unit_line(4, "active", "running"), file_line(4, "enabled"),
                                           validator_moves=False))

    def test_a_host_whose_user_manager_does_not_answer_fails_closed(self):
        self.assert_refused(self.run_guard(units_rc=1))

    def test_a_host_whose_unit_file_listing_fails_fails_closed(self):
        self.assert_refused(self.run_guard(unit_line(4, "failed", "failed"), file_line(4, "enabled"),
                                           files_rc=1))


if __name__ == "__main__":
    unittest.main()
