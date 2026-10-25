import unittest
from safe_benchmark import execute

class SafeBenchmarkTests(unittest.TestCase):
    def test_fixed_job(self):
        self.assertEqual(len(execute({"kind":"fixed_cpu_smoke","version":1}, consent=True)["sha256"]), 64)
    def test_requires_consent(self):
        with self.assertRaises(PermissionError):
            execute({"kind":"fixed_cpu_smoke","version":1}, consent=False)
    def test_revocation(self):
        with self.assertRaises(PermissionError):
            execute({"kind":"fixed_cpu_smoke","version":1}, consent=True, revoked=True)
    def test_rejects_arbitrary_code(self):
        with self.assertRaises(ValueError):
            execute({"kind":"shell","version":1,"command":"echo unsafe"}, consent=True)
    def test_rejects_unknown_manifest(self):
        with self.assertRaises(ValueError):
            execute({"kind":"gpu","version":1}, consent=True)
    def test_timeout_bound(self):
        with self.assertRaises(ValueError):
            execute({"kind":"fixed_cpu_smoke","version":1}, consent=True, timeout=500)

if __name__ == "__main__":
    unittest.main()
