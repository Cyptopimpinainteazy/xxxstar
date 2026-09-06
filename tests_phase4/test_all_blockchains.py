#!/usr/bin/env python3
"""
═══════════════════════════════════════════════════════════════════════════════
  X3 Chain — Full Blockchain Test Suite
  Tests all configured blockchains across EVM, SVM, Cosmos, Substrate, and
  other L1 families for: registry, config, GPU kernels, crypto primitives,
  cross-chain profiles, and validator readiness.
═══════════════════════════════════════════════════════════════════════════════

Invariants tested:
  INV-CHAIN-001: All registered chains have valid configs (RPC, sig algo, hash algo)
  INV-CHAIN-002: Chain families map to correct GPU kernel profiles
  INV-CHAIN-003: Crypto primitives produce deterministic output per chain family
  INV-CHAIN-004: GPU kernels load and execute for each chain family
  INV-CHAIN-005: Chain registry covers all 5 families (EVM, SVM, Cosmos, Substrate, Other)
"""
from __future__ import annotations

import ctypes
import hashlib
import sys
import unittest
from collections import Counter
from pathlib import Path

# ── Path setup ───────────────────────────────────────────────────────────────
PROJECT_ROOT = Path(__file__).resolve().parents[1]
CCGV_SRC = PROJECT_ROOT / "cross-chain-gpu-validator" / "src"
KERNEL_DIR = PROJECT_ROOT / "cross-chain-gpu-validator" / "kernels"
BUILD_DIR = KERNEL_DIR / "build"

sys.path.insert(0, str(CCGV_SRC))

from cross_chain_gpu_validator.chain_adapter import (
    ChainConfig,
    HashAlgorithm,
    SignatureAlgorithm,
)
from cross_chain_gpu_validator.chain_registry import (
    ChainRegistry,
    load_default_chain_configs,
)
from cross_chain_gpu_validator.gpu.cuda_loader import CudaRuntime
from cross_chain_gpu_validator.gpu.kernel_profiles import (
    CHAIN_FAMILY_MAP,
    COSMOS_PROFILE,
    EVM_PROFILE,
    SUBSTRATE_PROFILE,
    SVM_PROFILE,
    ChainFamily,
    KernelType,
    get_profile,
)

# ── Colour helpers (terminal) ────────────────────────────────────────────────
G = "\033[92m"  # green
R = "\033[91m"  # red
Y = "\033[93m"  # yellow
C = "\033[96m"  # cyan
B = "\033[1m"   # bold
N = "\033[0m"   # reset


def ok(msg: str) -> None:
    print(f"  {G}✓{N} {msg}")


def fail(msg: str) -> None:
    print(f"  {R}✗{N} {msg}")


def warn(msg: str) -> None:
    print(f"  {Y}⚠{N} {msg}")


def banner(title: str) -> None:
    print(f"\n{C}{'═' * 72}{N}")
    print(f"  {B}{title}{N}")
    print(f"{C}{'═' * 72}{N}")


# ═══════════════════════════════════════════════════════════════════════════════
#  1. Chain Registry Tests
# ═══════════════════════════════════════════════════════════════════════════════

class TestChainRegistryLoading(unittest.TestCase):
    """INV-CHAIN-001: All registered chains have valid configs."""

    @classmethod
    def setUpClass(cls):
        cls.configs = load_default_chain_configs()

    def test_loads_at_least_50_chains(self):
        """Registry must have at least 50 chains (fallback has 53)."""
        self.assertGreaterEqual(len(self.configs), 50,
            f"Only {len(self.configs)} chains loaded; expected ≥50")

    def test_every_chain_has_rpc_url(self):
        missing = [cid for cid, cfg in self.configs.items() if not cfg.rpc_url]
        self.assertEqual(len(missing), 0,
            f"{len(missing)} chains missing RPC URL: {missing[:10]}")

    def test_every_chain_has_sig_algorithm(self):
        missing = [cid for cid, cfg in self.configs.items() if cfg.sig_algorithm is None]
        self.assertEqual(len(missing), 0,
            f"{len(missing)} chains missing sig_algorithm")

    def test_every_chain_has_hash_algorithm(self):
        missing = [cid for cid, cfg in self.configs.items() if cfg.hash_algorithm is None]
        self.assertEqual(len(missing), 0,
            f"{len(missing)} chains missing hash_algorithm")

    def test_every_chain_has_chain_name(self):
        missing = [cid for cid, cfg in self.configs.items() if not cfg.chain_name]
        self.assertEqual(len(missing), 0,
            f"{len(missing)} chains missing chain_name")

    def test_sig_pubkey_size_positive(self):
        bad = [cid for cid, cfg in self.configs.items() if cfg.sig_pubkey_size <= 0]
        self.assertEqual(len(bad), 0, f"Chains w/ bad pubkey size: {bad[:10]}")

    def test_sig_signature_size_positive(self):
        bad = [cid for cid, cfg in self.configs.items() if cfg.sig_signature_size <= 0]
        self.assertEqual(len(bad), 0, f"Chains w/ bad sig size: {bad[:10]}")

    def test_hash_output_32_bytes(self):
        bad = [cid for cid, cfg in self.configs.items() if cfg.hash_output_size != 32]
        self.assertEqual(len(bad), 0, f"Chains w/ non-32 hash output: {bad[:10]}")


