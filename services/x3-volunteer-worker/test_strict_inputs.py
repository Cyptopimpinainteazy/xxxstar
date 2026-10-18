"""Consent and protocol versions must not use Python truthiness/coercion."""
import hashlib
import os
import tempfile
import unittest

from dispatch import execute_signed_lease
from lease_auth import LeaseVerifier, sign
from safe_benchmark import execute

MANIFEST = {"kind": "fixed_cpu_smoke", "version": 1}
MANIFEST_BYTES = b'{"kind":"fixed_cpu_smoke","version":1}'
SECRET = b"strict-input-test-key-material!!!"


class StrictInputsTests(unittest.TestCase):
    def test_direct_execution_requires_actual_boolean_consent(self):
        for value in ("false", "true", 1, 0, None, [], {}):
            with self.subTest(consent=value), self.assertRaises(PermissionError):
                execute(MANIFEST, consent=value)

    def test_direct_execution_requires_actual_boolean_revocation(self):
        for value in ("", 0, None, [], {}):
            with self.subTest(revoked=value), self.assertRaises(PermissionError):
                execute(MANIFEST, consent=True, revoked=value)

    def test_direct_execution_requires_integer_protocol_version(self):
        for value in (True, 1.0, "1", None):
            with self.subTest(version=value), self.assertRaises(ValueError):
                execute({"kind": "fixed_cpu_smoke", "version": value}, consent=True)

    def dispatch(self, payload=MANIFEST_BYTES, **flags):
        with tempfile.TemporaryDirectory() as directory:
            verifier = LeaseVerifier(os.path.join(directory, "leases.db"), SECRET)
            lease = {"worker_id": "node", "job_id": "job", "nonce": "nonce",
                     "expires": 200, "manifest_sha256": hashlib.sha256(payload).hexdigest()}
            try:
                try:
                    return execute_signed_lease(verifier, lease, sign(lease, SECRET),
                                                "node", payload, now=100, **flags)
                finally:
                    # Invalid safety inputs must fail before consuming a valid
                    # lease; a corrected local consent decision can still use it.
                    count = verifier.db.execute("SELECT COUNT(*) FROM used_leases").fetchone()[0]
                    self.assertEqual(count, 0, "invalid inputs consumed a lease")
            finally:
                verifier.close()

    def test_dispatch_requires_exact_boolean_safety_controls(self):
        for flags in ({"consent": "false"}, {"consent": 1},
                      {"consent": True, "revoked": None},
                      {"consent": True, "stop_requested": 0}):
            with self.subTest(flags=flags), self.assertRaises(PermissionError):
                self.dispatch(**flags)

    def test_dispatch_rejects_ambiguous_manifests_before_nonce_consumption(self):
        for payload in (
            b'{"kind":"fixed_cpu_smoke","version":true}',
            b'{"kind":"fixed_cpu_smoke","version":1.0}',
            b'{"kind":"fixed_cpu_smoke","version":2,"version":1}',
            b'{"kind":"shell","kind":"fixed_cpu_smoke","version":1}',
        ):
            with self.subTest(payload=payload), self.assertRaises(ValueError):
                self.dispatch(payload, consent=True)


if __name__ == "__main__":
    unittest.main()
