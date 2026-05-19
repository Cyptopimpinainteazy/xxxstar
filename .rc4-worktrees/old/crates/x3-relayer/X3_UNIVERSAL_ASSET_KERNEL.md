# X3 Universal Asset Kernel
## Canonical Asset Registry & Cross-Chain/Cross-VM Token System

**Version:** 1.0  
**Status:** Architecture Locked for Phase 13f  
**Audience:** Architecture, Core Team, Exchange Partners, Validators  
**Critical Path:** Build Phase 1 (cross-VM) before external chains  

---

## 🎯 Core Value Proposition

**X3 is not a bridge. X3 is a kernel.**

One canonical asset ID → Many VM representations → One supply ledger → Provable movement through X3 kernel.

**The mistake to avoid:** Separate random tokens per VM = supply chaos = bridge bankruptcy.

**The X3 way:** Every asset has a single canonical record. Every VM has a representation. Every transfer is provable. Supply is always balanced.

---

## 1. CORE MODEL

### Single Canonical Identity Per Asset

```
AssetId: X3:USDC (deterministic blake2_256 hash)
  ├─ Origin: Ethereum chain_id=1, token=0xA0b86991...
  ├─ Canonical Supply: 1,000,000 USDC (locked on Ethereum)
  │
  └─ Representations:
      ├─ X3 Native: balance pallet (asset_id)
      ├─ X3 EVM: ERC20 contract (mirrored)
      ├─ X3 SVM: token program (mirrored)
      └─ Collateral: Ethereum gateway vault (locked)
```

### Supply Invariant (Non-negotiable)

```
total_minted_representations <= canonical_supply_or_locked_collateral

native_supply + evm_vm_supply + svm_vm_supply + pending_supply <= locked_collateral
```

This is the line between professional bridge and money printer.

---

## 2. REQUIRED X3 MODULES

### A. x3-asset-registry
**Purpose:** Stores every token identity, metadata, and routing rules.

```rust
pub struct AssetMetadata {
    pub asset_id: AssetId,           // blake2_256 hash
    pub symbol: Vec<u8>,              // "USDC", "X3", "SOL"
    pub name: Vec<u8>,                // "USD Coin", "X3 Native"
    pub decimals: u8,                 // 6, 18
    pub origin_domain: DomainId,      // Ethereum, Solana, X3Native
    pub origin_chain_id: Option<u64>, // 1 (Ethereum), 8453 (Base)
    pub origin_asset_address: Option<Vec<u8>>, // 0xA0b86991... or SPL mint
    pub supply_policy: SupplyPolicy,  // NativeMintBurn, LockMint, etc.
    pub status: AssetStatus,          // Active, Paused, Deprecated
    pub allowed_routes: Vec<RouteId>, // which transfers are allowed
}

pub enum DomainId {
    X3Native,    // X3 runtime native
    X3Evm,       // X3 EVM pallet VM
    X3Svm,       // X3 SVM pallet VM
    Ethereum,    // external
    Base,        // external
    Arbitrum,    // external
    Bsc,         // external
    Solana,      // external
    Bitcoin,     // external (special handling)
}

pub enum SupplyPolicy {
    NativeMintBurn,    // asset minted/burned on origin chain
    LockMint,          // external token locked, X3 wrapped minted
    BurnRelease,       // X3 wrapped burned, external released
    LiquidityBacked,   // LP reserves hold collateral
}

pub enum AssetStatus {
    Active,
    Paused,        // no new transfers
    Deprecated,    // phase out over time
    Emergency,     // locked due to incident
}
```

**Required Runtime Extrinsics:**
```rust
register_asset(asset_metadata) → AssetId
pause_asset(asset_id)
unpause_asset(asset_id)
set_allowed_route(asset_id, source_domain, destination_domain, enabled)
set_asset_limits(asset_id, daily_limit, per_tx_limit, emergency_halt)
update_supply_policy(asset_id, new_policy)
```

**Storage:**
```
Assets: map AssetId => AssetMetadata
AssetStatus: map AssetId => AssetStatus
AllowedRoutes: double_map AssetId, (DomainId, DomainId) => bool
AssetLimits: map AssetId => (Balance, Balance)  // (daily_limit, per_tx_limit)
```

---

### B. x3-token-vault
**Purpose:** Custody, supply accounting, and invariant checking.

