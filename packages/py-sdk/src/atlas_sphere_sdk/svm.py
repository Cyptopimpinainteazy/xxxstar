"""
SVM (Solana VM) client for Solana-compatible interactions.
"""

from typing import Any, Dict, List, Optional, Tuple
from dataclasses import dataclass

from x3_chain_sdk.types import ExecutionReceipt


@dataclass
class SvmAccount:
    """Account metadata for SVM instruction."""
    pubkey: bytes  # 32-byte public key
    is_signer: bool = False
    is_writable: bool = False


@dataclass
class SvmInstruction:
    """SVM instruction parameters."""
    program_id: bytes  # 32-byte program public key
    accounts: List[SvmAccount]
    data: bytes  # Instruction data
    compute_limit: int = 200_000


class SvmClient:
    """
    Client for SVM (Solana) specific operations.
    
    Provides Solana-compatible interfaces for:
    - Program instruction building
    - Account metadata handling
    - Compute unit estimation
    """
    
    def __init__(self, client: Any):
        """
        Initialize SVM client.
        
        Args:
            client: AtlasClient instance
        """
        self._client = client
    
    def create_account(
        self,
        pubkey: bytes,
        is_signer: bool = False,
        is_writable: bool = False,
    ) -> SvmAccount:
        """
        Create an account metadata object.
        
        Args:
            pubkey: 32-byte public key
            is_signer: Whether account is a signer
            is_writable: Whether account is writable
            
        Returns:
            SvmAccount instance
        """
        if len(pubkey) != 32:
            raise ValueError("Public key must be 32 bytes")
        return SvmAccount(
            pubkey=pubkey,
            is_signer=is_signer,
            is_writable=is_writable,
        )
    
    def build_instruction(
        self,
        program_id: bytes,
        accounts: List[SvmAccount],
        data: bytes,
        compute_limit: int = 200_000,
    ) -> SvmInstruction:
        """
        Build an SVM instruction.
        
        Args:
            program_id: 32-byte program public key
            accounts: List of account metadata
            data: Instruction data bytes
            compute_limit: Compute unit limit
            
        Returns:
            SvmInstruction ready for inclusion in Comit
        """
        if len(program_id) != 32:
            raise ValueError("Program ID must be 32 bytes")
        
        return SvmInstruction(
            program_id=program_id,
            accounts=accounts,
            data=data,
            compute_limit=compute_limit,
        )
    
    def build_transfer(
        self,
        from_pubkey: bytes,
        to_pubkey: bytes,
        lamports: int,
        system_program: Optional[bytes] = None,
    ) -> SvmInstruction:
        """
        Build a system transfer instruction.
        
        Args:
            from_pubkey: Sender public key
            to_pubkey: Recipient public key  
            lamports: Amount to transfer
            system_program: System program ID (default: 11111111...)
            
        Returns:
            SvmInstruction for transfer
        """
        # System program ID (all 1s in base58 = 0x00...00 in bytes)
        if system_program is None:
            system_program = bytes(32)
        
        # Transfer instruction: type (4 bytes) + lamports (8 bytes)
        data = bytes([2, 0, 0, 0]) + lamports.to_bytes(8, "little")
        
        accounts = [
            SvmAccount(from_pubkey, is_signer=True, is_writable=True),
            SvmAccount(to_pubkey, is_signer=False, is_writable=True),
        ]
        
        return SvmInstruction(
            program_id=system_program,
            accounts=accounts,
            data=data,
            compute_limit=5000,  # Simple transfers are cheap
        )
    
    def to_comit_payload(self, instruction: SvmInstruction) -> bytes:
        """
        Convert SVM instruction to Comit payload format.
        
        Args:
            instruction: SvmInstruction to convert
            
        Returns:
            Bytes payload for ComitBuilder.with_svm_payload()
        """
        # Format:
        # program_id (32) + num_accounts (1) + 
        # [pubkey (32) + flags (1)]... +
        # data_len (2) + data
        
        payload = bytearray(instruction.program_id)
        payload.append(len(instruction.accounts))
        
        for account in instruction.accounts:
            payload.extend(account.pubkey)
            flags = 0
            if account.is_signer:
                flags |= 1
            if account.is_writable:
                flags |= 2
            payload.append(flags)
        
        payload.extend(len(instruction.data).to_bytes(2, "little"))
        payload.extend(instruction.data)
        
        return bytes(payload)
    
    def parse_program_logs(self, logs: List[bytes]) -> List[str]:
        """
        Parse program log messages.
        
        Args:
            logs: Raw log bytes from execution
            
        Returns:
            Human-readable log strings
        """
        return [log.decode("utf-8", errors="replace") for log in logs]
