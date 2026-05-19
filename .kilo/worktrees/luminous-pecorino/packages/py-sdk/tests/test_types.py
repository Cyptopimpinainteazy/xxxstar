"""Tests for X3 Chain SDK types."""

import pytest
from x3_chain_sdk.types import (
    AccountId,
    AssetId,
    ComitId,
    ComitPayload,
    ExecutionReceipt,
    ExecutionLog,
    StateChange,
    ChainInfo,
    AccountInfo,
    BlockHeader,
    AtlasError,
    ConnectionError,
    AuthorizationError,
)


class TestTypeAliases:
    """Tests for type aliases."""
    
    def test_account_id_is_str(self):
        """Test AccountId is a string alias."""
        account: AccountId = "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY"
        assert isinstance(account, str)
    
    def test_asset_id_is_int(self):
        """Test AssetId is an int alias."""
        asset: AssetId = 1
        assert isinstance(asset, int)
    
    def test_comit_id_is_str(self):
        """Test ComitId is a string alias."""
        comit: ComitId = "0x" + "ab" * 32
        assert isinstance(comit, str)


class TestComitPayload:
    """Tests for ComitPayload."""
    
    def test_create_evm_payload(self):
        """Test creating EVM payload."""
        payload = ComitPayload(
            evm_payload=b"\x01\x02\x03",
            svm_payload=None,
        )
        assert payload.evm_payload == b"\x01\x02\x03"
        assert payload.svm_payload is None
    
    def test_create_svm_payload(self):
        """Test creating SVM payload."""
        payload = ComitPayload(
            evm_payload=None,
            svm_payload=b"\x04\x05\x06",
        )
        assert payload.svm_payload == b"\x04\x05\x06"
        assert payload.evm_payload is None
    
    def test_create_dual_payload(self):
        """Test creating dual EVM+SVM payload."""
        payload = ComitPayload(
            evm_payload=b"\x01\x02\x03",
            svm_payload=b"\x04\x05\x06",
        )
        assert payload.evm_payload == b"\x01\x02\x03"
        assert payload.svm_payload == b"\x04\x05\x06"
    
    def test_is_empty(self):
        """Test is_empty method."""
        empty = ComitPayload()
        assert empty.is_empty() is True
        
        with_evm = ComitPayload(evm_payload=b"\x01")
        assert with_evm.is_empty() is False
    
    def test_compute_prepare_root(self):
        """Test prepare root computation."""
        payload = ComitPayload(evm_payload=b"\x01\x02\x03")
        root = payload.compute_prepare_root()
        assert len(root) == 32
        assert isinstance(root, bytes)


class TestExecutionReceipt:
    """Tests for ExecutionReceipt."""
    
    def test_success_receipt(self):
        """Test successful execution receipt."""
        receipt = ExecutionReceipt(
            success=True,
            gas_used=50_000,
            return_data=b"\x00\x00\x00\x01",
            logs=[],
            state_changes=[],
        )
        assert receipt.success is True
        assert receipt.gas_used == 50_000
        assert receipt.return_data == b"\x00\x00\x00\x01"
    
    def test_failure_receipt(self):
        """Test failed execution receipt."""
        receipt = ExecutionReceipt(
            success=False,
            gas_used=21_000,
            return_data=b"",
            logs=[],
            state_changes=[],
        )
        assert receipt.success is False
        assert receipt.gas_used == 21_000


class TestChainInfo:
    """Tests for ChainInfo."""
    
    def test_chain_info(self):
        """Test ChainInfo creation."""
        info = ChainInfo(
            chain_name="X3 Chain Testnet",
            chain_id=42,
            token_symbol="X3",
            token_decimals=18,
            ss58_format=42,
            genesis_hash="0x123abc...",
            best_number=1000,
            finalized_number=990,
        )
        assert info.chain_name == "X3 Chain Testnet"
        assert info.chain_id == 42
        assert info.token_decimals == 18


class TestAccountInfo:
    """Tests for AccountInfo."""
    
    def test_account_info(self):
        """Test AccountInfo creation."""
        info = AccountInfo(
            account_id="5GrwvaEF...",
            nonce=5,
            free_balance=1_000_000,
            reserved_balance=500_000,
            is_authorized=True,
        )
        assert info.nonce == 5
        assert info.free_balance == 1_000_000
        assert info.is_authorized is True


class TestBlockHeader:
    """Tests for BlockHeader."""
    
    def test_block_header(self):
        """Test BlockHeader creation."""
        header = BlockHeader(
            hash="0xabc123...",
            parent_hash="0xdef456...",
            number=100,
            state_root="0x111...",
            extrinsics_root="0x222...",
        )
        assert header.number == 100
        assert header.hash.startswith("0x")


class TestErrors:
    """Tests for error types."""
    
    def test_x3_error(self):
        """Test AtlasError."""
        error = AtlasError("Something went wrong")
        assert str(error) == "Something went wrong"
    
    def test_connection_error(self):
        """Test ConnectionError inherits from AtlasError."""
        error = ConnectionError("Failed to connect")
        assert isinstance(error, AtlasError)
    
    def test_authorization_error(self):
        """Test AuthorizationError inherits from AtlasError."""
        error = AuthorizationError("Not authorized")
        assert isinstance(error, AtlasError)
