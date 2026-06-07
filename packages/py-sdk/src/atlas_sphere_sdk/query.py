"""
Query client for read-only operations on X3 Chain.
"""

from typing import Any, Dict, List, Optional

from x3_chain_sdk.types import (
    AccountId,
    AssetId,
    Balance,
    BlockHeader,
    ChainInfo,
)


class QueryClient:
    """
    Read-only query client for X3 Chain.
    
    Provides efficient methods for querying chain state without
    requiring a keypair for signing.
    """
    
    def __init__(self, client: Any):
        """
        Initialize query client.
        
        Args:
            client: AtlasClient instance
        """
        self._client = client
    
    def get_chain_info(self) -> ChainInfo:
        """Get chain metadata."""
        return self._client.get_chain_info()
    
    def get_block(self, block_hash: Optional[str] = None) -> BlockHeader:
        """Get block header."""
        return self._client.get_block_header(block_hash)
    
    def get_balance(self, account: AccountId, asset_id: AssetId = 0) -> Balance:
        """Get canonical balance."""
        return self._client.get_canonical_balance(account, asset_id)
    
    def get_nonce(self, account: AccountId) -> int:
        """Get account nonce."""
        return self._client.get_nonce(account)
    
    def is_authorized(self, account: AccountId) -> bool:
        """Check if account is authorized."""
        return self._client.is_authorized(account)
    
    def get_authorities(self) -> List[AccountId]:
        """Get current authority set."""
        result = self._client._call_rpc("atlasKernel_getAuthorities")
        return result or []
    
    def get_authorized_accounts(self) -> List[AccountId]:
        """Get all authorized accounts."""
        result = self._client._call_rpc("atlasKernel_getAuthorizedAccounts")
        return result or []
    
    def query_storage(
        self,
        module: str,
        storage_function: str,
        params: Optional[List] = None,
    ) -> Any:
        """
        Query arbitrary storage.
        
        Args:
            module: Pallet name
            storage_function: Storage item name
            params: Storage key parameters
            
        Returns:
            Storage value
        """
        substrate = self._client._ensure_connected()
        result = substrate.query(module, storage_function, params or [])
        return result.value if result else None
    
    def query_multi(
        self,
        queries: List[Dict[str, Any]],
    ) -> List[Any]:
        """
        Execute multiple storage queries in batch.
        
        Args:
            queries: List of {module, function, params} dicts
            
        Returns:
            List of results
        """
        substrate = self._client._ensure_connected()
        
        storage_keys = []
        for q in queries:
            key = substrate.create_storage_key(
                q["module"],
                q["function"],
                q.get("params", []),
            )
            storage_keys.append(key)
        
        results = substrate.query_multi(storage_keys)
        return [r.value if r else None for r in results]
