# X3 Wallet API Documentation

## Overview

The X3 Wallet API provides comprehensive access to wallet operations, multisig management, hardware wallet integration, biometric verification, account recovery, and DEX swap execution. All operations are exposed through JSON-RPC endpoints and CLI commands.

## Base URL

- **RPC Endpoint**: `http://localhost:9944` (default testnet)
- **WebSocket**: `ws://localhost:9933` (default testnet)

## Authentication

All wallet operations are account-derived. Authentication occurs via:
1. Wallet signature verification
2. Hardware wallet confirmation
3. Biometric verification (if enabled)
4. Guardian approval (for recovery operations)

## JSON-RPC Methods

### Wallet Management

#### `walletDex_estimateSwap`

Estimate swap output with approval requirements.

**Request:**
```json
{
  "jsonrpc": "2.0",
  "method": "walletDex_estimateSwap",
  "params": {
    "token_in": "0x1234567890123456789012345678901234567890",
    "token_out": "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd",
    "amount_in": 1000000000000,
    "min_amount_out": 900000000000,
    "wallet_id": "0x1111111111111111111111111111111111111111",
    "require_approval": true,
    "approval_threshold": 500000000000
  },
  "id": 1
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "swap_id": "0x2222222222222222222222222222222222222222",
    "amount_out": 950000000000,
    "approval_required": true,
    "approval_request_id": "0x3333333333333333333333333333333333333333",
    "estimated_gas": 150000
  },
  "id": 1
}
```

**Parameters:**
- `token_in` (string): Token ID being swapped
- `token_out` (string): Token ID being received
- `amount_in` (u128): Amount to swap
- `min_amount_out` (u128): Minimum acceptable output
- `wallet_id` (bytes32): Wallet conducting swap
- `require_approval` (bool): Whether approval is required
- `approval_threshold` (u128): Amount threshold for approval requirement

**Returns:**
- `swap_id`: Unique swap identifier
- `amount_out`: Estimated output amount
- `approval_required`: Whether approval is needed
- `approval_request_id`: ID for approval tracking (if required)
- `estimated_gas`: Estimated transaction cost

---

#### `walletDex_executeSwap`

Execute a swap with wallet signatures.

**Request:**
```json
{
  "jsonrpc": "2.0",
  "method": "walletDex_executeSwap",
  "params": {
    "request": {
      "token_in": "0x1234567890123456789012345678901234567890",
      "token_out": "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd",
      "amount_in": 1000000000000,
      "min_amount_out": 900000000000,
      "wallet_id": "0x1111111111111111111111111111111111111111",
      "require_approval": true,
      "approval_threshold": 500000000000
    },
    "signatures": [
      "0xaabbcc...",
      "0xddeeff..."
    ]
  },
  "id": 1
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "swap_id": "0x2222222222222222222222222222222222222222",
    "amount_out": 950000000000,
    "approval_required": false,
    "approval_request_id": null,
    "estimated_gas": 150000
  },
  "id": 1
}
```

---

#### `walletDex_requestHardwareSignature`

Request signature from hardware wallet.

**Request:**
```json
{
  "jsonrpc": "2.0",
  "method": "walletDex_requestHardwareSignature",
  "params": {
    "wallet_id": "0x1111111111111111111111111111111111111111",
    "transaction_hash": "0x2222222222222222222222222222222222222222",
    "display_message": "Sign swap: 1000 USDC → 950 USDT"
  },
  "id": 1
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "transaction_hash": "0x2222222222222222222222222222222222222222",
    "display_message": "Sign swap: 1000 USDC → 950 USDT",
    "request_id": "0x3333333333333333333333333333333333333333",
    "timeout_seconds": 120
  },
  "id": 1
}
```

**Parameters:**
- `wallet_id` (bytes32): Hardware wallet ID
- `transaction_hash` (bytes32): Transaction hash to sign
- `display_message` (string): Message to display on hardware device

**Returns:**
- `transaction_hash`: Transaction being signed
- `display_message`: Confirmation message shown
- `request_id`: Unique signing request ID
- `timeout_seconds`: Signature timeout (120s default)

---

#### `walletDex_approveTransaction`

Approve transaction with multisig signature.

**Request:**
```json
{
  "jsonrpc": "2.0",
  "method": "walletDex_approveTransaction",
  "params": {
    "wallet_id": "0x1111111111111111111111111111111111111111",
    "transaction_hash": "0x2222222222222222222222222222222222222222",
    "approval_signature": "0xaabbccddee..."
  },
  "id": 1
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": true,
  "id": 1
}
```

---

#### `walletDex_getBalance`

