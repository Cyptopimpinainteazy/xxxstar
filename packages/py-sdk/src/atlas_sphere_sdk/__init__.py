"""
X3 Chain Python SDK

A comprehensive SDK for interacting with the X3 Chain blockchain,
featuring dual-VM execution (EVM + SVM) through the X3 Kernel.

Environment Variables:
    X3_RPC_ENDPOINT   - Custom WebSocket endpoint
    X3_NETWORK        - Network: 'mainnet' | 'testnet' | 'local' (default: 'local')
    X3_AUTO_RECONNECT - Enable auto-reconnect (default: 'true')
    X3_RECONNECT_MAX  - Maximum reconnect attempts (default: '5')
    X3_RECONNECT_DELAY - Reconnect delay in ms (default: '1000')
    X3_TIMEOUT        - Request timeout in ms (default: '30000')
"""

from x3_chain_sdk.client import AtlasClient
from x3_chain_sdk.comit import ComitBuilder, ComitTransaction
from x3_chain_sdk.query import QueryClient
from x3_chain_sdk.evm import EvmClient
from x3_chain_sdk.svm import SvmClient
from x3_chain_sdk.types import (
    AccountId,
    AssetId,
    Balance,
    ComitId,
    ExecutionReceipt,
)
from x3_chain_sdk.collateral import CollateralManagerClient, DepositReceipt, WithdrawRequest

# Re-export from atlas_sphere_sdk for convenience
from atlas_sphere_sdk import (
    AtlasClient as AtlasSphereClient,
    config as sdk_config,
)

__all__ = [
    "AtlasClient",
    "AtlasSphereClient",
    "ComitBuilder",
    "ComitTransaction",
    "QueryClient",
    "EvmClient",
    "SvmClient",
    "AccountId",
    "AssetId",
    "Balance",
    "ComitId",
    "ExecutionReceipt",
    "CollateralManagerClient",
    "DepositReceipt",
    "WithdrawRequest",
    "sdk_config",
]

__version__ = "0.1.0"
