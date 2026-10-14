#!/usr/bin/env python3
"""Tests for x3cluster.py. Every system call is mocked: nothing here touches
systemd, nvidia-smi, the network or the real ~/.config.

    python3 -m unittest scripts/x3-cluster/test_x3cluster.py
"""
import importlib.util
import json
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("x3cluster", HERE / "x3cluster.py")
x3cluster = importlib.util.module_from_spec(spec)
spec.loader.exec_module(x3cluster)

GPU0 = {"uuid": "GPU-aaaa", "name": "NVIDIA GeForce RTX 3080", "mem_mib": 10240, "compute_cap": "8.6", "pci": "01:00.0"}
GPU1 = {"uuid": "GPU-bbbb", "name": "NVIDIA GeForce GTX 1080", "mem_mib": 8192, "compute_cap": "6.1", "pci": "02:00.0"}


def entry(rank, state, models=("qwen3:8b",), **extra):
    return {"rank": rank, "gpu": GPU0, "port": 11434 + 2 * rank, "worker": f"n-gpu{rank}",
            "state": state, "models": list(models), **extra}


class RegistrationTests(unittest.TestCase):
    def test_only_adopted_or_started_workers_are_registered(self):
        plan = [entry(0, "adopted"), entry(1, "conflict"), entry(2, "system-unverified"),
                entry(3, "create", started=False), entry(4, "create", started=True)]
        reg = x3cluster.registration("n", "10.0.0.5", plan, "lan")
        workers = {p["worker"] for p in reg["providers"].values()}
        self.assertEqual(workers, {"n-gpu0", "n-gpu4"})

    def test_registration_uses_the_given_host_and_scope(self):
        reg = x3cluster.registration("n", "127.0.0.1", [entry(0, "adopted")], "local")
        self.assertEqual(reg["scope"], "local")
        self.assertTrue(all(p["base_url"].startswith("http://127.0.0.1:") for p in reg["providers"].values()))

    def test_no_registrable_model_means_no_registration(self):
        self.assertIsNone(x3cluster.registration("n", "h", [entry(0, "adopted", models=())], "lan"))
        self.assertIsNone(x3cluster.registration("n", "h", [entry(0, "conflict")], "lan"))


class PlanTests(unittest.TestCase):
    def plan(self, listener, ollama, service_active):
        run = lambda cmd, timeout=30: (0, "active" if service_active else "inactive")  # noqa: E731
        with mock.patch.object(x3cluster, "gpus", return_value=[GPU0]), \
                mock.patch.object(x3cluster, "ollama_process_on", return_value=listener), \
                mock.patch.object(x3cluster, "is_ollama", return_value=ollama), \
                mock.patch.object(x3cluster, "ollama_models", return_value=[]), \
                mock.patch.object(x3cluster, "run", side_effect=run):
            return x3cluster.plan_gpu_workers("n", "10.0.0.5", "127.0.0.1")

    def test_an_uninspectable_listener_that_is_not_ollama_is_a_conflict(self):
        plan, staged = self.plan(("other", None), ollama=False, service_active=True)
        self.assertEqual(plan[0]["state"], "conflict")
        self.assertEqual(staged, [], "nothing may overwrite and restart ollama.service")

    def test_an_uninspectable_ollama_without_the_system_unit_is_a_conflict(self):
        plan, staged = self.plan(("other", None), ollama=True, service_active=False)
        self.assertEqual(plan[0]["state"], "conflict")
        self.assertEqual(staged, [])

    def test_the_system_ollama_service_gets_a_staged_dropin(self):
        plan, staged = self.plan(("other", None), ollama=True, service_active=True)
        self.assertEqual(plan[0]["state"], "system-unverified")
        self.assertEqual(len(staged), 1)

    def test_a_listener_pinned_to_another_gpu_is_a_conflict(self):
        plan, _ = self.plan((123, {"CUDA_VISIBLE_DEVICES": "GPU-zzzz"}), ollama=True, service_active=True)
        self.assertEqual(plan[0]["state"], "conflict")


class BootstrapTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        root = Path(self.tmp.name)
        self.config, self.units = root / "config", root / "units"
        self.patches = [
            mock.patch.object(x3cluster, "CONFIG", self.config),
            mock.patch.object(x3cluster, "USER_UNITS", self.units),
            mock.patch.object(x3cluster.socket, "gethostname", return_value="x3gpu-test"),
            mock.patch.object(x3cluster, "nics", return_value=[{"ipv4": "192.168.0.99/24", "state": "up"}]),
            mock.patch.object(x3cluster, "gpus", return_value=[GPU0, GPU1]),
            mock.patch.object(x3cluster, "ollama_process_on", return_value=(None, None)),
            mock.patch.object(x3cluster, "wait_for_ollama", return_value=True),
            mock.patch.object(x3cluster, "run", return_value=(0, "yes")),
            mock.patch.object(x3cluster.shutil, "which", return_value="/usr/bin/x"),
        ]
        for patch in self.patches:
            patch.start()

    def tearDown(self):
        for patch in self.patches:
            patch.stop()
        self.tmp.cleanup()

    def bootstrap(self, lan, models):
        with mock.patch.object(x3cluster, "ollama_models", return_value=models), \
                mock.patch("builtins.print"):
            x3cluster.bootstrap(SimpleNamespace(role="gpu", apply=True, lan=lan))

    def test_lan_workers_start_on_loopback_and_rebind_after_the_firewall(self):
        self.bootstrap(lan=True, models=["qwen3:8b"])
        for unit in self.units.glob("*.service"):
            self.assertIn("OLLAMA_HOST=127.0.0.1:", unit.read_text())
            self.assertNotIn("0.0.0.0", unit.read_text())
        script = (self.config / "staged-privileged.sh").read_text()
        firewall = script.index("ufw --force enable")
        rebind = script.index("OLLAMA_HOST=0.0.0.0:11434")
        self.assertLess(firewall, rebind, "rebind must come after the firewall is on")
        reg = json.loads((self.config / "registration" / "x3gpu-test.json").read_text())
        self.assertTrue(all("192.168.0.99" in p["base_url"] for p in reg["providers"].values()))

    def test_without_lan_the_registration_points_at_loopback(self):
        self.bootstrap(lan=False, models=["qwen3:8b"])
        reg = json.loads((self.config / "registration" / "x3gpu-test.json").read_text())
        self.assertTrue(reg["scope"].startswith("local"))
        self.assertTrue(all(p["base_url"].startswith("http://127.0.0.1:") for p in reg["providers"].values()))

    def test_new_workers_without_models_get_a_staged_pull_and_no_registration(self):
        stale = self.config / "registration" / "x3gpu-test.json"
        stale.parent.mkdir(parents=True)
        stale.write_text("{}")
        self.bootstrap(lan=False, models=[])
        self.assertFalse(stale.exists(), "a stale registration must not keep advertising dead providers")
        script = (self.config / "staged-privileged.sh").read_text()
        self.assertIn("OLLAMA_HOST=127.0.0.1:11434 ollama pull qwen3:8b", script)


class GateTests(unittest.TestCase):
    def gate(self, local_services, remote):
        report = {"repo": {"commit": "c0ffee", "branch": "b", "dirty": False}, "nodes": [
            {"node": "me", "role": "control", "reachable": True, "ssh": True, "clock_synced": True,
             "disk_free_gb": 50, "failed_units": [], "services": local_services, "router": None,
             "ollama_workers": {}},
            {"node": "peer", "role": "gpu", "reachable": True, "ssh": True, "ip": "10.0.0.2", "remote": remote},
        ]}
        with tempfile.TemporaryDirectory() as tmp, \
                mock.patch.object(x3cluster, "health", return_value=report), \
                mock.patch.object(x3cluster.socket, "gethostname", return_value="me"), \
                mock.patch.object(x3cluster, "read_env", return_value={"X3_NODE_ROLE": "control"}), \
                mock.patch.object(x3cluster, "REPO", Path(tmp)), \
                mock.patch("builtins.print"):
            x3cluster.gate(SimpleNamespace())
            gate = json.loads((Path(tmp) / "audit-artifacts/x3-cluster/c0ffee/gate-me.json").read_text())
        return {c["check"]: c["pass"] for c in gate["checks"]}

    def test_a_stopped_declared_service_fails_the_gate(self):
        checks = self.gate({"ssh": "active", "x3-ai-router": "inactive"},
                           {"hostname": "peer", "node_name": "peer", "role": "gpu"})
        self.assertFalse(checks["me.service.x3-ai-router"])
        self.assertTrue(checks["me.service.ssh"])
        self.assertFalse(checks["me.router_health"])

    def test_a_remote_machine_with_another_identity_fails_the_gate(self):
        checks = self.gate({"ssh": "active"}, {"hostname": "imposter", "node_name": None, "role": None})
        self.assertFalse(checks["peer.identity"])
        checks = self.gate({"ssh": "active"}, {"hostname": "peer", "node_name": "peer", "role": "gpu"})
        self.assertTrue(checks["peer.identity"])


class SshTargetTests(unittest.TestCase):
    def test_name_only_when_it_resolves_to_the_probed_address(self):
        with mock.patch.object(x3cluster.socket, "gethostbyname", return_value="10.0.0.2"):
            self.assertEqual(x3cluster.ssh_target("peer", "10.0.0.2"), "peer")
            self.assertEqual(x3cluster.ssh_target("peer", "10.0.0.9"), "10.0.0.9")
        with mock.patch.object(x3cluster.socket, "gethostbyname", side_effect=OSError):
            self.assertEqual(x3cluster.ssh_target("peer", "10.0.0.2"), "10.0.0.2")


if __name__ == "__main__":
    unittest.main()
