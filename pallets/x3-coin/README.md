# X3 Coin Pallet

The X3 Coin pallet manages the canonical X3 token on the X3 Chain, providing cross-VM token operations, vesting schedules, and bonus pool management.

## Features

- **Genesis Issuance**: Initial token distribution with configurable allocations
- **Cross-Chain Operations**: Mint, burn, and transfer operations across EVM, SVM, and BTC chains
- **Vesting Schedules**: Team allocation with configurable vesting periods and cliffs
- **Bonus Pool**: Community rewards with claim limits and periods
- **Replay Protection**: Cryptographic proof validation and operation deduplication
- **Runtime API**: Query interfaces for total supply, balances, and vesting information

## Tokenomics

- **Total Supply**: 2,000,000,000 X3 (2 billion)
- **Decimals**: 12
- **Symbol**: X3
- **Asset ID**: 0 (canonical)

### Allocation Breakdown

- **Treasury**: 20% (400M X3) - Protocol development and operations
- **Team & Advisors**: 15% (300M X3) - 1-year vesting with 6-month cliff
- **Ecosystem**: 25% (500M X3) - Immediate distribution
- **Liquidity**: 30% (600M X3) - Exchange listings and liquidity provision
- **Bonus Pool**: 10% (200M X3) - Community rewards

## Configuration

```rust
impl pallet_x3_coin::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type Balance = Balance;
    type UnixTime = Timestamp;
    type WeightInfo = ();
    type TreasuryAccount = TreasuryAccount;
    type MaxBonusClaims = ConstU32<10>;
    type TeamVestingBlocks = ConstU64<15_768_000>; // ~1 year
    type TeamVestingCliff = ConstU64<7_884_000>;   // ~6 months
    type BonusClaimPeriod = ConstU64<3_942_000>;  // ~3 months
}
```

## Cross-Chain Operations

### Proof Types

- **EVM Proof**: Ethereum transaction hash, block number, and proof data
- **SVM Proof**: Solana transaction signature, block number, and proof data
- **BTC Proof**: Bitcoin transaction ID, block height, and merkle proof

### Operation Types

1. **Mint**: Create X3 tokens from external chain deposits
2. **Burn**: Destroy X3 tokens for external chain withdrawals
3. **Transfer**: Move tokens between chains

### Replay Protection

Each operation is assigned a unique ID based on:
- Target/source account
- Amount
- Proof data

Operation IDs are stored to prevent replay attacks.

## Vesting Schedules

Team allocations use a linear vesting schedule:

- **Start Block**: When vesting begins
- **Cliff Blocks**: Period before first claim (6 months)
- **Vesting Blocks**: Total vesting period (1 year)
- **Claimed**: Amount already claimed

Vested amount calculation:
```
if elapsed_blocks >= vesting_blocks:
    vested = total_amount
else:
    vested = total_amount * elapsed_blocks / vesting_blocks
```

## Bonus Pool

Community rewards system:

- **Pool Size**: 10% of total supply
- **Claim Limit**: 10 claims per account
- **Claim Period**: 3 months between claims
- **Claim Amount**: 10% of remaining pool per claim

## Runtime API

```rust
// Get total X3 supply
fn get_total_supply() -> Balance;

// Get treasury balance
fn get_treasury_balance() -> Balance;

// Get bonus pool balance
fn get_bonus_pool_balance() -> Balance;

// Get vested amount for account
fn get_vested_amount(account: AccountId) -> Balance;

// Get total bonus claims for account
fn get_total_bonus_claims(account: AccountId) -> Balance;

// Get team vesting schedule
fn get_team_vesting(account: AccountId) -> Option<(Balance, Balance, u64, u64, u64)>;

// Get bonus claims history
fn get_bonus_claims(account: AccountId) -> Vec<(Balance, u64, bool)>;
```

## Testing

The pallet includes comprehensive tests:

- Genesis configuration validation
- Vesting schedule operations
- Bonus pool claims
- Cross-chain operations
- Replay protection
- Serialization and invariants
- Integration with X3 Kernel
- Stress testing

Run tests with:
```bash
cargo test -p pallet-x3-coin
```

## Security Considerations

- All cross-chain operations require proof validation
- Replay protection prevents duplicate operations
- Treasury balance checks prevent over-minting
- Vesting schedules prevent premature team token access
- Bonus pool limits prevent abuse

## Integration

The X3 Coin pallet integrates with:

- **X3 Kernel**: For canonical balance management
- **X3 Bridge**: For cross-chain message routing
- **X3 Court**: For dispute resolution
- **X3 Swap Router**: For liquidity management

## Future Enhancements

- EVM mirror token contract implementation
- SVM mirror program implementation
- BTC HTLC script templates
- Relayer incentive mechanisms
- Advanced vesting options
- Multi-signature treasury operations