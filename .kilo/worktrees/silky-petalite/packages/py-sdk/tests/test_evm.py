"""Tests for EVM client."""

import pytest
from unittest.mock import Mock
from x3_chain_sdk.evm import EvmClient, EvmTransaction


class TestEvmClient:
    """Tests for EvmClient."""
    
    def test_init(self):
        """Test EvmClient initialization."""
        mock_client = Mock()
        evm = EvmClient(mock_client)
        assert evm._client is mock_client
    
    def test_get_chain_id(self):
        """Test getting chain ID."""
        mock_client = Mock()
        mock_client._call_rpc.return_value = "0x9eb10"  # 650000
        
        evm = EvmClient(mock_client)
        chain_id = evm.get_chain_id()
        
        assert chain_id == 650000
    
    def test_get_gas_price(self):
        """Test getting gas price."""
        mock_client = Mock()
        mock_client._call_rpc.return_value = "0x3b9aca00"  # 1 gwei
        
        evm = EvmClient(mock_client)
        gas_price = evm.get_gas_price()
        
        assert gas_price == 1_000_000_000
    
    def test_build_deployment(self):
        """Test building a deployment transaction."""
        mock_client = Mock()
        evm = EvmClient(mock_client)
        
        tx = evm.build_deployment(
            bytecode=b"\x60\x80\x60\x40",
            gas_limit=3_000_000,
        )
        
        assert isinstance(tx, EvmTransaction)
        assert tx.to is None  # Deployment has no target
        assert tx.data == b"\x60\x80\x60\x40"
    
    def test_to_comit_payload(self):
        """Test converting to Comit payload."""
        mock_client = Mock()
        evm = EvmClient(mock_client)
        
        tx = EvmTransaction(
            to="0x" + "ab" * 20,
            data=b"\x12\x34",
            value=0,
            gas_limit=500_000,
        )
        
        payload = evm.to_comit_payload(tx)
        
        assert isinstance(payload, bytes)
        # 20 bytes address + 32 bytes value + data
        assert len(payload) == 20 + 32 + 2


class TestEvmTransaction:
    """Tests for EvmTransaction."""
    
    def test_transaction_creation(self):
        """Test creating an EVM transaction."""
        tx = EvmTransaction(
            to="0x1234567890abcdef1234567890abcdef12345678",
            value=0,
            data=b"\x12\x34\x56\x78",
            gas_limit=100_000,
        )
        
        assert tx.to == "0x1234567890abcdef1234567890abcdef12345678"
        assert tx.value == 0
        assert tx.gas_limit == 100_000
    
    def test_deployment_transaction(self):
        """Test deployment transaction (no 'to' address)."""
        tx = EvmTransaction(
            to=None,
            data=b"\x60\x80\x60\x40",
            value=0,
            gas_limit=3_000_000,
        )
        
        assert tx.to is None
        assert tx.gas_limit == 3_000_000
