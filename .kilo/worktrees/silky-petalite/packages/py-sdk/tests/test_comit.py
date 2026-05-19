"""Tests for ComitBuilder."""

import pytest
from unittest.mock import Mock, patch
from x3_chain_sdk.comit import ComitBuilder, ComitTransaction
from x3_chain_sdk.types import InvalidPayloadError


class TestComitBuilder:
    """Tests for ComitBuilder."""
    
    def test_empty_builder(self):
        """Test that empty builder raises error."""
        builder = ComitBuilder()
        keypair = Mock()
        keypair.public_key = bytes(32)
        
        with pytest.raises(InvalidPayloadError, match="At least one payload"):
            builder.build(keypair, nonce=1)
    
    def test_evm_payload(self):
        """Test adding EVM payload."""
        builder = ComitBuilder()
        result = builder.with_evm_payload(b"\x01\x02\x03", gas_limit=100_000)
        
        assert result is builder  # Fluent API
        assert builder._evm_payload == b"\x01\x02\x03"
        assert builder._evm_gas_limit == 100_000
    
    def test_evm_call(self):
        """Test adding EVM contract call."""
        builder = ComitBuilder()
        result = builder.with_evm_call(
            to="0x1234567890abcdef1234567890abcdef12345678",
            data=b"\x12\x34",
            value=0,
            gas_limit=500_000,
        )
        
        assert result is builder
        assert builder._evm_gas_limit == 500_000
        assert builder._evm_payload is not None
    
    def test_svm_payload(self):
        """Test adding SVM payload."""
        builder = ComitBuilder()
        result = builder.with_svm_payload(b"\x04\x05\x06", compute_limit=200_000)
        
        assert result is builder
        assert builder._svm_payload == b"\x04\x05\x06"
        assert builder._svm_compute_limit == 200_000
    
    def test_svm_instruction(self):
        """Test adding SVM instruction."""
        builder = ComitBuilder()
        result = builder.with_svm_instruction(
            program_id=bytes(32),
            instruction_data=b"\x01\x02\x03",
            accounts=[],
            compute_limit=100_000,
        )
        
        assert result is builder
        assert builder._svm_compute_limit == 100_000
    
    def test_explicit_nonce(self):
        """Test setting explicit nonce."""
        builder = ComitBuilder()
        result = builder.with_nonce(42)
        
        assert result is builder
        assert builder._nonce == 42
    
    def test_build_creates_transaction(self):
        """Test that build creates ComitTransaction."""
        builder = ComitBuilder()
        builder.with_evm_payload(b"\x01\x02\x03", gas_limit=100_000)
        
        keypair = Mock()
        keypair.public_key = bytes(32)
        keypair.sign = Mock(return_value=bytes(64))
        
        tx = builder.build(keypair, nonce=1)
        
        assert isinstance(tx, ComitTransaction)
        assert tx.nonce == 1
    
    def test_dual_vm_payload(self):
        """Test building with both EVM and SVM payloads."""
        builder = ComitBuilder()
        builder.with_evm_payload(b"\x01\x02\x03", gas_limit=100_000)
        builder.with_svm_payload(b"\x04\x05\x06", compute_limit=200_000)
        
        keypair = Mock()
        keypair.public_key = bytes(32)
        keypair.sign = Mock(return_value=bytes(64))
        
        tx = builder.build(keypair, nonce=1)
        
        assert tx.evm_payload == b"\x01\x02\x03"
        assert tx.svm_payload == b"\x04\x05\x06"


class TestComitTransaction:
    """Tests for ComitTransaction."""
    
    def test_transaction_creation(self):
        """Test creating a ComitTransaction."""
        tx = ComitTransaction(
            comit_id="0x" + "ab" * 32,
            nonce=1,
            evm_payload=b"\x01\x02\x03",
            svm_payload=b"",
            prepare_root=bytes(32),
            evm_gas_limit=100_000,
            svm_compute_limit=0,
        )
        
        assert tx.nonce == 1
        assert tx.evm_payload == b"\x01\x02\x03"
        assert tx.evm_gas_limit == 100_000
    
    def test_to_dict(self):
        """Test converting to dictionary format."""
        tx = ComitTransaction(
            comit_id="0x" + "ab" * 32,
            nonce=1,
            evm_payload=b"\x01",
            svm_payload=b"\x02",
            prepare_root=bytes(32),
            evm_gas_limit=100_000,
            svm_compute_limit=200_000,
        )
        
        data = tx.to_dict()
        
        assert "evm_payload" in data
        assert "svm_payload" in data
        assert data["nonce"] == 1
