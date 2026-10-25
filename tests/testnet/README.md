# X3 seven-validator smoke test

This is a **read-only** RPC checker. It does not launch nodes, delete chain state, alter validator keys, or submit transactions.

## Usage

From the repository root, with an already running local seven-validator network:

```bash
python3 tests/testnet/check_seven_validators.py --nodes 7 --settle-seconds 12 --output /tmp/x3-seven-validator-report.json
```

The default RPC endpoints are http://127.0.0.1:9950 through :9956. Use `--host` and `--base-port` for a different topology. For validators on separate machines, use an SSH tunnel or run the checker from a secured management host with appropriate RPC routing. Do not expose unsafe RPC publicly.

## Assertions

- Every configured endpoint answers RPC.
- Every node has at least the configured minimum peer count and is not syncing.
- At the minimum finalized height across nodes, all nodes return the same block hash.
- Finalized height advances between samples.

Exit status 0 means all checks passed; 1 means failure. JSON output includes observations and failures.

## Scope and limits

This smoke test does **not** prove Byzantine fault tolerance, chain-spec identity, safety under network partitions, transaction execution, economic invariants, or production readiness. Add dedicated controlled failure-injection and transaction suites before claiming those properties.

The existing `scripts/testnet/run-fresh-validators.sh` removes prior node data under `TESTNET_BASE`; never invoke it on a directory containing valuable state. The checker does not invoke it.
