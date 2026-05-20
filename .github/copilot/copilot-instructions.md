# X3 Copilot Instructions

> **Purpose:** This file encodes all X3 codebase conventions, patterns, and safety rules so that
> Copilot generates code consistent with the existing codebase at every stage toward mainnet.
> When generating any code in this repo, follow every rule in this file.

---

## Adaptive Completion Scoreboard Requirement (Mandatory)

Every time you finish a task, file, module, subsystem, PR, RC, or version milestone, you MUST end your response with an adaptive progress scoreboard.

The scoreboard must reflect the exact section you worked on, not the entire project unless the whole project was inspected.

## Required Format

```txt
<SECTION_OR_MODULE>  <10_BLOCK_PROGRESS_BAR>  <PERCENT>%  <HONEST_STATUS>

Example:

x3-lang/parser        ███████░░░  70%  New constructs parse; examples and edge cases still thin
x3-ir/emitter         █████░░░░░  55%  Partial emission works; full pipeline is not wired
x3-runtime/dispatch   ██░░░░░░░░  25%  Pallets exist; runtime dispatch loop missing
```

Progress Bar Rules

Use exactly 10 blocks.

Filled block: █
Empty block: ░
Round the percentage to the nearest 10% for the bar.
The numeric percent may be more precise than the bar.

Examples:

  5%  ░░░░░░░░░░
 10%  █░░░░░░░░░
 25%  ██░░░░░░░░
 55%  █████░░░░░
 70%  ███████░░░
 85%  █████████░
100%  ██████████

Scoring Standard

Score based on real working condition, not intent, plans, or file count.

Use this scale:

0–5%      Empty, placeholder, idea only, or file exists with no real logic
6–15%     Skeleton exists, but mostly stubs
16–30%    Basic structure exists; key logic missing
31–50%    Partial implementation; not fully wired or tested
51–70%    Mostly implemented; integration/tests/examples incomplete
71–85%    Wired and working in basic cases; needs hardening, edge cases, audit
86–95%    Production candidate; needs stress testing, security review, polish
96–100%   Complete, tested, documented, wired, audited, and no known stubs

Evidence Rules

The score MUST be based on evidence.

Consider:

Does it compile?
Do tests pass?
Is it wired into the real runtime/app/CLI/API?
Are there examples?
Are docs updated?
Are there stubs, TODOs, mocks, fake data, or placeholder logic?
Is error handling real?
Is persistence real?
Is security considered?
Is the feature reachable from the user-facing flow?
Does it work end-to-end?

Do NOT give high scores for code that only exists but is not wired.

Adaptive Scope Rules

Choose the scoreboard label based on what was actually touched.

Good labels:

x3-lang/parser
x3-lang/compiler
x3-ir/emitter
x3-runtime/atomic-dispatch
x3-bridge/htlc
x3-gpu-validator/proof-kernel
x3-dex/batch-swap-router
x3-wallet/transaction-signer
x3-testnet/validator-bootstrap
RC7/settlement-harness
v0.3/runtime-wiring

Bad labels:

X3 System
Blockchain
Code
Project
Done

Unless the entire system was inspected, do not score the entire system.

Required End-of-Response Structure

At the end of every completed task, include this:

## Completion Scoreboard

```txt
<SECTION>  <BAR>  <PERCENT>%  <HONEST_STATUS>
```

What changed
...
Still missing
...
Next best action
...

## Multi-Section Rule

If you touched multiple parts, score each one separately.

Example:

```txt
x3-lang/parser          ███████░░░  72%  New syntax parses; malformed input tests still weak
x3-ir/lowering          █████░░░░░  58%  AST lowers to X3IR; cross-chain op lowering incomplete
x3-runtime/dispatch     ███░░░░░░░  35%  Dispatch trait exists; not connected to runtime execution
x3-tests/e2e-pipeline    ██░░░░░░░░  22%  Test shell exists; no full compiler-to-runtime path
```

Honesty Rules

Never say a section is complete unless ALL of this is true:

Code compiles cleanly.
Relevant tests pass.
Feature is wired into the actual execution path.
No fake/stub/mock logic remains in the core path.
Docs or examples exist.
Error handling exists.
It has been validated beyond a happy-path demo.

Use direct language.

Good status:

Emitter exists but only handles basic transfer ops; cross-chain settlement is not wired

Bad status:

Looks good

Good status:

Parser supports new syntax; no fuzz tests or invalid-input coverage yet

Bad status:

Almost done

Stub Detection Rule

If you find any of these, mention them in the status or missing section:

TODO
FIXME
unimplemented!()
todo!()
panic!("stub")
fake return values
hardcoded demo addresses
mock proof verification
in-memory-only persistence where production needs durable storage
commented-out tests
ignored failing tests
fake success responses
placeholder security checks

Final Instruction

Always finish with the adaptive scoreboard.
Do not inflate progress.
Do not show unrelated modules.
Do not hide blockers.
No fake 100s.

For your full X3 / Atlas / x3-lang system, use this stronger version:

## X3 Adaptive Build Progress Contract

After every implementation pass, audit pass, repair pass, or planning pass, output an adaptive X3 progress scoreboard for the exact subsystem touched.

The scoreboard must answer:

1. What did you actually change?
2. How complete is that exact subsystem?
3. What is still fake, stubbed, missing, or unwired?
4. What is the next best action?

## X3 Scoreboard Format

```txt
<DOMAIN>/<SUBSYSTEM>  <BAR>  <PERCENT>%  <STATUS>

Examples:

x3-lang/parser              ████████░░  78%  Core syntax parses; malformed cross-chain cases need tests
x3-lang/x3ir-emitter        █████░░░░░  52%  Emits basic X3IR; GPU and bridge ops are incomplete
x3-runtime/atomic-dispatch  ███░░░░░░░  34%  Pallet shell exists; dispatch loop not wired into runtime
x3-bridge/htlc              ██████░░░░  61%  HTLC SDK exists; X3IR-driven settlement path missing
x3-gpu-validator            ██░░░░░░░░  24%  Kernel scaffold exists; proof verification is fake
x3-e2e-tests                ██░░░░░░░░  20%  Test folders exist; no compiler-to-runtime validation
```

X3 Completion Categories

Use these categories when relevant:

x3-lang/parser
x3-lang/typechecker
x3-lang/compiler
x3-lang/x3ir
x3-lang/emitter
x3-runtime/pallets
x3-runtime/atomic-dispatch
x3-runtime/supply-invariants
x3-bridge/htlc
x3-bridge/btc
x3-bridge/evm
x3-bridge/svm
x3-gpu-validator/kernel
x3-gpu-validator/proof-verifier
x3-dex/router
x3-dex/liquidity
x3-wallet/signing
x3-testnet/bootstrap
x3-ci/validation
x3-docs/examples

Only include categories that were changed, inspected, or tested.

Required Ending

Every response must end like this:

## Completion Scoreboard

```txt
x3-lang/parser              ███████░░░  70%  New constructs parse; examples and fuzz tests missing
x3-ir/emitter               █████░░░░░  55%  Partial X3IR emission works; end-to-end pipeline not wired
x3-runtime/atomic-dispatch  ██░░░░░░░░  25%  Pallets exist; runtime dispatch loop missing
```

What changed
Added/modified/validated...
Still missing
Missing tests...
Missing runtime wiring...
Stubbed logic...
Next best action
Wire <specific subsystem> into <specific execution path> and add one end-to-end test.

## Brutal Rule

If it does not run end-to-end, it is not above 70%.

If it is not wired into the real system, it is not above 60%.

If it has stubs in the core path, it is not above 50%.

If it only has files and names, it is not above 25%.

If it is just an idea, it is below 10%.

And here is the short version to paste at the end of any Codex/Copilot prompt:

When finished, output an adaptive completion scoreboard for only the subsystem you touched. Use 10-block bars, honest percent complete, evidence-based status, changed items, missing blockers, and next best action. Do not score unrelated modules. Do not inflate progress. No fake 100s.

For an agent that edits your whole repo, use this one-liner too:

Before scoring, scan the touched files for TODO/FIXME/unimplemented!/todo!/panic/stub/mock/fake/hardcoded.

---

## 1. Project Identity

**X3** is a multi-VM blockchain built on `polkadot-sdk` (branch `stable2512`, rev `948fbd2`).

