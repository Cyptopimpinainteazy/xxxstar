# X3 Wallet CLI - User Guide

## Installation

### From Source

```bash
cargo build --release -p x3-wallet-cli
cp target/release/x3-wallet /usr/local/bin/
```

### Version Check

```bash
x3-wallet --version
x3-wallet -h
```

## Quick Start

### 1. Check Wallet Status

```bash
x3-wallet status
```

**Output:**
```
X3 Wallet Status
─────────────────────────────────────
Account: 5GrwvaEF5zXb26Fz9rcQkQvCnPLEW3efqNyV29w4bwMv
Balance: 1,234,567.89 X3T
Wallets Connected: 3
  - Hardware: Ledger Nano S+
  - Multisig: 3-of-5
  - Social Recovery: 2 guardians

Recent Transactions:
  1. Swap 1000 USDC → 950 USDT
  2. Transfer 100 X3T to Alice
```

### 2. Check Balance

```bash
x3-wallet account balance
x3-wallet account balance --token-id 0x1234...abcd
```

### 3. View Saved Contacts

```bash
x3-wallet account list-contacts
```

## Hardware Wallet Operations

### Register a Hardware Wallet

```bash
x3-wallet hardware register \
  --device-type "Ledger Nano S+" \
  --device-model "nano-s-plus-v1.0" \
  --public-key "0x1234567890123456789012345678901234567890"
```

**Requirements:**
- Hardware wallet must be connected
- Device must support X3 signing
- Public key must be exported from device

**Confirmation:**
Approve the registration on your hardware device when prompted.

---

### List Connected Hardware Wallets

```bash
x3-wallet hardware list
```

**Output:**
```
Connected Hardware Wallets:
  1. Ledger Nano S+ (connected)
     Device ID: 0x1234567890...
     Last used: Block 12,345
  
  2. Trezor One (connected)
     Device ID: 0xabcdefabcd...
     Last used: Block 12,340
```

---

### Verify Hardware Connection

```bash
x3-wallet hardware verify --device-id 0x1234567890...
```

**Output:**
```
Verifying device: 0x1234567890...
Status: ✓ Connected
Device Type: Ledger Nano S+
Public Key: 0x1234...7890
Firmware: v2.0.0
```

---

## Multisig Wallet Operations

### Create a Multisig Wallet

```bash
x3-wallet multisig create \
  --signers "5GrwvaEF5zXb26Fz9rcQkQvCnPLEW3efqNyV29w4bwMv,5FHneA46xpF1nqMvfU5x8MTkAe6BqrgZzJBnajiJUriMv" \
  --threshold 2 \
  --delay 10 # blocks
```

**Parameters:**
- `--signers`: Comma-separated list of signer addresses
- `--threshold`: Number of signatures required (must be ≤ number of signers)
- `--delay`: Optional timelock delay in blocks (0-1000)

**Output:**
```
Creating 2-of-2 multisig wallet
Timelock delay: 10 blocks
Status: ✓ Multisig wallet created
Wallet ID: 0x2222222222...
```

---

### Get Multisig Wallet Info

```bash
x3-wallet multisig info --wallet-id 0x2222222222...
```

**Output:**
```
Wallet ID: 0x2222222222...
Status: 2-of-2 multisig
Signers: 2
  1. 5GrwvaEF5zXb26Fz9rcQkQvCnPLEW3efqNyV29w4bwMv
  2. 5FHneA46xpF1nqMvfU5x8MTkAe6BqrgZzJBnajiJUriMv
Pending approvals: 1
Created: Block 12,000
```

---

### Propose a Multisig Transaction

```bash
x3-wallet multisig propose \
  --wallet-id 0x2222222222... \
  --to 5GrwvaEF5zXb26Fz9rcQkQvCnPLEW3efqNyV29w4bwMv \
  --amount 1000000000000 # 1M X3T
```

**Output:**
```
Proposing transaction from 0x2222...
Destination: 5GrwvaEF5zXb26Fz9rcQkQvCnPLEW3efqNyV29w4bwMv
Amount: 1000000000000 tokens
Status: ✓ Proposal created
TX ID: 0x3333333333...
Signatures needed: 2
```

---

### Approve a Multisig Transaction

```bash
x3-wallet multisig approve --tx-id 0x3333333333...
```

**Process:**
1. CLI prompts for signer account
2. Hardware wallet requests confirmation
3. Signature broadcasted to network

**Output:**
```
Approving transaction: 0x3333333333...
Signatures needed: 2
Current signatures: 1/2
Status: Pending final approval
```

---

### Execute a Multisig Transaction

```bash
x3-wallet multisig execute --tx-id 0x3333333333...
```

**Requirements:**
- All required signatures collected
- Timelock delay elapsed (if set)

**Output:**
```
Executing transaction: 0x3333333333...
Status: ✓ Transaction executed
Block: 12,050
TX Hash: 0x4444...
```

---

## Token Operations

### Check Token Balance

