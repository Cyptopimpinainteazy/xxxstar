# Rule: Proof Before Claim

## Purpose
No completion claim is valid without proof from actual source, tests, wiring, and command output.

## Required Behavior
- Before saying anything is "done", run the strongest proof command available.
- For Rust: `cargo check`, `cargo test`, `cargo clippy` on the relevant crate(s).
- For Solidity: `forge test` or `npx hardhat test`.
- For TypeScript/Node: `npm run build`, `npm test`, `npm run lint`.
- For Python: `python -m compileall`, `pytest`.
- Reference the exact command output. Do not paraphrase.
- If proof fails, say PARTIAL or FAILED — never pretend it passed.

## Forbidden Behavior
- Do NOT claim something works without running a verification command.
- Do NOT rely on "it compiles" alone as proof.
- Do NOT skip running tests because "they should pass."
- Do NOT run proof commands and then ignore their failures.
- Do NOT tell the user "tests pass" when they don't.

## Proof Required
- Paste the proof command and its exit code.
- Show test counts or build success indicators.
- If output is too long, show summary + saved location.