- **Language:** Rust 1.90.0 (`rust-toolchain.toml`). Target: `wasm32-unknown-unknown` for runtime, `x86_64-unknown-linux-gnu` for node.
- **Execution environments:**
  - `X3Native` — Substrate/FRAME pallets (Rust, no_std)
  - `X3Evm` — Frontier EVM (`pallet-evm`), Solidity contracts via Hardhat + Foundry
  - `X3Svm` — Anchor/rBPF Solana programs (currently `DISABLED_BLOCKED`)
- **Goal:** Stable testnet → mainnet release. Correctness and safety over features.
- **Workspace root:** `Cargo.toml` at repo root; 33 active crates under `crates/`; 47 pallets under `pallets/`.
- **Runtime:** `runtime/src/lib.rs` — four `construct_runtime!` variants gated by feature flags `mainnet-rc1` and `frontier`.

---

## 2. Rust / FRAME Pallet Conventions

### 2.1 File Header

Every pallet `lib.rs` starts with:

```rust
// SPDX-License-Identifier: Apache-2.0
//!
//! # Pallet Name
//!
//! **Scope:** One-line description of what this pallet is responsible for.
//!
//! **Guarantees:**
//! 1. Guarantee one (e.g. "Supply invariant is preserved across all extrinsics.")
//! 2. Guarantee two
//! 3. Guarantee three
```

### 2.2 Required Crate Attributes

```rust
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]
```

Both are **mandatory** in every pallet crate. Never omit them.

### 2.3 Mainnet RC-1 Scope Guards

Features that must not ship in `mainnet-rc1` are gated with compile-time guards:

```rust
#[cfg(all(feature = "mainnet-rc1", feature = "external-gateway"))]
compile_error!("MAINNET SCOPE VIOLATION: external-gateway must not be enabled with mainnet-rc1");
```

The 7 guarded features are:
- `external-gateway`
- `parallel-executor`
- `appzone-factory`
- `pq-experimental`
- `advanced-dex`
- `ai-optimizer`
- `gpu-acceleration`

When generating code for a new pallet that touches any of these features, add the relevant guard.

### 2.4 Module Layout

```rust
pub use pallet::*;

#[cfg(test)]
mod tests;

// Optional sub-modules (only if they exist or are needed):
pub mod weights;
mod types;
mod benchmarking;
```

### 2.5 Import Ordering

1. `codec::{Decode, Encode, MaxEncodedLen}`
2. `frame_support::{...}`
3. `frame_system::{...}`
4. `sp_core::{...}`, `sp_runtime::{...}`, `sp_std::{...}`
5. Local `x3_*` crates

### 2.6 Storage Patterns

```rust
// Map: always use Blake2_128Concat; OptionQuery for optional, ValueQuery for required
#[pallet::storage]
#[pallet::getter(fn my_value)]
pub type MyMap<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, MyStruct<T>, OptionQuery>;

// Value: ValueQuery with default
#[pallet::storage]
#[pallet::getter(fn my_count)]
pub type MyCount<T: Config> = StorageValue<_, u64, ValueQuery>;
```

**Replay protection:** Use `StorageMap<_, Blake2_128Concat, H256, (), OptionQuery>` for message deduplication — never a `Vec<H256>` (O(n) lookup breaks under load).

### 2.7 Config Trait

```rust
#[pallet::config]
pub trait Config: frame_system::Config {
    /// The overarching event type.
    type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    /// Named associated types with doc comments and trait bounds.
    type Balance: Parameter + Member + AtLeast32BitUnsigned + Codec + Copy + Default;
    /// Weights for this pallet.
    type WeightInfo: WeightInfo;
}
```

### 2.8 Events

```rust
#[pallet::event]
#[pallet::generate_deposit(pub(super) fn deposit_event)]
pub enum Event<T: Config> {
    /// Document what this event signals.
    /// TESTNET_ONLY: Remove this tag when event is stable.
    /// FROZEN under v1-alpha API (2026-05-19, commit 948fbd2): Do not rename fields.
    SomethingHappened {
        who: T::AccountId,
        amount: T::Balance,
    },
    /// DISABLED_POST_RC1: This event will be removed after RC-1.
    TemporaryDebugEvent { data: Vec<u8> },
}
```

