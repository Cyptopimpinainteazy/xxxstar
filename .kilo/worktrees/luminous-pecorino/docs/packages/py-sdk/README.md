# X3 Chain Python SDK

A comprehensive Python SDK for interacting with the X3 Chain blockchain, featuring dual-VM execution (EVM + SVM) through the X3 Kernel.

## Installation

```bash
pip install x3-chain-sdk
```

Or install from source:

```bash
cd packages/py-sdk
pip install -e ".[dev]"
```

## Quick Start

### Connect to Node

```python
from x3_chain_sdk import AtlasClient

# Using context manager (recommended)
with AtlasClient("ws://localhost:9944") as client:
    info = client.get_chain_info()
    print(f"Connected to {info.chain_name} (chain ID: {info.chain_id})")

# Manual connection
client = AtlasClient("ws://localhost:9944")
client.connect()
# ... use client ...
client.disconnect()
```

### Query Account Balance

```python
from x3_chain_sdk import AtlasClient

with AtlasClient("ws://localhost:9944") as client:
    account = "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY"
    
    # Get account info
    info = client.get_account_info(account)
    print(f"Balance: {info.free_balance}")
    print(f"Nonce: {info.nonce}")
    print(f"Authorized: {info.is_authorized}")
    
    # Get canonical ledger balance
    balance = client.get_canonical_balance(account, asset_id=0)
    print(f"Canonical balance: {balance}")
```

### Build and Submit Comit Transaction

```python
from x3_chain_sdk import AtlasClient, ComitBuilder
from substrateinterface import Keypair

# Create keypair
keypair = Keypair.create_from_uri("//Alice")

with AtlasClient("ws://localhost:9944") as client:
    # Get current nonce
    nonce = client.get_nonce(keypair.ss58_address)
    
    # Build Comit transaction
    comit = (
        ComitBuilder()
        .with_evm_call(
            to="0x1234567890abcdef1234567890abcdef12345678",
            data=b"\x12\x34",  # function calldata
            gas_limit=500_000,
        )
        .with_svm_instruction(
            program_id=bytes(32),
            instruction_data=b"\x01\x02\x03",
            compute_limit=200_000,
        )
        .build(keypair, nonce=nonce)
    )
    
    # Submit
    from x3_chain_sdk.comit import submit_comit
    result = submit_comit(client, comit, keypair)
    
    print(f"Comit ID: {result.comit_id}")
    print(f"Block: {result.block_number}")
    print(f"Fee: {result.fee_charged}")
```

### EVM-Specific Operations

```python
from x3_chain_sdk import AtlasClient
from x3_chain_sdk.evm import EvmClient

with AtlasClient("ws://localhost:9944") as client:
    evm = EvmClient(client)
    
    # Get chain info
    chain_id = evm.get_chain_id()
    gas_price = evm.get_gas_price()
    
    # Build contract call
    tx = evm.build_contract_call(
        to="0x1234567890abcdef1234567890abcdef12345678",
        function_signature="transfer(address,uint256)",
        "0xrecipient...",
        1000000000000000000,  # 1 token
    )
    
    # Convert to Comit payload
    payload = evm.to_comit_payload(tx)
```

### SVM-Specific Operations

```python
from x3_chain_sdk import AtlasClient
from x3_chain_sdk.svm import SvmClient

with AtlasClient("ws://localhost:9944") as client:
    svm = SvmClient(client)
    
    # Build transfer instruction
    instruction = svm.build_transfer(
        from_pubkey=sender_pubkey,
        to_pubkey=recipient_pubkey,
        lamports=1_000_000,
    )
    
    # Convert to Comit payload
    payload = svm.to_comit_payload(instruction)
```

### Subscribe to New Blocks

```python
from x3_chain_sdk import AtlasClient

with AtlasClient("ws://localhost:9944") as client:
    def on_new_block(header):
        print(f"New block #{header.number}: {header.hash[:16]}...")
    
    subscription_id = client.subscribe_new_heads(on_new_block)
    
    # Later: unsubscribe
    client.unsubscribe(subscription_id)
```

## CLI Usage

```bash
# Show chain info
x3-chain info

# Check account balance
x3-chain balance 5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY

# Show block info
x3-chain block 12345

# List authorities
x3-chain authorities

# Watch for new blocks
x3-chain watch

# Watch finalized blocks
x3-chain watch --finalized
```

## API Reference

### AtlasClient

- `connect()` - Connect to node
- `disconnect()` - Disconnect
- `get_chain_info()` - Get chain metadata
- `get_account_info(account)` - Get account details
- `get_canonical_balance(account, asset_id)` - Get canonical ledger balance
- `get_asset_metadata(asset_id)` - Get asset metadata
- `get_block_header(block_hash)` - Get block header
- `get_nonce(account)` - Get account nonce
- `is_authorized(account)` - Check Comit authorization
- `subscribe_new_heads(callback)` - Subscribe to new blocks
- `subscribe_finalized_heads(callback)` - Subscribe to finalized blocks

### ComitBuilder

- `with_evm_payload(payload, gas_limit)` - Set raw EVM payload
- `with_evm_call(to, data, value, gas_limit)` - Add EVM contract call
- `with_svm_payload(payload, compute_limit)` - Set raw SVM payload
- `with_svm_instruction(...)` - Add Solana-style instruction
- `with_nonce(nonce)` - Set explicit nonce
- `build(keypair, nonce)` - Build transaction

### EvmClient

- `get_chain_id()` - Get EVM chain ID
- `get_gas_price()` - Get gas price
- `build_contract_call(...)` - Build contract call
- `build_deployment(...)` - Build contract deployment
- `to_comit_payload(tx)` - Convert to Comit payload

### SvmClient

- `build_instruction(...)` - Build instruction
- `build_transfer(...)` - Build transfer instruction
- `to_comit_payload(instruction)` - Convert to Comit payload

## Development

```bash
# Install dev dependencies
pip install -e ".[dev]"

# Run tests
pytest

# Format code
black src tests
isort src tests

# Type check
mypy src
```

## License

Apache-2.0
