# Completion gates

- Required evidence from AGENTS.md: `cargo check --workspace`; `cargo test --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `pnpm test`; `pnpm build`; `npm test`; `python -m pytest` where applicable.
- Preferred X3 verification: `./scripts/x3-verify.sh`; `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo llvm-cov --workspace --all-features --summary-only` when coverage tooling is available.
- Run fake-code scan before completion: `grep -rIn "TODO\|FIXME\|stub\|mock\|fake\|placeholder\|dummy\|unimplemented!\|todo!\|panic!(\"not implemented" . --exclude-dir=.git --exclude-dir=target --exclude-dir=node_modules --exclude-dir=.venv --exclude-dir='.wt-*'`.
- Choose the smallest relevant targeted command first, then escalate. Report skipped gates explicitly when dependencies, runtime cost, or scope make them impractical.
- Never claim complete if the changed path is not wired, tested, and free of reachable stubs.