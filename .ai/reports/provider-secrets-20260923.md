# Four live credentials were committed to this repository

Date: 2026-09-23. Found while looking for where to put a DRPC key the operator had just bought.

## What was in tracked source

| where | what | state |
| --- | --- | --- |
| `crates/external-chains/src/env_config.rs` | an Alchemy key, a paid DRPC key, an Ankr key — **and a wallet private key with its address** | removed; credentials and wallet now come from the environment |
| `crates/external-chains/src/rpc.rs` | the same three provider keys, in the endpoint table | removed; paid endpoints are built from env, public ones stay in source |
| `infra/mcp-config.json` | a live Infura key, six times | cleared |
| `infra-structure/config/mcp-config.json` | the same Infura key, six times | cleared |
| `infra-structure/services/rpc-crawler/crawler_state.json` | keyed Ankr/Alchemy endpoints the crawler recorded | allowed, with a reason: captured data, not our credentials (TICKET-103 stops tracking it) |
| `packages/blockchain-connector/{src,dist}/chains/generated/*` | a third-party Alchemy demo key published by the chain registry that file is copied from | allowed, with a reason |

The private key was the serious one: `EnvConfig::from_env()` used it as its **default** wallet, so any
build of that crate could sign with an account whose key anyone reading the repository knows. That
address holds **0 ETH on Arbitrum and 0 on Base** as of this writing (checked against public RPCs), so
nothing has been taken — but it must not be funded, and it must not be reused.

## What changed

* `ProviderCredentials::from_env()` reads `ALCHEMY_API_KEY`, `DRPC_API_KEY`, `ANKR_API_KEY` — absent
  means absent, with no default that could be mistaken for a key.
* `EnvConfig::new(network)` builds **keyless public endpoints** (`arb1.arbitrum.io`, `mainnet.base.org`,
  `polygon-rpc.com`, …). `EnvConfig::from_env()` promotes any configured paid endpoint ahead of them,
  so the operator's DRPC key is first when `DRPC_API_KEY` is set and the configuration still works
  without one.
* The wallet comes from `X3_BOT_PRIVATE_KEY` + `X3_BOT_ADDRESS`; with them unset there is **no**
  wallet, and a caller that needs one gets `None` rather than someone else's key.
* `scripts/check-no-provider-secrets.sh` scans `git ls-files` for keyed provider URLs and for
  64-hex values assigned to `private_key`-shaped fields, printing file, line and *pattern* only —
  never the value, which would leak it into the CI log it prints to. It is wired into `make guard`
  and into `local-ci`. Exceptions live in `scripts/allowed-provider-secrets.txt`, each with a stated
  reason; vendored trees (`forge-std`, `tauri-vendor`, `node_modules`) are skipped because they ship
  upstream demo keys that are not ours.

The guard found two files my manual scan had missed — the second MCP config and the generated chain
list — which is the argument for having it rather than a good intention.

## What this does not fix, and what the owner has to do

**Removing a secret from HEAD does not un-expose it.** These values are in this repository's history:

1. **Rotate** the Alchemy key, the DRPC key, the Ankr key and the Infura key — assume all four are
   public.
2. Treat the wallet `0x7f1d163dBe1d42F9813820996e039E6f81D5f62c` as burned: do not fund it, do not
   reuse its key anywhere. (Its key is in history verbatim.)
3. Set the replacements in the environment. For the DRPC endpoints the operator has bought:
   `export DRPC_API_KEY=…` — `EnvConfig::from_env()` and `arbitrum_mainnet_config()` will then put
   `https://lb.drpc.org/<network>/<key>` first for every supported network, with public RPCs behind
   it as fallback.
4. Decide whether history should be rewritten (the repository has done this once before, for the
   validator seeds — see the SEC-v1 entry in `TESTNET_GAP_LEDGER.md`). That is a destructive,
   operator-level call, so it is not made here.