Tags to use in event doc comments:
- `TESTNET_ONLY:` — event may not exist in mainnet
- `FROZEN under v1-alpha API (date, commit):` — external wallets/SDKs depend on this event; field names must not change without a migration plan
- `DISABLED_POST_RC1:` — event will be removed after RC-1

### 2.9 Errors

```rust
#[pallet::error]
pub enum Error<T> {
    /// Attempted to exceed the maximum allowed value.
    ExceedsMaximum,
    /// Sender does not have sufficient balance.
    InsufficientBalance,
    /// Operation would violate supply invariant.
    SupplyInvariantViolation,
}
```

Rules: PascalCase, single-line `///` doc per variant, no trailing commas.

### 2.10 Extrinsics

```rust
#[pallet::call]
impl<T: Config> Pallet<T> {
    /// Brief doc: what this extrinsic does.
    #[pallet::call_index(0)]
    #[pallet::weight(T::WeightInfo::do_something())]
    pub fn do_something(origin: OriginFor<T>, value: T::Balance) -> DispatchResult {
        let who = ensure_signed(origin)?;  // origin check FIRST
        Self::do_something_inner(who, value)  // delegate to internal helper
    }
}
```

- **Always** use `#[pallet::call_index(N)]`
- **Always** use `T::WeightInfo::method()` for weights
- Extrinsics are thin wrappers: origin check + dispatch to `Self::do_*_inner`

For unbenched pallets: `Weight::from_parts(10_000_000, 0)` as a temporary placeholder, tagged with `// UNBENCHED: replace with benchmark weight before mainnet`.

### 2.11 Internal Helpers

```rust
impl<T: Config> Pallet<T> {
    fn do_something_inner(who: T::AccountId, value: T::Balance) -> DispatchResult {
        // real logic here
        Ok(())
    }
}
```

Naming: `fn do_<verb>` or `fn do_<verb>_inner`. Never write business logic directly inside extrinsics.

### 2.12 Arithmetic — CRITICAL

**NEVER** use raw `+`, `-`, `*` on `Balance` or any `u128`/`u64` in pallet code.

```rust
// CORRECT:
let new_balance = balance.checked_add(amount).ok_or(Error::<T>::Overflow)?;
let fee = amount.saturating_mul(rate);
let remaining = total.saturating_sub(used);

// FORBIDDEN:
let new_balance = balance + amount;  // NO
let fee = amount * rate;              // NO
```

Use:
- `checked_add` / `checked_sub` / `checked_mul` → return `Option`, map to error
- `saturating_add` / `saturating_sub` / `saturating_mul` → for values where floor/ceil at bounds is acceptable
- `defensive_saturating_add` (from `sp_runtime::traits::Defensive`) for supply-critical paths

---

## 3. Rust Test Conventions

### 3.1 File Location

Tests live in `src/tests.rs`. Reference from `lib.rs`:

```rust
#[cfg(test)]
mod tests;
```

### 3.2 Mock Runtime Structure

```rust
// src/tests.rs
use crate as pallet_x3_<name>;
use frame_support::{construct_runtime, parameter_types, traits::Everything};
use sp_runtime::BuildStorage;

construct_runtime!(
    pub enum Test {
        System: frame_system,
        <Name>: pallet_x3_<name>,
    }
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
    type Block = frame_system::mocking::MockBlock<Test>;
}

impl pallet_x3_<name>::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    // ... other associated types
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}
```

### 3.3 Fixture Functions

```rust
fn alice_native() -> <Test as frame_system::Config>::AccountId {
    1u64
}
fn bob_native() -> <Test as frame_system::Config>::AccountId {
    2u64
}
```

### 3.4 Test Naming

```rust
fn test_<subject>_<expected_result>()
// Examples:
fn test_transfer_succeeds_with_valid_balance()
fn test_transfer_fails_when_balance_insufficient()
fn test_replay_protection_rejects_duplicate_message()
```

### 3.5 Test Body Pattern

```rust
#[test]
fn test_something_succeeds() {
    new_test_ext().execute_with(|| {
        // Arrange
        let who = alice_native();
        let amount = 1_000u64;

        // Act
        let result = <Name>::do_something(RuntimeOrigin::signed(who), amount);

        // Assert
        assert_ok!(result);
        assert_eq!(<Name>::my_storage_value(), expected_value);
        System::assert_has_event(Event::SomethingHappened { who, amount }.into());
    });
}

#[test]
fn test_something_fails_with_correct_error() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            <Name>::do_something(RuntimeOrigin::signed(alice_native()), 0u64),
            Error::<Test>::InvalidAmount
        );
    });
}
```