Query token balance for account.

**Request:**
```json
{
  "jsonrpc": "2.0",
  "method": "walletDex_getBalance",
  "params": {
    "account": "5GrwvaEF5zXb26Fz9rcQkQvCnPLEW3efqNyV29w4bwMv",
    "token_id": "0x1111111111111111111111111111111111111111"
  },
  "id": 1
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": 1234567890000,
  "id": 1
}
```

---

#### `walletDex_getApprovalStatus`

Get approval transaction status.

**Request:**
```json
{
  "jsonrpc": "2.0",
  "method": "walletDex_getApprovalStatus",
  "params": {
    "approval_id": "0x3333333333333333333333333333333333333333"
  },
  "id": 1
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": ["pending", 2],
  "id": 1
}
```

**Returns:**
- `[status, signatures_needed]`: Current approval status and signatures still needed
  - Status: "pending" | "approved" | "rejected" | "executed"
  - Signatures needed: Number of additional signatures required

---

## Runtime Extrinsic Calls

### Hardware Wallet Operations

#### `register_hardware_wallet`

Register a new hardware wallet.

**Extrinsic:**
```rust
pallet_x3_wallet::Call::register_hardware_wallet {
    device_type: "Ledger Nano S+",
    device_model: "device-model-hash",
    public_key: [u8; 32]
}
```

**Weight:** 10,000 (PoW)

**Events:**
- `HardwareWalletConnected { account, device_type }`

**Errors:**
- `TooManyWallets`: Account already has max wallets (10)
- `Unauthorized`: Invalid device or public key

---

### Multisig Operations

#### `create_multisig_wallet`

Create a new multisig wallet.

**Extrinsic:**
```rust
pallet_x3_wallet::Call::create_multisig_wallet {
    signers: vec![/*addresses*/],
    threshold: 3,
    timelock_delay: Some(10) // blocks
}
```

**Weight:** 15,000 (PoW)

**Events:**
- `MultisigWalletCreated { account, threshold }`

**Constraints:**
- Max signers: 50
- Threshold ≤ number of signers
- Timelock delay: 0-1000 blocks

---

### Token Operations

#### `transfer_tokens`

Transfer tokens with approval checking.

**Extrinsic:**
```rust
pallet_x3_wallet::Call::transfer_tokens {
    token_id: [u8; 32],
    to: AccountId,
    amount: 1000000000000
}
```

**Weight:** 10,000 (PoW)

**Checks:**
- Sufficient balance
- Approval status if required
- Wallet policy compliance

**Events:**
- `BalanceUpdated { account, token_id, amount }`

---

#### `mint_tokens`

Mint new tokens (admin only).

**Extrinsic:**
```rust
pallet_x3_wallet::Call::mint_tokens {
    token_id: [u8; 32],
    to: AccountId,
    amount: 1000000000000
}
```

**Weight:** 5,000 (PoW)

**Requires:** Root origin

---

### Biometric Operations

#### `register_biometric`

Register biometric profile.

**Extrinsic:**
```rust
pallet_x3_wallet::Call::register_biometric {
    biometric_type: "fingerprint", // or "face", "iris"
    template_hash: [u8; 32],
    pin_hash: [u8; 32]
}
```

**Weight:** 8,000 (PoW)

**Events:**
- `BiometricProfileCreated { account }`

---

### Recovery Operations

#### `initiate_recovery`

Initiate account recovery.

**Extrinsic:**
```rust
pallet_x3_wallet::Call::initiate_recovery {
    new_owner: AccountId
}
```

**Weight:** 12,000 (PoW)

**Requirements:**
- At least 1 recovery guardian must exist
- Guardian approval period: 10 blocks

**Events:**
- `RecoveryInitiated { account, new_owner }`

---

## Storage Queries

### Hardware Wallets

**Storage Key:**
```
HardwareWallets: (AccountId, [u8; 32]) → HardwareWallet
```

**Query Example:**
```rust
let wallet = HardwareWallets::get((account_id, wallet_id));
```

**Returns:**
```rust
HardwareWallet {
    device_type: String,
    device_model: String,
    public_key: [u8; 32],
    created_block: BlockNumber,
    last_used_block: BlockNumber,
    is_active: bool
}
```

---

### Multisig Wallets

**Storage Key:**
```
MultisigWallets: (AccountId, [u8; 32]) → MultisigWallet
```

**Returns:**
```rust
MultisigWallet {
    signers: Vec<AccountId>,
    threshold: u32,
    timelock_delay: u32,
    created_block: BlockNumber,
    is_active: bool
}
```

---

### Token Balances

