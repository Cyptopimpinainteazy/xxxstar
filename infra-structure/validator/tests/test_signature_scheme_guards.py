"""Signature-scheme guards found by the PR-Agent review of #677.

- SR25519 chains must never reach the Ed25519 verifier.
- A missing ``cryptography`` package must fail loudly, not reject everything.
- secp256k1 signatures over a 32-byte digest are made with ``Prehashed``.
"""

from __future__ import annotations

import hashlib
import sys
import unittest
from unittest import mock
from unittest.mock import Mock

from cryptography.hazmat.primitives import hashes
from cryptography.hazmat.primitives.asymmetric import ec, utils

from cross_chain_gpu_validator import chain_registry
from cross_chain_gpu_validator.chain_adapter import (
    ChainConfig,
    HashAlgorithm,
    SignatureAlgorithm,
)
from cross_chain_gpu_validator.cli import _build_chain_registry
from cross_chain_gpu_validator.gpu.ed25519_gpu import Ed25519BatchVerifier
from cross_chain_gpu_validator.gpu.secp256k1_gpu import Secp256k1BatchVerifier


def _config(chain_id: str, sig: SignatureAlgorithm) -> ChainConfig:
    return ChainConfig(
        chain_id=chain_id,
        chain_name=chain_id,
        rpc_url="http://127.0.0.1:1",
        sig_algorithm=sig,
        hash_algorithm=HashAlgorithm.SHA256,
        sig_pubkey_size=32,
        sig_signature_size=64,
        hash_output_size=32,
        supports_gpu=True,
    )


class TestSr25519IsNotServedAsEd25519(unittest.TestCase):
    def test_fallback_registry_labels_polkadot_and_kusama_sr25519(self):
        with mock.patch.object(
            chain_registry, "_load_configs_from_file", return_value={}
        ):
            configs = chain_registry.load_default_chain_configs()
        for chain_id in ("polkadot", "kusama"):
            self.assertEqual(
                configs[chain_id].sig_algorithm, SignatureAlgorithm.SR25519
            )

    def test_sr25519_chain_is_skipped_not_routed_to_ed25519(self):
        configs = {
            "polkadot": _config("polkadot", SignatureAlgorithm.SR25519),
            "solana": _config("solana", SignatureAlgorithm.ED25519),
        }
        logger = Mock()
        with mock.patch(
            "cross_chain_gpu_validator.cli.load_default_chain_configs",
            return_value=configs,
        ):
            registry = _build_chain_registry(Mock(), Mock(), Mock(), Mock(), logger)

        self.assertIsNotNone(registry.get_validator("solana"))
        self.assertIsNone(registry.get_validator("polkadot"))
        warnings = [call.args[0] for call in logger.warning.call_args_list]
        self.assertTrue(any("SR25519" in w for w in warnings), warnings)


class TestCpuVerifiersFailLoudly(unittest.TestCase):
    def test_missing_cryptography_raises_instead_of_rejecting(self):
        # A None entry makes `import cryptography.exceptions` raise ImportError.
        with mock.patch.dict(sys.modules, {"cryptography.exceptions": None}):
            with self.assertRaises(RuntimeError):
                Secp256k1BatchVerifier._verify_cpu(
                    [b"\x01" * 64], [b"\x02" * 32], [b"\x03" * 64]
                )
            with self.assertRaises(RuntimeError):
                Ed25519BatchVerifier._verify_cpu(
                    [b"\x01" * 64], [b"\x02" * 32], [b"\x03" * 32]
                )


class TestSecp256k1DigestConvention(unittest.TestCase):
    def _sign(self, digest: bytes, algorithm) -> tuple[bytes, bytes]:
        key = ec.generate_private_key(ec.SECP256K1())
        r, s = utils.decode_dss_signature(key.sign(digest, ec.ECDSA(algorithm)))
        numbers = key.public_key().public_numbers()
        pubkey = numbers.x.to_bytes(32, "big") + numbers.y.to_bytes(32, "big")
        return r.to_bytes(32, "big") + s.to_bytes(32, "big"), pubkey

    def test_prehashed_signature_verifies_and_double_hashed_does_not(self):
        digest = hashlib.sha256(b"atomic swap leg").digest()
        good_sig, good_pub = self._sign(digest, utils.Prehashed(hashes.SHA256()))
        bad_sig, bad_pub = self._sign(digest, hashes.SHA256())

        results = Secp256k1BatchVerifier._verify_cpu(
            [good_sig, bad_sig], [digest, digest], [good_pub, bad_pub]
        )
        self.assertEqual(results, [True, False])


if __name__ == "__main__":
    unittest.main()
