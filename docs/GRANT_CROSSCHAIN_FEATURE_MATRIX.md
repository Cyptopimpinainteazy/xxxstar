# Grant feature matrix

Evidence scope: repository branch `feat/x3-lang-crosschain-integration-20260909`.
“Implemented” means code exists. “Proven” requires a passing check on the exact branch head.
No row below claims Ethereum, Solana, or any other public-chain deployment.

| Feature | Code status | Proof status | Evidence / limit |
|---|---|---|---|
| Parse supported `.x3` gateway source | Implemented | Existing unit tests; branch rerun pending | `x3_compiler::lower_gateway_call` accepts literal `xvm_transfer` and one flat `submit_atomic_bundle` leg. |
| Compile ordinary `.x3` source to X3 bytecode | Implemented on this branch | Pending CI | `x3_integration::compile_source` invokes the workspace compiler and emits canonical `X3BC` bytes. |
| Runtime-load compiled bytecode | Test added | Pending CI | Integration test decodes compiler output with `BytecodeModule::from_bytes`. |
| Versioned runtime bytecode | Implemented before this branch | Existing tests; branch rerun pending | X3BC header carries semantic version, flags, checksum, minimum version, and feature flags. |
| Signed outer bytecode envelope | Not implemented | None | X3BC has a checksum, not an origin signature. Do not describe compiled programs as signed yet. |
| X3-internal Native/EVM/SVM routing | Implemented | Existing pallet tests; branch rerun pending | Router has typed domains, expiry, refund, replay guards, and supply-ledger calls. |
| Compiled bridge instruction dispatched through atomic kernel | Partial | Existing gateway/router test does not prove a signed bytecode-to-kernel path | Gateway lowering emits typed calls, but a signed compiler-envelope dispatcher is still required. |
| Atomic commit/rollback | Implemented for X3 runtime state | Existing tests; exact branch proof pending | This does not make external-chain finality atomic. |
| Timeout and refund | Implemented in router | Existing tests; exact branch proof pending | Applies to the runtime transfer lifecycle. |
| Replay protection | Implemented in router | Existing tests; exact branch proof pending | Nonce and message-state checks exist. |
| Canonical supply invariant | Implemented in ledger/router | Existing tests; exact branch proof pending | Represented supply must not exceed canonical supply. |
| EVM local-network proof | Environment exists | Cross-chain proof not yet recorded on this branch | Anvil is configured in the validator compose file. RPC health alone is not a trade proof. |
| SVM local-network proof | Environment exists | Cross-chain proof not yet recorded on this branch | `solana-test-validator` is configured. No public Solana claim. |
| Public-chain deployment | Not performed | None | Explicitly out of scope for this grant demonstration. |

## Current completion estimate

The requested compiler-to-atomic-kernel funding demonstration is **45% complete** on an evidence-weighted basis. The source compiler handoff and runtime-load test are now present, while the signed envelope, authenticated dispatcher, exact-head CI, and local Anvil/Solana execution receipts remain open.

## Claim language

Safe: “X3 has an implemented internal cross-VM router and a compiler integration under test, with local-network EVM/SVM proof work in progress.”

Not supported: “X3 already performs trustless atomic swaps across Ethereum and Solana mainnet.”
