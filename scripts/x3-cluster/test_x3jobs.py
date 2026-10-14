"""Tests for x3jobs routing, result classification and a real local job run.

    python3 -m unittest scripts/x3-cluster/test_x3jobs.py
"""
import json
import socket
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import x3jobs  # noqa: E402

REPO = Path(__file__).resolve().parents[2]
INV = {"nodes": {
    "ctl": {"role": "control", "ip": "10.0.0.1"},
    "b1": {"role": "build", "ip": "10.0.0.2"},
    "b2": {"role": "build", "ip": "10.0.0.3"},
    "g1": {"role": "gpu", "ip": "10.0.0.4"},
    "s1": {"role": "sim", "ip": None},
}}


def fake_probe(table):
    def probe(name, node, me):
        return table.get(name)
    return probe


def facts(load=0.0, threads=8, free=100.0):
    return {"load": load, "threads": threads, "free_gb": free, "env": {}}


class Routing(unittest.TestCase):
    def test_least_loaded_owner_wins(self):
        probe = fake_probe({"b1": facts(load=6, threads=8), "b2": facts(load=2, threads=8), "ctl": facts()})
        name, _, label, _ = x3jobs.choose_worker("BUILD", INV, "ctl", probe)
        self.assertEqual((name, label), ("b2", "PHYSICAL"))

    def test_load_is_normalised_by_threads(self):
        probe = fake_probe({"b1": facts(load=8, threads=32), "b2": facts(load=4, threads=8)})
        self.assertEqual(x3jobs.choose_worker("TEST", INV, "ctl", probe)[0], "b1")

    def test_build_falls_back_to_control_and_says_so(self):
        probe = fake_probe({"ctl": facts()})
        name, _, label, rejected = x3jobs.choose_worker("BUILD", INV, "ctl", probe)
        self.assertEqual((name, label), ("ctl", "LOCAL-FALLBACK"))
        self.assertEqual({r["node"] for r in rejected}, {"b1", "b2"})

    def test_sim_node_without_ip_falls_back(self):
        name, _, label, rejected = x3jobs.choose_worker("SIMULATION", INV, "ctl", fake_probe({"ctl": facts()}))
        self.assertEqual((name, label), ("ctl", "LOCAL-FALLBACK"))
        self.assertEqual(rejected[0]["node"], "s1")

    def test_gpu_never_falls_back_to_a_cpu_node(self):
        name, _, label, rejected = x3jobs.choose_worker("GPU", INV, "ctl", fake_probe({"ctl": facts()}))
        self.assertIsNone(name)
        self.assertEqual(rejected, [{"node": "g1", "reason": "unreachable or SSH key auth failed"}])

    def test_disk_guard_rejects_full_node(self):
        probe = fake_probe({"b1": facts(free=11), "b2": facts(free=12), "ctl": facts(free=300)})
        name, _, label, rejected = x3jobs.choose_worker("BUILD", INV, "ctl", probe)
        self.assertEqual(name, "ctl")
        self.assertIn("< 30 GB", rejected[0]["reason"])

    def test_pin_is_respected_and_labelled(self):
        probe = fake_probe({"b1": facts(), "g1": facts()})
        self.assertEqual(x3jobs.choose_worker("TEST", INV, "ctl", probe, pin="g1")[:3:2], ("g1", "PHYSICAL"))
        self.assertIsNone(x3jobs.choose_worker("TEST", INV, "ctl", probe, pin="nope")[0])


class Records(unittest.TestCase):
    def test_result_codes(self):
        self.assertEqual(x3jobs.result_for(0, {}), "PASS")
        self.assertEqual(x3jobs.result_for(1, {}), "FAIL")
        self.assertEqual(x3jobs.result_for(124, {}), "TIMEOUT")
        self.assertEqual(x3jobs.result_for(97, {"reject": "x"}), "REJECTED")
        self.assertEqual(x3jobs.result_for(96, {}), "REJECTED")

    def test_meta_parsing_keeps_values_with_equals(self):
        meta = x3jobs.parse_meta("noise\nX3JOB-META rustc=rustc 1.90.0 (a=b)\nX3JOB-META host=n1\n")
        self.assertEqual(meta, {"rustc": "rustc 1.90.0 (a=b)", "host": "n1"})

    def test_new_job_validates(self):
        with self.assertRaises(ValueError):
            x3jobs.new_job("BUILD", "true", "abc", "HEAD")
        with self.assertRaises(ValueError):
            x3jobs.new_job("NOPE", "true", "a" * 40, "HEAD")
        with self.assertRaises(ValueError):
            x3jobs.new_job("BUILD", "true", "a" * 40, "HEAD", priority="URGENT")
        job = x3jobs.new_job("TEST", "true", "a" * 40, "HEAD", name="My Test!")
        self.assertRegex(job["id"], r"^\d{8}T\d{6}Z-my-test-[0-9a-f]{6}$")


