# The paid endpoint drill, and what it proved before a key existed

Date: 2026-09-23.

## Why it exists

The operator buys RPC endpoints; the repository had been shipping their keys in `HEAD`, and the
four leaked credentials are being rotated (TICKET-102). What replaces them is
`ProviderCredentials::from_env()` — `DRPC_API_KEY` in, `https://lb.drpc.org/<network>/<key>` out.
Nothing in the repository can tell a rotated-in key from a revoked one: that needs egress and the
key. So the check is a drill, not a gate — `make provider-drill`, evidence under `.ai/reports/`.

It asks each endpoint three questions that only a real node on the right chain can answer:

1. `eth_chainId` equals the chain this repository believes that network is — chain identity, not a
   hostname;
2. the paid endpoint and an independent keyless public endpoint return the **same block hash** at the
   same height, 64 blocks behind the head — an endpoint that invents a block cannot agree with
   another, and that agreement is what the RPC-quorum path assumes;
3. a transaction in that block has a receipt on **both** endpoints with `status == 0x1`, naming the
   agreed block — the receipt is the evidence, submission is not, the rule the EVM adapters follow.

The key is never printed and never written: endpoints are printed with the key redacted, and the
evidence file holds no key.

## How it was validated with no key to test

The drill takes `--paid-url-template`, and a template with no `{key}` in it needs no key. That is how
the drill itself was run against two real, independent providers today:

* **Rule 2, live and passing.** `https://base.publicnode.com` (paid side) against
  `https://mainnet.base.org` (public side): both answered `eth_chainId` `0x2105` (= 8453), and both
  returned block `51677151` as
  `0xd6cbb3210123bbd6b097622921e2a9a6360073e249aa1711bbcfb88ce629daba`, with the same transaction
  `0x808fda5864fcc878b68dc8133f3fc72e5e137b77629ac78063b1d3fa06cd2276` reported `status == 0x1`
  by each. Exit 0.
* **A provider that cannot serve the history the check needs is a failure, not an excuse.**
  `base.publicnode.com` answered the block but refused the receipt:
  `Archive requests require a personal token. Get one at: https://www.allnodes.com/publicnode`
  (their words). The drill reported that string and failed the network rather than treating the
  missing receipt as a pass. This is the property the paid endpoint is being bought for.
* **A revoked or expired key is caught in the provider's own words.** With a placeholder key, DRPC
  answered `Your token is invalid or expired`; the report carries that string, the printed endpoint
  read `https://lb.drpc.org/base/<key>`, and the key appears nowhere in the log or the evidence file.
* **No key at all stops before anything else.** Exit 2, with the one command that sets it —
  so a missing key can never look like a passing check.

## What it does not prove

It proves the endpoints. It does not prove X3's own settlement path against a public chain: that is
the relayer work in TICKET-095, and it needs a funded key before anything is submitted. Nothing here
submits a transaction — every call is `eth_chainId`, `eth_blockNumber`, `eth_getBlockByNumber`,
`eth_getTransactionReceipt`.
