# Universal economic platform execution record

`MASTER_REQUEST.md` preserves the complete authorized scope. Generate its source coverage
index with `python3 scripts/x3_universal_catalog.py`; verify freshness with `--check`.
`FEATURE_MATRIX.toml` and `FEATURE_REGISTRY.toml` remain the canonical readiness sources.
Unmapped rows are BLOCKED pending audit, not claims that existing code is missing.
The catalog includes every nonempty requirement line outside illustrative code blocks.
It is an intake index; per-feature audit, dependencies, acceptance evidence, and business
prioritization remain unfinished.

## Active milestone: genuine liquidity and execution

Existing dirty work disables synthetic Rust router quotes and adds a Base 0x adapter.
The current hardening rejects incomplete upstream simulation, uint256 output overflow,
and minimum outputs below the authorized slippage policy. Integer rounding must not
permit additional loss. These checks validate provider fields; they do not independently
prove calldata semantics, chain execution, finality, ProfitProof, or realized revenue.

Build with the installed Node 24 runtime:

```bash
PATH=/home/lojak/.nvm/versions/node/v24.21.0/bin:$PATH npm --prefix apps/dex run build
npm --prefix apps/dex test
python3 -m pytest tests/test_universal_catalog.py tests/test_feature_matrix.py -q
python3 scripts/x3_universal_catalog.py --check
```

## Next ten dependency-ordered actions

1. Map catalog entries to existing canonical feature IDs and inspect their execution paths.
2. Reconcile open liquidity/compiler PRs and exact-head CI with this dirty checkout.
3. Provision adequate build disk capacity and complete workspace quality gates.
4. Verify the 0x request/response contract against current official provider documentation.
5. Obtain a configured quote-provider account and prove live executable quotes.
6. Bind approved transaction destinations and independently validate calldata constraints.
7. Add a supported testnet execution path without enabling mainnet spending.
8. Execute an authorized funded testnet trade and record balance and fee reconciliation.
9. Verify receipt inclusion, reorganization behavior, and finality against trusted chain state.
10. Feed verified outcomes into canonical economics, then submit a focused verified PR.

No production route activation, real-money execution, deployment, or merge is authorized
by successful local tests. No defensible whole-platform completion percentage is available.