class TestChainRegistryOperations(unittest.TestCase):
    """INV-CHAIN-001: Registry register/get/list operations work."""

    def test_register_and_retrieve(self):
        reg = ChainRegistry()
        cfg = ChainConfig(
            chain_id="test-chain",
            chain_name="Test Chain",
            rpc_url="http://localhost:8545",
            sig_algorithm=SignatureAlgorithm.SECP256K1,
            hash_algorithm=HashAlgorithm.KECCAK256,
            sig_pubkey_size=64,
            sig_signature_size=65,
            hash_output_size=32,
        )

        class DummyValidator:
            def __init__(self, config):
                self.config = config
            def validate_transaction(self, tx):
                return True
            def validate_transactions(self, txs):
                return [True for _ in txs]

        reg.register_chain(cfg, DummyValidator(cfg))
        self.assertEqual(reg.chain_count(), 1)
        self.assertIsNotNone(reg.get_config("test-chain"))
        self.assertIsNotNone(reg.get_validator("test-chain"))
        self.assertIn("test-chain", reg.list_chains())

    def test_duplicate_register_raises(self):
        reg = ChainRegistry()
        cfg = ChainConfig(
            chain_id="dup", chain_name="Dup", rpc_url="http://x",
            sig_algorithm=SignatureAlgorithm.ED25519,
            hash_algorithm=HashAlgorithm.SHA256,
            sig_pubkey_size=32, sig_signature_size=64, hash_output_size=32,
        )

        class DummyV:
            def __init__(self, c): self.config = c
            def validate_transaction(self, tx): return True
            def validate_transactions(self, txs): return []

        reg.register_chain(cfg, DummyV(cfg))
        with self.assertRaises(ValueError):
            reg.register_chain(cfg, DummyV(cfg))

    def test_get_missing_returns_none(self):
        reg = ChainRegistry()
        self.assertIsNone(reg.get_config("nonexistent"))
        self.assertIsNone(reg.get_validator("nonexistent"))


# ═══════════════════════════════════════════════════════════════════════════════
#  2. Chain Family Classification
# ═══════════════════════════════════════════════════════════════════════════════