```rust
pub struct SupplyLedger {
    pub asset_id: AssetId,
    pub native_supply: Balance,       // balance pallet holdings
    pub evm_vm_supply: Balance,       // ERC20 circulating
    pub svm_vm_supply: Balance,       // SVM token accounts
    pub external_locked_supply: Balance, // on external chain gateways
    pub pending_supply: Balance,      // transfers in flight
    pub canonical_supply: Balance,    // source of truth total
}

pub struct LedgerEntry {
    pub asset_id: AssetId,
    pub domain: DomainId,
    pub balance: Balance,
    pub pending: Balance,
}
```

**Required Runtime Extrinsics:**
```rust
mint_representation(asset_id, domain, account, amount)
burn_representation(asset_id, domain, account, amount)
lock_collateral(asset_id, amount)
release_collateral(asset_id, amount)
```

**Storage:**
```
SupplyLedgers: map AssetId => SupplyLedger
DomainBalances: double_map AssetId, DomainId => Balance
PendingSupply: map AssetId => Balance
```

**Core Invariant (runs after every mint/burn/transfer):**
```rust
pub fn assert_supply_invariant(asset_id: AssetId) -> DispatchResult {
    let ledger = SupplyLedgers::<T>::get(asset_id);
    
    let represented = ledger.native_supply
        .checked_add(ledger.evm_vm_supply)?
        .checked_add(ledger.svm_vm_supply)?
        .checked_add(ledger.pending_supply)?;
    
    ensure!(
        represented <= ledger.canonical_supply,
        Error::<T>::SupplyInvariantBroken
    );
    
    Ok(())
}
```

---

### C. x3-cross-vm-router
**Purpose:** Atomic token movement between X3 Native, X3 EVM, X3 SVM.

```rust
pub struct X3TransferMessage {
    pub version: u16,
    pub message_id: H256,         // blake2_256(nonce || source || dest || asset || amount)
    pub route_id: RouteId,
    pub asset_id: AssetId,
    pub source_domain: DomainId,  // X3Native, X3Evm, X3Svm
    pub destination_domain: DomainId,
    pub sender: AccountBytes,
    pub recipient: AccountBytes,
    pub amount: Balance,
    pub fee: Balance,
    pub nonce: u128,
    pub expiry_block: BlockNumber,
    pub proof_hash: H256,
}

pub enum TransferStatus {
    Created,
    SourceDebited,
    ProofSubmitted,
    DestinationCredited,
    Finalized,
    Expired,
    Refunded,
    Failed,
}
```

**Required Runtime Extrinsics:**
```rust
xvm_transfer(
    asset_id: AssetId,
    source_vm: DomainId,
    destination_vm: DomainId,
    recipient: AccountBytes,
    amount: Balance,
) → message_id

complete_xvm_transfer(message_id: H256)

cancel_expired_xvm_transfer(message_id: H256)
```

**Storage:**
```
PendingTransfers: map H256 => X3TransferMessage
TransferStatus: map H256 => TransferStatus
UsedMessages: map H256 => bool              // replay protection
UsedNonces: double_map DomainId, AccountId, u128 => bool
```

**Key: No Vec-based nonce scanning.** Use storage maps for O(1) replay checks.

**Transfer Lifecycle:**
```
Created
  ↓ (debit source VM)
SourceDebited
  ↓ (create proof)
ProofSubmitted
  ↓ (credit destination VM)
DestinationCredited
  ↓ (kernel finalizes)
Finalized ✅

If expiry reached:
Created / SourceDebited
  ↓
Expired
  ↓ (return funds)
Refunded ✅
```

---

### D. x3-crosschain-gateway
**Purpose:** Handles deposits/withdrawals from external chains.

```rust
pub struct ExternalChainConfig {
    pub chain_id: u64,
    pub gateway_address: Option<Vec<u8>>, // Ethereum: 0x..., Solana: pubkey
    pub finality_requirement: FinalityRequirement,
    pub supported_assets: Vec<AssetId>,
}

pub enum FinalityRequirement {
    EvmBlocks(u32),           // 64 for Ethereum, 32 for Base
    SolanaCommitment,         // "finalized"
    BitcoinConfirmations(u32), // 6+
    X3Blocks(u32),            // 12 for X3
}
```

