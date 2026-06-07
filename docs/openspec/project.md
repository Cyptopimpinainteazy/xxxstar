# Project Context

## Purpose

X3 Chain is a Substrate-based Layer-1 chain that targets dual execution environments (EVM + SVM) behind a single canonical ledger. The X3 Kernel pallet coordinates atomic "Comit" transactions, authorization, and asset metadata/registry.

Current reality:

- On-chain execution runs the runtime WASM (`no_std`), so VM execution is currently mocked at the adapter boundary.
- Native (`std`) builds include real adapter implementations used for native testing and developer workflows.


## Tech Stack

- Rust + Cargo for the node binary, runtime, pallets, adapters, and CLI tooling (targets include `wasm32-unknown-unknown`).
- Substrate/FRAME runtime with Aura block production (~6s blocks) and GRANDPA finality.
- Runtime pallets: X3 Kernel, Atomic Trade Engine, Evolution Core, X3 Verifier, governance/treasury primitives, and supporting agent/account pallets.
- RPC: `jsonrpsee`-based HTTP JSON-RPC with runtime APIs for custom methods.
- Frontend tooling built on Next.js 14 + React 18 + Tailwind CSS, powered by Zustand, @tanstack/react-query, SWR, and ethers for browser interactions.
- Node.js (>=20) scripts for orchestration, the BMAD Method integration (`crates/vibe-bmad`), and Next.js builds.
- Supporting CLIs: `subkey`, `subxt`, `node`/`npm`, and OpenSpec itself for spec-driven requirements.

## Project Conventions


### Code Style

- Rust code follows Substrate + Polkadot conventions: use `cargo fmt --all` before commits, enforce `cargo clippy --all-targets -- -D warnings`, prefer `Result` propagation via `?`, and keep runtime logic inside pallets/modules rather than free functions.
- Frontend/JS code uses TypeScript, runs ESLint/Prettier via the Next.js stack, and keeps styling in Tailwind CSS utilities; avoid introducing new CSS frameworks without a clear justification.
- Documentation and specs are maintained inside `openspec/changes` and `openspec/specs`; every new capability must have a proposal, delta spec, and task list before implementation.

### Architecture Patterns

- Layered runtime: core orchestration in `pallets/x3-kernel/`, runtime wiring in `runtime/src/lib.rs`, and node service + RPC plumbing in `node/src/`.
- Dual-execution via adapter traits: runtime selects real adapters in `std` builds and mock adapters in `no_std` (WASM) builds.
- Canonical ledger and `Comit` flow (high-level): validate payload sizes, check authorization, execute via adapters, verify `prepare_root` against inputs (intentional design), then finalize by updating canonical ledger state and emitting events.
- Node RPC is implemented with `jsonrpsee` in `node/src/rpc.rs`, merging multiple modules into a single server.

### Testing Strategy

- Run `cargo test --all`, targeted pallet suites like `cargo test -p pallet-x3-kernel`, and `./RUN_ALL_TESTS.sh` for full integration coverage.
- Enforce formatting/linting with `cargo fmt --all` and `cargo clippy --all-targets --all-features -- -D warnings` as part of CI.
- Validate OpenSpec proposals via `openspec validate <change-id> --strict` before implementation; spec scenarios must be concrete and executable.
- Expect local tooling to have their own scripts (`npm run test`, etc.) where relevant; new UI work should ship with storybook or React testing as needed.

### RPC Surface (Current)

X3 Chain currently exposes custom JSON-RPC methods implemented in `node/src/rpc.rs`. The set includes:

- `system_accountNextIndex`
- `atlasKernel_*` (canonical balance, asset metadata, authorization, authorities)
- Minimal Ethereum compatibility: `eth_chainId`, `eth_gasPrice`, `eth_blockNumber`
- Health endpoints: `system_health`, `system_version`, `system_ping`
- `atomicTrade_*`, `evolutionCore_*`, `x3Verifier_*`
- Subscription handlers: `chain_subscribeNewHeads`, `chain_subscribeFinalizedHeads` (WebSocket exposure may still be environment/config dependent)

### Git Workflow

- Branch from `main` for each change, use descriptive commit messages, and reference related issues/PRs.
- Create feature/fix proposals under `openspec/changes/<change-id>/` before touching runtime or protocol behavior; update `tasks.md`, delta specs, and designs as required.
- Always run the relevant test suite (runtime, node, or frontend) locally before pushing; the GitHub Actions CI will rerun `cargo build`, `cargo test`, and WASM checks.
- Keep working trees clean; do not reset or revert unrelated changes unless the user explicitly asks.

## Domain Context

- X3 Chain is a heterogeneous blockchain: Substrate runtime with Aura + GRANDPA consensus, a canonical ledger (X3 Kernel), and two VM adapters (Frontier-based EVM + SVM bridge) that aim to execute within the same block for atomic cross-domain transactions.
- The key security focus is on account authorization (only authorized accounts can submit `Comit`s) and matching `prepare_root` values, ensuring finality across VM executions without trusted intermediaries.
- The project bundles CLI tools, wallet/explorer frontends, a dex playground, and a BMAD-powered planning workflow, all grounded on dual-VM interoperability and deterministic state transitions.

## Important Constraints

- WASM runtime builds are fragile (`InvalidTableReference(128)` appears if dependencies drift); every contribution must respect the pinned Substrate revision (commit `948fbd2`) and `patches/` folder overrides.
- On-chain (WASM) builds use mock VM adapters; do not assume real EVM/SVM execution paths exist in production runtime behavior yet.
- Authorization checks default to strict mode; the `dev-bypass` feature is only for development and must never ship in production builds.
- `Comit` payloads must stay within the enforced size limits (≤16KiB per payload; combined limits exist, including a larger v2 combined cap).

## External Dependencies

- Substrate (FRAME pallets, node/cli helpers) and Polkadot primitives (Aura, GRANDPA, SCALE codec) underpin the runtime.
- Frontier EVM adapter and Solana rBPF/SVM bridge libraries live in `crates/` and are orchestrated through adapter traits; dependencies include `evm`, `solana_rbpf`, and `parity-scale-codec`.
- Frontend stack depends on Next.js 14, React 18, Tailwind CSS, Zustand, @tanstack/react-query, SWR, ethers, and related tooling across `apps/{wallet,explorer,dex}`.
- Planning & automation integrate BMAD (Node.js-based) in `crates/vibe-bmad`; Node 20+ is required along with npm/yarn for scripts.
- CLI/key tooling uses `subkey`, `subxt`, `openssl`, `cmake`, and standard Linux packages documented in the README prerequisites.