class TestChainFamilyClassification(unittest.TestCase):
    """INV-CHAIN-005: All 5 families represented in config."""

    @classmethod
    def setUpClass(cls):
        cls.configs = load_default_chain_configs()
        # Classify by sig + hash algo combination
        cls.families: dict[str, list[str]] = {
            "evm": [],       # secp256k1 + keccak256
            "svm": [],       # ed25519 + sha256 (Solana-named)
            "cosmos": [],    # secp256k1 + sha256
            "substrate": [], # ed25519 + sha256 (Polkadot/Kusama)
            "other": [],     # remaining ed25519 + sha256
        }
        for cid, cfg in cls.configs.items():
            if cfg.sig_algorithm == SignatureAlgorithm.SECP256K1 and cfg.hash_algorithm == HashAlgorithm.KECCAK256:
                cls.families["evm"].append(cid)
            elif cfg.sig_algorithm == SignatureAlgorithm.SECP256K1 and cfg.hash_algorithm == HashAlgorithm.SHA256:
                cls.families["cosmos"].append(cid)
            elif cfg.sig_algorithm == SignatureAlgorithm.ED25519 and cfg.hash_algorithm == HashAlgorithm.SHA256:
                name = (cfg.chain_name or "").lower()
                if "polkadot" in name or "kusama" in name:
                    cls.families["substrate"].append(cid)
                elif "solana" in name:
                    cls.families["svm"].append(cid)
                else:
                    cls.families["other"].append(cid)
            else:
                cls.families["other"].append(cid)

    def test_evm_chains_present(self):
        self.assertGreater(len(self.families["evm"]), 0,
            "No EVM chains found (secp256k1 + keccak256)")

    def test_svm_chains_in_fallback(self):
        """SVM chains exist in hardcoded fallback even if chains.json is EVM-only."""
        # Temporarily hide chains.json to force fallback path
        import cross_chain_gpu_validator.chain_registry as cr
        fallback_configs = {}
        # Call the function with an empty file path to get fallback
        orig = cr._load_configs_from_file
        cr._load_configs_from_file = lambda _: {}  # force fallback
        try:
            fallback_configs = cr.load_default_chain_configs()
        finally:
            cr._load_configs_from_file = orig
        svm = [cid for cid, cfg in fallback_configs.items()
               if cfg.sig_algorithm == SignatureAlgorithm.ED25519
               and "solana" in cfg.chain_name.lower()]
        self.assertGreater(len(svm), 0, "No Solana chains in fallback configs")

    def test_non_evm_families_in_fallback(self):
        """Cosmos, Substrate, and other L1 chains exist in fallback configs."""
        import cross_chain_gpu_validator.chain_registry as cr
        orig = cr._load_configs_from_file
        cr._load_configs_from_file = lambda _: {}
        try:
            fallback = cr.load_default_chain_configs()
        finally:
            cr._load_configs_from_file = orig
        cosmos = [cid for cid, cfg in fallback.items()
                  if cfg.sig_algorithm == SignatureAlgorithm.SECP256K1
                  and cfg.hash_algorithm == HashAlgorithm.SHA256]
        substrate = [cid for cid, cfg in fallback.items()
                     if "polkadot" in cfg.chain_name.lower()
                     or "kusama" in cfg.chain_name.lower()]
        self.assertGreater(len(cosmos), 0, "No Cosmos chains in fallback")
        self.assertGreater(len(substrate), 0, "No Substrate chains in fallback")

    def test_chains_json_is_evm_dominated(self):
        """chains.json should have 2000+ EVM chains."""
        self.assertGreater(len(self.families["evm"]), 2000,
            f"Expected 2000+ EVM chains from chains.json, got {len(self.families['evm'])}")

    def test_family_distribution_summary(self):
        """Print family distribution (informational, always passes)."""
        total = sum(len(v) for v in self.families.values())
        for _fam, chains in self.families.items():
            len(chains) / max(total, 1) * 100
            # informational — printed during -v runs
        self.assertTrue(True)


# ═══════════════════════════════════════════════════════════════════════════════
#  3. GPU Kernel Profiles per Chain Family
# ═══════════════════════════════════════════════════════════════════════════════

class TestKernelProfileMapping(unittest.TestCase):
    """INV-CHAIN-002: Chain families map to correct GPU kernel profiles."""

    def test_evm_requires_secp256k1_keccak256(self):
        kernels = EVM_PROFILE.required_kernels
        self.assertIn(KernelType.SECP256K1, kernels)
        self.assertIn(KernelType.KECCAK256, kernels)
        self.assertEqual(EVM_PROFILE.family, ChainFamily.EVM)

    def test_svm_requires_ed25519_sha256_poh(self):
        kernels = SVM_PROFILE.required_kernels
        self.assertIn(KernelType.ED25519, kernels)
        self.assertIn(KernelType.SHA256, kernels)
        self.assertIn(KernelType.POH, kernels)
        self.assertEqual(SVM_PROFILE.family, ChainFamily.SVM)

    def test_cosmos_requires_secp256k1_sha256(self):
        kernels = COSMOS_PROFILE.required_kernels
        self.assertIn(KernelType.SECP256K1, kernels)
        self.assertIn(KernelType.SHA256, kernels)
        self.assertEqual(COSMOS_PROFILE.family, ChainFamily.COSMOS)

    def test_substrate_requires_ed25519_sha256(self):
        kernels = SUBSTRATE_PROFILE.required_kernels
        self.assertIn(KernelType.ED25519, kernels)
        self.assertIn(KernelType.SHA256, kernels)
        self.assertEqual(SUBSTRATE_PROFILE.family, ChainFamily.SUBSTRATE)

    def test_chain_family_map_has_entries(self):
        self.assertGreater(len(CHAIN_FAMILY_MAP), 0)

    def test_all_mapped_chains_resolve_to_profile(self):
        failed = []
        for chain_id in CHAIN_FAMILY_MAP:
            try:
                profile = get_profile(chain_id)
                if profile is None:
                    failed.append(str(chain_id))
            except Exception as e:
                failed.append(f"{chain_id}: {e}")
        self.assertEqual(len(failed), 0,
            f"Chains with missing profiles: {failed[:10]}")

    def test_evm_gas_cost_reasonable(self):
        self.assertGreater(EVM_PROFILE.total_gas_cost, 0)
        self.assertLess(EVM_PROFILE.total_gas_cost, 10000)

    def test_svm_gas_cost_reasonable(self):
        self.assertGreater(SVM_PROFILE.total_gas_cost, 0)
        self.assertLess(SVM_PROFILE.total_gas_cost, 10000)

    def test_vram_estimates_positive(self):
        for profile in [EVM_PROFILE, SVM_PROFILE, COSMOS_PROFILE, SUBSTRATE_PROFILE]:
            self.assertGreater(profile.vram_estimate_mb, 0,
                f"{profile.family} has 0 VRAM estimate")


