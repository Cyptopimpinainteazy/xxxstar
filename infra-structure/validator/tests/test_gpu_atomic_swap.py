"""Tests for the optional GPU atomic-swap verification kernel."""

import ctypes
import hashlib
import os
import shutil
import subprocess
import unittest


class TestGpuAtomicSwapKernel(unittest.TestCase):
    def setUp(self):
        self.repo_root = os.path.abspath(
            os.path.join(os.path.dirname(__file__), "..")
        )
        self.kernel_build_dir = os.path.join(self.repo_root, "kernels", "build")
        self.lib_path = os.path.join(self.kernel_build_dir, "libatomic_swap.so")

        if shutil.which("nvcc") is None:
            self.skipTest("nvcc is not available; cannot validate GPU atomic-swap kernels")

        if not os.path.exists(self.lib_path):
            subprocess.run(
                ["bash", os.path.join(self.repo_root, "kernels", "build.sh")],
                check=True,
            )
        if not os.path.exists(self.lib_path):
            self.skipTest("libatomic_swap.so not found after build")

        self._lib = ctypes.CDLL(self.lib_path)
        self._lib.atomic_verify_host.argtypes = [
            ctypes.POINTER(ctypes.c_uint8),
            ctypes.POINTER(ctypes.c_uint8),
            ctypes.c_int,
            ctypes.POINTER(ctypes.c_uint8),
        ]
        self._lib.atomic_verify_host.restype = ctypes.c_int

    def _make_atomic_input(self, valid: bool = True):
        try:
            from cryptography.hazmat.primitives import hashes, serialization
            from cryptography.hazmat.primitives.asymmetric import ec, ed25519, utils
        except ImportError:
            self.skipTest("cryptography library required for signature generation")

        msg_hash = hashlib.sha256(b"x3-chain-atomic-swap-test").digest()

        ed_private = ed25519.Ed25519PrivateKey.generate()
        ed_signature = bytearray(ed_private.sign(msg_hash))
        ed_public = ed_private.public_key().public_bytes(
            encoding=serialization.Encoding.Raw,
            format=serialization.PublicFormat.Raw,
        )

        ec_private = ec.generate_private_key(ec.SECP256K1())
        der_signature = ec_private.sign(msg_hash, ec.ECDSA(hashes.SHA256()))
        r, s = utils.decode_dss_signature(der_signature)
        secp_signature = (
            int.to_bytes(r, 32, "big")
            + int.to_bytes(s, 32, "big")
            + b"\x00"
        )
        public_numbers = ec_private.public_key().public_numbers()
        secp_public = (
            int.to_bytes(public_numbers.x, 32, "big")
            + int.to_bytes(public_numbers.y, 32, "big")
        )

        if not valid:
            ed_signature[-1] ^= 1

        return bytes(ed_signature) + ed_public + msg_hash, secp_signature + secp_public + msg_hash

    def test_atomic_verify_host_requires_both_valid_signatures(self):
        svm_entry, evm_entry = self._make_atomic_input(valid=True)
        svm_buffer = (ctypes.c_uint8 * len(svm_entry)).from_buffer_copy(svm_entry)
        evm_buffer = (ctypes.c_uint8 * len(evm_entry)).from_buffer_copy(evm_entry)
        status = (ctypes.c_uint8 * 1)()

        return_code = self._lib.atomic_verify_host(
            svm_buffer, evm_buffer, 1, status
        )
        self.assertEqual(return_code, 0)
        self.assertEqual(status[0], 1)

    def test_atomic_verify_host_rejects_invalid_signature(self):
        svm_entry, evm_entry = self._make_atomic_input(valid=False)
        svm_buffer = (ctypes.c_uint8 * len(svm_entry)).from_buffer_copy(svm_entry)
        evm_buffer = (ctypes.c_uint8 * len(evm_entry)).from_buffer_copy(evm_entry)
        status = (ctypes.c_uint8 * 1)()

        return_code = self._lib.atomic_verify_host(
            svm_buffer, evm_buffer, 1, status
        )
        self.assertEqual(return_code, 0)
        self.assertEqual(status[0], 0)


if __name__ == "__main__":
    unittest.main()
