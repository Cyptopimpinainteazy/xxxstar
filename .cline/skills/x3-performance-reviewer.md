# Skill: X3 Performance Reviewer

## Purpose
Inspect bottlenecks, benchmark coverage, parallelism, batching, GPU paths, and TPS claims.

## Use When
- Before claiming performance characteristics.
- When optimizing critical paths.
- When making claims about TPS, latency, or throughput.

## Inputs To Inspect
- `pallets/` — hot-path pallet logic.
- `runtime/` — runtime dispatch.
- `crates/confidential-gpu/` — GPU validator.
- Benchmark configurations.
- `Makefile` bench targets.

## Checks To Perform
- Are benchmarks available for the modified code?
- Are O(n) operations bounded?
- Is parallelism/batching used where appropriate?
- Are GPU paths actually invoked (not just compiled)?
- Are TPS claims backed by benchmark data?

## Proof To Require
- Benchmarks run and produce numbers.
- No unbounded loops in hot paths.
- GPU path exercised in benchmarks.

## Output Format
- Benchmarks: PRESENT / MISSING
- Hot path complexity: O(1) / O(n) bounded / O(n) unbounded (WARN)
- GPU path: WIRED / COMPILED-ONLY / NONE
- TPS claim: BACKED by <benchmark> / UNVERIFIED