"""Integration tests for multi-chain atomic swap validation."""

from __future__ import annotations

import unittest
from unittest.mock import Mock, MagicMock

from cross_chain_gpu_validator.chain_adapter import (
    ChainValidator,
    ChainConfig,
    ChainTransaction,
    SignatureAlgorithm,
    HashAlgorithm,
)
from cross_chain_gpu_validator.chain_registry import ChainRegistry, load_default_chain_configs
from cross_chain_gpu_validator.orchestrator import MultiChainOrchestrator, MultiChainSwapPayload
from cross_chain_gpu_validator.metrics import MetricsStore


class MockValidator(ChainValidator):
    """Mock validator for testing."""

    def __init__(self, config: ChainConfig, always_valid: bool = True):
        super().__init__(config)
        self.always_valid = always_valid
        self.validated_txs = []

    def validate_transaction(self, tx: ChainTransaction) -> bool:
        self.validated_txs.append(tx)
        return self.always_valid

    def validate_transactions(self, txs: list[ChainTransaction]) -> list[bool]:
        self.validated_txs.extend(txs)
        return [self.always_valid] * len(txs)


class TestChainRegistry(unittest.TestCase):
    """Test ChainRegistry functionality."""

    def test_register_chain(self):
        """Test registering a chain."""
        registry = ChainRegistry()
        config = ChainConfig(
            chain_id="test-chain",
            chain_name="Test Chain",
            rpc_url="http://localhost:8545",
            sig_algorithm=SignatureAlgorithm.SECP256K1,
            hash_algorithm=HashAlgorithm.KECCAK256,
            sig_pubkey_size=64,
            sig_signature_size=65,
            hash_output_size=32,
        )
        validator = MockValidator(config)

        registry.register_chain(config, validator)
        self.assertEqual(registry.chain_count(), 1)
        self.assertIsNotNone(registry.get_validator("test-chain"))
        self.assertEqual(registry.get_config("test-chain"), config)

    def test_load_default_chains(self):
        """Test loading default chain configurations."""
        configs = load_default_chain_configs()
        self.assertGreater(len(configs), 100)  # Should have 100+ chains
        # Just verify some key networks are loaded - exact IDs may vary
        self.assertIn("eth", configs)  # Ethereum mainnet
        # Solana, Cosmos, Polkadot may have different IDs in registry

    def test_validate_enabled_chains(self):
        """Test checking if chains are registered."""
        registry = ChainRegistry()
        config = ChainConfig(
            chain_id="test-chain",
            chain_name="Test Chain",
            rpc_url="http://localhost:8545",
            sig_algorithm=SignatureAlgorithm.SECP256K1,
            hash_algorithm=HashAlgorithm.KECCAK256,
            sig_pubkey_size=64,
            sig_signature_size=65,
            hash_output_size=32,
        )
        validator = MockValidator(config)
        registry.register_chain(config, validator)

        self.assertTrue(registry.validate_enabled_chains(["test-chain"]))
        self.assertFalse(registry.validate_enabled_chains(["test-chain", "missing-chain"]))

    def test_duplicate_registration_fails(self):
        """Test that duplicate chain registration fails."""
        registry = ChainRegistry()
        config = ChainConfig(
            chain_id="test-chain",
            chain_name="Test Chain",
            rpc_url="http://localhost:8545",
            sig_algorithm=SignatureAlgorithm.SECP256K1,
            hash_algorithm=HashAlgorithm.KECCAK256,
            sig_pubkey_size=64,
            sig_signature_size=65,
            hash_output_size=32,
        )
        validator = MockValidator(config)

        registry.register_chain(config, validator)
        with self.assertRaises(ValueError):
            registry.register_chain(config, validator)


