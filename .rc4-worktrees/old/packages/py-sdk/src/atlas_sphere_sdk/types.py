"""
Type definitions for X3 Chain SDK.
"""

from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional, Union
from enum import Enum
import hashlib


# Type aliases
AccountId = str  # SS58 encoded account address
AssetId = int  # Asset identifier (u32)
Balance = int  # Native balance (u128)
ComitId = str  # Hex-encoded H256 comit identifier


class VmType(Enum):
    """Virtual machine type for execution."""
    EVM = "evm"
    SVM = "svm"


@dataclass
class ExecutionLog:
    """Log entry from VM execution."""
    address: bytes
    topics: List[bytes]
    data: bytes


@dataclass
class StateChange:
    """State change from VM execution."""
    address: bytes
    key: bytes
    value: bytes


@dataclass
class ExecutionReceipt:
    """Result of VM execution."""
    success: bool
    gas_used: int
    return_data: bytes
    logs: List[ExecutionLog]
    state_changes: List[StateChange]


@dataclass
class ComitPayload:
    """Payload for a Comit transaction."""
    evm_payload: Optional[bytes] = None
    svm_payload: Optional[bytes] = None
    
    def is_empty(self) -> bool:
        """Check if both payloads are empty."""
        return not self.evm_payload and not self.svm_payload
    
    def compute_prepare_root(self) -> bytes:
        """Compute the prepare root hash of the input payloads."""
        data = b""
        if self.evm_payload:
            data += self.evm_payload
        if self.svm_payload:
            data += self.svm_payload
        return hashlib.blake2b(data, digest_size=32).digest()


@dataclass
class ComitResult:
    """Result of a submitted Comit transaction."""
    comit_id: ComitId
    block_hash: str
    block_number: int
    evm_receipt: Optional[ExecutionReceipt] = None
    svm_receipt: Optional[ExecutionReceipt] = None
    fee_charged: Balance = 0
    finalized: bool = False


@dataclass
class AssetMetadata:
    """Metadata for a registered asset."""
    asset_id: AssetId
    symbol: str
    decimals: int


@dataclass
class AccountInfo:
    """Account information."""
    account_id: AccountId
    nonce: int
    free_balance: Balance
    reserved_balance: Balance
    is_authorized: bool


@dataclass
class BlockHeader:
    """Block header information."""
    number: int
    hash: str
    parent_hash: str
    state_root: str
    extrinsics_root: str


@dataclass
class ChainInfo:
    """Chain metadata."""
    chain_id: int
    chain_name: str
    token_symbol: str
    token_decimals: int
    ss58_format: int
    genesis_hash: str
    best_number: int
    finalized_number: int


class AtlasError(Exception):
    """Base exception for X3 Chain SDK errors."""
    pass


class ConnectionError(AtlasError):
    """Failed to connect to the node."""
    pass


class AuthorizationError(AtlasError):
    """Account is not authorized for Comit submissions."""
    pass


class InvalidPayloadError(AtlasError):
    """Payload validation failed."""
    pass


class NonceError(AtlasError):
    """Invalid nonce for transaction."""
    pass


class InsufficientBalanceError(AtlasError):
    """Insufficient balance for fee payment."""
    pass


class ExecutionError(AtlasError):
    """VM execution failed."""
    def __init__(self, message: str, receipt: Optional[ExecutionReceipt] = None):
        super().__init__(message)
        self.receipt = receipt