**Required Runtime Extrinsics:**
```rust
submit_deposit_proof(
    source_chain: u64,
    proof: DepositProof,
)

request_withdrawal(
    asset_id: AssetId,
    destination_chain: u64,
    recipient: Vec<u8>,
    amount: Balance,
) → message_id

submit_release_proof(
    message_id: H256,
    proof: ReleaseProof,
)

finalize_crosschain_transfer(message_id: H256)

refund_expired_transfer(message_id: H256)
```

**Storage:**
```
ExternalChains: map u64 => ExternalChainConfig
PendingDeposits: map H256 => DepositProof
PendingWithdrawals: map H256 => WithdrawalRequest
UsedExternalMessages: map H256 => bool   // replay at destination
```

---

### E. x3-finality-oracle
**Purpose:** Verifies external chain finality before crediting X3.

```rust
pub enum FinalityProof {
    EvmReceipt {
        block_number: u64,
        receipt_hash: H256,
        confirmations: u32,
    },
    SolanaCommitment {
        slot: u64,
        blockhash: H256,
    },
    BitcoinConfirmations {
        block_height: u64,
        txid: [u8; 32],
        confirmations: u32,
    },
    X3FinalizedBlock {
        block_number: BlockNumber,
        block_hash: H256,
    },
}
```

**Required Runtime Extrinsics:**
```rust
submit_finality_header(chain_id: u64, header: Vec<u8>)

verify_receipt_proof(
    chain_id: u64,
    receipt_proof: Vec<u8>,
) → Result<DepositEvent, Error>

verify_x3_finality_proof(
    block_hash: H256,
    message_id: H256,
) → Result<TransferEvent, Error>
```

**Critical:** Do not mint on X3 just because a relayer says "trust me bro." Require proof.

---

### F. x3-message-bus
**Purpose:** Every transfer is a signed, verifiable message.

```rust
pub fn message_hash(
    source_domain: DomainId,
    destination_domain: DomainId,
    asset_id: AssetId,
    sender: AccountBytes,
    recipient: AccountBytes,
    amount: Balance,
    nonce: u128,
    expiry_block: BlockNumber,
) -> H256 {
    let mut data = Vec::new();
    data.extend_from_slice(b"X3_CROSS_DOMAIN_TRANSFER_V1");
    data.extend_from_slice(&source_domain.encode());
    data.extend_from_slice(&destination_domain.encode());
    data.extend_from_slice(&asset_id.encode());
    data.extend_from_slice(&sender);
    data.extend_from_slice(&recipient);
    data.extend_from_slice(&amount.encode());
    data.extend_from_slice(&nonce.encode());
    data.extend_from_slice(&expiry_block.encode());
    
    sp_io::hashing::blake2_256(&data).into()
}
```

Every message must be domain-separated to prevent accidental replay between different transfer types.

---

## 3. CROSS-VM TOKEN TRANSFER FLOW

**Example: X3 EVM USDC → X3 SVM USDC**

```
1. User calls ERC20 adapter on X3 EVM
   ├─ balanceOf[user] >= amount ✓
   └─ allowance[user][adapter] >= amount ✓

2. ERC20 adapter burns user's EVM-side balance
   └─ emit Transfer(user, address(0), amount)

3. Adapter emits CrossVmTransferRequested
   └─ (asset_id, source_domain, destination_domain, recipient, amount)

4. X3 kernel receives the VM event
   ├─ route allowed? ✓
   ├─ asset active? ✓
   ├─ nonce unused? ✓
   ├─ amount > 0? ✓
   ├─ amount <= per-tx limit? ✓
   ├─ within daily limit? ✓
   └─ supply invariant holds after debit? ✓

5. Kernel creates X3TransferMessage with message_id

6. Kernel marks transfer status: SourceDebited

7. X3 SVM adapter receives transfer message
   └─ checks message signature/proof

8. Kernel mints or credits recipient in SVM
   ├─ update SVM-side balance
   ├─ update supply ledger
   └─ assert_supply_invariant() ✓

9. Mark transfer status: Finalized

10. BOTH VMs atomically debited + credited in same X3 block
    └─ This is the crown jewel. Normal bridges cannot do this.
```

**This achieves:** Atomic cross-VM transfers. No pending forever. No liquidity problems.

---

## 4. CROSS-CHAIN TOKEN TRANSFER FLOW

### Example: Ethereum USDC → X3 EVM

