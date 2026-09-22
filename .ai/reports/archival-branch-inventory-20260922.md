# Archival branch inventory — what the unmerged pile actually contains

Date: 2026-09-22. Measured against `origin/master` = `1ed282487` (later `46e65d215`,
then `f1f857922` after the two salvages below). Question asked: *is there good work
sitting on the archival branches?*

Method: every remote branch not merged into `origin/master` (`git branch -r
--no-merged origin/master`, 142 refs) was measured three ways, each one stricter
than the last. Branch names were ignored.

1. **Patch identity** — `git rev-list --left-right --cherry-pick --no-merges
   origin/master...origin/<b>`: does a commit already in master carry this patch-id?
2. **File content** — for each file the branch changed against its merge base, is
   the branch's copy byte-identical to master's? If not, did master change that file
   after the fork (contested), or is the branch's change absent from master entirely?
3. **Symbol identity** — every `fn|struct|enum|trait|const|static` the branch adds in
   a `.rs` file, checked against a set built from master's whole `*.rs` tree
   (118,594 symbols).

Scripts: `/tmp/x3-archival-scan.sh`, `scan2.sh`, `scan3.sh`; raw output in
`/tmp/x3-archival-report.txt`, `/tmp/x3-archival-salvage.txt`,
`/tmp/x3-archival-unseen.txt`.

## The pile, by measurement

| Bucket | Branches | Meaning |
|---|---|---|
| Patch-equivalent | 68 | the same patch-id already exists in master |
| Redundant at file level | 49 | every changed file's content already on master (1 branch byte-identical on every file) or the file was rewritten on master after the fork (48 contested) |
| Carries something absent | 19 | at least one changed file whose content master never took |
| — of those, one consolidation snapshot | 1 | `wip/consolidation-20260917/main`: 581 of the 679 absent file-changes, dominated by committed build output (`site/_next/**`, `apps/*/out/**`, `dist/**`) |

Symbol sweep over the 74 non-patch-equivalent branches: **138 added Rust symbols
exist nowhere on master, spread over 27 branches** — overwhelmingly test function
names (`a_decided_contract_is_refused_at_build_because_the_generator_does_not_exist`,
`halted_chain_rejects_new_bundle_submission`) and whole-tree snapshot content.

## Every critical-path candidate, checked against master

| Candidate from the pile | Master's state | Verdict |
|---|---|---|
| second `arb` implementation, `compiler/src/arbitrage.rs` (699 lines) + its `allowed_chains`/`ChainNotAllowed` API and test | master's `compiler/src/arb.rs` names the branch and calls itself the surviving surface (TICKET-076); carried the graph-grounded validation over as `venue_standings` and deliberately refused the branch's `flash = enabled` treatment | superseded |
| coordinator replay tracking (`save_used_secret_claims`, `load_used_secrets`) | on master in `crates/cross-vm-coordinator/src/persistence.rs`, with `&[[u8; 32]]`/`&[([u8; 32], String)]` where the branch used `&Vec<…>` | present, stricter |
| proof-bundle emptiness + intent binding | on master, `crates/x3-atomic-swap/src/proof_bundle.rs:174`, `:322`, `:341` | present |
| `intent_bridge` amount/min_output parsing, unsupported-step error | on master; master's `field_amount` returns `Result<Option<u128>, X3Error>` where the branch returned `Option<u128>` with `unwrap_or(0)` — a silent fallback the repo's rules forbid | present, stricter |
| economic halt gate (branch: `TestEconomicHalt`, `set_halted`) | on master: `pallets/x3-atomic-kernel/src/lib.rs:750` (`!T::EconomicHalt::is_halted()` → `Error::EconomicHaltActive`), trait in `crates/x3-asset-kernel-types`, halt-capable mock in `pallets/x3-atomic-kernel/src/mock.rs` | present |
| refund terminality on both domains (`real_x3vm_evm_refund_is_terminal_on_both_domains`, `real_x3vm_svm_refund_is_terminal_on_both_domains`) | master's live tests already carry it: `node/tests/x3vm_evm_live.rs` 42 refund references, `x3vm_svm_live.rs` 47, `x3vm_live_lifecycle.rs` 53, and the cross-domain gates run them | present |
| `codex/x3-economic-safety-kernel` (verify.rs, emitter.rs, economic.rs, trading.rs, trading_lowering.rs) | every symbol the branch adds to those five files exists on master; one helper (`literal_u64`) does not, a test-side helper | present |
| SVM custody / secret-release-firewall clusters | that branch's non-doc "unseen" lines land in `scripts_infrastructure/pr_supervisor.py`, `scripts/agent_guard.py`, workflow docs and ProofForge JSON; the SVM leg runs green on master (`cross-domain SVM PASS`, 1039 s, this session) | no code gap found |
| 7-crate June prototype `prototypes/x3-lang-20260621/**` | master has `x3-lang/**` and `crates/x3-lsp/**` (backend, completion, hover, diagnostics, semantic) | superseded |
| `scripts/x3-proof-check.sh` dropping `|| true` | cosmetic: `run_check` counts `PASS`/`FAIL` itself and the summary exits 1 when `FAIL > 0`, so the `|| true` never hid a failure | noise |
| dependabot workflow bumps (`actions/checkout@v7` across 36 workflows, `configure-pages@6`, nonmajor) | master pins a mix of v3/v4/v5 and a v4.2.2 SHAs | mechanical, not landed |
| `primitive-types 0.12.2 → 0.13.1` (evm-integration), `k256 0.13 → 0.14` (x3-atomic-swap) | master's lock already carries `primitive-types 0.13.1` from elsewhere; but four crates pin `k256 0.13.4` (one exactly `=0.13.4`), so the k256 bump splits a crypto version, unverified | not landed |
| `.github/workflows/queue-drain.yml` (69 lines) | ops tool hard-coded to PRs 130/181 | not landed |
| `CONTRIBUTING.md` (53 lines, absent on master) | content asserts the Python pipeline is "the authoritative MVP surface" and names `tests_phase4/`, `proof/` — drifted | not landed |
| `crates/confidential-gpu/Cargo.lock` | that crate is a root-workspace member (no `[workspace]` in its manifest), so a nested lock is wrong; the root lock is authoritative | not landed |

