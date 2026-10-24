import hashlib
import os
import tempfile
import unittest
from lease_auth import LeaseVerifier, sign

SECRET = b"x" * 32
MANIFEST = b'{"kind":"benchmark","seconds":2}'

def lease(nonce="n1"):
    return {"worker_id":"node-1","job_id":"job-1","nonce":nonce,
            "expires":2000,"manifest_sha256":hashlib.sha256(MANIFEST).hexdigest()}

class LeaseTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.path = os.path.join(self.tmp.name, "leases.db")
        self.verifier = LeaseVerifier(self.path, SECRET)

    def tearDown(self):
        self.verifier.close()
        self.tmp.cleanup()

    def test_valid_then_replay(self):
        l = lease()
        self.assertTrue(self.verifier.accept(l, sign(l, SECRET), "node-1", "job-1", MANIFEST, 100))
        with self.assertRaisesRegex(ValueError, "replayed"):
            self.verifier.accept(l, sign(l, SECRET), "node-1", "job-1", MANIFEST, 100)

    def test_restart_preserves_nonce(self):
        l = lease()
        self.verifier.accept(l, sign(l, SECRET), "node-1", "job-1", MANIFEST, 100)
        self.verifier.close()
        self.verifier = LeaseVerifier(self.path, SECRET)
        with self.assertRaisesRegex(ValueError, "replayed"):
            self.verifier.accept(l, sign(l, SECRET), "node-1", "job-1", MANIFEST, 100)

    def test_tampering_and_wrong_worker(self):
        l = lease()
        with self.assertRaisesRegex(ValueError, "unauthenticated"):
            self.verifier.accept(l, "0"*64, "node-1", "job-1", MANIFEST, 100)
        with self.assertRaisesRegex(ValueError, "wrong worker"):
            self.verifier.accept(l, sign(l, SECRET), "node-2", "job-1", MANIFEST, 100)
        with self.assertRaisesRegex(ValueError, "manifest mismatch"):
            self.verifier.accept(l, sign(l, SECRET), "node-1", "job-1", b"other", 100)

    def test_expired(self):
        l = lease()
        with self.assertRaisesRegex(ValueError, "expired"):
            self.verifier.accept(l, sign(l, SECRET), "node-1", "job-1", MANIFEST, 2000)

    def test_fail_closed_bad_secret_and_lease(self):
        with self.assertRaises(ValueError):
            LeaseVerifier(self.path, b"short")
        l = lease()
        l["extra"] = "unexpected"
        with self.assertRaises(ValueError):
            sign(l, SECRET)

if __name__ == "__main__":
    unittest.main()