```bash
x3-wallet account balance
x3-wallet account balance --account 5GrwvaEF5zXb26Fz9rcQkQvCnPLEW3efqNyV29w4bwMv
x3-wallet account balance --token-id 0x1234...abcd
```

---

### Transfer Tokens

```bash
x3-wallet transaction estimate-fee \
  --to 5GrwvaEF5zXb26Fz9rcQkQvCnPLEW3efqNyV29w4bwMv \
  --amount 100000000000 # 100 X3T
```

**Output:**
```
Transfer 100000000000 tokens to 5GrwvaEF5...
Estimated fee: 0.001 tokens
```

---

## DEX Swap Operations

### Estimate Swap

```bash
x3-wallet swap estimate \
  --token-in 0x1111111111... \
  --token-out 0x2222222222... \
  --amount 1000000000000 # 1M USDC
```

**Output:**
```
Swap estimate: 1000000000000 USDC → USDT
You will receive: 950000000000 USDT (-5% impact)
Fee: 5000000000 USDT
Price: 0.95 USDT per USDC
Slippage: 5%
```

---

### Execute Swap

```bash
x3-wallet swap execute \
  --token-in 0x1111111111... \
  --token-out 0x2222222222... \
  --amount 1000000000000 \
  --min-output 900000000000 \
  --wallet-id 0x1234... # optional
```

**Process:**
1. Validates balance
2. Checks approval status
3. Requests hardware wallet signature (if needed)
4. Broadcasts swap transaction

**Output:**
```
Execute swap: 1000000000000 USDC → USDT
Minimum output: 900000000000
Wallet: 0x1234...
Status: ✓ Swap executed
Amount received: 950000000000 USDT
TX Hash: 0x5555...
Block: 12,100
```

---

### Approve Token for Swapping

```bash
x3-wallet swap approve \
  --token 0x1111111111... \
  --amount 1000000000000
```

**Output:**
```
Approving 0x1111... for 1000000000000 tokens
Status: ✓ Approval granted
Allowance: 1000000000000
Expires: Block 12,200
```

---

### View Swap History

```bash
x3-wallet swap history --limit 10
```

**Output:**
```
Swap History (last 10)
1. 1000 USDC → 950 USDT (block 12,100) - ✓ Success
2. 500 ETH → 8500 USDC (block 12,090) - ✓ Success
3. 100 DAI → 98 USDC (block 12,080) - ✓ Success
```

---

## Biometric Operations

### Enroll Biometric

```bash
x3-wallet biometric enroll --biometric-type fingerprint
```

**Types:**
- `fingerprint`: Fingerprint biometric
- `face`: Facial recognition
- `iris`: Iris scanning

**Process:**
1. Place your finger/face/eye on sensor
2. Wait for capture
3. Repeat for multiple samples

**Output:**
```
Enrolling fingerprint
Status: Ready for biometric input...
⠀
Captured 3 samples - ✓ Success
Fingerprint enrolled: 0x7890...
Biometric template hash: 0x6666...
```

---

### Verify Biometric

```bash
x3-wallet biometric verify --biometric-type fingerprint
```

**Output:**
```
Verifying fingerprint
Status: Ready for biometric input...
⠀
Verification: ✓ Match (99.2% confidence)
```

---

### Require Biometric for Approvals

```bash
x3-wallet biometric require-for-approval --enabled true
x3-wallet biometric require-for-approval --enabled false
```

**Output:**
```
Biometric verification: REQUIRED for approvals
```

---

## Account Recovery

### Add Recovery Guardian

```bash
x3-wallet recovery add-guardian \
  --guardian-address 5GrwvaEF5zXb26Fz9rcQkQvCnPLEW3efqNyV29w4bwMv \
  --guardian-type family
```

**Types:**
- `family`: Family member
- `friend`: Trusted friend
- `service`: Professional recovery service

**Output:**
```
Adding family guardian: 5GrwvaEF5...
Status: ✓ Guardian added
Total guardians: 2
```

---

### List Guardians

```bash
x3-wallet recovery list-guardians
```

**Output:**
```
Recovery Guardians:
  1. Mom (family) - 5X9n...hQw3
  2. Best Friend (friend) - 3kLp...mBx7
Total: 2 guardians
```

---

### Initiate Recovery

```bash
x3-wallet recovery initiate --new-owner 5NewOwner...
```

**Process:**
1. Initiates recovery period (10 blocks default)
2. Sends notifications to all guardians
3. Guardians vote to approve/reject
4. On majority approval, ownership transfers

**Output:**
```
Initiating recovery for new owner: 5NewOwner...
Recovery ID: 0x8888...
Status: Pending guardian approvals
Guardians needed: 2/2
```

---

### Approve Recovery

```bash
x3-wallet recovery approve --recovery-id 0x8888...
```

**Process:**
1. Only guardian can execute
2. Signs recovery approval
3. Submits to chain

**Output:**
```
Approving recovery: 0x8888...
Status: ✓ Recovery approved
Approvals: 1/2
Recovery status: Pending final approval
```

---

## Transaction Signing

### Sign a Transaction

```bash
x3-wallet transaction sign \
  --tx-data "0x1234..." \
  --wallet-id 0x1234...
```

