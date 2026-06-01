# x3-lang v0.1 — Minimal parser and pipeline

This folder contains a minimal, pragmatic starting point for the X3 Lang MVP.

Files added:

- `cli.py` — tiny CLI that parses a small `intent` DSL and emits JSON.
- `registry.py` — small asset, DEX, and bridge registry for validation.
- `typechecker.py` — validates the JSON bridge format and known symbols.
- `planner.py` — converts parsed intent JSON into a step-by-step execution plan.
- `simulator.py` — estimates gas, bridge fees, slippage, and expected profit.
- `emitter/` — simple EVM/SVM/X3 emission skeletons for generated plans.
- `schema.json` — JSON Schema for the bridge format.
- `runner.py` — end-to-end pipeline from DSL to planning, simulation, emission, and constraint evaluation.
- `tests/` — pytest coverage for parser, schema validation, typechecker, planner, simulator, and runner.
- `examples/arb_solana_eth.x3` — example intent demonstrating Solana->X3->Ethereum flow.

Quick start

1. Run the parser against the example:

```bash
python3 x3-lang/cli.py x3-lang/examples/arb_solana_eth.x3
```

2. Run the full pipeline:

```bash
python3 x3-lang/runner.py x3-lang/examples/arb_solana_eth.x3
```

3. Run tests:

```bash
python3 -m pytest -q x3-lang/tests -q
```

Notes and next steps

- This is intentionally tiny: it supports `intent`, `from`, `to`, `path` (with `swap` and `bridge`), and `constraints` entries used in the MVP spec.
- The runner wires parser → typechecker → planner → simulator and reports constraint results.
- Next work: add emitters for EVM/Solana/X3, richer simulation, and actual rollback/atomic semantics.
