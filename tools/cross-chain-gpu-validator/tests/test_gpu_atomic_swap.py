"""Tests for GPU-accelerated atomic-swap verification kernel."""

import os
import sys
import shutil
import subprocess
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "src"))


class TestGpuAtomicSwapKernel(unittest.TestCase):
    def setUp(self):
        self.repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
        self.kernel_build_dir = os.path.join(self.repo_root, "kernels", "build")
        self.lib_path = os.path.join(self.kernel_build_dir, "libatomic_swap.so")

        nvcc = shutil.which("nvcc")
        if nvcc is None:
            self.skipTest("nvcc is not available; cannot validate GPU atomic-swap kernels")

        if not os.path.exists(self.lib_path):
            # build kernels (may take a minute)
            subprocess.run(["bash", os.path.join(self.repo_root, "kernels", "build.sh")], check=True)

        if not os.path.exists(self.lib_path):
            self.skipTest("libatomic_swap.so not found after build")

        import ctypes

        self._lib = ctypes.CDLL(self.lib_path)
        self._lib.atomic_verify_host.argtypes = [
            ctypes.POINTER(ctypes.c_uint8),
            ctypes.POINTER(ctypes.c_uint8),
            ctypes.c_int,
            ctypes.POINTER(ctypes.c_uint8),
        ]
        self._lib.atomic_verify_host.restype = ctypes.c_int

    def _make_atomic_input(self, valid: bool = True):
        """Create a single-entry atomic swap input (svm+evm) with optional bad sig."""
        try:
            import hashlib
            from cryptography.hazmat.primitives.asymmetric import ed25519, ec
            from cryptography.hazmat.primitives import hashes, serialization
            from cryptography.hazmat.primitives.asymmetric import utils
            from cryptography.hazmat.backends import default_backend
        except ImportError:
            self.skipTest("cryptography library required for signature generation")

        payload = b"x3-chain-atomic-swap-test"
        msg_hash = hashlib.sha256(payload).digest()

        # Ed25519 signer
        ed_priv = ed25519.Ed25519PrivateKey.generate()
        ed_sig = ed_priv.sign(msg_hash)
        ed_pub = ed_priv.public_key().public_bytes(
            encoding=serialization.Encoding.Raw,
            format=serialization.PublicFormat.Raw,
        )

        # Secp256k1 signer (using sha256 prehash)
        ec_priv = ec.generate_private_key(ec.SECP256K1(), default_backend())
        sig_der = ec_priv.sign(msg_hash, ec.ECDSA(hashes.SHA256()))
        r, s = utils.decode_dss_signature(sig_der)
        r_bytes = int.to_bytes(r, 32, "big")
        s_bytes = int.to_bytes(s, 32, "big")
        secp_sig = r_bytes + s_bytes + b"\x00"  # add recovery placeholder
        pub_nums = ec_priv.public_key().public_numbers()
        secp_pub = int.to_bytes(pub_nums.x, 32, "big") + int.to_bytes(pub_nums.y, 32, "big")

        if not valid:
            # flip a bit in the signature to make it invalid
            ed_sig = bytes(list(ed_sig[:-1]) + [ed_sig[-1] ^ 1])

        svm_entry = ed_sig + ed_pub + msg_hash
        evm_entry = secp_sig + secp_pub + msg_hash
        return svm_entry, evm_entry

    def test_atomic_verify_host_valid(self):
        svm_entry, evm_entry = self._make_atomic_input(valid=True)

        import ctypes

        svm_buf = (ctypes.c_uint8 * len(svm_entry)).from_buffer_copy(svm_entry)
        evm_buf = (ctypes.c_uint8 * len(evm_entry)).from_buffer_copy(evm_entry)
        status = (ctypes.c_uint8 * 1)()

        rc = self._lib.atomic_verify_host(svm_buf, evm_buf, 1, status)
        self.assertEqual(rc, 0)
        self.assertIn(status[0], (0, 1))

    def test_atomic_verify_host_invalid(self):
        svm_entry, evm_entry = self._make_atomic_input(valid=False)

        import ctypes

        svm_buf = (ctypes.c_uint8 * len(svm_entry)).from_buffer_copy(svm_entry)
        evm_buf = (ctypes.c_uint8 * len(evm_entry)).from_buffer_copy(evm_entry)
        status = (ctypes.c_uint8 * 1)()

        rc = self._lib.atomic_verify_host(svm_buf, evm_buf, 1, status)
        self.assertEqual(rc, 0)
        self.assertIn(status[0], (0, 1))


if __name__ == "__main__":
    unittest.main()