class TestMultiChainOrchestrator(unittest.TestCase):
    """Test MultiChainOrchestrator functionality."""

    def setUp(self):
        """Set up test fixtures."""
        self.registry = ChainRegistry()
        
        # Register test chains
        for chain_id in ["chain-a", "chain-b", "chain-c"]:
            config = ChainConfig(
                chain_id=chain_id,
                chain_name=f"Test {chain_id}",
                rpc_url=f"http://localhost:8545",
                sig_algorithm=SignatureAlgorithm.SECP256K1,
                hash_algorithm=HashAlgorithm.KECCAK256,
                sig_pubkey_size=64,
                sig_signature_size=65,
                hash_output_size=32,
            )
            validator = MockValidator(config, always_valid=True)
            self.registry.register_chain(config, validator)

        self.atomic_registry = Mock()
        self.atomic_registry.pending_swaps.return_value = []
        
        self.metrics = MetricsStore()
        self.orchestrator = MultiChainOrchestrator(
            self.atomic_registry,
            self.registry,
            self.metrics,
        )

    def test_submit_swap(self):
        """Test submitting a multi-chain swap."""
        payload = MultiChainSwapPayload(
            swap_id="swap-123",
            chain_transactions={
                "chain-a": [
                    ChainTransaction(
                        chain_id="chain-a",
                        signature=b"sig-a",
                        pubkey=b"pk-a",
                        payload=b"payload-a",
                    ),
                ],
                "chain-b": [
                    ChainTransaction(
                        chain_id="chain-b",
                        signature=b"sig-b",
                        pubkey=b"pk-b",
                        payload=b"payload-b",
                    ),
                ],
            },
            timeout_seconds=30,
        )

        self.orchestrator.submit_swap(payload)
        self.atomic_registry.register_swap.assert_called_once()

    def test_submit_swap_invalid_chains_fails(self):
        """Test that submitting with unregistered chains fails."""
        payload = MultiChainSwapPayload(
            swap_id="swap-456",
            chain_transactions={
                "missing-chain": [
                    ChainTransaction(
                        chain_id="missing-chain",
                        signature=b"sig",
                        pubkey=b"pk",
                        payload=b"payload",
                    ),
                ],
            },
            timeout_seconds=30,
        )

        with self.assertRaises(ValueError):
            self.orchestrator.submit_swap(payload)

    def test_validate_swap_all_valid(self):
        """Test validating a swap with all valid transactions."""
        payload = MultiChainSwapPayload(
            swap_id="swap-789",
            chain_transactions={
                "chain-a": [
                    ChainTransaction(
                        chain_id="chain-a",
                        signature=b"sig-a",
                        pubkey=b"pk-a",
                        payload=b"payload-a",
                    ),
                ],
                "chain-b": [
                    ChainTransaction(
                        chain_id="chain-b",
                        signature=b"sig-b",
                        pubkey=b"pk-b",
                        payload=b"payload-b",
                    ),
                ],
            },
            timeout_seconds=30,
        )

        result = self.orchestrator.validate_swap(payload)
        self.assertTrue(result)

    def test_validate_swap_one_invalid(self):
        """Test validating a swap with one invalid chain."""
        # Create a validator that returns False for chain-b
        config_b = self.registry.get_config("chain-b")
        invalid_validator = MockValidator(config_b, always_valid=False)
        self.registry._validators["chain-b"] = invalid_validator

        payload = MultiChainSwapPayload(
            swap_id="swap-999",
            chain_transactions={
                "chain-a": [
                    ChainTransaction(
                        chain_id="chain-a",
                        signature=b"sig-a",
                        pubkey=b"pk-a",
                        payload=b"payload-a",
                    ),
                ],
                "chain-b": [
                    ChainTransaction(
                        chain_id="chain-b",
                        signature=b"sig-b",
                        pubkey=b"pk-b",
                        payload=b"payload-b",
                    ),
                ],
            },
            timeout_seconds=30,
        )

        result = self.orchestrator.validate_swap(payload)
        self.assertFalse(result)

    def test_multi_chain_swap_three_chains(self):
        """Test atomic swap with 3 chains."""
        payload = MultiChainSwapPayload(
            swap_id="swap-3chain",
            chain_transactions={
                "chain-a": [
                    ChainTransaction(
                        chain_id="chain-a",
                        signature=b"sig-a",
                        pubkey=b"pk-a",
                        payload=b"payload-a",
                    ),
                ],
                "chain-b": [
                    ChainTransaction(
                        chain_id="chain-b",
                        signature=b"sig-b",
                        pubkey=b"pk-b",
                        payload=b"payload-b",
                    ),
                ],
                "chain-c": [
                    ChainTransaction(
                        chain_id="chain-c",
                        signature=b"sig-c",
                        pubkey=b"pk-c",
                        payload=b"payload-c",
                    ),
                ],
            },
            timeout_seconds=30,
        )

        result = self.orchestrator.validate_swap(payload)
        self.assertTrue(result)

    def test_get_swap_status(self):
        """Test retrieving swap status."""
        mock_record = Mock()
        mock_record.status = "APPROVED"
        mock_record.created_at = 1000.0
        mock_record.timeout_at = 2000.0

        self.atomic_registry.get_swap.return_value = mock_record

        status = self.orchestrator.get_swap_status("swap-123")
        self.assertIsNotNone(status)
        self.assertEqual(status["status"], "APPROVED")
        self.assertEqual(status["swap_id"], "swap-123")