class LocalRun(unittest.TestCase):
    """Runs a real job on this machine against this checkout's HEAD."""

    def setUp(self):
        self.me = socket.gethostname()
        self.head = subprocess.run(["git", "-C", str(REPO), "rev-parse", "HEAD"], capture_output=True, text=True).stdout.strip()
        self.inv = {"nodes": {self.me: {"role": "control", "ip": "127.0.0.1"}}}
        self.probe = lambda name, node, me: dict(facts(), env={"X3_REPO": str(REPO)})

    def run_local(self, cmd, **kw):
        with tempfile.TemporaryDirectory() as root:
            job = x3jobs.new_job("TEST", cmd, self.head, "HEAD", timeout=kw.pop("timeout", 120), min_free_gb=1, **kw)
            record = x3jobs.run_job(job, self.inv, self.me, root, self.probe)
            on_disk = json.loads((Path(root) / self.head / "jobs" / job["id"] / "job.json").read_text())
            return record, on_disk

    def test_pass_runs_at_exact_commit_and_collects_artifacts(self):
        record, on_disk = self.run_local('test "$(git rev-parse HEAD)" = "$X3_COMMIT" && echo hi > "$X3_JOB_OUT/a.txt"')
        self.assertEqual(record["result"], "PASS", record["log_tail"])
        self.assertEqual(record["label"], "LOCAL-FALLBACK")
        self.assertEqual(record["machine"], self.me)
        self.assertTrue(record["kernel"])
        self.assertEqual(record["artifacts"][0]["path"], "a.txt")
        self.assertEqual(len(record["artifacts"][0]["sha256"]), 64)
        self.assertEqual(on_disk["log_sha256"], record["log_sha256"])

    def test_failure_is_fail_not_pass(self):
        record, _ = self.run_local("exit 3")
        self.assertEqual((record["result"], record["exit_code"]), ("FAIL", 3))

    def test_timeout_is_reported(self):
        record, _ = self.run_local("sleep 30", timeout=2)
        self.assertEqual(record["result"], "TIMEOUT")

    def test_unknown_commit_is_rejected_without_running(self):
        with tempfile.TemporaryDirectory() as root:
            job = x3jobs.new_job("TEST", "touch /tmp/should-not-run", "0" * 40, "HEAD", min_free_gb=1)
            script = x3jobs.job_script(job, str(REPO), None, 1, remote="/nonexistent-remote")
            out = subprocess.run(["bash", "-s"], input=script, capture_output=True, text=True, timeout=60)
            self.assertEqual(out.returncode, 97, out.stdout + out.stderr)
            self.assertIn("reject=", out.stdout)
            del root


import x3cluster  # noqa: E402

HERE = Path(__file__).resolve().parent


class Discovery(unittest.TestCase):
    INV = {"nodes": {"x3star1": {"role": "control", "ip": "192.168.0.70"},
                     "x3gpu1": {"role": "gpu", "ip": "192.168.0.30"},
                     "x3gpu2": {"role": "gpu", "ip": None}}}

    def test_node_is_claimed_only_by_its_own_hostname(self):
        found, pending, conflicts = x3cluster.classify_hosts(
            self.INV, {"192.168.0.41": "x3gpu2", "192.168.0.42": "laptop", "192.168.0.43": None})
        self.assertEqual(found, {"x3gpu2": "192.168.0.41"})
        self.assertEqual(pending, ["192.168.0.43"])
        self.assertEqual(conflicts, [])

    def test_fixed_ip_is_not_silently_overridden(self):
        found, _, conflicts = x3cluster.classify_hosts(self.INV, {"192.168.0.99": "x3gpu1"})
        self.assertEqual(found, {})
        self.assertIn("inventory.json says 192.168.0.30", conflicts[0])

    def test_overlay_fills_only_missing_ips(self):
        with tempfile.TemporaryDirectory() as tmp:
            old = x3cluster.DISCOVERED
            x3cluster.DISCOVERED = Path(tmp) / "discovered.json"
            x3cluster.DISCOVERED.write_text(json.dumps({"x3build1": {"ip": "192.168.0.41"}, "x3gpu1": {"ip": "10.9.9.9"}}))
            try:
                inv = x3cluster.load_inventory()
            finally:
                x3cluster.DISCOVERED = old
        self.assertEqual(inv["nodes"]["x3build1"]["ip"], "192.168.0.41")
        self.assertTrue(inv["nodes"]["x3build1"]["discovered"])
        self.assertEqual(inv["nodes"]["x3gpu1"]["ip"], "192.168.0.30")


