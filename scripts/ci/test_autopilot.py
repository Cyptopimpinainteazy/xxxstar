"""Unit tests for the read-only CI Autopilot collector."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location("autopilot", Path(__file__).with_name("autopilot.py"))
module = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(module)


class AutopilotTests(unittest.TestCase):
    def setUp(self):
        self.inventory = {"nodes": {
            "x3star1": {"role": "control", "required": True},
            "x3star2": {"role": "build", "required": True},
        }}
        self.capacity = {"hostname": "x3star1", "disk_free_gb": 40, "cpu_threads": 16}
        self.runners = [{"name": "x3star1", "status": "online", "busy": True,
                         "labels": [{"name": "x3"}, {"name": "self-hosted"}]},
                        {"name": "x3star2", "status": "offline", "busy": False,
                         "labels": [{"name": "x3"}]}]

    def test_registered_states(self):
        report = module.evaluate(self.inventory, self.runners, self.capacity)
        self.assertEqual([n["ci"] for n in report["nodes"]], ["BUSY", "OFFLINE"])
        self.assertEqual(report["mode"], "read-only")
        self.assertTrue(any("required inventory" in n for n in report["notes"]))

    def test_unknown_runner_is_not_dropped(self):
        report = module.evaluate(self.inventory, self.runners + [
            {"name": "x3gpu1", "status": "online", "busy": False, "labels": []}], self.capacity)
        self.assertEqual(report["nodes"][0]["name"], "x3gpu1")
        self.assertEqual(report["nodes"][0]["ci"], "IDLE")

    def test_missing_registration_not_online(self):
        report = module.evaluate(self.inventory, [], self.capacity)
        self.assertTrue(all(n["ci"] == "UNREGISTERED" for n in report["nodes"]))

    def test_low_disk_warns(self):
        self.capacity["disk_free_gb"] = 4
        report = module.evaluate(self.inventory, self.runners, self.capacity)
        self.assertTrue(any("disk" in n for n in report["notes"]))

    def test_local_only_offline(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            inventory = root / "inventory.json"
            inventory.write_text(json.dumps(self.inventory))
            out = root / "report.json"
            self.assertEqual(module.main(["--inventory", str(inventory),
                                          "--local-only", "--output", str(out)]), 0)
            data = json.loads(out.read_text())
            self.assertTrue(all(n["ci"] == "UNKNOWN" for n in data["nodes"]))
            self.assertEqual(data["mode"], "read-only")

    def test_invalid_fixture_fails_closed(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            inv = root / "inventory.json"
            inv.write_text(json.dumps(self.inventory))
            fixture = root / "runners.json"
            fixture.write_text('{"runners": {"wrong": true}}')
            self.assertEqual(module.main(["--inventory", str(inv),
                                           "--runners-json", str(fixture)]), 2)


if __name__ == "__main__":
    unittest.main()
