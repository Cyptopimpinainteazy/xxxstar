"""Pytest configuration and fixtures."""

import pytest
from unittest.mock import Mock, MagicMock


@pytest.fixture
def mock_substrate():
    """Create a mock SubstrateInterface."""
    substrate = MagicMock()
    substrate.chain = "X3 Chain Testnet"
    substrate.ss58_format = 42
    substrate.token_symbol = "X3"
    substrate.token_decimals = 18
    substrate.get_block_hash.return_value = "0x" + "ab" * 32
    substrate.get_block_header.return_value = {
        "hash": "0x" + "ab" * 32,
        "parentHash": "0x" + "cd" * 32,
        "number": 100,
        "stateRoot": "0x" + "11" * 32,
        "extrinsicsRoot": "0x" + "22" * 32,
    }
    substrate.get_block_number.return_value = 100
    substrate.finalized_block_hash = "0x" + "ff" * 32
    substrate.get_chain_head.return_value = "0x" + "ab" * 32
    return substrate


@pytest.fixture
def mock_keypair():
    """Create a mock Keypair."""
    keypair = Mock()
    keypair.ss58_address = "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY"
    keypair.public_key = bytes(32)
    keypair.sign = Mock(return_value=bytes(64))
    return keypair


@pytest.fixture
def mock_client(mock_substrate):
    """Create a mock AtlasClient."""
    from x3_chain_sdk import AtlasClient
    from x3_chain_sdk.types import ChainInfo, AccountInfo, BlockHeader
    
    client = Mock(spec=AtlasClient)
    client._substrate = mock_substrate
    
    client.get_chain_info.return_value = ChainInfo(
        chain_name="X3 Chain Testnet",
        chain_id=42,
        token_symbol="X3",
        token_decimals=18,
        ss58_format=42,
        genesis_hash="0x" + "00" * 32,
        best_number=100,
        finalized_number=90,
    )
    
    client.get_account_info.return_value = AccountInfo(
        account_id="5GrwvaEF...",
        nonce=5,
        free_balance=1_000_000_000_000,
        reserved_balance=0,
        is_authorized=True,
    )
    
    client.get_block_header.return_value = BlockHeader(
        hash="0x" + "ab" * 32,
        parent_hash="0x" + "cd" * 32,
        number=100,
        state_root="0x" + "11" * 32,
        extrinsics_root="0x" + "22" * 32,
    )
    
    client.get_nonce.return_value = 5
    client.get_canonical_balance.return_value = 500_000_000_000
    client.is_authorized.return_value = True
    
    return client


@pytest.fixture
def live_client():
    """Create a live AtlasClient connected to the X3 testnet node.
    
    Skips if X3_RPC_ENDPOINT environment variable is not set.
    """
    import os
    endpoint = os.environ.get("X3_RPC_ENDPOINT")
    if not endpoint:
        pytest.skip("X3_RPC_ENDPOINT environment variable not set - skipping live integration test")
    
    from atlas_sphere_sdk import AtlasClient
    return AtlasClient(endpoint=endpoint)