class JoinScript(unittest.TestCase):
    def test_constants_match_inventory(self):
        inv = json.loads((HERE / "inventory.json").read_text())
        text = (HERE / "x3-join.sh").read_text()
        self.assertIn(f'LAN="{inv["lan"]}"', text)
        self.assertIn(f'CONTROL_IP="{inv["nodes"]["x3star1"]["ip"]}"', text)
        self.assertIn(f'CONTROL_KEY="{inv["control_ssh_pubkey"]}"', text)
        self.assertNotIn("PRIVATE KEY", text)

    def test_dry_run_changes_nothing_and_opens_ssh_before_enabling_ufw(self):
        with tempfile.TemporaryDirectory() as home:
            out = subprocess.run(["bash", str(HERE / "x3-join.sh"), "--role", "gpu", "--name", "x3gpu2", "--dry-run"],
                                 capture_output=True, text=True, timeout=60, env={"HOME": home, "PATH": "/usr/bin:/bin",
                                                                                   "USER": "lojak"})
            self.assertEqual(out.returncode, 0, out.stderr)
            # A dry run writes nothing at all, not even an empty authorized_keys.
            self.assertFalse(Path(home, ".ssh", "authorized_keys").exists())
        lines = out.stdout.splitlines()
        ssh_rule = next(i for i, l in enumerate(lines) if "port 22" in l)
        enable = next(i for i, l in enumerate(lines) if "ufw --force enable" in l)
        self.assertLess(ssh_rule, enable)
        self.assertTrue(all(l.startswith("+ ") for l in lines if "sudo" in l))

    def test_rejects_unknown_role(self):
        out = subprocess.run(["bash", str(HERE / "x3-join.sh"), "--role", "miner"], capture_output=True, text=True)
        self.assertEqual(out.returncode, 2)


class ExporterParsing(unittest.TestCase):
    SAMPLE = """# HELP x
node_cpu_seconds_total{cpu="0",mode="idle"} 10
node_cpu_seconds_total{cpu="0",mode="user"} 3
node_cpu_seconds_total{cpu="1",mode="idle"} 11
node_memory_MemTotal_bytes 2.5061326848e+10
node_filesystem_avail_bytes{device="/dev/sda2",fstype="ext4",mountpoint="/"} 8.88809472e+09
node_filesystem_avail_bytes{device="/dev/sdb1",fstype="ext4",mountpoint="/data"} 1e+12
node_timex_sync_status 1
node_uname_info{nodename="x3gpu2",release="6.8.0-138-generic"} 1
node_systemd_unit_state{name="ollama.service",state="active",type="simple"} 1
node_systemd_unit_state{name="ollama.service",state="failed",type="simple"} 0
node_systemd_unit_state{name="bad.service",state="failed",type="simple"} 1
x3_gpu_memory_total_mib{gpu="0",uuid="GPU-1"} 8192
x3_gpu_temperature_celsius{gpu="0",uuid="GPU-1"} 33
"""

    def test_parse(self):
        f = x3cluster.parse_exporter(self.SAMPLE)
        self.assertEqual((f["threads"], f["ram_gb"], f["free_gb"], f["clock_synced"]), (2, 25.1, 8.9, True))
        self.assertEqual((f["hostname"], f["kernel"]), ("x3gpu2", "6.8.0-138-generic"))
        self.assertEqual(f["gpus"], [{"uuid": "GPU-1", "memory_total_mib": 8192.0, "temperature_celsius": 33.0}])
        self.assertEqual(f["units"]["ollama.service"], "active")
        self.assertEqual(f["failed_units"], ["bad.service"])


class AnsibleInventory(unittest.TestCase):
    def test_groups_by_role_and_skips_nodes_without_ip(self):
        inv = {"nodes": {"ctl": {"role": "control", "ip": "10.0.0.1", "required": True},
                         "g1": {"role": "gpu", "ip": "10.0.0.2"},
                         "b1": {"role": "build", "ip": None},
                         "g2": {"role": "gpu", "ip": "10.0.0.3", "discovered": True}}}
        out = x3cluster.ansible_inventory(inv, me="ctl")
        self.assertEqual(out["gpu"]["hosts"], ["g1", "g2"])
        self.assertNotIn("build", out)
        self.assertEqual(sorted(out["cluster"]["children"]), ["control", "gpu"])
        self.assertEqual(out["_meta"]["hostvars"]["ctl"]["ansible_connection"], "local")
        self.assertEqual(out["_meta"]["hostvars"]["g1"]["ansible_host"], "10.0.0.2")
        self.assertTrue(out["_meta"]["hostvars"]["g2"]["x3_discovered"])
        self.assertNotIn("b1", out["_meta"]["hostvars"])


if __name__ == "__main__":
    unittest.main()