```
1. User deposits USDC into Ethereum Gateway contract
   └─ USDC.transferFrom(user, gateway, amount)

2. Gateway locks USDC and emits DepositLocked event
   └─ event DepositLocked(messageId, token, sender, x3Recipient, amount, nonce)

3. Relayer watches Ethereum (via eth_logs or block indexer)
   ├─ waits 64 block confirmations
   └─ builds DepositProof

4. Relayer submits receipt proof to X3.submit_deposit_proof()
   └─ includes block header, receipt inclusion proof, log index

5. X3 finality oracle verifies proof
   ├─ checks block exists and finalized
   ├─ verifies receipt is in block
   ├─ verifies DepositLocked was emitted
   └─ confirms 64+ confirmations

6. X3 asset registry maps external ERC20 to X3 AssetId
   └─ asset_id = hash("X3_ASSET_ID_V1", Ethereum, 1, 0xA0b86991...)

7. X3 token vault increments external_locked_supply
   └─ evm_vm_supply += amount (mint to recipient)

8. X3 EVM ERC20 adapter mints wrapped USDC to recipient
   └─ balanceOf[recipient] += amount

9. User receives X3-side token
   └─ can now move freely within X3 Native/EVM/SVM
```

### Example: X3 EVM → Ethereum USDC

```
1. User burns X3 wrapped ERC20
   └─ X3VmERC20.sendToVm(destination_vm=Ethereum, recipient, amount)

2. ERC20 adapter burns user's balance
   └─ balanceOf[user] -= amount

3. Adapter emits CrossChainWithdrawalRequested
   └─ (asset_id, destination_chain, recipient, amount)

4. X3 kernel receives event
   ├─ asset active? ✓
   ├─ route allowed? ✓
   ├─ amount <= locked collateral? ✓
   └─ supply invariant? ✓

5. X3 creates withdrawal message with proof
   └─ includes X3 block hash, finality proof

6. Relayer submits X3 finalized proof to Ethereum Gateway
   ├─ includes X3 block header
   ├─ includes message inclusion proof
   └─ includes quorum signature if using validator attestations

7. Ethereum Gateway verifies proof
   ├─ checks X3 block is finalized
   ├─ verifies message is in block
   ├─ checks signature (if validator quorum)
   └─ confirms not replayed

8. Gateway releases USDC to recipient
   └─ USDC.transfer(recipient, amount)

9. User receives Ethereum USDC
   └─ back on external chain
```

---

## 5. TOKEN REPRESENTATION STRATEGY

### Native X3 Asset (runtime pallet)

Handled by standard balance pallet with asset-aware trait:

```rust
pub trait TokenLedger {
    fn mint(asset_id: AssetId, who: AccountId, amount: Balance) -> DispatchResult;
    fn burn(asset_id: AssetId, who: AccountId, amount: Balance) -> DispatchResult;
    fn transfer(asset_id: AssetId, from: AccountId, to: AccountId, amount: Balance) -> DispatchResult;
    fn balance(asset_id: AssetId, who: &AccountId) -> Balance;
}
```

### X3 EVM ERC20 Adapter

Every registered asset can have an EVM ERC20 mirror:

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

interface IX3Kernel {
    function requestCrossVmTransfer(
        bytes32 assetId,
        uint8 destinationVm,
        bytes calldata recipient,
        uint256 amount
    ) external;
}

