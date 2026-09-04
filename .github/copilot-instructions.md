# Adaptive Completion Scoreboard Requirement

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

# X3 Adaptive Build Progress Contract

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