**Storage Key:**
```
TokenBalances: (AccountId, [u8; 32]) → u128
```

**Query Example:**
```rust
let balance = TokenBalances::get((account_id, token_id));
```

---

### Recovery Accounts

**Storage Key:**
```
RecoveryAccounts: AccountId → GuardianAccount
```

**Returns:**
```rust
GuardianAccount {
    guardians: Vec<(AccountId, GuardianType)>,
    recovery_in_progress: bool,
    new_owner_candidate: Option<AccountId>,
    recovery_initiated_block: BlockNumber
}
```

---

## CLI Commands

### Wallet Status

```bash
x3-wallet status --full --json
```

**Output:**
```json
{
  "account": "5GrwvaEF5zXb26Fz9rcQkQvCnPLEW3efqNyV29w4bwMv",
  "balance": 1234567890000,
  "wallets_connected": 3,
  "pending_approvals": 1
}
```

---

### Hardware Wallet

```bash
x3-wallet hardware register \
  --device-type "Ledger Nano S+" \
  --device-model "nano-s-plus" \
  --public-key "0x1234..."

x3-wallet hardware list

x3-wallet hardware verify --device-id "0x1234..."
```

---

### Multisig

```bash
x3-wallet multisig create \
  --signers "5GrwvaEF5...,5Ggdto..." \
  --threshold 2 \
  --delay 10

x3-wallet multisig info --wallet-id "0x1234..."

x3-wallet multisig propose \
  --wallet-id "0x1234..." \
  --to "5GrwvaEF5..." \
  --amount 1000000000000

x3-wallet multisig approve --tx-id "0x5678..."

x3-wallet multisig execute --tx-id "0x5678..."
```

---

### Token Operations

```bash
x3-wallet account balance

x3-wallet swap estimate \
  --token-in "0x1234..." \
  --token-out "0xabcd..." \
  --amount 1000000000000

x3-wallet swap execute \
  --token-in "0x1234..." \
  --token-out "0xabcd..." \
  --amount 1000000000000 \
  --min-output 900000000000
```

---

### Biometric

```bash
x3-wallet biometric enroll --biometric-type fingerprint

x3-wallet biometric verify --biometric-type fingerprint

x3-wallet biometric require-for-approval --enabled true
```

---

### Recovery

```bash
x3-wallet recovery add-guardian \
  --guardian-address "5GrwvaEF5..." \
  --guardian-type family

x3-wallet recovery list-guardians

x3-wallet recovery initiate --new-owner "5NewOwner..."

x3-wallet recovery approve --recovery-id "0x1234..."
```

---

## Error Responses

### Common RPC Errors

**Invalid Parameters:**
```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32602,
    "message": "Invalid params",
    "data": "amount_in cannot be zero"
  },
  "id": 1
}
```

**Wallet Not Found:**
```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32000,
    "message": "Wallet not found"
  },
  "id": 1
}
```

**Insufficient Balance:**
```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32000,
    "message": "Insufficient balance"
  },
  "id": 1
}
```

---

## Rate Limiting

- **Public methods** (estimates, balances): 100 req/minute
- **Write operations** (swaps, transfers): 10 req/minute per account
- **Hardware signing**: 1 concurrent signature per device

---

## Examples

### Complete Swap Flow

```bash
# 1. Estimate swap
curl -X POST http://localhost:9944 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "walletDex_estimateSwap",
    "params": {
      "token_in": "0x1234...",
      "token_out": "0xabcd...",
      "amount_in": 1000000000000,
      "min_amount_out": 900000000000,
      "wallet_id": "0x1111...",
      "require_approval": true,
      "approval_threshold": 500000000000
    },
    "id": 1
  }'

# 2. If approval needed, request hardware signature
curl -X POST http://localhost:9944 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "walletDex_requestHardwareSignature",
    "params": {
      "wallet_id": "0x1111...",
      "transaction_hash": "0x2222...",
      "display_message": "Approve swap: 1000 USDC → 950 USDT"
    },
    "id": 2
  }'

# 3. Wait for user to confirm on hardware wallet
# 4. Execute swap
curl -X POST http://localhost:9944 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "walletDex_executeSwap",
    "params": {
      "request": { ... },
      "signatures": ["0xaabbcc..."]
    },
    "id": 3
  }'
```

---

## Support & Resources

- **Bug Reports**: Submit via GitHub Issues
- **Feature Requests**: Create GitHub Discussion
- **Security Issues**: Email security@x3.chain
- **Documentation**: https://docs.x3.chain/wallet

---

**API Version:** v0.1.0  
**Last Updated:** 2024-12-19
