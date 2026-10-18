import subprocess
import unittest
from unittest.mock import patch
from gpu_discovery import discover, discover_nvidia, discover_amd

class DiscoveryTests(unittest.TestCase):
    @patch("gpu_discovery.subprocess.run")
    def test_nvidia_devices(self, run):
        run.return_value.stdout = "GPU-123, NVIDIA GeForce GTX 1070, 8192, 6.1\n"
        devices = discover_nvidia()
        self.assertEqual(devices[0]["vram_mib"], 8192)
        self.assertEqual(devices[0]["compute_capability"], "6.1")
        args, kwargs = run.call_args
        self.assertFalse(kwargs.get("shell", False))
        self.assertEqual(kwargs["timeout"], 3)
    @patch("gpu_discovery.subprocess.run", side_effect=FileNotFoundError())
    def test_missing_driver(self, run):
        self.assertEqual(discover_nvidia(), [])
    @patch("gpu_discovery.subprocess.run", side_effect=subprocess.TimeoutExpired("nvidia-smi",3))
    def test_timeout(self, run):
        self.assertEqual(discover_nvidia(), [])
    @patch("gpu_discovery.subprocess.run")
    def test_malformed_output_ignored(self, run):
        run.return_value.stdout = "junk\nGPU-1, Card, not-a-number, 6.1\n"
        self.assertEqual(discover_nvidia(), [])
    @patch("gpu_discovery.discover_nvidia", return_value=[])
    def test_no_automatic_execution_or_enrollment(self, probe):
        self.assertEqual(discover(), {"schema_version":1,"devices":[],
            "execution_enabled":False,"public_enrollment_enabled":False})
        self.assertEqual(discover_amd(), [])

if __name__ == "__main__":
    unittest.main()