**Output:**
```
Signing transaction with wallet: 0x1234...
Status: Waiting for hardware confirmation...
⠀
Signature: 0xaabbcc...
Recovery ID: 1
Block signed: 12,150
```

---

### Submit a Signed Transaction

```bash
x3-wallet transaction submit --signed-tx "0xaabbcc..."
```

**Output:**
```
Submitting signed transaction...
TX Hash: 0x5555...
Status: Pending (0/3 confirmations)
```

---

### Check Transaction Status

```bash
x3-wallet transaction status --tx-hash 0x5555...
```

**Output:**
```
Transaction: 0x5555...
Status: Confirmed (2/3 blocks)
Block: 12,150
Fee used: 0.001 X3T
```

---

## Contact Management

### Add Contact

```bash
x3-wallet account add-contact \
  --name "Alice" \
  --address 5GrwvaEF5zXb26Fz9rcQkQvCnPLEW3efqNyV29w4bwMv \
  --network "Polkadot"
```

**Output:**
```
Adding contact: Alice (Polkadot)
Address: 5GrwvaEF5zXb26Fz9rcQkQvCnPLEW3efqNyV29w4bwMv
Status: ✓ Contact saved
```

---

### List Contacts

```bash
x3-wallet account list-contacts
```

**Output:**
```
Saved Contacts:
  1. Alice (5GrwvaEF5zXb26Fz9rcQkQvCnPLEW3efqNyV29w4bwMv)
  2. Bob (3kLp...mBx7)
  3. Charlie (2X9n...hQw3)
Total: 3 contacts
```

---

## Account Import/Export

### Import Account from Seed

```bash
x3-wallet account import \
  --mnemonic "word1 word2 word3 ... word12"
```

**Output:**
```
Importing account from mnemonic...
⠀
Account: 5GrwvaEF5zXb26Fz9rcQkQvCnPLEW3efqNyV29w4bwMv
Balance: 0 X3T
Status: ✓ Account imported
⠀
⚠️  Security: Store your seed phrase securely offline
```

---

### Export Account (Encrypted)

```bash
x3-wallet account export --password "your-strong-password"
```

**Output:**
```
Exporting encrypted account backup...
⠀
Export saved: ./wallet-backup-2024-12-19.enc
Size: 2.3 KB
Encryption: AES-256-GCM
⠀
⚠️  Keep this backup safe in multiple locations
```

---

## Advanced Options

### Custom RPC Endpoint

```bash
x3-wallet status --rpc-endpoint "http://validator-1:9944"
```

---

### Verbose Logging

```bash
x3-wallet status --verbose
```

**Output:**
```
X3 Wallet CLI - Verbose Mode
RPC Endpoint: http://127.0.0.1:9944

[DEBUG] Connecting to RPC...
[DEBUG] RPC connection established
[DEBUG] Querying account balance...
[DEBUG] Query completed: 1234567890000 tokens
...
```

---

### JSON Output

```bash
x3-wallet status --json
```

**Output:**
```json
{
  "account": "5GrwvaEF5zXb26Fz9rcQkQvCnPLEW3efqNyV29w4bwMv",
  "balance": 1234567890000,
  "wallets_connected": 3,
  "pending_approvals": 1,
  "status": "active"
}
```

---

## Troubleshooting

### Hardware Wallet Not Detected

```bash
x3-wallet hardware verify --device-id 0x1234...
```

**Solutions:**
1. Check USB connection
2. Unlock hardware device
3. Approve permission on device
4. Try different USB port
5. Update firmware

---

### Insufficient Balance

```bash
x3-wallet account balance
```

Check balance before swap/transfer. If insufficient:
1. Convert staked tokens to free balance
2. Wait for unlock period to elapse
3. Receive tokens from other accounts

---

### Swap Slippage Too High

Increase acceptable slippage:

```bash
x3-wallet swap execute \
  --token-in ... \
  --token-out ... \
  --amount ... \
  --min-output 800000000000 # Increase tolerance
```

---

### Multisig Timeout

If multisig approval times out:

```bash
x3-wallet multisig info --wallet-id 0x2222...
```

Check pending approvals, re-request signatures if needed.

---

## Configuration

Create `~/.x3wallet/config.toml`:

```toml
[network]
rpc_endpoint = "http://localhost:9944"
ws_endpoint = "ws://localhost:9933"

[wallet]
default_account = "5GrwvaEF5zXb26Fz9rcQkQvCnPLEW3efqNyV29w4bwMv"

[hardware]
enable_auto_detect = true
timeout_seconds = 120

[swap]
slippage_tolerance = 1.0 # percent
```

---

## Support

- **Help**: `x3-wallet --help`
- **Version**: `x3-wallet --version`
- **Bugs**: https://github.com/x3-chain/x3-wallet-cli/issues
- **Docs**: https://docs.x3.chain/wallet
- **Community**: https://discord.gg/x3-chain

---

**CLI Version:** v0.1.0  
**Last Updated:** 2024-12-19  
**License:** MIT
