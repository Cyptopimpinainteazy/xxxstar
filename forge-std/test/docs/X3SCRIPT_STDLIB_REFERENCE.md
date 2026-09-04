# X3Script Standard Library Reference

> **Status**: Canonical | **Version**: 1.0.0 | **Last Updated**: 2025-12-10

The X3Script Standard Library (`x3_std`) provides production-ready primitives for building cross-VM DeFi contracts, AI agents, flashloan strategies, and cross-chain vaults.

---

## Table of Contents

1. [Overview](#1-overview)
2. [x3_std::core](#2-x3_stdcore)
3. [x3_std::token](#3-x3_stdtoken)
4. [x3_std::dex](#4-x3_stddex)
5. [x3_std::flashloan](#5-x3_stdflashloan)
6. [x3_std::oracle](#6-x3_stdoracle)
7. [x3_std::vault](#7-x3_stdvault)
8. [x3_std::bridge](#8-x3_stdbridge)
9. [x3_std::agent](#9-x3_stdagent)
10. [x3_std::ai](#10-x3_stdai)
11. [x3_std::zk](#11-x3_stdzk)
12. [x3_std::safety](#12-x3_stdsafety)
13. [x3_std::devtools](#13-x3_stddevtools)
14. [Complete Example: FlashArb](#14-complete-example-flasharb)
15. [Integration Guide](#15-integration-guide)

---

## 1. Overview

### 1.1 Module Map

```
stdlib/
├── core.x3        # Primitives: events, errors, logging, math
├── token.x3       # ERC20/SPL-compatible token operations
├── dex.x3         # Router + aggregator primitives
├── flashloan.x3   # Generic flashloan adapters
├── oracle.x3      # Price feeds, TWAP, Chainlink abstraction
├── vault.x3       # Yield vaults, governors
├── bridge.x3      # Cross-chain messaging
├── agent.x3       # Agent toolkit
├── ai_helpers.x3  # AI annotations, model hints
├── zk.x3          # ZK-friendly primitives
├── safety.x3      # Reentrancy, slippage, signatures
└── devtools.x3    # Debug, profiling, simulation
```

### 1.2 Design Principles

| Principle           | Description                                    |
| ------------------- | ---------------------------------------------- |
| **Cross-VM Native** | Every module works on both EVM and SVM         |
| **Gas Annotated**   | All functions have `@vm.hint` gas metadata     |
| **AI-Safe**         | Sidecar operations produce verifiable receipts |
| **Battle-Tested**   | Patterns from Aave, Uniswap, production DeFi   |

### 1.3 Import Convention

```x3script
import x3_std::core::*;
import x3_std::dex::{swap_exact_in, find_best_route};
import x3_std::flashloan as fl;
```

---

## 2. x3_std::core

**Purpose:** Basic types, events, helpers, math primitives.

### 2.1 Events & Errors

```x3script
// Emit a log event
event Log(topic: string, data: bytes);

// Revert with reason
error Revert(reason: string);

// Panic helper
fn panic(msg: string) -> never;
```

### 2.2 Assertions

```x3script
// Require condition or panic
@vm.hint("cheap")
fn require(cond: bool, msg: string) {
    if !cond { panic(msg) }
}

// Debug assertion (removed in release)
@vm.hint("debug_only")
fn debug_assert(cond: bool, msg: string);
```

### 2.3 Math Helpers

```x3script
@vm.hint("cheap")
fn min(a: u128, b: u128) -> u128 {
    if a < b { a } else { b }
}

@vm.hint("cheap")
fn max(a: u128, b: u128) -> u128 {
    if a > b { a } else { b }
}

@vm.hint("cheap")
fn abs_diff(a: u128, b: u128) -> u128 {
    if a > b { a - b } else { b - a }
}

// Overflow-safe arithmetic
@vm.hint("cheap")
fn safe_add(a: u128, b: u128) -> u128;  // traps on overflow

@vm.hint("cheap")
fn safe_sub(a: u128, b: u128) -> u128;  // traps on underflow

@vm.hint("cheap")
fn safe_mul(a: u128, b: u128) -> u128;  // traps on overflow

// Percentage calculations (basis points)
@vm.hint("cheap")
fn bps(amount: u128, basis_points: u32) -> u128 {
    amount * basis_points / 10000
}

// Fixed-point multiplication (18 decimals)
@vm.hint("cheap")
fn mul_wad(a: u128, b: u128) -> u128 {
    a * b / 1e18
}

// Fixed-point division (18 decimals)
@vm.hint("cheap")
fn div_wad(a: u128, b: u128) -> u128 {
    a * 1e18 / b
}
```

### 2.4 Gas Hints

```x3script
// Attach gas metadata to operation
fn gas_hint(kind: string);

// Hint kinds:
// "cheap"          - 1-3 gas
// "moderate"       - 10-100 gas
// "expensive"      - 100-1000 gas
// "very_expensive" - 1000+ gas (cross-VM, storage)
```

---

## 3. x3_std::token

**Purpose:** Unified ERC20/SPL token operations across EVM and SVM.

### 3.1 Types

```x3script
type Token = struct {
    symbol: string,
    decimals: u8,
    evm_addr: Option<address>,    // EVM contract address
    svm_account: Option<pubkey>,  // SVM token mint
}

type TokenBalance = struct {
    token: Token,
    amount: u128,
    owner: address,
}
```

### 3.2 Core Operations

```x3script
// Get balance for owner
@vm.hint("moderate")
fn balance_of(token: Token, owner: address) -> u128;

// Transfer tokens
@vm.hint("expensive")
fn transfer(token: Token, to: address, amount: u128) -> bool;

// Transfer from (requires approval)
@vm.hint("expensive")
fn transfer_from(
    token: Token, 
    from: address, 
    to: address, 
    amount: u128
) -> bool;

// Approve spender
@vm.hint("expensive")
fn approve(token: Token, spender: address, amount: u128) -> bool;

// Get allowance
@vm.hint("moderate")
fn allowance(token: Token, owner: address, spender: address) -> u128;
```

### 3.3 Safe Transfer Helpers

```x3script
// Transfer with return value check
@ai.hint("minimize_gas")
@vm.hint("expensive")
fn safe_transfer(token: Token, to: address, amount: u128) -> bool {
    let success = transfer(token, to, amount);
    require(success, "transfer failed");
    return true;
}

// Transfer from with return value check
@vm.hint("expensive")
fn safe_transfer_from(
    token: Token,
    from: address,
    to: address,
    amount: u128
) -> bool {
    let success = transfer_from(token, from, to, amount);
    require(success, "transfer_from failed");
    return true;
}
```

### 3.4 Cross-VM Behavior

```x3script
// Token operations automatically route based on token type:
//
// If token.evm_addr is Some:
//   - Uses evm.call / evm.sstore paths
//   - Follows ERC20 interface
//
// If token.svm_account is Some:
//   - Uses svm.cpi for SPL token program
//   - Follows SPL Token interface
//
// If both present:
//   - Prefers native VM of current execution context
//   - Cross-VM transfers use bridge module
```

---

## 4. x3_std::dex

**Purpose:** DEX router, aggregator, and swap primitives.

### 4.1 Types

```x3script
type Route = struct {
    path: array[Token],      // Token sequence
    fees: array[u32],        // Fee tiers (e.g., 3000 = 0.3%)
    venues: array[string],   // DEX names per hop
}

type Quote = struct {
    amount_out: u128,
    price_impact: u32,       // Basis points
    gas_estimate: u64,
}

type SwapResult = struct {
    amount_in: u128,
    amount_out: u128,
    route: Route,
}
```

### 4.2 Quoting

```x3script
// Get quote for a route
@vm.hint("moderate")
fn quote_swap(route: Route, amount_in: u128) -> Quote;

// Get quote for direct pair
@vm.hint("moderate")
fn quote_direct(
    token_in: Token,
    token_out: Token,
    amount_in: u128
) -> Quote;

// Find best route across venues
@vm.hint("expensive")
fn find_best_route(
    token_in: Token,
    token_out: Token,
    amount_in: u128
) -> Route;

// Find candidate routes (for AI ranking)
@vm.hint("expensive")
fn find_candidate_routes(
    token_in: Token,
    token_out: Token,
    amount_in: u128,
    max_routes: u32
) -> array[Route];
```

### 4.3 Execution

```x3script
// Swap exact input amount
@vm.hint("very_expensive")
fn swap_exact_in(
    route: Route,
    amount_in: u128,
    min_out: u128,
    recipient: address
) -> u128;  // Returns actual output

// Swap for exact output amount
@vm.hint("very_expensive")
fn swap_exact_out(
    route: Route,
    amount_out: u128,
    max_in: u128,
    recipient: address
) -> u128;  // Returns actual input used

// Batch multiple swaps atomically
@vm.hint("very_expensive")
fn multicall_swap(calls: array[SwapCall]) -> array[u128];
```

### 4.4 Price Utilities

```x3script
// Get spot price
@vm.hint("moderate")
fn get_price(token_in: Token, token_out: Token) -> u128;

// Get price with specified venue
@vm.hint("moderate")
fn get_price_at(
    venue: string,
    token_in: Token,
    token_out: Token
) -> u128;

// Check arbitrage opportunity
@vm.hint("moderate")
fn check_arb_opportunity(
    token: Token,
    venue_a: string,
    venue_b: string
) -> Option<(u128, Route)>;  // (profit, route)
```

### 4.5 Example Usage

```x3script
// Find and execute optimal swap
let route = dex::find_best_route(USDC, WETH, 1000 * 1e6);
let min_out = dex::quote_swap(route, 1000 * 1e6).amount_out * 995 / 1000;
let received = dex::swap_exact_in(route, 1000 * 1e6, min_out, self);
```

---

## 5. x3_std::flashloan

**Purpose:** Generic flashloan adapters supporting Aave and custom lenders.

### 5.1 Types

```x3script
type FlashProvider = enum {
    AaveV2(address),      // Aave V2 lending pool
    AaveV3(address),      // Aave V3 pool
    Balancer(address),    // Balancer vault
    Uniswap(address),     // Uniswap V3 flash
    Custom(address),      // Custom provider
}

type FlashLoanParams = struct {
    provider: FlashProvider,
    assets: array[Token],
    amounts: array[u128],
    receiver: address,
    data: bytes,
}

type FlashFee = struct {
    asset: Token,
    amount: u128,
    fee: u128,
}
```

### 5.2 Core Operations

```x3script
// Request a flashloan
@vm.hint("very_expensive")
fn request_flashloan(
    provider: FlashProvider,
    assets: array[Token],
    amounts: array[u128],
    receiver: address,
    data: bytes
);

// Compute fees for flashloan
@vm.hint("moderate")
fn compute_fee(
    provider: FlashProvider,
    amounts: array[u128]
) -> array[u128];

// Get provider address
@vm.hint("cheap")
fn provider_address(provider: FlashProvider) -> address;
```

### 5.3 Callback Interface

```x3script
// Implement this in your strategy
trait FlashLoanReceiver {
    // Called by the lending protocol
    fn on_flashloan_received(
        assets: array[Token],
        amounts: array[u128],
        fees: array[u128],
        initiator: address,
        params: bytes
    ) -> bool;
}
```

### 5.4 Verifier Rules

The compiler verifier ensures:

1. **Repayment Invariant**: All code paths must repay `principal + fee`
2. **Atomic Execution**: No external calls between loan receipt and repayment
3. **Balance Check**: Final balance >= initial balance + fee

```x3script
// Verifier annotation
@flashloan.verify_repayment
fn on_flashloan_received(...) {
    // Verifier statically analyzes all branches
    // to ensure repayment occurs
}
```

### 5.5 Example Pattern

```x3script
strategy FlashArb {
    fn execute(provider: FlashProvider, token: Token, amount: u128) {
        flashloan::request_flashloan(
            provider,
            [token],
            [amount],
            self,
            bytes("arb")
        );
    }

    fn on_flashloan_received(
        assets: array[Token],
        amounts: array[u128],
        fees: array[u128],
        initiator: address,
        params: bytes
    ) -> bool {
        let token = assets[0];
        let amount = amounts[0];
        let fee = fees[0];

        // Execute arbitrage
        let profit = execute_arb(token, amount);

        // Repay loan
        let repay_amount = amount + fee;
        token::approve(provider_address(provider), repay_amount);

        require(profit >= fee, "unprofitable");
        return true;
    }
}
```

---

## 6. x3_std::oracle

**Purpose:** Price feeds, TWAP, and oracle adapters.

### 6.1 Types

```x3script
type OracleHandle = struct {
    provider: OracleProvider,
    pair: (Token, Token),
}

type OracleProvider = enum {
    Chainlink(address),
    UniswapTWAP(address),
    Pyth(address),
    Custom(address),
}

type PriceData = struct {
    price: u128,           // Price in quote decimals
    timestamp: u64,        // Last update time
    confidence: u32,       // Confidence interval (bps)
}
```

### 6.2 Price Queries

```x3script
// Get current price
@vm.hint("moderate")
fn get_price(
    oracle: OracleHandle,
    base: Token,
    quote: Token
) -> PriceData;

// Get price with freshness check
@vm.hint("moderate")
fn get_price_safe(
    oracle: OracleHandle,
    base: Token,
    quote: Token,
    max_age: u64
) -> PriceData {
    let data = get_price(oracle, base, quote);
    require(
        system::timestamp() - data.timestamp <= max_age,
        "stale oracle"
    );
    return data;
}
```

### 6.3 TWAP Operations

```x3script
// Register TWAP observation
@vm.hint("expensive")
fn register_twap(
    oracle: OracleHandle,
    pair: (Token, Token),
    window: u32  // Seconds
);

// Get TWAP price
@vm.hint("moderate")
fn get_twap(
    oracle: OracleHandle,
    pair: (Token, Token),
    window: u32
) -> u128;

// Consult historical price
@vm.hint("moderate")
fn consult(
    oracle: OracleHandle,
    token: Token,
    amount: u128,
    period: u32
) -> u128;
```

### 6.4 Subscriptions

```x3script
// Subscribe to price updates
@vm.hint("expensive")
fn subscribe_feed(
    oracle: OracleHandle,
    pair: (Token, Token),
    callback_agent: address
);

// Unsubscribe
@vm.hint("moderate")
fn unsubscribe_feed(subscription_id: bytes32);
```

### 6.5 AI Integration

```x3script
// Predict price movement
@ai.hint("sidecar")
fn predict_price_delta(
    oracle: OracleHandle,
    pair: (Token, Token),
    horizon: u64  // Blocks ahead
) -> (prob, u128);  // (confidence, expected_price)
```

### 6.6 Safety Checks

```x3script
// Always validate oracle data before use
fn safe_get_price(oracle: OracleHandle, base: Token, quote: Token) -> u128 {
    let data = get_price(oracle, base, quote);
    
    // Freshness check
    require(
        system::timestamp() - data.timestamp <= 3600,
        "oracle stale"
    );
    
    // Sanity check
    require(data.price > 0, "invalid price");
    require(data.confidence < 500, "low confidence");  // < 5%
    
    return data.price;
}
```

---

## 7. x3_std::vault

**Purpose:** Yield vaults, pooled strategies, and governance.

### 7.1 Types

```x3script
type Vault = struct {
    strategy: address,
    asset: Token,
    total_shares: u128,
    total_assets: u128,
    fee_rate: u32,         // Basis points
    governance: address,
}

type VaultConfig = struct {
    max_deposit: u128,
    min_deposit: u128,
    lock_period: u64,
    emergency_shutdown: bool,
}
```

### 7.2 User Operations

```x3script
// Deposit assets, receive shares
@vm.hint("expensive")
fn deposit(vault: Vault, user: address, amount: u128) -> u128 {
    let shares = preview_deposit(vault, amount);
    // ... transfer and mint logic
    return shares;
}

// Withdraw assets by burning shares
@vm.hint("expensive")
fn withdraw(vault: Vault, user: address, shares: u128) -> u128 {
    let assets = preview_withdraw(vault, shares);
    // ... burn and transfer logic
    return assets;
}

// Preview deposit (view)
@vm.hint("cheap")
fn preview_deposit(vault: Vault, amount: u128) -> u128 {
    if vault.total_shares == 0 {
        return amount;
    }
    return amount * vault.total_shares / vault.total_assets;
}

// Preview withdraw (view)
@vm.hint("cheap")
fn preview_withdraw(vault: Vault, shares: u128) -> u128 {
    return shares * vault.total_assets / vault.total_shares;
}
```

### 7.3 Strategy Operations

```x3script
// Harvest yield from strategy
@vm.hint("very_expensive")
fn harvest(vault: Vault) -> u128 {
    // Call strategy to realize gains
    let profit = strategy::harvest(vault.strategy);
    vault.total_assets += profit;
    return profit;
}

// Rebalance vault positions
@vm.hint("very_expensive")
fn rebalance(vault: Vault, route: Route);

// Emergency withdraw all
@vm.hint("very_expensive")
fn emergency_withdraw(vault: Vault) {
    require(vault.config.emergency_shutdown, "not in emergency");
    // ... withdraw all from strategy
}
```

### 7.4 Governance

```x3script
// Propose strategy change
@vm.hint("expensive")
fn propose_strategy(
    vault: Vault,
    new_strategy: address,
    timelock: u64
) -> bytes32;

// Execute after timelock
@vm.hint("expensive")
fn execute_proposal(vault: Vault, proposal_id: bytes32);

// Emergency pause
@vm.hint("moderate")
fn pause(vault: Vault) {
    require(msg.sender == vault.governance, "not governance");
    vault.config.emergency_shutdown = true;
}
```

---

## 8. x3_std::bridge

**Purpose:** Cross-chain messaging and attestations.

### 8.1 Types

```x3script
type ChainId = u32;

type Message = struct {
    src_chain: ChainId,
    dst_chain: ChainId,
    sender: address,
    payload: bytes,
    nonce: u64,
}

type Proof = struct {
    message: Message,
    attestations: array[bytes],
    block_header: bytes,
}

type MsgId = bytes32;
```

### 8.2 Sending

```x3script
// Send cross-chain message
@vm.hint("very_expensive")
fn send_message(
    dst_chain: ChainId,
    payload: bytes
) -> MsgId;

// Send with specific adapter
@vm.hint("very_expensive")
fn send_via(
    adapter: BridgeAdapter,
    dst_chain: ChainId,
    payload: bytes
) -> MsgId;
```

### 8.3 Receiving

```x3script
// Receive and verify message
@vm.hint("very_expensive")
fn receive_message(
    src_chain: ChainId,
    proof: Proof,
    payload: bytes
) -> bool;

// Verify attestation
@vm.hint("expensive")
fn verify_attestation(proof: Proof) -> bool;

// Check if message processed
@vm.hint("moderate")
fn is_processed(msg_id: MsgId) -> bool;
```

### 8.4 Atomic Patterns

```x3script
// Pattern 1: Atomic window with receipts
atomic {
    // Local state update
    local_storage.locked = true;
    
    // Cross-chain message
    let msg_id = bridge::send_message(DST_CHAIN, payload);
    
    // Store receipt for verification
    receipts[msg_id] = AtomicReceipt {
        action: "cross_chain_transfer",
        timestamp: system::timestamp(),
    };
}

// Pattern 2: Two-phase commit
fn initiate_transfer(dst_chain: ChainId, amount: u128) {
    // Phase 1: Lock locally
    local_storage.pending[nonce] = PendingTransfer {
        amount: amount,
        dst_chain: dst_chain,
        expiry: system::timestamp() + TIMEOUT,
    };
    
    // Send message
    bridge::send_message(dst_chain, encode_transfer(amount, nonce));
}

fn finalize_transfer(proof: Proof) {
    // Phase 2: Confirm with proof
    require(bridge::verify_attestation(proof), "invalid proof");
    delete local_storage.pending[proof.message.nonce];
}
```

---

## 9. x3_std::agent

**Purpose:** Agent toolkit for AI-driven autonomous actors.

### 9.1 Types

```x3script
type Observation = struct {
    timestamp: u64,
    block: u64,
    data: map<string, bytes>,
    features: tensor<256>,
}

type Action = struct {
    name: string,
    params: bytes,
    confidence: prob,
}

type AgentState = struct {
    last_action: u64,
    total_actions: u64,
    cumulative_reward: i128,
}
```

### 9.2 Core Loop

```x3script
// Observe current state
@vm.hint("moderate")
fn observe(ctx: Context) -> Observation;

// Make decision based on observation
@ai.hint("sidecar")
fn decide(obs: Observation) -> Action;

// Execute action
@vm.hint("expensive")
fn act(action: Action) -> ActionResult;

// Main agent loop
fn run_loop(interval: u64) {
    loop {
        let obs = observe(get_context());
        let action = decide(obs);
        
        if action.confidence > CONFIDENCE_THRESHOLD {
            let result = act(action);
            update_state(result);
        }
        
        wait_blocks(interval);
    }
}
```

### 9.3 Backtesting

```x3script
// Run backtest on historical data
@vm.hint("expensive")
fn backtest(
    strategy: Strategy,
    samples: array[HistoricalSample],
    config: BacktestConfig
) -> BacktestResult;

// Swarm-integrated backtest
@ai.hint("swarm")
fn swarm_backtest(
    strategy: Strategy,
    iterations: u32
) -> array[BacktestResult];
```

### 9.4 Reward Shaping

```x3script
// Attach reward signal to action
@ai.reward("pnl")
fn execute_trade(params: TradeParams) -> i128 {
    let result = dex::swap_exact_in(...);
    return result.profit;  // This becomes the reward signal
}

// Custom reward function
@ai.reward_fn
fn compute_reward(state_before: State, action: Action, state_after: State) -> i128 {
    let pnl = state_after.balance - state_before.balance;
    let risk_penalty = compute_risk(action) * RISK_WEIGHT;
    return pnl - risk_penalty;
}
```

---

## 10. x3_std::ai

**Purpose:** AI annotations, model hints, and inference helpers.

### 10.1 Annotations

```x3script
// Specify model for inference
@ai.model("host:gpt-4-turbo")
fn make_decision() -> Action { ... }

// Optimization hint
@ai.hint("minimize_gas")
fn efficient_operation() { ... }

// Mutation configuration
@ai.mutable(rate: 0.1)  // 10% mutation rate
fn evolvable_logic() { ... }

// Prevent mutation
@ai.frozen
fn critical_security() { ... }
```

### 10.2 Embeddings

```x3script
// Generate text embedding
@ai.hint("sidecar")
fn embed(text: string) -> tensor<128>;

// Similarity search
@ai.hint("sidecar")
fn similarity(a: tensor<128>, b: tensor<128>) -> prob;

// Batch embedding
@ai.hint("sidecar")
fn embed_batch(texts: array[string]) -> array[tensor<128>];
```

### 10.3 Inference

```x3script
// Local model prediction (runs in sidecar)
@ai.hint("sidecar")
fn predict_local(
    model: string,
    input: tensor
) -> tensor;

// Rank candidate actions
@ai.hint("sidecar")
fn rank_actions(
    model: string,
    candidates: array[Action]
) -> array[(Action, prob)];

// Classification
@ai.hint("sidecar")
fn classify(
    model: string,
    input: bytes
) -> (class: u32, confidence: prob);
```

### 10.4 Receipt Verification

```x3script
// AI operations in X3Script compile to sidecar hostcalls
// Results include signed receipts for verification

@ai.model("host:dolphin-local")
fn decide_route(market_state: MarketState) -> Route {
    // Compiler transforms this to:
    // 1. Serialize market_state
    // 2. Hostcall to sidecar
    // 3. Receive (Route, SignedReceipt)
    // 4. Verify receipt signature
    // 5. Return Route
}
```

---

## 11. x3_std::zk

**Purpose:** ZK-friendly primitives and circuit generation.

### 11.1 Hash Functions

```x3script
// Poseidon hash (ZK-friendly)
@vm.hint("moderate")
fn poseidon_hash(inputs: array[u256]) -> u256;

// Hash to field element
@vm.hint("moderate")
fn hash_to_field(data: bytes) -> u256;

// Pedersen commitment
@vm.hint("expensive")
fn pedersen_commit(value: u256, blinding: u256) -> u256;
```

### 11.2 Merkle Trees

```x3script
// Verify Merkle proof
@vm.hint("moderate")
fn merkle_verify(
    root: u256,
    leaf: u256,
    proof: array[u256],
    index: u64
) -> bool;

// Compute Merkle root
@vm.hint("expensive")
fn merkle_root(leaves: array[u256]) -> u256;
```

### 11.3 ZK Blocks

```x3script
// Code in zk blocks compiles to constraint-friendly MIR
// and can optionally generate circuits
zk {
    let hash = poseidon_hash([a, b, c]);
    let valid = merkle_verify(root, leaf, proof, index);
    require(valid, "invalid proof");
}

// Compiler can emit:
// - Circom circuit
// - Halo2 circuit
// - Noir program
```

### 11.4 Private Operations

```x3script
// Private balance check (ZK proof)
@zk.private
fn check_sufficient_balance(
    commitment: u256,
    amount: u256,
    proof: ZkProof
) -> bool;

// Private transfer
@zk.private
fn private_transfer(
    from_commitment: u256,
    to_commitment: u256,
    amount: u256,
    proof: ZkProof
);
```

---

## 12. x3_std::safety

**Purpose:** Reentrancy protection, slippage limits, signatures.

### 12.1 Reentrancy Guard

```x3script
type GuardHandle = struct { id: u64 };

// Acquire reentrancy guard
@vm.hint("cheap")
fn non_reentrant() -> GuardHandle;

// Usage pattern
fn withdraw(amount: u128) {
    let guard = safety::non_reentrant();
    
    // Protected code
    let balance = get_balance(msg.sender);
    require(balance >= amount, "insufficient");
    
    // External call
    token::transfer(msg.sender, amount);
    
    // Update state
    set_balance(msg.sender, balance - amount);
    
    // Guard released on scope exit
}
```

### 12.2 Slippage Protection

```x3script
// Check slippage within bounds
@vm.hint("cheap")
fn check_slippage(
    expected: u128,
    actual: u128,
    max_slippage_bps: u32
) {
    let min_acceptable = expected * (10000 - max_slippage_bps) / 10000;
    require(actual >= min_acceptable, "excessive slippage");
}

// Deadline check
@vm.hint("cheap")
fn check_deadline(deadline: u64) {
    require(system::timestamp() <= deadline, "expired");
}
```

### 12.3 Signature Verification

```x3script
// Verify ECDSA signature
@vm.hint("expensive")
fn verify_signature(
    message: bytes32,
    signature: bytes,
    signer: address
) -> bool;

// Recover signer from signature
@vm.hint("expensive")
fn recover_signer(
    message: bytes32,
    signature: bytes
) -> address;

// EIP-712 typed data hash
@vm.hint("moderate")
fn hash_typed_data(
    domain_separator: bytes32,
    struct_hash: bytes32
) -> bytes32;

// Usage
fn permit_transfer(permit: Permit, signature: bytes) {
    let hash = hash_typed_data(DOMAIN_SEPARATOR, permit.hash());
    let signer = recover_signer(hash, signature);
    require(signer == permit.owner, "invalid signature");
}
```

### 12.4 Access Control

```x3script
// Role-based access
@vm.hint("cheap")
fn require_role(account: address, role: bytes32) {
    require(has_role(account, role), "missing role");
}

// Owner check
@vm.hint("cheap")
fn only_owner() {
    require(msg.sender == storage.owner, "not owner");
}
```

---

## 13. x3_std::devtools

**Purpose:** Debugging, gas profiling, and simulation.

### 13.1 Benchmarking

```x3script
type BenchResult = struct {
    iterations: u64,
    total_gas: u64,
    avg_gas: u64,
    min_gas: u64,
    max_gas: u64,
}

// Benchmark a strategy
@vm.hint("debug_only")
fn bench_strategy(
    strategy: Strategy,
    samples: u32
) -> BenchResult;

// Benchmark specific function
@vm.hint("debug_only")
fn bench_fn(
    f: fn() -> (),
    iterations: u32
) -> BenchResult;
```

### 13.2 Gas Profiling

```x3script
type GasProfile = struct {
    total: u64,
    by_opcode: map<string, u64>,
    by_function: map<string, u64>,
    hotspots: array[Hotspot],
}

// Profile bytecode
@vm.hint("debug_only")
fn gas_profile(bytecode: bytes) -> GasProfile;

// Profile execution trace
@vm.hint("debug_only")
fn profile_trace(trace: ExecutionTrace) -> GasProfile;
```

### 13.3 Simulation

```x3script
type SimulationReport = struct {
    iterations: u32,
    success_rate: prob,
    avg_profit: i128,
    max_drawdown: u128,
    sharpe_ratio: f64,
}

// Simulate strategy in swarm
@vm.hint("expensive")
fn simulate_swarm(
    strategy: Strategy,
    iterations: u32,
    config: SimConfig
) -> SimulationReport;

// Dry run transaction
@vm.hint("moderate")
fn dry_run(
    tx: Transaction
) -> (success: bool, gas_used: u64, return_data: bytes);
```

### 13.4 Debugging

```x3script
// Debug logging (removed in release)
@vm.hint("debug_only")
fn debug_log(msg: string, data: bytes);

// Assertion with debug info
@vm.hint("debug_only")
fn debug_assert(cond: bool, msg: string, context: bytes);

// Snapshot state for debugging
@vm.hint("debug_only")
fn snapshot_state() -> StateSnapshot;
```

---

## 14. Complete Example: FlashArb

A production-ready flashloan arbitrage strategy:

```x3script
import x3_std::core::*;
import x3_std::token::*;
import x3_std::dex::*;
import x3_std::flashloan::*;
import x3_std::oracle::*;
import x3_std::ai::*;
import x3_std::safety::*;

strategy FlashArb {
    storage {
        owner: address;
        vault: address;
        min_profit_bps: u32;
        total_profit: u128;
    }

    @ai.model("host:dolphin-local")
    const MODEL: string = "dolphin3-mini";

    // Entry point: initiate flashloan
    external fn execute(
        provider: FlashProvider,
        token_in: Token,
        amount: u128
    ) {
        require(msg.sender == storage.owner, "unauthorized");
        
        flashloan::request_flashloan(
            provider,
            [token_in],
            [amount],
            self,
            encode_params(token_in, amount)
        );
    }

    // Flashloan callback
    @flashloan.verify_repayment
    fn on_flashloan_received(
        assets: array[Token],
        amounts: array[u128],
        fees: array[u128],
        initiator: address,
        params: bytes
    ) -> bool {
        let guard = safety::non_reentrant();
        
        let token = assets[0];
        let amount = amounts[0];
        let fee = fees[0];

        // Find candidate routes using AI
        let routes = dex::find_candidate_routes(token, USDC, amount, 5);
        
        // AI ranks routes by expected profit
        let scored = ai::rank_actions(MODEL, routes);
        require(scored.len() > 0, "no viable routes");

        // Execute best route
        let best_route = scored[0].0;
        let confidence = scored[0].1;
        require(confidence > 0.8, "low confidence");

        // Calculate minimum output with slippage protection
        let quote = dex::quote_swap(best_route, amount);
        let min_out = quote.amount_out * (10000 - 50) / 10000;  // 0.5% slippage

        // Execute swap
        let out = dex::swap_exact_in(best_route, amount, min_out, self);

        // Calculate profit and verify
        let repay_amount = amount + fee;
        let profit = if out > repay_amount { out - repay_amount } else { 0 };
        
        require(
            profit >= core::bps(amount, storage.min_profit_bps),
            "insufficient profit"
        );

        // Repay flashloan
        token::approve(flashloan::provider_address(provider), repay_amount);

        // Send profit to vault
        if profit > 0 {
            token::transfer(storage.vault, profit);
            storage.total_profit += profit;
        }

        return true;
    }

    // View: check potential arbitrage
    fn check_opportunity(
        token: Token,
        amount: u128
    ) -> (bool, u128) {
        let routes = dex::find_candidate_routes(token, USDC, amount, 3);
        
        if routes.len() == 0 {
            return (false, 0);
        }

        let best_quote = dex::quote_swap(routes[0], amount);
        let expected_profit = if best_quote.amount_out > amount {
            best_quote.amount_out - amount
        } else {
            0
        };

        let profitable = expected_profit >= core::bps(amount, storage.min_profit_bps);
        return (profitable, expected_profit);
    }

    // Admin: update configuration
    external fn set_min_profit(bps: u32) {
        require(msg.sender == storage.owner, "unauthorized");
        require(bps <= 1000, "too high");  // Max 10%
        storage.min_profit_bps = bps;
    }

    external fn set_vault(new_vault: address) {
        require(msg.sender == storage.owner, "unauthorized");
        storage.vault = new_vault;
    }
}
```

---

## 15. Integration Guide

### 15.1 Compiler Setup

```bash
# Add stdlib to compiler path
x3-compiler --stdlib=stdlib/ strategies/flash_arb.x3

# Compile with optimizations
x3-compiler --stdlib=stdlib/ --optimize=3 strategies/flash_arb.x3
```

### 15.2 Verifier Rules

Configure the verifier to enforce:

```toml
# x3-verifier.toml
[rules]
flashloan_repay = true      # Verify repayment on all paths
cross_vm_atomic = true      # Verify cross-VM atomicity
ai_receipt_validation = true # Verify AI sidecar receipts
reentrancy_check = true     # Check for reentrancy vulnerabilities
```

### 15.3 Benchmarking

```bash
# Run gas profiler
cargo run -p x3-bench -- --strategy=flash_arb --samples=1000

# Output: gas_profile.csv
```

### 15.4 Swarm Integration

```bash
# Connect stdlib agent module to GPU swarm
x3-swarm connect --model=dolphin3-mini --endpoint=http://swarm:8080

# Run distributed simulation
x3-swarm simulate --strategy=flash_arb --iterations=1000000
```

---

## Appendix: Gas Hints Reference

| Hint             | Gas Range | Use Case                    |
| ---------------- | --------- | --------------------------- |
| `cheap`          | 1-3       | Pure math, getters          |
| `moderate`       | 10-100    | Memory ops, simple calls    |
| `expensive`      | 100-1000  | Storage writes, token ops   |
| `very_expensive` | 1000+     | Cross-VM, flashloans, swaps |
| `debug_only`     | N/A       | Removed in release builds   |
| `sidecar`        | N/A       | Executes off-chain          |

---

**Document Version:** 1.0.0  
**Specification Status:** Canonical  
**Maintainer:** X3 Chain Core Engineering
