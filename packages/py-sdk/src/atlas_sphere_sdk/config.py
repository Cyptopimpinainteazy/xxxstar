"""
X3 Chain Python SDK Environment Configuration

Provides environment variable support for configuring SDK connections
to live node endpoints (mainnet, testnet, or custom).

Environment Variables:
    X3_RPC_ENDPOINT   - Custom WebSocket endpoint (overrides network selection)
    X3_NETWORK        - Network to connect to: 'mainnet' | 'testnet' | 'local' (default: 'local')
    X3_AUTO_RECONNECT - Enable auto-reconnect (default: 'true')
    X3_RECONNECT_MAX  - Maximum reconnect attempts (default: '5')
    X3_RECONNECT_DELAY - Reconnect delay in ms (default: '1000')
    X3_TIMEOUT        - Request timeout in ms (default: '30000')
"""

import os
from typing import Optional, Dict, Any
from dataclasses import dataclass, field


# Network configuration
NetworkType = str
NETWORK_MAINNET: NetworkType = "mainnet"
NETWORK_TESTNET: NetworkType = "testnet"
NETWORK_LOCAL: NetworkType = "local"


@dataclass
class X3SdkConfig:
    """Configuration for X3 Chain SDK."""
    
    # Network settings
    network: NetworkType = NETWORK_LOCAL
    endpoint: Optional[str] = None
    
    # Connection settings
    auto_reconnect: bool = True
    reconnect_max_attempts: int = 5
    reconnect_delay: int = 1000  # milliseconds
    
    # Timeout settings
    timeout: int = 30000  # milliseconds
    
    # Debug settings
    debug: bool = False
    
    # Additional options
    ss58_format: int = 42
    chain_id: int = 650000


def get_env(name: str, default: str = "") -> str:
    """Get environment variable with default value."""
    return os.environ.get(name, default)


def get_env_int(name: str, default: int) -> int:
    """Get environment variable as integer."""
    try:
        return int(get_env(name, str(default)))
    except (ValueError, TypeError):
        return default


def get_env_bool(name: str, default: bool) -> bool:
    """Get environment variable as boolean."""
    value = get_env(name, str(default).lower())
    return value.lower() in ('true', '1', 'yes')


def get_network_endpoints() -> Dict[NetworkType, str]:
    """Get default endpoints for each network."""
    return {
        NETWORK_MAINNET: get_env('X3_RPC_ENDPOINT', 'wss://rpc.x3chain.io:9944'),
        NETWORK_TESTNET: get_env('X3_RPC_ENDPOINT', 'wss://testnet.x3chain.io:9944'),
        NETWORK_LOCAL: get_env('X3_RPC_ENDPOINT', 'ws://127.0.0.1:9944'),
    }


def get_sdk_config() -> X3SdkConfig:
    """Get SDK configuration from environment variables."""
    network_env = get_env('X3_NETWORK', 'local').lower()
    
    # Validate network
    if network_env not in (NETWORK_MAINNET, NETWORK_TESTNET, NETWORK_LOCAL):
        network_env = NETWORK_LOCAL
    
    return X3SdkConfig(
        network=network_env,
        endpoint=get_env('X3_RPC_ENDPOINT', None),
        auto_reconnect=get_env_bool('X3_AUTO_RECONNECT', True),
        reconnect_max_attempts=get_env_int('X3_RECONNECT_MAX', 5),
        reconnect_delay=get_env_int('X3_RECONNECT_DELAY', 1000),
        timeout=get_env_int('X3_TIMEOUT', 30000),
        debug=get_env_bool('X3_DEBUG', False),
    )


def get_endpoint(network: NetworkType) -> str:
    """Get endpoint for a specific network."""
    endpoints = get_network_endpoints()
    return endpoints.get(network, endpoints[NETWORK_LOCAL])


def get_current_network() -> NetworkType:
    """Get current network from environment."""
    return get_sdk_config().network


def get_current_endpoint() -> str:
    """Get current endpoint (custom or network-based)."""
    config = get_sdk_config()
    return config.endpoint or get_endpoint(config.network)


# Default configuration instance
default_config = get_sdk_config()