## What was real, and is now on master

Two defects in the pile were reproduced on master and fixed.

**1 — `--chain x3-local3-raw` is not a chain id (PR #428).**
`node/src/chain_spec.rs` resolves ids from a fixed list (`dev`, `local`, `local3`,
`staging`, `testnet`, `production`) and treats anything else as a JSON path. The
name of the file `chain-specs/x3-local3-raw.json` was being resolved as a file with
that exact extension-less name:

```
$ ./target/debug/x3-chain-node build-spec --chain x3-local3-raw --raw
Error: Input("Failed to read chain spec file x3-local3-raw: No such file ...")   # exit 1
$ ./target/debug/x3-chain-node build-spec --chain local3 --raw                   # exit 0, 17214398 bytes
$ ./target/debug/x3-chain-node build-spec --chain chain-specs/x3-local3-current-raw.json --raw   # exit 0
```

`docs/Zombienet-template.toml` names that id in `[relaychain] chain`, and
`tests/zombienet/finality-smoke.zndsl` loads the template, so that leg could not
have started a network. Three `benchmark pallet` calls in
`frame-benchmarking.yml` passed the same name as an id. Fixed in all four places
(the archival branch `wip/prompts-to-skills-20260918` and its four twins had the
template fix).

**2 — `cargo test -p x3-parser --test golden` races itself (PR #428).**
`generate_golden_fixtures` writes the checked-in fixture files; `test_golden_fixtures`
reads them; both are `#[test]` in one binary on the harness's parallel threads.
Measured on master: the reader lost 2 runs in 15 and saw a file the writer had just
truncated —

```
assertion `left == right` failed: Fixture 07_shadowing_scoping_return failed golden test
  right: ""
```

The generator is an update tool, not an assertion, so it is `#[ignore]`d and still
runnable (`cargo test -p x3-parser --test golden -- --ignored
generate_golden_fixtures`). 15 consecutive plain runs pass afterwards, and a plain
run no longer rewrites tracked files. `make guard` passes with it.

Also landed this session (in-flight work from the previous one):

**3 — the cross-domain SVM gate never reached the SVM path (PR #429).**
`cargo build-sbf` shells out to `cargo +1.89.0-sbpf-solana-v1.54`, and `+toolchain`
only works through the rustup shim, which `local-ci.sh` puts behind the pinned
toolchain on purpose. The `SVM contract lifecycle` gate already answered this with
`env PATH="$HOME/.cargo/bin:$PATH"`; `cross-domain SVM` did not, so it died with
`error: no such command: +1.89.0-sbpf-solana-v1.54`. With the shim: `PASS
cross-domain SVM 1039s`.

## Commands

```bash
# the three scans (rebuild the branch list first)
git branch -r --no-merged origin/master | grep -v 'origin/HEAD' | sed 's|origin/||' | sort > /tmp/x3-unmerged-branches.txt
bash /tmp/x3-archival-scan.sh && bash /tmp/x3-archival-scan2.sh

# the two reproductions
./target/debug/x3-chain-node build-spec --chain x3-local3-raw --raw; echo $?   # 1
for i in $(seq 1 15); do cargo test -p x3-parser --test golden || break; done # failed on master
```
