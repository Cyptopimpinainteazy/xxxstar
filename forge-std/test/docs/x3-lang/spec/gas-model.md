# X3 VM Gas Model (v0.1)

Purpose: Gas model defines deterministic pricing for opcodes, memory growth, cross-VM operations, and JIT hints.

Principles:
- Costing is deterministic and architecture-independent.
- Gas is taken prior to instruction execution.
- Gas costs should prefer simple integer math to avoid FP nondeterminism.

Base per-opcost (suggested initial table)
- 1 unit: cheap operations (register move, small arithmetic)
- 2 units: arithmetic + memory referencing
- 5 units: memory load/store
- 10 units: cryptographic hash of register-size data
- 50 units: signature verification
- 100 units: external bridge call to another VM
- 500 units: cross-chain commit

Memory growth
- 10 units per 64KiB page growth.
- Additional per-byte cost for zero-initialized pages can be added per runtime policy.

Branch penalties and mispredict
- No dynamic branch mispredict penalty to remain deterministic across platforms.

Cross-VM call cost
- EVM_CALL / SVM_CALL: base cost 100 units + per-argument size cost (1 unit per 32 bytes)
- ATOMIC_BEGIN / ATOMIC_COMMIT additional fixed cost 250 units to account for proof generation.

Gas refunds
- Deterministic refunds allowed but must not rely on external state; e.g., for `drop` of large owned memory types.

On Out-Of-Gas
- A deterministic trap triggers, with the VM rolling back all ephemeral state to the previous atomic commit point.

Example: ADD_RRR (register add)
- Cost: 1 unit
- Behavior: read registers, add, write register result.

Persistence and proof
- Gas accounting stored in the transaction receipt; clients verify correct gas consumption on receipt validation.
