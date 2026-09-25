# The fuzz suite is nominal, and it does not build

2026-09-25. Reached while looking for the next parser to damage. The repository has a fuzz suite, and
the last report recommended adding a target to it. Before doing that I read the suite. Every number
below is measured; the verdict is that the suite is not fuzz coverage, and that no target in it can
be compiled from this tree.

## What is there

```
tracked fuzz targets                              48   (14 fuzz workspaces)
identical stubs (TODO + `let _ = data`)            30
hand-rolled no-ops ("Simulate safe intent…")       18
targets that call a decoder of any kind             0
targets whose fuzz/Cargo.toml does not exist        2
```

The 30 stubs are byte-for-byte the same file: a `fuzz_target!` body whose own comment says *"This is a
basic template. Add specific structure imports and tests as needed."* and whose entire implementation
is `let _ = data; // Prevent unused variable warning`.

The other 18 are not better, only differently written. `crates/x3-intent/fuzz/fuzz_targets/intent_decode.rs`,
for instance, reads a version nibble out of `data[0]`, checks `data.len()` in two places, and says so
in its own comments: *"Simulate safe intent parsing"* and *"Real X3 would use codec::Decode or
similar"*. It never calls a decoder. No target in the suite does — `grep -l 'Decode::decode'
$(git ls-files '*fuzz/fuzz_targets/*.rs')` returns nothing.

Two targets have no manifest at all and can never be built: `crates/x3-intent/fuzz/fuzz_targets/intent_decode.rs`
and `crates/x3-proof/fuzz/fuzz_targets/bridge_proof_verify.rs`.

## They do not compile

`cargo check` on a fuzz workspace fails, and it fails the same way in every one I tried — three of
them, chosen across the tree:

```
pallets/x3-atomic-kernel/fuzz   error[E0425]: cannot find value `ext` in this scope
                                (macro `environmental::environmental!` expansion)
pallets/x3-vrf/fuzz             error: cannot find macro `thread_local` in this scope
pallets/treasury/fuzz           error: cannot find macro `thread_local` in this scope
```

**Running cargo there has a side effect worth knowing about.** The three `cargo check` commands above
rewrote the committed lockfiles underneath them — `pallets/x3-atomic-kernel/fuzz/Cargo.lock` came out
1,178 lines longer and 1,794 shorter, and `pallets/x3-vrf/fuzz/Cargo.lock` was *created* where the tree
had none. They were restored before this commit, and `runtime hash freshness` reported them as
runtime-affecting files in the meantime, because the gate watches the package graph rather than the
target that builds. Anyone who runs a fuzz target here will pick up that diff by accident.

`environmental` and `getrandom` are two of the crates the *root* workspace patches — `Cargo.toml`'s
`[patch.crates-io]` section carries local fixes for both. A `[patch]` section applies to a workspace
root only, and each `*/fuzz/Cargo.toml` declares its own `[workspace]` precisely so it can build
standalone. They therefore get the unpatched crates and fail, on a dependency the rest of the
repository has already solved.

Two more measured facts about the same directory tree, from the census a cycle earlier: 13 of the 48
fuzz `Cargo.lock` files are in the 17 that fail `cargo metadata --locked`, and no gate in
`scripts/local-ci.sh` names a fuzz workspace at all. `cargo-fuzz` is not installed on this box
(nightly *is* available), so nothing here could run a fuzzer even if one built.

## And the SDK pin is different

The fuzz manifests declare:

```toml
sp-core = { default-features = false, git = "https://github.com/paritytech/polkadot-sdk", tag = "polkadot-stable2603" }
```

The rest of the repository is on `branch = "stable2512"`. A suite that did build would be fuzzing
against a different Substrate than the one this chain compiles — the same class of defect as the
`libp2p-yamux` and `evm` divergences from earlier cycles, in a place where it is easy to miss because
nothing builds.

## What changed

One gate, deliberately a ratchet rather than a verdict: `fuzz targets` fails when a target that is not
on `security/fuzz-placeholder-baseline.txt` grows a `TODO` or loses its manifest, and when a listed
target is fixed (so the file cannot describe a tree that no longer exists). It is proven both ways:
appending a `// TODO` to `pallets/x3-atomic-kernel/fuzz/fuzz_targets/fuzz_codec_parsing.rs` fails the
check, and removing it passes.

The baseline holds all 32 placeholders, and it may only shrink. It does **not** claim the other 16 are
good — a target without the marker can still fuzz nothing, and most of them do; the report is where
that is written down. What the ratchet buys is that the suite cannot grow while it is already nominal,
which is how all thirty stubs arrived.

## The decision (this needs the maintainer, not an agent)

The repository has 48 fuzz targets, 13 stale locks, a different SDK pin from production, no runner and
no gate. That is not a state to incrementally patch. Two honest paths:

1. **Make it real, starting from one place.** Align the manifest with the root (same `sp-core`
   revision), add the `[patch.crates-io]` entries a nested workspace needs, install `cargo-fuzz` where
   the gates run, and write *one* real target for the atomic path — the pallet whose invariants the
   roadmap asks to prove. Then apply the pattern. The cost is a per-workspace lock refresh, which the
   census already measured as cascading (one fuzz workspace moved 525 packages to 637).
2. **Delete it.** If nobody is going to run a fuzzer, the workspaces are not coverage; they are a
   claim of coverage, and their stale locks and unbuildable manifests cost time every time someone
   touches the tree.

I have not taken either decision: deleting 48 files is the maintainer's call, and doing path 1
halfway — a target that compiles nowhere, run by nothing — would repeat the problem this report is
about. The ratchet holds the line in the meantime.