contract X3VmERC20 {
    string public name;
    string public symbol;
    uint8 public immutable decimals;
    bytes32 public immutable assetId;
    address public immutable kernel;
    
    mapping(address => uint256) public balanceOf;
    mapping(address => mapping(address => uint256)) public allowance;
    
    event Transfer(address indexed from, address indexed to, uint256 amount);
    event CrossVmTransferRequested(
        bytes32 indexed assetId,
        address indexed sender,
        uint8 destinationVm,
        bytes recipient,
        uint256 amount
    );
    
    modifier onlyKernel() {
        require(msg.sender == kernel, "ONLY_KERNEL");
        _;
    }
    
    constructor(
        string memory _name,
        string memory _symbol,
        uint8 _decimals,
        bytes32 _assetId,
        address _kernel
    ) {
        name = _name;
        symbol = _symbol;
        decimals = _decimals;
        assetId = _assetId;
        kernel = _kernel;
    }
    
    function approve(address spender, uint256 amount) external returns (bool) {
        allowance[msg.sender][spender] = amount;
        return true;
    }
    
    function transfer(address to, uint256 amount) external returns (bool) {
        _transfer(msg.sender, to, amount);
        return true;
    }
    
    function transferFrom(address from, address to, uint256 amount) external returns (bool) {
        uint256 allowed = allowance[from][msg.sender];
        require(allowed >= amount, "ALLOWANCE");
        allowance[from][msg.sender] = allowed - amount;
        _transfer(from, to, amount);
        return true;
    }
    
    function kernelMint(address to, uint256 amount) external onlyKernel {
        balanceOf[to] += amount;
        emit Transfer(address(0), to, amount);
    }
    
    function kernelBurn(address from, uint256 amount) external onlyKernel {
        require(balanceOf[from] >= amount, "BALANCE");
        balanceOf[from] -= amount;
        emit Transfer(from, address(0), amount);
    }
    
    function sendToVm(uint8 destinationVm, bytes calldata recipient, uint256 amount) external {
        require(balanceOf[msg.sender] >= amount, "BALANCE");
        balanceOf[msg.sender] -= amount;
        emit Transfer(msg.sender, address(0), amount);
        
        IX3Kernel(kernel).requestCrossVmTransfer(
            assetId,
            destinationVm,
            recipient,
            amount
        );
        
        emit CrossVmTransferRequested(
            assetId,
            msg.sender,
            destinationVm,
            recipient,
            amount
        );
    }
    
    function _transfer(address from, address to, uint256 amount) internal {
        require(to != address(0), "ZERO_TO");
        require(balanceOf[from] >= amount, "BALANCE");
        balanceOf[from] -= amount;
        balanceOf[to] += amount;
        emit Transfer(from, to, amount);
    }
}
```

### X3 SVM Token Adapter

SVM-side token adapter with kernel-only mint/burn:

```rust
// programs/x3_svm_token_adapter/src/lib.rs

use solana_program::{
    account_info::{AccountInfo, next_account_info},
    entrypoint,
    msg,
    pubkey::Pubkey,
    program_error::ProgramError,
};

pub enum X3TokenInstruction {
    Transfer {
        asset_id: [u8; 32],
        to: Pubkey,
        amount: u128,
    },
    SendToVm {
        asset_id: [u8; 32],
        destination_vm: u8,
        recipient: Vec<u8>,
        amount: u128,
    },
    KernelMint {
        asset_id: [u8; 32],
        to: Pubkey,
        amount: u128,
    },
    KernelBurn {
        asset_id: [u8; 32],
        from: Pubkey,
        amount: u128,
    },
}

pub struct X3TokenAccount {
    pub asset_id: [u8; 32],
    pub owner: Pubkey,
    pub balance: u128,
}

// Only X3 kernel allowed to call:
// - KernelMint
// - KernelBurn
// - KernelCredit
// - KernelDebit

// No normal user should ever directly mint VM representations.
```

---

## 6. ASSET ID GENERATION (DETERMINISTIC)

```rust
pub fn asset_id_from_origin(
    origin_domain: DomainId,
    origin_chain_id: Option<u64>,
    origin_asset_address: &[u8],
    symbol: &str,
) -> AssetId {
    let mut data = Vec::new();
    data.extend_from_slice(b"X3_ASSET_ID_V1");
    data.extend_from_slice(&origin_domain.encode());
    if let Some(chain_id) = origin_chain_id {
        data.extend_from_slice(&chain_id.to_le_bytes());
    }
    data.extend_from_slice(origin_asset_address);
    data.extend_from_slice(symbol.as_bytes());
    
    sp_io::hashing::blake2_256(&data).into()
}
```

**Examples:**

```
X3 native token:
  asset_id = hash("X3_ASSET_ID_V1" || X3Native || "X3")

Ethereum USDC (mainnet, chain_id=1):
  asset_id = hash("X3_ASSET_ID_V1" || Ethereum || 1 || 0xA0b86991c6218b36c1d19d4a2e9eb0ce3606eb48 || "USDC")

Base USDC (chain_id=8453):
  asset_id = hash("X3_ASSET_ID_V1" || Base || 8453 || 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913 || "USDC")

Solana USDC (mint: EPjFWaLb3oCRY46oRFkceVrignGZ1tm6zNsoeP3xgSA):
  asset_id = hash("X3_ASSET_ID_V1" || Solana || EPjFWaLb3oCRY46oRFkceVrignGZ1tm6zNsoeP3xgSA || "USDC")
