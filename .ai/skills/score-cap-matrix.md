# Skill: Score Cap Matrix

Final score must be the lower of the estimated score and the strictest applicable cap.

This is how we prevent inflated claims.

## Score Cap Table

```txt
SCAFFOLDING / SKELETON

Only idea/planning/docs written                         max 25%
Only scaffold/structure created                         max 20%


COMPILATION / BASIC TESTS

Code exists but does not compile                        max 35%
Compiles but no unit tests                              max 60%
Unit tests exist but no integration tests               max 65%


RUNTIME INTEGRATION

Runtime path not reachable from public entrypoint       max 60%
Public API exists but no end-to-end test               max 70%
End-to-end path missing or fails                       max 70%


CORE PATH QUALITY

Core path has stubs/mocks/fake returns                  max 50%
Unwrap/panic in error handling path                     max 35%
Secrets logged in output                                max 25%


BRIDGE / SETTLEMENT SPECIFIC

Bridge logic has no replay test                         max 55%
Bridge logic has no timeout test                        max 55%
Bridge logic has no invalid proof test                  max 50%
Bridge logic has no finality verification               max 60%
Fake proof verifier in core path                        max 35%


SUPPLY / BALANCE SPECIFIC

Bridge/supply logic has no invariant test               max 45%
Asset/transfer/mint operation untested                  max 50%


CONSENSUS / VALIDATOR SPECIFIC

Consensus/validator changes have no invariant test      max 40%
Economic correctness not proven                         max 45%


SECURITY / ERROR HANDLING

Security-sensitive code has no negative tests           max 55%
Code logs private keys / secrets / seeds                max 25%
Panic on user input (instead of error)                  max 35%
Hardcoded private keys / RPC secrets in code            max 10%


TEST QUALITY / COMPLETENESS

Tests weakened (modified to always pass)                max 40%
No error path tests                                     max 55%
No invalid input tests                                  max 60%
No edge case tests                                      max 65%


GATES / VALIDATION

No evidence gate (proof logs/results)                   max 60%
No regression gate (upstream/downstream checked)        max 65%
No acceptance criteria defined                          max 70%
No failure reproduction (for bug fix)                   max 65%


CROSS-VM / MULTI-DOMAIN

No Cross-VM trace documented                            max 60%
Cross-VM atomicity unclear                              max 65%


CONFIGURATION / DEPLOYMENT

Hardcoded local-only paths                              max 55%
Hardcoded demo values in core code                      max 55%
No .env.example updated                                 max 60%
Secret leakage risk                                     max 40%


DOCUMENTATION / MIGRATION

No migration documented (for storage change)            max 65%
Backward compatibility unknown                          max 70%
No release notes                                        max 75%


VERSIONING / COMPATIBILITY

Breaking change without version bump                    max 70%
Public API change without migration note                max 65%


MAINNET CLAIMS

Mainnet claim without audit/stress/invariant proof      max 50%
Testnet ready claimed as mainnet ready                  max 45%


WORST CASE / CRITICAL ISSUES

No compiler output (does not compile)                   max 35%
No tests pass                                           max 45%
Core logic is a stub                                    max 25%
Feature is unreachable                                  max 55%
```

## How to Apply Caps

### Step 1: Estimate your score

```txt
I wrote a new parser for Custom::Transfers.

- Code compiles? YES
- Tests pass? YES (10/10)
- Syntax works? YES (15 test cases)
- AST is lowered? YES
- X3IR emit works? PARTIAL (50% of operations)
- End-to-end test? NO

Estimated: 70%
```

### Step 2: Check all applicable caps

```txt
Estimated: 70%

Applicable caps:
- X3IR exists but emitter incomplete              → max 65%
- No end-to-end compiler path test                → max 70%
- Syntax + AST work but emitter partial           → max 65%
- (Apply strictest cap)

Strictest cap: 65%
```

### Step 3: Final score is minimum of estimate and cap

```txt
Final score = min(70%, 65%) = 65%

Honest status: "Parser and AST working; X3IR emission incomplete for 3 operations"
```

## Common Combinations

### Bridge Settlement Feature

```txt
Estimated score: 75%

Applicable caps:
☐ Bridge logic has no replay test               max 55%
☐ Bridge logic has no timeout test              max 55%
☐ Bridge logic has no invalid proof test        max 50%
☐ Asset/transfer operation untested             max 50%

Strictest cap: 50%
Final score: min(75%, 50%) = 50%
Status: "Settlement flow works; replay, timeout, and invalid proof paths untested"
```

### Runtime Storage Change

```txt
Estimated score: 80%

Applicable caps:
☐ Runtime path not reachable                    max 60%
☐ No migration documented                       max 65%
☐ Asset operation untested                      max 50%

Strictest cap: 50%
Final score: min(80%, 50%) = 50%
Status: "Storage updated; needs migration plan, invariant tests, and proof that old storage can upgrade safely"
```

### Cross-VM Feature

```txt
Estimated score: 85%

Applicable caps:
☐ No Cross-VM trace documented                  max 60%
☐ Cross-VM atomicity unclear                    max 65%
☐ Bridge logic has no replay test               max 55%
☐ Bridge logic has no timeout test              max 55%
☐ Asset operation untested                      max 50%

Strictest cap: 50%
Final score: min(85%, 50%) = 50%
Status: "Feature written; needs Cross-VM trace, atomicity proof, replay/timeout tests, and invariant tests"
```

## Rule of the Matrix

**You cannot escape the caps by writing more code.**

If your cap is 50%, you can write twice as much code, but your score stays 50% until you address the blocking gap (e.g., missing tests).

Fix the gap first. Then your cap increases.

## Approval Checklist

Before finalizing score:

- [ ] Estimated score is honest (backed by evidence)
- [ ] All applicable caps are checked
- [ ] Strictest cap is identified
- [ ] Final score = min(estimate, cap)
- [ ] Status text explains why the cap applies
- [ ] Path to raising score is clear

If any box is unchecked, score needs revision.

---

**When this skill is complete:** You have an honest, evidence-backed score that reflects true completion.
