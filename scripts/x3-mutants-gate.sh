#!/usr/bin/env bash
# X3 mutation gate — cargo-mutants on the supply ledger, bounded and loud.
#
# The verification harness (scripts/x3-verification-harness.sh) runs cargo-mutants as an
# operator-invoked section over crates/x3-common, crates/x3-fees and crates/x3-packet-schema. No
# gate ran mutants against the pallet that enforces the king invariant. The first campaign
# (2026-10-02, master 2cfa3f06) found two survivors in `on_finalize` proof pruning that the whole
# suite missed — the guard and the subtraction that keep `HistoricalProofs` bounded. Both are now
# pinned by tests_retention.rs (#576). This gate keeps the campaign running: every surviving
# mutant is a behaviour change no test observed, and the gate fails until a test pins it.
#
# Opt-in (--mutants), like --loom/--fuzz: a full campaign is minutes, not seconds.
#
# A mutant that turns a loop into a hang is run under cargo-nextest with the
# `mutants` profile (.config/nextest.toml), which kills a hung test after 60s:
# that is an ordinary test failure, so cargo-mutants classifies the mutant as
# CAUGHT. cargo-mutants' own TIMEOUT classification (outer --timeout) counts as
# NOT caught and fails the gate without pinning anything — two real
# non-terminating mutants exist today (SupplyMerkleTree::build_tree `> 1` ->
# `>= 1` / `== 1`), so the watchdog is load-bearing.
#
# Exit codes, matching the harness contract in scripts/x3-verification-harness.sh:
#   0  every mutant was caught
#   1  at least one mutant survived, or the campaign failed
#   2  cargo-mutants or cargo-nextest is not installed — BLOCKED, skip loudly
set -u

PACKAGES=(
  pallet-x3-supply-ledger
)
JOBS="${X3_MUTANTS_JOBS:-4}"
# An over-timeout mutant is classed TIMEOUT, not CAUGHT (cargo-mutants exit 3),
# so a tight cap under parallel load turns real signal into noise (observed:
# `--timeout 120` with 8 jobs build-timed-out the two pruning mutants the suite
# genuinely catches; contended builds here have taken 364s). Ten minutes stays
# bounded while letting contended builds finish; test hangs are cut off by the
# nextest watchdog at 60s instead of by this backstop.
TIMEOUT="${X3_MUTANTS_TIMEOUT:-600}"
# cargo-mutants has no flag to pass a nextest profile, but nextest reads this
# env var in the child process. The profile is defined in .config/nextest.toml.
NEXTEST_PROFILE="${X3_MUTANTS_NEXTEST_PROFILE:-mutants}"
# `benchmarking.rs` is `#![cfg(feature = "runtime-benchmarks")]`. Without the feature the
# module is not in the test build at all, so cargo-mutants "tests" mutated benchmark bodies
# that cannot run: they survive as no-ops and masquerade as survivors (observed 2026-10-02:
# 6 mutants in benchmarking.rs). Enabling the feature compiles the module and lets
# `impl_benchmark_test_suite!` execute the benchmarks, so those mutants get a real trial.
FEATURES="${X3_MUTANTS_FEATURES:-runtime-benchmarks}"

if ! cargo mutants --version >/dev/null 2>&1; then
  echo "x3-mutants: cargo-mutants is not installed (cargo install cargo-mutants --locked)"
  echo "x3-mutants: BLOCKED - no mutation campaign was run"
  exit 2
fi
if ! cargo nextest --version >/dev/null 2>&1; then
  echo "x3-mutants: cargo-nextest is not installed (cargo install cargo-nextest --locked)"
  echo "x3-mutants: BLOCKED - the hang watchdog needs nextest"
  exit 2
fi

rc=0
for package in "${PACKAGES[@]}"; do
  echo "x3-mutants: cargo mutants -p $package (features=$FEATURES, jobs=$JOBS, timeout=${TIMEOUT}s, test-tool=nextest profile=$NEXTEST_PROFILE)"
  if ! NEXTEST_PROFILE="$NEXTEST_PROFILE" cargo mutants -p "$package" --features "$FEATURES" --jobs "$JOBS" --timeout "$TIMEOUT" --test-tool nextest; then
    echo "x3-mutants: survivors or campaign failure in $package (see mutants.out/)"
    rc=1
  fi
done
exit "$rc"