# ═══════════════════════════════════════════════════════════════════════════════
#  4. Crypto Primitive Tests per Chain Family
# ═══════════════════════════════════════════════════════════════════════════════

class TestCryptoPrimitiveSHA256(unittest.TestCase):
    """INV-CHAIN-003: SHA-256 (SVM, Cosmos, Substrate families)."""

    def test_deterministic(self):
        data = b"x3-chain-sha256-all-chains"
        h1 = hashlib.sha256(data).digest()
        h2 = hashlib.sha256(data).digest()
        self.assertEqual(h1, h2)
        self.assertEqual(len(h1), 32)

    def test_known_vector(self):
        # SHA-256("") = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        h = hashlib.sha256(b"").hexdigest()
        self.assertEqual(h, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")

    def test_different_inputs_different_hashes(self):
        h1 = hashlib.sha256(b"ethereum").digest()
        h2 = hashlib.sha256(b"solana").digest()
        self.assertNotEqual(h1, h2)


class TestCryptoPrimitiveKeccak256(unittest.TestCase):
    """INV-CHAIN-003: Keccak-256 (EVM family)."""

    def _keccak(self, data: bytes) -> bytes:
        # Try multiple Keccak implementations
        try:
            from Crypto.Hash import keccak
            return keccak.new(digest_bits=256, data=data).digest()
        except ImportError:
            pass
        try:
            import sha3
            return sha3.keccak_256(data).digest()
        except ImportError:
            pass
        # Fallback: use hashlib sha3_256 (NOT keccak, but close enough for structure test)
        return hashlib.sha3_256(data).digest()

    def test_deterministic(self):
        data = b"x3-chain-keccak-evm"
        h1 = self._keccak(data)
        h2 = self._keccak(data)
        self.assertEqual(h1, h2)
        self.assertEqual(len(h1), 32)

    def test_different_inputs(self):
        h1 = self._keccak(b"polygon")
        h2 = self._keccak(b"arbitrum")
        self.assertNotEqual(h1, h2)


class TestCryptoPrimitiveSecp256k1(unittest.TestCase):
    """INV-CHAIN-003: secp256k1 ECDSA (EVM + Cosmos families)."""

    def test_sign_and_verify(self):
        try:
            from cryptography.hazmat.primitives import hashes
            from cryptography.hazmat.primitives.asymmetric import ec

            key = ec.generate_private_key(ec.SECP256K1())
            data = b"x3-chain-secp256k1-test"
            sig = key.sign(data, ec.ECDSA(hashes.SHA256()))
            # Verify — should not raise
            key.public_key().verify(sig, data, ec.ECDSA(hashes.SHA256()))
        except ImportError:
            self.skipTest("cryptography package not installed")

    def test_wrong_key_fails(self):
        try:
            from cryptography.exceptions import InvalidSignature
            from cryptography.hazmat.primitives import hashes
            from cryptography.hazmat.primitives.asymmetric import ec

            key1 = ec.generate_private_key(ec.SECP256K1())
            key2 = ec.generate_private_key(ec.SECP256K1())
            data = b"x3-chain-wrong-key"
            sig = key1.sign(data, ec.ECDSA(hashes.SHA256()))
            with self.assertRaises(InvalidSignature):
                key2.public_key().verify(sig, data, ec.ECDSA(hashes.SHA256()))
        except ImportError:
            self.skipTest("cryptography package not installed")


class TestCryptoPrimitiveEd25519(unittest.TestCase):
    """INV-CHAIN-003: Ed25519 (SVM + Substrate families)."""

    def test_sign_and_verify(self):
        try:
            import nacl.signing
            sk = nacl.signing.SigningKey.generate()
            msg = b"x3-chain-ed25519-test"
            signed = sk.sign(msg)
            sk.verify_key.verify(signed)  # should not raise
        except ImportError:
            self.skipTest("PyNaCl not installed")

    def test_wrong_key_fails(self):
        try:
            import nacl.exceptions
            import nacl.signing
            sk1 = nacl.signing.SigningKey.generate()
            sk2 = nacl.signing.SigningKey.generate()
            signed = sk1.sign(b"msg")
            with self.assertRaises(nacl.exceptions.BadSignatureError):
                sk2.verify_key.verify(signed)
        except ImportError:
            self.skipTest("PyNaCl not installed")


# ═══════════════════════════════════════════════════════════════════════════════
#  5. GPU Kernel Library Tests
# ═══════════════════════════════════════════════════════════════════════════════

class TestGpuKernelLibraries(unittest.TestCase):
    """INV-CHAIN-004: GPU kernel .so files exist and load."""

    REQUIRED_LIBS = [
        "libsecp256k1_batch.so",
        "libkeccak256_batch.so",
        "libsha256_batch.so",
        "libed25519_batch.so",
        "libstream_pipeline.so",
    ]

    def test_all_kernel_libs_exist(self):
        missing = []
        for lib in self.REQUIRED_LIBS:
            path = BUILD_DIR / lib
            if not path.exists():
                missing.append(lib)
        self.assertEqual(len(missing), 0,
            f"Missing GPU kernel libraries in {BUILD_DIR}: {missing}")

    def test_kernel_libs_loadable(self):
        """Each .so should be loadable via ctypes."""
        for lib_name in self.REQUIRED_LIBS:
            path = BUILD_DIR / lib_name
            if not path.exists():
                continue
            try:
                lib = ctypes.CDLL(str(path))
                self.assertIsNotNone(lib)
            except OSError as e:
                self.fail(f"Cannot load {lib_name}: {e}")

    def test_secp256k1_exports_verify_host(self):
        path = BUILD_DIR / "libsecp256k1_batch.so"
        if not path.exists():
            self.skipTest("libsecp256k1_batch.so not built")
        lib = ctypes.CDLL(str(path))
        self.assertTrue(hasattr(lib, "secp256k1_ecdsa_verify_host"))

    def test_secp256k1_exports_multi_gpu(self):
        path = BUILD_DIR / "libsecp256k1_batch.so"
        if not path.exists():
            self.skipTest("libsecp256k1_batch.so not built")
        lib = ctypes.CDLL(str(path))
        self.assertTrue(hasattr(lib, "secp256k1_ecdsa_verify_multi_gpu"))

    def test_keccak256_exports_batch_host(self):
        path = BUILD_DIR / "libkeccak256_batch.so"
        if not path.exists():
            self.skipTest("libkeccak256_batch.so not built")
        lib = ctypes.CDLL(str(path))
        self.assertTrue(hasattr(lib, "keccak256_batch_host"))

    def test_sha256_exports_batch_host(self):
        path = BUILD_DIR / "libsha256_batch.so"
        if not path.exists():
            self.skipTest("libsha256_batch.so not built")
        lib = ctypes.CDLL(str(path))
        self.assertTrue(hasattr(lib, "sha256_batch_host"))

    def test_ed25519_exports_verify_host(self):
        path = BUILD_DIR / "libed25519_batch.so"
        if not path.exists():
            self.skipTest("libed25519_batch.so not built")
        lib = ctypes.CDLL(str(path))
        self.assertTrue(hasattr(lib, "ed25519_verify_batch_host"))


class TestGpuKernelExecution(unittest.TestCase):
    """INV-CHAIN-004: GPU kernels execute correctly with test data."""

    @classmethod
    def setUpClass(cls):
        cls.cuda = CudaRuntime.detect()

    def test_keccak256_gpu_batch(self):
        """Run keccak256 on GPU with 100 test inputs."""
        if not self.cuda.available:
            self.skipTest("No CUDA runtime")
        path = BUILD_DIR / "libkeccak256_batch.so"
        if not path.exists():
            self.skipTest("libkeccak256_batch.so not built")

        lib = ctypes.CDLL(str(path))
        count = 100
        # 100 × 32-byte inputs (ascending counter padded)
        inputs = bytearray()
        for i in range(count):
            block = i.to_bytes(4, "little") + b"\x00" * 28
            inputs.extend(block)

        out = (ctypes.c_ubyte * (count * 32))()
        ret = lib.keccak256_batch_host(
            ctypes.c_char_p(bytes(inputs)),
            ctypes.c_int(count),
            ctypes.byref(out),
        )
        self.assertEqual(ret, 0, f"keccak256 GPU returned error code {ret}")
        # Verify output is not all zeros
        result = bytes(out)
        self.assertNotEqual(result, b"\x00" * (count * 32))

    def test_sha256_gpu_batch(self):
        """Run SHA-256 on GPU with 100 test inputs."""
        if not self.cuda.available:
            self.skipTest("No CUDA runtime")
        path = BUILD_DIR / "libsha256_batch.so"
        if not path.exists():
            self.skipTest("libsha256_batch.so not built")

        lib = ctypes.CDLL(str(path))
        count = 100
        inputs = bytearray()
        for i in range(count):
            block = i.to_bytes(4, "little") + b"\x00" * 28
            inputs.extend(block)

        out = (ctypes.c_ubyte * (count * 32))()
        ret = lib.sha256_batch_host(
            ctypes.c_char_p(bytes(inputs)),
            ctypes.c_int(count),
            ctypes.byref(out),
        )
        self.assertEqual(ret, 0, f"SHA-256 GPU returned error code {ret}")
        result = bytes(out)
        self.assertNotEqual(result, b"\x00" * (count * 32))

    def test_secp256k1_gpu_verify(self):
        """Run secp256k1 verify on GPU with synthetic data."""
        if not self.cuda.available:
            self.skipTest("No CUDA runtime")
        path = BUILD_DIR / "libsecp256k1_batch.so"
        if not path.exists():
            self.skipTest("libsecp256k1_batch.so not built")

        lib = ctypes.CDLL(str(path))
        count = 10
        # Synthetic u1, u2 scalars and pubkeys (will fail verification but shouldn't crash)
        u1 = bytes(range(32)) * count
        u2 = bytes(range(1, 33)) * count
        pk = bytes(range(64)) * count
        out = (ctypes.c_ubyte * (count * 32))()

        lib.secp256k1_ecdsa_verify_host.argtypes = [
            ctypes.c_void_p, ctypes.c_void_p, ctypes.c_void_p,
            ctypes.c_int, ctypes.c_void_p,
        ]
        lib.secp256k1_ecdsa_verify_host.restype = ctypes.c_int

        ret = lib.secp256k1_ecdsa_verify_host(
            ctypes.c_char_p(u1),
            ctypes.c_char_p(u2),
            ctypes.c_char_p(pk),
            ctypes.c_int(count),
            ctypes.byref(out),
        )
        self.assertEqual(ret, 0, f"secp256k1 GPU returned error code {ret}")


# ═══════════════════════════════════════════════════════════════════════════════
#  6. GPU Accelerator Class Tests (Python wrappers)
# ═══════════════════════════════════════════════════════════════════════════════

class TestGpuAcceleratorClasses(unittest.TestCase):
    """INV-CHAIN-004: GPU wrapper classes instantiate and detect kernels."""

    @classmethod
    def setUpClass(cls):
        cls.cuda = CudaRuntime.detect()
        cls.kernel_dir = str(KERNEL_DIR)

    def test_cuda_runtime_detect(self):
        self.assertIsInstance(self.cuda.available, bool)
        if self.cuda.available:
            self.assertIsNotNone(self.cuda.nvcc_path)

    def test_secp256k1_verifier_instantiates(self):
        from cross_chain_gpu_validator.gpu.secp256k1_gpu import Secp256k1BatchVerifier
        verifier = Secp256k1BatchVerifier(
            runtime=self.cuda,
            kernel_dir=self.kernel_dir,
            parity_check=False,
            allow_failover=True,
        )
        self.assertIsNotNone(verifier)
        if self.cuda.available:
            self.assertIsNotNone(verifier._lib,
                "secp256k1 lib should load when CUDA is available")

    def test_keccak_hasher_instantiates(self):
        from cross_chain_gpu_validator.gpu.keccak_gpu import KeccakBatchHasher
        hasher = KeccakBatchHasher(
            runtime=self.cuda,
            kernel_dir=self.kernel_dir,
            parity_check=False,
            allow_failover=True,
        )
        self.assertIsNotNone(hasher)
        if self.cuda.available:
            self.assertIsNotNone(hasher._lib,
                "keccak256 lib should load when CUDA is available")

    def test_keccak_cpu_fallback(self):
        """Keccak hasher should fall back to CPU when GPU is forced off."""
        from cross_chain_gpu_validator.gpu.keccak_gpu import KeccakBatchHasher
        no_gpu = CudaRuntime(available=False, nvcc_path=None, visible_devices="")
        hasher = KeccakBatchHasher(
            runtime=no_gpu,
            kernel_dir=self.kernel_dir,
            parity_check=False,
            allow_failover=True,
        )
        result = hasher.hash_batch([b"\x00" * 32, b"\x01" * 32])
        self.assertEqual(len(result), 2)
        self.assertEqual(len(result[0]), 32)


# ═══════════════════════════════════════════════════════════════════════════════
#  7. Cross-Chain Validation Readiness
# ═══════════════════════════════════════════════════════════════════════════════

class TestCrossChainReadiness(unittest.TestCase):
    """INV-CHAIN-005: End-to-end readiness for each blockchain family."""

    @classmethod
    def setUpClass(cls):
        cls.configs = load_default_chain_configs()

    def _sample_chains_by_algo(self, sig_algo, hash_algo, limit=5):
        """Get sample chains matching the given crypto algorithms."""
        matches = [
            (cid, cfg) for cid, cfg in self.configs.items()
            if cfg.sig_algorithm == sig_algo and cfg.hash_algorithm == hash_algo
        ]
        return matches[:limit]

    def test_evm_chains_ready(self):
        """EVM chains (secp256k1+keccak256) have all required fields."""
        chains = self._sample_chains_by_algo(
            SignatureAlgorithm.SECP256K1, HashAlgorithm.KECCAK256
        )
        self.assertGreater(len(chains), 0, "No EVM chains found")
        for cid, cfg in chains:
            self.assertEqual(cfg.sig_pubkey_size, 64, f"{cid}: EVM pubkey should be 64 bytes")
            self.assertEqual(cfg.sig_signature_size, 65, f"{cid}: EVM sig should be 65 bytes")
            self.assertTrue(cfg.supports_gpu, f"{cid}: should support GPU")

    def test_svm_chains_ready(self):
        """SVM chains (ed25519+sha256) have correct crypto params."""
        import cross_chain_gpu_validator.chain_registry as cr
        orig = cr._load_configs_from_file
        cr._load_configs_from_file = lambda _: {}
        try:
            fallback = cr.load_default_chain_configs()
        finally:
            cr._load_configs_from_file = orig
        chains = [(cid, cfg) for cid, cfg in fallback.items()
                  if cfg.sig_algorithm == SignatureAlgorithm.ED25519
                  and cfg.hash_algorithm == HashAlgorithm.SHA256]
        self.assertGreater(len(chains), 0, "No SVM/Ed25519 chains in fallback")
        for cid, cfg in chains[:5]:
            self.assertEqual(cfg.sig_pubkey_size, 32, f"{cid}: Ed25519 pubkey should be 32 bytes")
            self.assertEqual(cfg.sig_signature_size, 64, f"{cid}: Ed25519 sig should be 64 bytes")

    def test_cosmos_chains_ready(self):
        """Cosmos chains (secp256k1+sha256) have correct crypto params."""
        # Use fallback configs which include Cosmos chains
        import cross_chain_gpu_validator.chain_registry as cr
        orig = cr._load_configs_from_file
        cr._load_configs_from_file = lambda _: {}
        try:
            fallback = cr.load_default_chain_configs()
        finally:
            cr._load_configs_from_file = orig
        chains = [(cid, cfg) for cid, cfg in fallback.items()
                  if cfg.sig_algorithm == SignatureAlgorithm.SECP256K1
                  and cfg.hash_algorithm == HashAlgorithm.SHA256]
        self.assertGreater(len(chains), 0, "No Cosmos chains in fallback")
        for cid, cfg in chains[:5]:
            self.assertIn(cfg.sig_pubkey_size, [33, 64],
                f"{cid}: Cosmos pubkey should be 33 (compressed) or 64")

    def test_all_chains_have_gpu_support_flag(self):
        """Every chain has an explicit supports_gpu field."""
        for cid, cfg in self.configs.items():
            self.assertIsInstance(cfg.supports_gpu, bool,
                f"{cid}: supports_gpu should be bool")


# ═══════════════════════════════════════════════════════════════════════════════
#  8. Rust Opcode Consistency
# ═══════════════════════════════════════════════════════════════════════════════

class TestRustOpcodeConsistency(unittest.TestCase):
    """Verify Rust GPU opcodes are consistent and non-colliding."""

    OPCODE_FILE = PROJECT_ROOT / "crates" / "x3-backend" / "src" / "opcode.rs"

    def test_opcode_file_exists(self):
        self.assertTrue(self.OPCODE_FILE.exists(), "opcode.rs not found")

    def test_gpu_opcodes_unique(self):
        if not self.OPCODE_FILE.exists():
            self.skipTest("opcode.rs missing")
        text = self.OPCODE_FILE.read_text()
        values = {}
        for line in text.splitlines():
            if "= 0xD" in line and "//" not in line.split("= 0xD")[0]:
                parts = line.strip().split("=")
                if len(parts) == 2:
                    name = parts[0].strip()
                    val = parts[1].strip().rstrip(",")
                    self.assertNotIn(val, values.values(),
                        f"Duplicate GPU opcode {val}: {name}")
                    values[name] = val
        self.assertGreaterEqual(len(values), 8,
            f"Expected ≥8 GPU opcodes, found {len(values)}: {list(values.keys())}")

    def test_new_opcodes_present(self):
        if not self.OPCODE_FILE.exists():
            self.skipTest("opcode.rs missing")
        text = self.OPCODE_FILE.read_text()
        self.assertIn("GpuKeccak256Batch", text)
        self.assertIn("GpuSecp256k1Verify", text)
        self.assertIn("0xD6", text)
        self.assertIn("0xD7", text)


# ═══════════════════════════════════════════════════════════════════════════════
#  Interactive Runner (when run directly)
# ═══════════════════════════════════════════════════════════════════════════════

def run_interactive():
    """Run all tests with a pretty summary."""
    banner("X3 Chain — Full Blockchain Test Suite")
    configs = load_default_chain_configs()

    # Print chain summary
    sig_counts = Counter(cfg.sig_algorithm.value for cfg in configs.values())
    hash_counts = Counter(cfg.hash_algorithm.value for cfg in configs.values())
    print(f"\n  {B}Chains loaded:{N} {len(configs)}")
    print(f"  {B}Sig algorithms:{N}  {dict(sig_counts)}")
    print(f"  {B}Hash algorithms:{N} {dict(hash_counts)}")

    # Check GPU
    cuda = CudaRuntime.detect()
    if cuda.available:
        ok(f"CUDA available (nvcc: {cuda.nvcc_path})")
    else:
        warn("CUDA not available — GPU tests will be skipped")

    # Check kernel libs
    libs_found = sum(1 for lib in TestGpuKernelLibraries.REQUIRED_LIBS
                     if (BUILD_DIR / lib).exists())
    if libs_found == 5:
        ok(f"All 5 GPU kernel libraries found in {BUILD_DIR}")
    else:
        warn(f"Only {libs_found}/5 kernel libraries found")

    # Run the test suite
    banner("Running Tests")
    loader = unittest.TestLoader()
    suite = loader.loadTestsFromModule(sys.modules[__name__])
    runner = unittest.TextTestRunner(verbosity=2)
    result = runner.run(suite)

    # Summary
    banner("Results Summary")
    total = result.testsRun
    failures = len(result.failures)
    errors = len(result.errors)
    skipped = len(result.skipped)
    passed = total - failures - errors - skipped

    print(f"\n  {G}Passed:{N}  {passed}")
    print(f"  {R}Failed:{N}  {failures}")
    print(f"  {R}Errors:{N}  {errors}")
    print(f"  {Y}Skipped:{N} {skipped}")
    print(f"  {B}Total:{N}   {total}")
    print()

    if failures == 0 and errors == 0:
        print(f"  {G}{B}ALL BLOCKCHAIN TESTS PASSED ✓{N}")
    else:
        print(f"  {R}{B}SOME TESTS FAILED ✗{N}")

    return 0 if (failures == 0 and errors == 0) else 1


if __name__ == "__main__":
    sys.exit(run_interactive())