```

**Do not identify assets by symbol alone.** USDC is not an identity. It is a label.

---

## 7. ROUTE TABLE CONFIG

```rust
pub struct RouteConfig {
    pub route_id: RouteId,
    pub asset_id: AssetId,
    pub source_domain: DomainId,
    pub destination_domain: DomainId,
    pub enabled: bool,
    pub min_amount: Balance,
    pub max_amount: Balance,
    pub daily_limit: Balance,
    pub fee_bps: u16,
    pub finality_requirement: FinalityRequirement,
}
```

**Example configs:**

```
Route: Ethereum USDC → X3 EVM USDC
  finality: 64 Ethereum blocks (~15 mins)
  fee: 10 bps
  max_tx: 50,000 USDC
  daily_limit: 500,000 USDC
  status: active

Route: X3 EVM USDC → X3 SVM USDC
  finality: 12 X3 blocks (~36 secs)
  fee: 0 bps (internal)
  max_tx: unlimited (within supply)
  daily_limit: unlimited
  status: active

Route: X3 Native X3 → X3 EVM X3
  finality: 1 X3 block (~3 secs)
  fee: 0 bps (internal)
  max_tx: unlimited
  daily_limit: unlimited
  status: active
```

---

## 8. REQUIRED VALIDATION RULES

Every transfer must validate:

```rust
fn validate_transfer(msg: &X3TransferMessage) -> Result<(), Error> {
    // Asset exists
    ensure!(AssetRegistry::contains_key(&msg.asset_id), Error::UnknownAsset);
    
    // Asset is active
    ensure!(
        AssetStatus::get(&msg.asset_id) == AssetStatus::Active,
        Error::AssetPaused
    );
    
    // Route is allowed
    ensure!(
        AllowedRoutes::get((&msg.asset_id, (&msg.source_domain, &msg.destination_domain))),
        Error::RouteDisabled
    );
    
    // Amount is positive
    ensure!(msg.amount > 0, Error::ZeroAmount);
    
    // Not a replay
    ensure!(!UsedMessages::contains_key(&msg.message_id), Error::Replay);
    
    // Not expired
    ensure!(current_block() <= msg.expiry_block, Error::Expired);
    
    // Within limits
    let limits = AssetLimits::get(&msg.asset_id);
    ensure!(msg.amount <= limits.per_tx_limit, Error::TxLimitExceeded);
    ensure!(daily_volume + msg.amount <= limits.daily_limit, Error::DailyLimitExceeded);
    
    Ok(())
}
```

---

## 9. EXTERNAL GATEWAY CONTRACT (EVM)

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

interface IERC20 {
    function transferFrom(address from, address to, uint256 amount) external returns (bool);
    function transfer(address to, uint256 amount) external returns (bool);
}

contract X3ExternalGateway {
    address public owner;
    mapping(bytes32 => bool) public usedMessages;
    mapping(address => bool) public supportedTokens;
    
    event DepositLocked(
        bytes32 indexed messageId,
        address indexed token,
        address indexed sender,
        bytes x3Recipient,
        uint256 amount,
        uint256 nonce
    );
    
    event WithdrawalReleased(
        bytes32 indexed messageId,
        address indexed token,
        address indexed recipient,
        uint256 amount
    );
    
    modifier onlyOwner() {
        require(msg.sender == owner, "ONLY_OWNER");
        _;
    }
    
    constructor() {
        owner = msg.sender;
    }
    
    function setSupportedToken(address token, bool enabled) external onlyOwner {
        supportedTokens[token] = enabled;
    }
    
    function depositToX3(
        address token,
        bytes calldata x3Recipient,
        uint256 amount,
        uint256 nonce
    ) external {
        require(supportedTokens[token], "TOKEN_NOT_SUPPORTED");
        require(amount > 0, "ZERO_AMOUNT");
        
        bytes32 messageId = keccak256(
            abi.encodePacked(
                "X3_DEPOSIT_V1",
                block.chainid,
                token,
                msg.sender,
                x3Recipient,
                amount,
                nonce
            )
        );
        
        require(!usedMessages[messageId], "REPLAY");
        usedMessages[messageId] = true;
        
        require(IERC20(token).transferFrom(msg.sender, address(this), amount), "TRANSFER_FAIL");
        
        emit DepositLocked(
            messageId,
            token,
            msg.sender,
            x3Recipient,
            amount,
            nonce
        );
    }
    
    function releaseFromX3(
        bytes32 messageId,
        address token,
        address recipient,
        uint256 amount,
        bytes calldata x3Proof
    ) external {
        require(!usedMessages[messageId], "REPLAY");
        require(supportedTokens[token], "TOKEN_NOT_SUPPORTED");
        require(_verifyX3Proof(messageId, token, recipient, amount, x3Proof), "BAD_PROOF");
        
        usedMessages[messageId] = true;
        require(IERC20(token).transfer(recipient, amount), "TRANSFER_FAIL");
        
        emit WithdrawalReleased(messageId, token, recipient, amount);
    }
    
    function _verifyX3Proof(
        bytes32 messageId,
        address token,
        address recipient,
        uint256 amount,
        bytes calldata proof
    ) internal view returns (bool) {
        // TODO: Before mainnet, implement real verifier:
        // 1. Verify X3 finalized block/header
        // 2. Verify message inclusion proof
        // 3. Verify route and asset mapping
        // 4. Verify quorum/threshold signature if using validator attestations
        
        // TESTNET ONLY: Basic sanity checks
        return proof.length > 0 && messageId != bytes32(0) && 
               token != address(0) && recipient != address(0) && amount > 0;
    }
}
```