### 3.6 Required Test Coverage Checklist

Every pallet must have tests for:
- [ ] Happy path for every public extrinsic
- [ ] Error path for every `Error<T>` variant that can be triggered from an extrinsic
- [ ] Replay protection: duplicate message/nonce is rejected
- [ ] Supply invariant: after every mint/burn/transfer, `ledger.check_invariant()` passes
- [ ] Unauthorized origin: extrinsics requiring specific origin fail with wrong origin
- [ ] Boundary conditions: zero amounts, max balance, overflow edge cases

---

## 4. TypeScript Conventions

### 4.1 File Naming

- Source files: `kebab-case.ts`
- React components: `PascalCase.tsx`
- API services: `kebab-case-api.ts`

### 4.2 Type Definitions

```typescript
// Branded type aliases for IDs
export type AccountId = string;
export type BlockNumber = number;
export type XxxId = string;

// String enums for serializable state
export enum XxxState {
  Pending = "pending",
  Active = "active",
  Closed = "closed",
}

// Interfaces for structured data
export interface Xxx {
  id: XxxId;
  owner: AccountId;
  state: XxxState;
  createdAt: number;
}
```

### 4.3 Services (Non-Class Pattern)

```typescript
const API_BASE = '/api/v1';

async function fetchJson<T>(path: string, options?: RequestInit): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`, options);
  if (!res.ok) throw new Error(`API error: ${res.status}`);
  return res.json();
}

export async function getXxx(id: XxxId): Promise<Xxx> {
  return fetchJson<Xxx>(`/xxx/${id}`);
}

export async function createXxx(data: CreateXxxInput): Promise<Xxx> {
  return fetchJson<Xxx>('/xxx', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data),
  });
}
```

Do NOT use class-based service pattern (no `class XxxService { ... }`).

### 4.4 React Hooks

```typescript
interface UseXxxResult {
  data: Xxx | null;
  loading: boolean;
  error: Error | null;
  refetch: () => void;
}

export function useXxx(id: XxxId): UseXxxResult {
  const [data, setData] = useState<Xxx | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const fetch = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      setData(await getXxx(id));
    } catch (e) {
      setError(e instanceof Error ? e : new Error(String(e)));
    } finally {
      setLoading(false);
    }
  }, [id]);

  useEffect(() => { fetch(); }, [fetch]);
  return { data, loading, error, refetch: fetch };
}
```

### 4.5 Tauri Integration

Use `invoke<T>('command_name', { params })` for Tauri IPC calls:

```typescript
import { invoke } from '@tauri-apps/api/tauri';

async function getBalance(address: AccountId): Promise<Balance> {
  return invoke<Balance>('get_balance', { address });
}
```

### 4.6 Next.js App Router

- Use `app/` directory (App Router), not `pages/`
- Add `'use client'` directive only when component uses hooks or browser APIs
- Export static metadata from server components:

```typescript
export const metadata = {
  title: 'X3 Wallet',
  description: 'X3 multi-VM blockchain wallet',
};
```

- Data fetching in Server Components via `async function` — no `useEffect` in server components

---

## 5. Python Conventions

### 5.1 File Header

```python
#!/usr/bin/env python3
"""
Module description here.
"""
```

### 5.2 Imports

```python
from typing import Dict, Any, Optional, List, Set
from dataclasses import dataclass, asdict, field
import asyncio
import logging
```

### 5.3 Async Handlers

```python
from aiohttp import web

async def handle_request(request: web.Request) -> web.Response:
    data = await request.json()
    result = await process(data)
    return web.json_response(result)
```

### 5.4 Lazy Import Pattern (for optional dependencies)

```python
_store = None

def get_store():
    global _store
    if _store is not None:
        return _store
    try:
        import some_module
        _store = some_module.Store()
        return _store
    except ImportError:
        return None
```

### 5.5 Storage Fallback Chain

Always follow this priority for persistence:
1. Check `POSTGRES_URL` or `DATABASE_URL` env var → try PostgreSQL
2. Fallback to SQLite
3. Final fallback to JSON at `/tmp/x3_<service>_data.json`

```python
import os

