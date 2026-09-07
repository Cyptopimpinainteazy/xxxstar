#import "../style.typ": *
#import "../components.typ": *
#import "../data.typ": *

= Prioritized Recovery Plan

Every item below traces to a specific finding ID in `findings.json` — nothing here is invented backlog. Effort estimates assume a small team already familiar with this codebase (2–4 engineers), not a hypothetical large staffed team; no calendar promise is made beyond the ranges already stated in Chapter 1's 7/30/60/90-day plan.

== P0 — Security, Fund-Safety, Consensus, and Data-Integrity Blockers

#table(
  columns: (auto, 2.4fr, 1fr, 1.6fr),
  fill: (x, y) => if y == 0 { c-brand } else if calc.even(y) { c-bg-panel } else { white },
  [*ID*], [*Task*], [*Files*], [*Verification*],
  [CRIT-03], [Remove the `wallet_*` custodial RPC methods from the production RPC surface, or gate them behind an explicit dev-only flag that hard-refuses non-dev chain specs.], [`crates/x3-rpc/src/wallet_service_rpc.rs`; `node/src/rpc.rs:462-650`], [`wallet_*` methods return "Method not found" on `--chain testnet`/`mainnet-rc1`.],
  [CRIT-02], [Require real on-chain-verifiable evidence for `report_misbehavior`, mirroring the GRANDPA equivocation-proof pattern; or restrict origin to a privileged reporter with a dispute window.], [`pallets/x3-consensus/src/lib.rs:262`], [New test: bare call with no evidence is rejected; valid-proof call still succeeds.],
  [CRIT-01], [Rotate all 3 validator identities + the Sepolia key offline via `subkey`; run `git filter-repo`/BFG to purge history; force-push and require all collaborators re-clone.], [git history; `deployment/keys/*`, `sepolia-deployer-wallet.txt`], [`git log --all -- <path>` returns zero commits post-rewrite; `git fsck --unreachable` clean.],
)

== P1 — Public Testnet Blockers

#table(
  columns: (auto, 2.4fr, 1fr, 1.6fr),
  fill: (x, y) => if y == 0 { c-brand } else if calc.even(y) { c-bg-panel } else { white },
  [*ID*], [*Task*], [*Files*], [*Verification*],
  [HIGH-02], [Replace all DEX pricing math with integer/fixed-point arithmetic (`checked_mul`/`checked_div`, widen to U256 where needed), matching the supply-ledger's pattern.], [`crates/x3-dex/src/amm_pools.rs`], [Property test vs. a decimal reference across randomized reserves incl. near-`u128::MAX`.],
  [HIGH-05], [Re-run `scripts/run-srtool.sh build` on the audited commit with Docker present; commit the real `.log`/`.json` output.], [`launch-gates/evidence/substrate/`], [`find launch-gates/evidence -name '*.log'` returns >0 files matching valid checksums.],
  [HIGH-06], [Implement the `/health/<feature>` endpoints for real, or remove the `health_endpoint` field from `FEATURE_REGISTRY.toml` entries with no backing code.], [`FEATURE_REGISTRY.toml`; new handler code], [`curl` each declared path returns 200 + a real dependency-checked payload.],
  [MED-12], [Re-run the 8/8 cold-start and 7/7 kill-survival mesh tests across 3+ physically separate hosts.], [`scripts/testnet/run-mesh.py`], [Equivalent convergence/survival results on real hosts, not loopback.],
  [MED-11], [Broaden the secret-scan CI gate to the full `deployment/keys/` and `deployment/chain-specs/fresh/validator-keys/` trees, add a git-history scan step.], [`.github/workflows/ci.yml`], [A planted secret in newly-covered paths is caught by CI.],
)

== P2 — Mainnet and Operational-Hardening Work

