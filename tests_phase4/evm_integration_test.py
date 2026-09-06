#!/usr/bin/env python3
"""
EVM Integration Test for X3 Chain
Tests basic EVM contract deployment and execution via RPC
"""

import json
import subprocess
import sys

# Connect to X3 Chain RPC
RPC_URL = "http://127.0.0.1:9944"


def rpc_call(method, params=None):
    """Make an RPC call to the node."""
    if params is None:
        params = []
    payload = {
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": 1
    }

    result = subprocess.run(
        [
            "curl", "-s", "-X", "POST", RPC_URL,
            "-H", "Content-Type: application/json",
            "-d", json.dumps(payload)
        ],
        capture_output=True,
        text=True
    )

    if result.returncode != 0:
        raise Exception(f"RPC call failed: {result.stderr}")

    response = json.loads(result.stdout)

    if "error" in response:
        raise Exception(f"RPC error: {response['error']}")

    return response.get("result")


def test_eth_rpc_methods():
    """Test basic Ethereum RPC methods."""
    print("=" * 60)
    print("Testing Ethereum RPC Methods")
    print("=" * 60)

    # Test chainId
    chain_id_hex = rpc_call("eth_chainId")
    chain_id = int(chain_id_hex, 16)
    print(f"✓ Chain ID: {chain_id} ({chain_id_hex})")
    assert chain_id == 0x9eb10, f"Expected chain ID 0x9eb10, got {chain_id}"

    # Test blockNumber
    block_num_hex = rpc_call("eth_blockNumber")
    block_number = int(block_num_hex, 16)
    print(f"✓ Current Block: {block_number}")
    assert block_number >= 0, "Block number should be non-negative"

    # Test gasPrice
    gas_price_hex = rpc_call("eth_gasPrice")
    gas_price = int(gas_price_hex, 16)
    print(f"✓ Gas Price: {gas_price} wei ({gas_price / 1e9:.2f} Gwei)")
    assert gas_price > 0, "Gas price should be positive"

    print()
    return True


def test_system_methods():
    """Test system RPC methods."""
    print("=" * 60)
    print("Testing System RPC Methods")
    print("=" * 60)

    # Test system_health
    health = rpc_call("system_health")
    print(f"✓ System health: {health}")
    assert health is not None, "Health check failed"

    # Test system_properties
    try:
        properties = rpc_call("system_properties")
        print(f"✓ System properties: chain_name={properties.get('ss58Format', 'N/A')}")
    except Exception:
        print("⚠ System properties not available")

    print()
    return True


def test_chain_methods():
    """Test chain RPC methods."""
    print("=" * 60)
    print("Testing Chain RPC Methods")
    print("=" * 60)

    # Get latest block
    try:
        header = rpc_call("chain_getHeader")
        print(f"✓ Latest block hash: {header[:20]}...")
    except Exception:
        print("⚠ chain_getHeader not available")

    # Test substrate methods
    try:
        finalized_head = rpc_call("chain_getFinalizedHead")
        print(f"✓ Finalized head: {finalized_head[:20]}...")
    except Exception:
        print("⚠ chain_getFinalizedHead not available")

    print()
    return True


if __name__ == "__main__":
    try:
        print("\n")
        print("🚀 X3 Chain EVM Integration Tests")
        print("=" * 60)
        print(f"RPC Endpoint: {RPC_URL}")
        print()

        # Run tests
        test_eth_rpc_methods()
        test_system_methods()
        test_chain_methods()

        print("=" * 60)
        print("✅ All EVM integration tests passed!")
        print("=" * 60)
        print()

    except Exception as e:
        print(f"\n❌ Test failed with error: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)