**Critical:** The `_verifyX3Proof()` stub must become a real verifier before mainnet. Until then, this is testnet-only.

---

## 10. BUILD PHASES

### Phase 1: Cross-VM (Testnet, Weeks 1-2)

**Goal:** X3 Native ↔ X3 EVM ↔ X3 SVM atomic transfers

1. x3-asset-registry
2. x3-token-vault (with supply invariant)
3. x3-cross-vm-router
4. X3VmERC20.sol
5. X3 SVM adapter
6. Cross-VM tests (EVM→SVM, SVM→EVM, Native→EVM, etc.)

**Success Criteria:**
- Round-trip transfers preserve supply
- Replay fails
- Expired messages fail
- Paused assets fail
- Disabled routes fail

### Phase 2: External EVM Chains (Weeks 3-4)

**Goal:** Ethereum/Base/Arbitrum ERC20 ↔ X3

1. X3ExternalGateway.sol deployment
2. EVM receipt relayer
3. x3-finality-oracle (EVM proof verifier)
4. x3-crosschain-gateway
5. Deposit/withdrawal proofs
6. Cross-chain tests

**Success Criteria:**
- Ethereum USDC → X3 EVM USDC
- X3 EVM USDC → Ethereum USDC
- Supply invariant holds
- Finality blocks required
- Double-proof rejected

### Phase 3: Solana/SVM External (Weeks 5-6)

**Goal:** Solana SPL-style token ↔ X3

1. Solana token lock program
2. Solana event watcher
3. x3-finality-oracle (Solana finalization verifier)
4. Solana proof submission
5. Integration tests

### Phase 4: BTC (Post-Mainnet)

BTC requires special handling (federated vault, DLC, or threshold multisig).
Not for initial launch.

---

## 11. SECURITY REQUIREMENTS (BEFORE MAINNET)

| Area | Rule |
|------|------|
| **Asset Identity** | No symbol-only mapping. Use blake2_256(origin_domain \|\| origin_chain \|\| address \|\| symbol) |
| **Replay Protection** | Use StorageMap<H256, bool> for message IDs, not Vec scanning |
| **Finality** | Never mint from unfinalized external events. Require finality threshold. |
| **Admin Keys** | Multisig or governance only for critical functions |
| **Limits** | Per-route tx limits and daily limits enforced |
| **Emergency** | Pause asset and pause route separately |
| **Proofs** | Verify source chain, asset, amount, recipient, nonce before crediting |
| **Supply** | Check invariant after every state transition |
| **Expiry** | Every pending transfer must expire. No pending forever. |
| **Cleanup** | Dead sessions must be removable by governance |
| **Logging** | Every transfer is traceable by message_id |

---

## 12. TESTING STRATEGY

### Cross-VM Tests
- Native → EVM → SVM → Native round trip
- Replay protection (same message twice)
- Expired message failure
- Paused asset failure
- Disabled route failure
- Supply invariant maintenance

