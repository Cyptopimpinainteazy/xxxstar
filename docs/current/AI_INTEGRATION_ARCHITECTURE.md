# X3 AI Integration Architecture — June 2026

## Core Rule

**AI suggests, plans, explains, scores, audits, and optimizes. X3 rules, proofs, contracts, signatures, and safety gates execute.**

Do not let AI directly move funds without deterministic guardrails. That is how you build a casino with a keyboard and a death wish.

## The X3 AI Stack

```
User request
   ↓
AI Intent Assistant        ← drafts .x3 from natural language
   ↓
.x3 Draft
   ↓
X3 Compiler                ← static analysis + type checking
   ↓
Static Analyzer            ← invariant enforcement
   ↓
Route Engine               ← finds possible execution paths
   ↓
Risk Engine                ← scores routes, chains, contracts
   ↓
Proof Requirements          ← determines what must be proven
   ↓
Human / Wallet Signature   ← final authority gate
   ↓
Execution Engine           ← VM adapters, bridges, relayers
   ↓
Proof Ledger               ← append-only execution record
   ↓
AI Auditor + Scoreboard    ← explains results, catches anomalies
```

The AI is surrounded by steel walls at every step.

## AI Agents X3 Must Have

| Agent | Job | Priority |
|-------|-----|----------|
| **Intent Agent** | Turns plain English into .x3 intents | P0 |
| **Route Agent** | Finds and compares possible execution routes | P0 |
| **Risk Agent** | Detects dangerous chains/routes/contracts | P0 |
| **Contract Agent** | Builds contract/program/module connectors from ABIs/IDLs/schemas | P1 |
| **Proof Agent** | Audits proof ledger records for correctness | P1 |
| **Test Agent** | Generates tests/fuzz/chaos cases for intents | P1 |
| **Refund Agent** | Detects stuck intents and prepares refund flows | P2 |
| **Solver Agent** | Helps solvers price and bid on intents | P2 |
| **Security Agent** | Looks for stubs, unsafe paths, missing proofs | P1 |
| **Scoreboard Agent** | Explains route scores, risk profiles, proof gaps | P2 |

## Detailed Agent Designs

### 1. AI Intent Builder (P0)

A user says: "Swap 500 USDC from Arbitrum to SOL on Solana, safest route, refund me if anything fails."

AI converts it to .x3:

```
intent ai_generated_swap {
  from arbitrum.USDC amount 500
  to solana.SOL receiver user.wallet

  route safest
  require slippage <= 0.5%
  require route_score >= 90
  require refund_to == user.wallet
  require source_finality >= safe
  require destination_finality >= confirmed

  on timeout refund
  on solver_fail refund_and_slash
}
```

Then the compiler verifies it before anything runs. AI writes the mission. X3 checks if the mission is legal.

### 2. AI Route Optimizer (P0)

AI compares:
- LI.FI route
- Rango route
- Wormhole route
- LayerZero route
- Hyperlane route
- deBridge route
- THORChain route
- Chainflip route
- Native DEX route

Scored by:
- price
- speed
- liquidity
- finality risk
- bridge risk
- MEV risk
- solver reputation
- relayer health
- RPC health
- refund safety
- historical failure rate

Final route selection passes through the deterministic X3 route engine — AI recommends, protocol enforces.

### 3. AI Risk Agent (P0)

Watches:
- chain halts
- RPC disagreement
- weird liquidity movement
- bridge exploit news
- solver failure patterns
- relayer failures
- high reorg risk
- stuck intents
- abnormal slippage

Can recommend:
- pause route
- lower route score
- increase finality threshold
- disable adapter
- require higher solver bond
- force refund path
- ban risky bridge

AI recommends. The protocol enforces only after rule-based checks.

### 4. AI Proof Auditor (P1)

Reads the proof ledger and explains:
- Was source locked?
- Was destination filled?
- Was finality reached?
- Did relayers agree?
- Did RPC quorum agree?
- Was the solver honest?
- Was the refund path valid?
- Was anything missing?

Example output:
```
Intent 0xabc completed correctly.
Source lock confirmed on Arbitrum.
Destination fill confirmed on Solana.
3/5 relayers signed.
3/5 RPCs agreed.
No refund needed.
No slashing triggered.
```

### 5. AI Contract Linker (P1)

Developer gives X3 an ABI, IDL, Move module, or CosmWasm schema. AI maps it into an X3 connector:

```
contract uniswap_v3 on arbitrum {
  vm evm
  address 0x...
  abi "./uniswap_v3.json"
}

action swap_usdc_to_weth {
  call uniswap_v3.exactInputSingle {
    tokenIn arbitrum.USDC
    tokenOut arbitrum.WETH
    amountIn input.amount
    amountOutMinimum input.min_out
  }

  require slippage <= input.max_slippage
  prove event Swap
}
```

AI generates the wrapper. X3 tests and verifies it.

### 6. AI Test Generator (P1)

Given a .x3 intent, generates:
- happy path test
- timeout test
- refund test
- bad solver test
- bad relayer test
- RPC disagreement test
- chain halt test
- slippage failure test
- double claim test
- double refund test
- route mutation test

Commands:
```bash
x3c ai testgen swap.x3
x3c test swap.x3
x3c fuzz swap.x3
x3c prove swap.x3
```

### 7. AI Scoreboard Explainer (P2)

Shows hard numbers, AI explains them:

```
Route score: 72/100
Reason:
- Solana RPC quorum is weak
- Wormhole adapter has degraded latency
- Solver reputation is below preferred threshold
- Refund path exists but timeout is too long
```

## What AI Must Never Be Allowed To Do Alone

Do not allow AI to bypass:
- wallet signature
- compiler safety checks
- route constraints
- proof requirements
- refund requirements
- finality thresholds
- solver bond rules
- slashing rules
- mainnet release gates

AI can be smart. It cannot be trusted. Same as any tool — wearing a nicer tie doesn't change the rule.

## Implementation Status

| Component | Status | Crate/Path |
|-----------|--------|------------|
| AI Intent Builder | Not started | — |
| Route Optimizer | Not started | — |
| Risk Agent | Not started | — |
| Contract Linker | Not started | — |
| Proof Auditor | Not started | — |
| Test Generator | Not started | — |
| Refund Agent | Not started | — |
| Solver Agent | Not started | — |
| Security Agent | Not started | `crates/x3-security-agent/` (stub) |
| Scoreboard Explainer | Not started | — |
| x3-ai-command-system | Exists | `x3-ai-command-system/` |
| AIGovernanceBot | Exists | `ai-hooks/AIGovernanceBot.ts` |
| AIYieldOptimizer | Exists | `ai-hooks/AIYieldOptimizer.ts` |

## Grant Positioning for AI Component

X3 integrates AI as a developer and security copilot for cross-VM execution. AI assists with intent generation, route analysis, contract connector creation, risk scoring, proof-ledger auditing, test generation, and human-readable explanations. Critical execution remains controlled by deterministic compiler checks, finality rules, proof requirements, wallet signatures, and refund-safe state machines.

This says: AI improves usability and safety but does not blindly control funds. Extremely grant-friendly.