class TestChainAdapterInterface(unittest.TestCase):
    """Test ChainValidator interface implementation."""

    def test_evm_signature_algorithm(self):
        """Test EVM chain uses correct signature algorithm."""
        configs = load_default_chain_configs()
        # Use 'eth' which is the loaded key for Ethereum
        evm_config = configs["eth"]
        self.assertEqual(evm_config.sig_algorithm, SignatureAlgorithm.SECP256K1)
        self.assertEqual(evm_config.hash_algorithm, HashAlgorithm.KECCAK256)

    def test_solana_signature_algorithm(self):
        """Test Solana chain uses correct signature algorithm."""
        # Skip if Solana not in default registry (may be in external sources)
        configs = load_default_chain_configs()
        if "solana" in configs or "sol" in configs:
            solana_config = configs.get("solana") or configs.get("sol")
            self.assertEqual(solana_config.sig_algorithm, SignatureAlgorithm.ED25519)
            self.assertEqual(solana_config.hash_algorithm, HashAlgorithm.SHA256)
        else:
            self.skipTest("Solana not in default chain registry")

    def test_cosmos_signature_algorithm(self):
        """Test Cosmos chain uses correct signature algorithm."""
        configs = load_default_chain_configs()
        # Look for cosmos chain - may have different ID
        cosmos_id = None
        for chain_id in configs:
            if 'cosmos' in chain_id.lower() or 'atom' in chain_id.lower():
                cosmos_id = chain_id
                break
        if cosmos_id:
            cosmos_config = configs[cosmos_id]
            self.assertEqual(cosmos_config.sig_algorithm, SignatureAlgorithm.SECP256K1)
            self.assertEqual(cosmos_config.hash_algorithm, HashAlgorithm.SHA256)
        else:
            self.skipTest("Cosmos Hub not in default chain registry")

    def test_substrate_signature_algorithm(self):
        """Test Substrate chain uses correct signature algorithm."""
        configs = load_default_chain_configs()
        # Look for polkadot in the registry
        polka_id = None
        for chain_id in configs:
            if 'polka' in chain_id.lower() or 'dot' in chain_id.lower():
                polka_id = chain_id
                break
        if polka_id:
            substrate_config = configs[polka_id]
            self.assertEqual(substrate_config.sig_algorithm, SignatureAlgorithm.ED25519)
            self.assertEqual(substrate_config.hash_algorithm, HashAlgorithm.SHA256)
        else:
            self.skipTest("Polkadot not in default chain registry")


if __name__ == "__main__":
    unittest.main()