### Cross-Chain Tests
- Ethereum deposit → X3 mint
- X3 burn → Ethereum release
- Double proof rejected
- Wrong asset address fails
- Wrong recipient fails
- Wrong amount fails
- Wrong chain ID fails
- Unfinalized proof fails
- Expired withdrawal fails

### Fuzz Tests
Fuzz: amount, nonce, recipient encoding, route ID, asset ID, proof bytes, expiry block, domain ID

**Invariant:** No sequence of valid or invalid calls may increase total supply beyond collateral/canonical supply.

### Property Tests
- Every finalized transfer is immutable
- Every burned representation decreases supply exactly
- Every minted representation increases supply exactly
- Sum of all VM supplies ≤ canonical supply
- Every transfer message has unique message_id
- Replay protection is unbypassable

---

## 13. X3 ASSET ID REGISTRY (PHASE 13F MAINNET)

### Pre-Registered Assets

```
X3:USDC
  origin: Ethereum (chain_id=1)
  token: 0xA0b86991c6218b36c1d19d4a2e9eb0ce3606eb48
  decimals: 6
  representations: X3-Native, X3-EVM, X3-SVM
  routes: ETH→X3, X3→ETH, X3-EVM↔X3-SVM, etc.
  status: active
  daily_limit: 1,000,000 USDC

X3:USDT
  origin: Ethereum (chain_id=1)
  token: 0xdAC17F958D2ee523a2206206994597C13D831ec7
  decimals: 6
  representations: X3-Native, X3-EVM, X3-SVM
  routes: ETH→X3, X3→ETH, X3-EVM↔X3-SVM
  status: active
  daily_limit: 500,000 USDT

X3:X3 (Native)
  origin: X3Native
  decimals: 12
  representations: X3-Native, X3-EVM, X3-SVM
  routes: X3-Native↔X3-EVM, X3-Native↔X3-SVM, X3-EVM↔X3-SVM
  status: active
  supply_policy: NativeMintBurn
  daily_limit: unlimited (governance vote)

X3:SOL
  origin: Solana
  token: So11111111111111111111111111111111111111112 (wrapped SOL)
  decimals: 9
  representations: X3-Native, X3-EVM, X3-SVM
  routes: Solana→X3, X3→Solana, X3-EVM↔X3-SVM
  status: active
  daily_limit: 100,000 SOL
```

---

## 14. MAINNET LAUNCH CHECKLIST (APRIL 19 - MAY 19)

### T-30 Days (April 19)
- [ ] All 6 modules code-complete and tested
- [ ] External gateway contracts audited
- [ ] Finality oracle verifiers production-ready
- [ ] Relayer fully operational

### T-14 Days (May 5)
- [ ] Testnet round-trip tests 100% pass
- [ ] Mainnet configuration validated
- [ ] Validator nodes configured with token system
- [ ] Exchange integration documents complete

### T-7 Days (May 12)
- [ ] Stakeholder approval obtained
- [ ] VP Engineering sign-off
- [ ] Emergency pause procedures tested
- [ ] Incident response team trained

### T-2 Days (May 17)
- [ ] Mainnet gateway contracts deployed
- [ ] RPC nodes synced and ready
- [ ] Relayers staged but not running
- [ ] Final verification of all limits

### T-0 (May 19)
- [ ] Relayer service starts
- [ ] First cross-VM transfer executed
- [ ] First external deposit processed
- [ ] Monitoring alerts active

---

## 15. COMPETITIVE ADVANTAGE

What X3 is building is **not just a bridge.**

**X3 Universal Asset Kernel**

Core claim:
- Any asset can move between X3 Native, X3 EVM, X3 SVM, and external chains through one canonical asset registry
- One supply ledger
- One finality-aware settlement kernel

Real value:
- **Cross-VM transfers inside X3 can be atomic.** No liquidity problems.
- **External cross-chain transfers can be finality-verified.** No "trust me bro."
- **Assets never fragment supply.** One canonical record.
- **Developers deploy once, expose to EVM, SVM, and native users.** True multi-VM ecosystem.

That is the strong version. Build the cross-VM atomic token layer first, then bolt on external chains.

Cross-VM is where X3 can actually beat normal chains. Cross-chain is where you need to be paranoid, slow, and proof-heavy.

---

**Status:** Architecture locked for Phase 13f execution  
**Next:** Exchange integration document + validator token operations guide  
**Critical Path:** Phase 1 (cross-VM) must complete before external chains