def get_db():
    if url := os.environ.get('POSTGRES_URL') or os.environ.get('DATABASE_URL'):
        try:
            import asyncpg
            return asyncpg.connect(url)
        except Exception:
            pass
    try:
        import aiosqlite
        return aiosqlite.connect('/tmp/x3_fallback.db')
    except Exception:
        pass
    return None  # caller must handle JSON fallback
```

### 5.6 Logging

```python
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s [%(levelname)s] %(name)s: %(message)s'
)
logger = logging.getLogger(__name__)
```

### 5.7 Enums and Dataclasses

```python
from enum import Enum
from dataclasses import dataclass, field

class AgentStatus(Enum):
    PENDING = "pending"
    RUNNING = "running"
    DONE = "done"

@dataclass
class AgentTask:
    agent_id: str
    status: AgentStatus = AgentStatus.PENDING
    results: List[Dict[str, Any]] = field(default_factory=list)
    error: Optional[str] = None
```

---

## 6. Solidity / EVM Conventions

### 6.1 Tooling

- **Hardhat:** `hardhat.config.ts` at repo root. Solidity 0.8.24.
- **Foundry:** `X3-contracts/evm/foundry.toml`. Use for testing with `forge test`.
- **OpenZeppelin:** Use for standard ERC-20/ERC-721/access control patterns.

### 6.2 Secrets — CRITICAL

**NEVER** hardcode RPC endpoints or private keys:

```typescript
// hardhat.config.ts — CORRECT:
networks: {
  x3testnet: {
    url: process.env.RPC_ENDPOINT ?? "",
    accounts: process.env.PRIVATE_KEY ? [process.env.PRIVATE_KEY] : [],
  },
},

// FORBIDDEN:
url: "<RPC_ENDPOINT>",   // NO
accounts: ["0xdeadbeef"], // NO
```

### 6.3 Contract Structure

```solidity
// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "@openzeppelin/contracts/access/Ownable.sol";

contract X3Token is ERC20, Ownable {
    // ...
}
```

---

## 7. Runtime / Cross-VM Rules

### 7.1 VM Adapter Selection

The runtime uses different adapters per build target:

```rust
// Native (binary) build:
// NativeEvmAdapter — uses pallet_evm::Runner::create() from Frontier (REAL)
// RbpfSvmAdapter — uses rBPF (REAL, but DISABLED_BLOCKED via SVM feature flag)
// X3VmAdapter — X3 native VM (REAL)

// Wasm (runtime) build:
// WasmEvmAdapter, WasmSvmAdapter, WasmX3Adapter (thin wasm-safe wrappers)
```

- `runtime_uses_mock_vm_adapters()` at `runtime/src/lib.rs:4208` — must return `false` in production chain specs.
- Never add a mock adapter into a production execution path.

### 7.2 Construct Runtime — Scope-Gated Pallets

RC-1 only pallets are gated in `construct_runtime!`:

```rust
#[cfg(feature = "mainnet-rc1")]
X3LaunchPad: pallet_x3_launchpad,
```

When adding a new pallet:
1. Determine if it is RC-1 or post-RC-1
2. Add the appropriate `#[cfg(feature = ...)]` gate
3. Add to all 4 `construct_runtime!` variants that should include it

### 7.3 Dual Nonce System Warning

Two nonce systems currently exist:
- `pallet-x3-cross-vm-router`: `NextNonce` + `UsedMessages` (canonical for cross-VM)
- `pallet-x3-account-registry`: `CrossVmNonces` (wallet-facing nonce query)

When writing any code that reads or increments nonces for cross-VM operations, use the router's `NextNonce`. Do NOT create a third nonce system.

### 7.4 Supply Invariant

After every operation that mints, burns, or transfers tokens across VM boundaries:

```rust
// In tests: always check supply invariant
let ledger = SupplyLedger::<Test>::get();
assert!(ledger.check_invariant().is_ok(), "Supply invariant violated");
```

---

## 8. Critical Never-Do Rules

These are hard rules. Never violate them:

| Rule | What to do instead |
|------|--------------------|
| Never raw arithmetic (`+`/`-`/`*`) on Balance/u128 | Use `checked_add`, `saturating_add`, etc. |
| Never replay protection in a `Vec` | Use `StorageMap<_, Blake2_128Concat, H256, (), OptionQuery>` |
| Never `todo!()` or `unimplemented!()` in production extrinsic paths | Implement it or gate behind `#[cfg(test)]` |
| Never hardcode account addresses in pallet storage initializers | Use genesis config or runtime Config parameter |
| Never enable the 7 RC-1-guarded features with `mainnet-rc1` | Keep them separate; add compile_error! guard |
| Never omit `#![cfg_attr(not(feature = "std"), no_std)]` in a pallet | It's required for Wasm compilation |
| Never commit `<RPC_ENDPOINT>` or `<PRIVATE_KEY>` literals in any config | Use `process.env.VAR_NAME` |
| Never use `StubAiProvider` in production without a startup alert | Check `OPENROUTER_API_KEY` env var at startup and log a warning if absent |
| Never set `runtime_uses_mock_vm_adapters()` to return `true` in a production chain spec | Keep mocks only in dev/test chain specs |
| Never skip `#[pallet::call_index(N)]` on a dispatchable | Required for stable call index encoding |
| Never use well-known keys (`//Alice`, `//Bob`) in public testnet chain specs | Generate real keys with `subkey` |
| Never create a new `main()` entry point alongside `node/src/main.rs` | Extend main.rs or use the existing binary target |

---

## 9. Mainnet Safety Gates

Before any code ships to the public testnet, these gates must pass:

### 9.1 Compile Gates
- `cargo check --workspace` passes with zero errors
- `cargo check --workspace --features mainnet-rc1` passes with zero errors
- No `MAINNET SCOPE VIOLATION` compile_error! triggers

### 9.2 Test Gates
- All pallets have at minimum: happy path + error path + replay test + supply invariant test
- `pallets/northern-swarm`, `pallets/pallet-x3-control`, `pallets/fraud-proofs` each have test coverage
- All 25+ fuzz targets replaced with real structure decoding tests
- Integration tests under `integration-tests/` are registered as Cargo workspace members

### 9.3 Security Gates
- `hardhat.config.ts` uses no literal `<RPC_ENDPOINT>` or `<PRIVATE_KEY>` placeholders
- `runtime_uses_mock_vm_adapters()` returns `false` in both dev and mainnet chain specs
- Chain specs use real keys (not `//Alice`/`//Bob` well-known seed phrases)
- `pallets/x3-agent-law/src/lib.rs` origin check upgraded from `ensure_signed` to `ensure_root` or governance origin for admin extrinsics
- `StubAiProvider` logs a startup warning when `OPENROUTER_API_KEY` is absent

### 9.4 Feature Gates
- RC+1 items (`CapabilityEnvelopeCheck`, `AtomicSettlementCheck`, `FlashFinalityExtension`) are either implemented or explicitly deferred with tracking tickets
- `pallets/x3-launchpad` graduation path is wired to `pallets/x3-token-factory`
- x3-automation oracle integration (line 382) is implemented or explicitly disabled with an error

---

## 10. Workspace Layout Quick Reference

```
Cargo.toml              # workspace root — 33 active member crates
rust-toolchain.toml     # Rust 1.90.0
hardhat.config.ts       # EVM/Hardhat (Solidity 0.8.24)
X3-contracts/evm/       # Foundry project

crates/                 # 124 crate dirs; only 33 in workspace members
pallets/                # 47 FRAME pallets
runtime/src/lib.rs      # main runtime — 4 construct_runtime! variants
node/                   # Substrate node binary
apps/
  wallet/               # Next.js App Router, Tauri hooks
  dex/                  # Next.js App Router
  dashboard/            # Next.js App Router
  explorer/             # package.json ONLY — no src/ (NOT implemented)
x3-lang/                # X3 compiler toolchain (lexer/vm broken, IR stable)
swarm_infrastructure/   # Python async AI agent swarm (aiohttp, OpenRouter)
programs/               # Anchor/SVM programs (DISABLED_BLOCKED)
integration-tests/      # NOT in Cargo workspace — tests do NOT run in CI
```

---

*Last updated: 2026-05-19. Source: deep codebase exploration, all 47 pallets and runtime verified.*
