# The chain followed Bitcoin: a real header push, end to end

Date: 2026-09-23. Host worktree `<repo>/.wt-agent` (not `/tmp` — see
`.ai/reports/tmp-not-durable-20260923.md` for why that matters here).

## What was exercised

A live X3 **dev** chain, born anchored on a real Bitcoin regtest header, told to follow the real
Bitcoin chain by headers pushed from a Bitcoin node — and then asked to accept one it should refuse.

```
# 1. a real Bitcoin node
$ bitcoin-28.1/bin/bitcoind -datadir=btc/regtest -daemon          # v28.1.0, checksum-verified
$ bitcoin-cli … generatetoaddress 120 <addr>                      # height 119 captured, then 121-127

# 2. a dev chain born anchored on real header 119
$ X3_BTC_CHECKPOINTS="<header 119 hex>@119" x3-chain-node build-spec --chain dev > btc/spec18.json
$ python3 … cfg['sudo']['key'] = <Alice>                          # the dev spec ships sudo.key = null,
                                                                  # so root is unreachable otherwise
$ setsid nohup ./btc/run-node18.sh &                               # --alice --validator --rpc-port 12444

# 3. push real headers 120..125
$ node scripts/btc/push-headers.mjs --ws ws://127.0.0.1:12444 \
      --datadir btc/regtest --bitcoin-cli …/bitcoin-cli \
      --from-height 120 --to-height 125 --suri //Alice --via sudo
```

Result:

```
[push] on-chain btcBestHeight before: 119
[push] block 120 (04f62b7696e6a9a7…) in 0x322e193a…   … through …
[push] block 125 (67b5fb9c9d20e8fa…) in 0xa84e083e…
[push] on-chain btcBestHeight after:  125
[push] header meta at 125: {"height":"125","anchored":true}
[push] OK — the chain followed Bitcoin by 6 headers
```

**Negative control** — after mining two more blocks, only header 127 was pushed, so its parent (126)
was never admitted:

```
$ node scripts/btc/push-headers.mjs … --from-height 127 --to-height 127
[push] on-chain btcBestHeight before: 125
[push] block 127 refused: x3SettlementEngine.BtcParentMissing: The header's parent is not in
        storage, so the chain cannot be linked.
```

## The defect this found, and the fix

The first attempt refused header 120 with **`BtcTimestampTooOld`**, and the header was legitimate.
Bitcoin's median-time-past rule is the median of the previous **eleven** blocks; the pallet walked
the ancestors it had (one, at first) and compared against that. Padding a median with fewer samples
starting at the parent produces a *higher* number than Bitcoin's, and a rule that is stricter than
the chain it follows refuses real headers: this regtest chain was mined inside one second, so blocks
119 and 120 share a timestamp — which Bitcoin accepts.

`btc_median_time_past` now returns `Option<u32>`: `None` until eleven ancestors are stored, and the
check is skipped rather than made stricter. From the twelfth header onward it is exactly Bitcoin's
rule, and the two tests say so:

* `a_header_close_to_the_checkpoint_may_share_its_timestamp` — a child dated the same second as the
  anchor is accepted (this is the drill's case, as a unit test);
* `a_header_below_the_median_of_its_eleven_ancestors_is_refused` — with eleven ancestors stored, a
  header dated below the median is refused; the arithmetic is asserted, not assumed.

`spec_version` 17 → 18. Two further bugs were found and fixed in the sender while making this run:
`--from-height`/`--to-height` were parsed into keys the script never read (so every invocation died
with "required" — `node --check` had passed), and `pallet_sudo` **reports the inner call's failure in
a `Sudid` event while the outer extrinsic succeeds**, so a refused header looked exactly like an
accepted one until the script read the event.

## What this does and does not prove

* **Proved:** the receiving rules accept real Bitcoin headers from a real Bitcoin node, in order,
  and refuse one whose parent was never admitted; the anchor takes effect from genesis; the sender
  works against a live chain.
* **Not proved:** anything about a *public* network. The chain here is dev, the headers are regtest,
  the sender runs on the same host, and the push goes through `sudo` because this runtime sets
  `BtcHeaderOrigin = EnsureRoot`. No bond prices withholding, and the operator still has to run the
  sender — TICKET-095.
* **Not yet a gate:** the steps above are a session, not a script. Turning them into
  `scripts/testnet/btc-header-push-drill.sh` (start bitcoind if present, mine, anchor, push, assert
  both the advance and the refusal) is the next step for this ticket.
