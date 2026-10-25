import hashlib
import json
import os
import tempfile
import unittest
from dispatch import execute_signed_lease
from lease_auth import LeaseVerifier, sign

SECRET = b"k" * 32
MANIFEST = b'{"kind":"fixed_cpu_smoke","version":1}'

def lease(data=MANIFEST):
    return {"worker_id":"node-1","job_id":"job-1","nonce":"unique-nonce",
            "expires":2000,"manifest_sha256":hashlib.sha256(data).hexdigest()}

class DispatchTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.verifier = LeaseVerifier(os.path.join(self.tmp.name, "replay.db"), SECRET)
    def tearDown(self):
        self.verifier.close()
        self.tmp.cleanup()
    def run_job(self, data=MANIFEST, **kwargs):
        l = lease(data)
        return execute_signed_lease(self.verifier, l, sign(l, SECRET),
                                    "node-1", data, now=100, **kwargs)
    def test_authenticated_fixed_workload(self):
        self.assertEqual(len(self.run_job(consent=True)["sha256"]), 64)
    def test_replay_denied(self):
        self.run_job(consent=True)
        with self.assertRaisesRegex(ValueError, "replayed"):
            self.run_job(consent=True)
    def test_consent_and_stop(self):
        for kw in ({"consent":False}, {"consent":True,"revoked":True},
                   {"consent":True,"stop_requested":True}):
            with self.subTest(kw=kw), self.assertRaises(PermissionError):
                self.run_job(**kw)
    def test_arbitrary_execution_denied(self):
        malicious = json.dumps({"kind":"shell","command":"echo hi"}).encode()
        with self.assertRaisesRegex(ValueError, "allowlisted"):
            self.run_job(malicious, consent=True)
    def test_manifest_tampering_denied(self):
        l = lease()
        with self.assertRaisesRegex(ValueError, "manifest mismatch"):
            execute_signed_lease(self.verifier, l, sign(l, SECRET), "node-1",
                                 b'{ "kind": "fixed_cpu_smoke", "version": 1 }',
                                 consent=True, now=100)
    def test_invalid_manifest_denied(self):
        with self.assertRaisesRegex(ValueError, "invalid manifest"):
            self.run_job(b"not json", consent=True)
    def test_oversized_manifest_denied(self):
        with self.assertRaisesRegex(ValueError, "size"):
            self.run_job(b"x"*1025, consent=True)

if __name__ == "__main__":
    unittest.main()
