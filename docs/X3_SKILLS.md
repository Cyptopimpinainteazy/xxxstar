# X3 Skills

Summaries of all skill files in `.cline/skills/`.

## x3-repo-mapper.md
Map real repo structure: languages, build systems, test runners, runtime entry points, pallet lists, contract locations.

## x3-proof-auditor.md
Verify completion claims against actual command output. Final arbiter of whether something is done.

## x3-runtime-wiring-inspector.md
Prove code is wired into runtime, router, CLI, API, or pallet. File existence is not wiring.

## x3-cross-vm-reviewer.md
Review EVM/SVM/X3VM/BTC/CosmWasm routing correctness. Check atomicity, timeout, refund, replay, finality.

## x3-atomic-rollback-reviewer.md
Verify rollback, timeout, replay protection, refund paths, and finality for cross-chain ops.

## x3-security-reviewer.md
Check auth, signing, replay protection, unsafe code, secret handling, bridge safety, external calls.

## x3-test-gap-finder.md
Identify missing unit, integration, property, fuzz, and failure-path tests.

## x3-performance-reviewer.md
Inspect bottlenecks, benchmarks, parallelism, batching, GPU paths, TPS claims.

## x3-doc-sync-checker.md
Compare docs against source. Mark outdated claims. Documentation that lies is worse than none.

## x3-next-task-planner.md
Produce exactly 10 executable next tasks with proof criteria. Prevents wandering.