"""
Comit Transaction Builder for X3 Chain.

Provides a fluent API for constructing and submitting Comit transactions
that execute atomically across EVM and SVM.
"""

from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional, Tuple
import hashlib

from substrateinterface import Keypair

from x3_chain_sdk.types import (
    AccountId,
    Balance,
    ComitId,
    ComitPayload,
    ComitResult,
    ExecutionReceipt,
    ExecutionLog,
    StateChange,
    AtlasError,
    InvalidPayloadError,
    AuthorizationError,
)


MAX_EVM_PAYLOAD_SIZE = 16 * 1024  # 16 KB
MAX_SVM_PAYLOAD_SIZE = 16 * 1024  # 16 KB
MAX_COMBINED_SIZE = 32 * 1024  # 32 KB


@dataclass
class ComitTransaction:
    """
    Represents a prepared Comit transaction ready for submission.
    """
    comit_id: ComitId
    nonce: int
    evm_payload: bytes
    svm_payload: bytes
    prepare_root: bytes
    evm_gas_limit: int
    svm_compute_limit: int
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary for serialization."""
        return {
            "comit_id": self.comit_id,
            "nonce": self.nonce,
            "evm_payload": self.evm_payload.hex() if self.evm_payload else "",
            "svm_payload": self.svm_payload.hex() if self.svm_payload else "",
            "prepare_root": self.prepare_root.hex(),
            "evm_gas_limit": self.evm_gas_limit,
            "svm_compute_limit": self.svm_compute_limit,
        }


class ComitBuilder:
    """
    Fluent builder for Comit transactions.
    
    Example:
        >>> comit = (
        ...     ComitBuilder()
        ...     .with_evm_call(contract_addr, calldata, gas_limit=500000)
        ...     .with_svm_instruction(program_id, instruction_data)
        ...     .build(keypair)
        ... )
        >>> result = client.submit_comit(comit)
    """
    
    def __init__(self):
        """Initialize empty Comit builder."""
        self._evm_payload: Optional[bytes] = None
        self._svm_payload: Optional[bytes] = None
        self._evm_gas_limit: int = 10_000_000
        self._svm_compute_limit: int = 200_000
        self._nonce: Optional[int] = None
        self._comit_id: Optional[str] = None
    
    def with_evm_payload(
        self,
        payload: bytes,
        gas_limit: int = 10_000_000,
    ) -> "ComitBuilder":
        """
        Set raw EVM bytecode payload.
        
        Args:
            payload: Raw EVM bytecode/calldata
            gas_limit: Maximum gas for EVM execution
            
        Returns:
            Self for method chaining
        """
        if len(payload) > MAX_EVM_PAYLOAD_SIZE:
            raise InvalidPayloadError(
                f"EVM payload exceeds max size ({len(payload)} > {MAX_EVM_PAYLOAD_SIZE})"
            )
        self._evm_payload = payload
        self._evm_gas_limit = gas_limit
        return self
    
    def with_evm_call(
        self,
        to: str,
        data: bytes,
        value: int = 0,
        gas_limit: int = 500_000,
    ) -> "ComitBuilder":
        """
        Add an EVM contract call.
        
        Args:
            to: Contract address (0x prefixed)
            data: ABI-encoded calldata
            value: Wei value to send
            gas_limit: Maximum gas
            
        Returns:
            Self for method chaining
        """
        # Encode as simple call format: to_address (20 bytes) + value (32 bytes) + data
        to_bytes = bytes.fromhex(to[2:] if to.startswith("0x") else to)
        value_bytes = value.to_bytes(32, "big")
        payload = to_bytes + value_bytes + data
        
        return self.with_evm_payload(payload, gas_limit)
    
    def with_svm_payload(
        self,
        payload: bytes,
        compute_limit: int = 200_000,
    ) -> "ComitBuilder":
        """
        Set raw SVM (Solana) instruction payload.
        
        Args:
            payload: Raw BPF instruction data
            compute_limit: Maximum compute units
            
        Returns:
            Self for method chaining
        """
        if len(payload) > MAX_SVM_PAYLOAD_SIZE:
            raise InvalidPayloadError(
                f"SVM payload exceeds max size ({len(payload)} > {MAX_SVM_PAYLOAD_SIZE})"
            )
        self._svm_payload = payload
        self._svm_compute_limit = compute_limit
        return self
    
    def with_svm_instruction(
        self,
        program_id: bytes,
        instruction_data: bytes,
        accounts: Optional[List[Tuple[bytes, bool, bool]]] = None,
        compute_limit: int = 200_000,
    ) -> "ComitBuilder":
        """
        Add a Solana-style instruction.
        
        Args:
            program_id: 32-byte program public key
            instruction_data: Instruction data bytes
            accounts: List of (pubkey, is_signer, is_writable) tuples
            compute_limit: Maximum compute units
            
        Returns:
            Self for method chaining
        """
        accounts = accounts or []
        
        # Encode instruction format:
        # program_id (32) + num_accounts (1) + accounts... + data_len (2) + data
        payload = bytearray(program_id)
        payload.append(len(accounts))
        
        for pubkey, is_signer, is_writable in accounts:
            payload.extend(pubkey)
            payload.append((1 if is_signer else 0) | (2 if is_writable else 0))
        
        payload.extend(len(instruction_data).to_bytes(2, "little"))
        payload.extend(instruction_data)
        
        return self.with_svm_payload(bytes(payload), compute_limit)
    
    def with_nonce(self, nonce: int) -> "ComitBuilder":
        """
        Set explicit nonce (otherwise fetched from chain).
        
        Args:
            nonce: Transaction nonce
            
        Returns:
            Self for method chaining
        """
        self._nonce = nonce
        return self
    
    def with_comit_id(self, comit_id: str) -> "ComitBuilder":
        """
        Set explicit Comit ID (otherwise generated).
        
        Args:
            comit_id: Hex-encoded 32-byte identifier
            
        Returns:
            Self for method chaining
        """
        self._comit_id = comit_id
        return self
    
    def validate(self) -> List[str]:
        """
        Validate the current configuration.
        
        Returns:
            List of validation error messages (empty if valid)
        """
        errors = []
        
        if not self._evm_payload and not self._svm_payload:
            errors.append("At least one payload (EVM or SVM) is required")
        
        total_size = len(self._evm_payload or b"") + len(self._svm_payload or b"")
        if total_size > MAX_COMBINED_SIZE:
            errors.append(f"Combined payload size exceeds limit ({total_size} > {MAX_COMBINED_SIZE})")
        
        return errors
    
    def _compute_prepare_root(self) -> bytes:
        """Compute prepare root hash from payloads."""
        data = b""
        if self._evm_payload:
            data += self._evm_payload
        if self._svm_payload:
            data += self._svm_payload
        return hashlib.blake2b(data, digest_size=32).digest()
    
    def _generate_comit_id(self, keypair: Keypair) -> str:
        """Generate a unique Comit ID."""
        import time
        import os
        
        data = (
            keypair.public_key
            + str(time.time_ns()).encode()
            + os.urandom(8)
        )
        return "0x" + hashlib.blake2b(data, digest_size=32).hexdigest()
    
    def build(self, keypair: Keypair, nonce: Optional[int] = None) -> ComitTransaction:
        """
        Build the Comit transaction.
        
        Args:
            keypair: Keypair for signing
            nonce: Override nonce (uses builder's nonce if set)
            
        Returns:
            ComitTransaction ready for submission
            
        Raises:
            InvalidPayloadError: If validation fails
        """
        errors = self.validate()
        if errors:
            raise InvalidPayloadError("; ".join(errors))
        
        actual_nonce = nonce or self._nonce
        if actual_nonce is None:
            raise InvalidPayloadError("Nonce is required (set via with_nonce or pass to build)")
        
        comit_id = self._comit_id or self._generate_comit_id(keypair)
        prepare_root = self._compute_prepare_root()
        
        return ComitTransaction(
            comit_id=comit_id,
            nonce=actual_nonce,
            evm_payload=self._evm_payload or b"",
            svm_payload=self._svm_payload or b"",
            prepare_root=prepare_root,
            evm_gas_limit=self._evm_gas_limit,
            svm_compute_limit=self._svm_compute_limit,
        )


def submit_comit(
    client: Any,  # AtlasClient
    comit: ComitTransaction,
    keypair: Keypair,
    wait_for_finalization: bool = False,
) -> ComitResult:
    """
    Submit a Comit transaction to the chain.
    
    Args:
        client: AtlasClient instance
        comit: ComitTransaction to submit
        keypair: Keypair for signing
        wait_for_finalization: Wait for GRANDPA finalization
        
    Returns:
        ComitResult with execution details
        
    Raises:
        AuthorizationError: If account not authorized
        AtlasError: If submission fails
    """
    substrate = client._ensure_connected()
    
    # Check authorization
    if not client.is_authorized(keypair.ss58_address):
        raise AuthorizationError(
            f"Account {keypair.ss58_address} is not authorized for Comit submissions"
        )
    
    # Build extrinsic call
    call = substrate.compose_call(
        call_module="AtlasKernel",
        call_function="submit_comit",
        call_params={
            "comit_id": comit.comit_id,
            "nonce": comit.nonce,
            "evm_payload": list(comit.evm_payload),
            "svm_payload": list(comit.svm_payload),
            "prepare_root": list(comit.prepare_root),
            "evm_gas_limit": comit.evm_gas_limit,
            "svm_compute_limit": comit.svm_compute_limit,
        },
    )
    
    # Sign and submit
    extrinsic = substrate.create_signed_extrinsic(call=call, keypair=keypair)
    
    if wait_for_finalization:
        receipt = substrate.submit_extrinsic(
            extrinsic,
            wait_for_finalization=True,
        )
    else:
        receipt = substrate.submit_extrinsic(
            extrinsic,
            wait_for_inclusion=True,
        )
    
    if not receipt.is_success:
        raise AtlasError(f"Comit submission failed: {receipt.error_message}")
    
    # Parse events for execution receipts
    evm_receipt = None
    svm_receipt = None
    fee_charged = 0
    
    for event in receipt.triggered_events:
        if event.value["event_id"] == "AtlasKernel.ComitSubmitted":
            data = event.value["attributes"]
            fee_charged = data.get("fee", 0)
        elif event.value["event_id"] == "AtlasKernel.EvmExecuted":
            data = event.value["attributes"]
            evm_receipt = ExecutionReceipt(
                success=data.get("success", False),
                gas_used=data.get("gas_used", 0),
                return_data=bytes(data.get("return_data", [])),
                logs=[],
                state_changes=[],
            )
        elif event.value["event_id"] == "AtlasKernel.SvmExecuted":
            data = event.value["attributes"]
            svm_receipt = ExecutionReceipt(
                success=data.get("success", False),
                gas_used=data.get("compute_units_used", 0),
                return_data=bytes(data.get("return_data", [])),
                logs=[],
                state_changes=[],
            )
    
    return ComitResult(
        comit_id=comit.comit_id,
        block_hash=receipt.block_hash,
        block_number=receipt.block_number,
        evm_receipt=evm_receipt,
        svm_receipt=svm_receipt,
        fee_charged=fee_charged,
        finalized=wait_for_finalization,
    )