#table(
  columns: (auto, 2.4fr, 1fr, 1.6fr),
  fill: (x, y) => if y == 0 { c-brand } else if calc.even(y) { c-bg-panel } else { white },
  [*ID*], [*Task*], [*Files*], [*Verification*],
  [HIGH-01], [Replace `pallet_collective::propose{threshold:1}` bridge mint framing with a real multi-key threshold, or relabel as a single-key admin path.], [`node/src/rpc.rs:1058-1093`], [Mint requires N-of-M independent signers before executing.],
  [HIGH-03], [Decouple `x3-vrf`'s `dev` feature from `std`; implement a real VRF (schnorrkel) behind `VrfProvider`.], [`pallets/x3-vrf/Cargo.toml`; `crates/x3-vrf/src/lib.rs`], [`cargo tree -e features --features std` shows `x3-vrf/dev` NOT resolved.],
  [HIGH-04], [Hard-fail relayer startup on a well-known dev seed against a non-dev chain-spec; implement threshold proof verification.], [`crates/x3-relayer/src/main.rs`], [Relayer refuses to start with `//Alice` against `--chain != dev/local`.],
  [HIGH-07], [Update `LAUNCH_SCOPE.md` to explicitly state permissionless staking is deferred, with the same prominence as other gated-out features.], [`LAUNCH_SCOPE.md`], [Doc review only — no code change required.],
  [MED-01], [Relabel the 10 no-op migrations honestly, or add one real `try-runtime` rehearsal proving the pattern.], [`pallets/*/src/migrations.rs`], [`try-runtime` upgrade test against an actually-changed storage shape.],
  [MED-03], [Add dispatchable `propose_transaction`/`sign_proposal`/execute calls wiring the existing multisig engine into the runtime.], [`pallets/x3-wallet-pallet/src/lib.rs`], [Integration test: full M-of-N flow through real signed extrinsics.],
  [MED-08], [Build a real Anchor test suite (failure paths, replay, unauthorized signer, overflow) for the 6 SVM programs.], [`X3-contracts/svm/programs/*`], [Adversarial test suite passing per program, run in CI.],
  [MED-09 / MED-10], [Add dependency-checked `/ready` endpoints to gateway, sidecar, bot; fix analytics-service's `/health` to stop hardcoding DB status.], [`crates/x3-gateway`, `crates/x3-sidecar`, `apps/x3-bot`, `apps/analytics/.../handlers.rs`], [Endpoint returns non-200 when the real dependency is down in a test environment.],
  [MED-07], [Gate first funded EVM Treasury deployment on migrating to a Safe multisig owner.], [`X3-contracts/evm/contracts/treasury/Treasury.sol`], [On-chain `owner()` resolves to a Safe address before any funds are routed.],
)

== P3 — Optimization and Cleanup

#table(
  columns: (auto, 2fr, 1.5fr),
  fill: (x, y) => if y == 0 { c-brand } else if calc.even(y) { c-bg-panel } else { white },
  [*ID*], [*Task*], [*Files*],
  [MED-02], [Add a benchmarked `on_finalize` weight or amortized sweep to `x3-supply-ledger`, tied to `MaxAssets`.], [#text(size: 8pt)[`pallets/x3-supply-ledger/src/lib.rs`]],
  [MED-04], [Delete the dead `TransactionSigner` module, or implement real verification before it is ever wired up.], [#text(size: 8pt)[`crates/x3-wallet/src/transaction_signer.rs`]],
  [MED-05], [Wire `private-mempool` into `node/src/service.rs`'s real pool construction, or remove the capability claim.], [`crates/private-mempool`; `node/src/service.rs`],
  [MED-06], [Correct `TREASURY_POLICY.md`'s factually wrong `routeFee` claim.], [`TREASURY_POLICY.md`],
  [LOW-01..07], [Constant-time hash comparisons in biometric unlock; checked arithmetic on wallet receiver credit; test-name accuracy; rename "PoAE"; align x3-lang JIT claims; make benchmark gate blocking or document as advisory; update README's CI job count.], [see `findings.json` for each],
)

== Critical Path

CRIT-03, CRIT-02, and CRIT-01 have no dependency on each other and can proceed fully in parallel. HIGH-01 and HIGH-04 both depend conceptually on deciding the bridge's production trust model (multi-key threshold design) before either fix is finalized — treat them as one design decision with two implementation sites. HIGH-02 (DEX float math) is fully independent and can start immediately. Everything in P2/P3 can proceed in parallel with P0/P1 except MED-01's `try-runtime` rehearsal, which benefits from HIGH-05's srtool evidence existing first so the rehearsal itself is reproducible.
