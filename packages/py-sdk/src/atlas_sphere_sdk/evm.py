"""
EVM client for Ethereum-compatible interactions.
"""

from typing import Any, Dict, List, Optional, Union
from dataclasses import dataclass

from x3_chain_sdk.types import ExecutionReceipt


@dataclass
class EvmTransaction:
    """EVM transaction parameters."""
    to: Optional[str]  # Contract address or None for deployment
    data: bytes  # Calldata or init code
    value: int = 0  # Wei value
    gas_limit: int = 500_000
    gas_price: Optional[int] = None
    nonce: Optional[int] = None


class EvmClient:
    """
    Client for EVM-specific operations.
    
    Provides Ethereum-compatible interfaces for:
    - Contract calls and deployments
    - Gas estimation
    - Event log queries
    """
    
    def __init__(self, client: Any):
        """
        Initialize EVM client.
        
        Args:
            client: AtlasClient instance
        """
        self._client = client
    
    def get_chain_id(self) -> int:
        """Get EVM chain ID."""
        result = self._client._call_rpc("eth_chainId")
        return int(result, 16) if result else 650000
    
    def get_gas_price(self) -> int:
        """Get current gas price in wei."""
        result = self._client._call_rpc("eth_gasPrice")
        return int(result, 16) if result else 1_000_000_000
    
    def get_block_number(self) -> int:
        """Get latest block number."""
        result = self._client._call_rpc("eth_blockNumber")
        return int(result, 16) if result else 0
    
    def encode_function_call(
        self,
        function_signature: str,
        *args,
    ) -> bytes:
        """
        Encode a function call.
        
        Args:
            function_signature: e.g. "transfer(address,uint256)"
            *args: Function arguments
            
        Returns:
            ABI-encoded calldata
        """
        from eth_abi import encode
        from eth_utils import function_signature_to_4byte_selector
        
        selector = function_signature_to_4byte_selector(function_signature)
        
        # Parse types from signature
        types_str = function_signature[function_signature.index("(") + 1:-1]
        types = [t.strip() for t in types_str.split(",")] if types_str else []
        
        if types and args:
            encoded_args = encode(types, args)
            return selector + encoded_args
        return selector
    
    def decode_function_result(
        self,
        output_types: List[str],
        data: bytes,
    ) -> List[Any]:
        """
        Decode function return data.
        
        Args:
            output_types: List of Solidity types
            data: Raw return data
            
        Returns:
            Decoded values
        """
        from eth_abi import decode
        return list(decode(output_types, data))
    
    def build_contract_call(
        self,
        to: str,
        function_signature: str,
        *args,
        value: int = 0,
        gas_limit: int = 500_000,
    ) -> EvmTransaction:
        """
        Build a contract call transaction.
        
        Args:
            to: Contract address (0x prefixed)
            function_signature: e.g. "transfer(address,uint256)"
            *args: Function arguments
            value: Wei value to send
            gas_limit: Gas limit
            
        Returns:
            EvmTransaction ready for inclusion in Comit
        """
        data = self.encode_function_call(function_signature, *args)
        return EvmTransaction(
            to=to,
            data=data,
            value=value,
            gas_limit=gas_limit,
        )
    
    def build_deployment(
        self,
        bytecode: bytes,
        constructor_args: Optional[bytes] = None,
        gas_limit: int = 3_000_000,
    ) -> EvmTransaction:
        """
        Build a contract deployment transaction.
        
        Args:
            bytecode: Contract bytecode
            constructor_args: ABI-encoded constructor arguments
            gas_limit: Gas limit for deployment
            
        Returns:
            EvmTransaction for contract creation
        """
        data = bytecode
        if constructor_args:
            data = bytecode + constructor_args
        
        return EvmTransaction(
            to=None,
            data=data,
            value=0,
            gas_limit=gas_limit,
        )
    
    def to_comit_payload(self, tx: EvmTransaction) -> bytes:
        """
        Convert EVM transaction to Comit payload format.
        
        Args:
            tx: EvmTransaction to convert
            
        Returns:
            Bytes payload for ComitBuilder.with_evm_payload()
        """
        # Format: to (20 bytes, zero for deploy) + value (32 bytes) + data
        if tx.to:
            to_bytes = bytes.fromhex(tx.to[2:] if tx.to.startswith("0x") else tx.to)
        else:
            to_bytes = bytes(20)
        
        value_bytes = tx.value.to_bytes(32, "big")
        return to_bytes + value_bytes + tx.data
