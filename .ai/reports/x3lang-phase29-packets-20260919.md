# X3Lang — 2026-09-19 — opportunity packets are decided by their own claims (PHASE 29)

Working tree (uncommitted) on `master` `0a68cb883`. Row 29 of
`.ai/reports/x3lang-phase-ledger-20260918.md` ("no signed/versioned packet type")
is closed by `x3-lang/vm/src/opportunity_packet.rs`.

## What landed

`OpportunityPacket` is the portable execution package the phase asks for: strategy
commitment, artifact hash, state roots per domain, the route the opportunity graph
already decided (`compiler::opportunity::Opportunity`, reused rather than
re-modelled), the capital window, expected output, profit floor, fee and slippage
ceilings, a deadline, proof requirements, an execution commitment, a packet hash and
an ed25519 signature.

The phase's five adjectives are each a mechanism, not a label:

- **versioned** — `version: u16` checked against `OPPORTUNITY_PACKET_VERSION`, and
  the schema version is also carried in both hash domains, so terms committed under
  a future schema cannot collide with these.
- **hashable** — `packet_hash` is SHA-256 over `PACKET_DOMAIN || bincode(packet)`
  with the hash and the signature cleared, so a hash never covers itself.
- **deterministic** — the packet's maps and sets are `BTreeMap`/`BTreeSet` and the
  encoding is bincode, so insertion order cannot change the bytes or the hashes. The
  test that proves it builds the same packet in two insertion orders.
- **signed** — `sign_packet` commits then signs the packet hash; verification
  requires the signer to be in the caller's trusted map *and* the public key carried
  in the packet to equal the trusted one, mirroring `verify_receipt_attestation`.
- **replay-protected** — `OpportunityPacketLedger::admit` records the packet hash and
  refuses the same packet a second time; the deadline is checked first, so a packet
  presented late reports the deadline and a packet presented twice reports the
  replay.

## The properties it decides, and the ones it refuses to fake

Decided from the packet alone:

- the capital window is real (`required > 0`, `required <= max`);
- the profit floor is reachable by the packet's own numbers **after its own
  worst-case fee**: `expected_output - required_capital - maximum_fee >= minimum_profit`;
- the route can carry the size — the thinnest venue's declared liquidity must cover
  `max_capital`, which is the opportunity graph's own reading of `min_liquidity`;
- the route's declared fee at maximum capital (`fee_bps` of `max_capital`) fits the
  packet's absolute `maximum_fee`, and its worst leg slippage fits
  `maximum_slippage_bps`, which itself cannot exceed 100% (`MAX_SLIPPAGE_BPS`);
- the route is a route (`venues` non-empty, `assets == venues + 1`, no unnamed venue);
- the state roots name domains and are non-zero, and the packet names a strategy and
  an artifact;
- `execution_commitment` really covers the execution terms and `packet_hash` really
  covers the packet.

Not decided, deliberately: whether the opportunity is *real* — that the state roots
are current, that the venues will trade at the claimed prices, or that `strategy_id`
names the strategy that produced the route. Those need host evidence this crate does
not have; pretending to check them would be a no-op check. Recorded as `TICKET-071`.

An empty `proof_requirements` set is allowed and means "no proof attached". The
vocabulary of proof kinds belongs to the hosts and venue adapters, so this module
checks only that a requirement is named rather than inventing a closed set nothing
reads yet.

The execution commitment deliberately excludes `strategy_id`: the packet sells its
execution and hides the strategy, so a builder can act on the commitment without
learning what generated it.

## Wiring

- `x3-lang/vm/src/lib.rs` — module + re-export.
- `x3c packet inspect <file.json>` — prints the packet as read plus both commitments;
  it recomputes nothing, so a mismatched packet shows both figures rather than a
  "corrected" one.
- `x3c packet verify <file.json> --block N --trusted <key_id>=<hex>` — requires the
  trusted key on the command line, because a signature checked against a key that
  travelled inside the packet is a restatement, not a check.

## Evidence

```
cargo check --workspace                                  -> exit 0
cargo test --workspace --no-fail-fast                    -> 0 failed (97 vm lib, 4 new
                                                            opportunity_packets tests,
                                                            37 cli, 8 cli_integration)
cargo clippy --workspace --all-targets -- -D warnings    -> exit 0
cargo fmt --all -- --check                               -> exit 0
.venv/bin/python -m pytest -q x3-lang/tests              -> 16 passed
fake-code scan over the new files                        -> no matches
```

Logs: `.ai/runlogs/x3lang-phase29-packets-tests.log`,
`.ai/runlogs/x3lang-phase29-packets-main.log`,
`.ai/runlogs/x3lang-phase29-packets.log`.

`pnpm test` / `pnpm build` / `npm test` are not applicable: this change touches no
JavaScript or TypeScript package.

## Things worth recording

- **A peer agent's in-flight edit broke the shared tree mid-task.** `Item::Arb` was
  added to `x3-ast` while `compiler/src/formatter.rs` had no arm for it, so
  `x3-lang-compiler` did not compile for ~5 minutes and nothing in the workspace
  could be built. I proved my work meanwhile against a pristine `HEAD` copy under
  `/tmp` (there I also learned that a bare `rustfmt` run ignores
  `x3-lang/rustfmt.toml` and formats to 100 columns, while `cargo fmt` honours 120).
  Their fix landed at 10:54 and the final proof above is from the real tree.
- **The pinned toolchain matters.** `/tmp` had no `rust-toolchain.toml`, so the first
  clippy run used the machine default (1.98.1) and reported five lints in *pristine*
  compiler code. Under the pinned 1.90.0 the same command is clean. A clippy result
  from outside the repo root is not evidence about this repository.
- **Three tests initially asserted the wrong commitment.** Editing a field that is
  part of the execution terms trips *that* commitment first, not the packet hash;
  only a field outside the terms (for example `strategy_id`) reaches the hash check.
  The code was right and the expectations were not, which is exactly what the
  integration test caught.

## Next task seed

- **PHASE 30 (dedicated execution lanes)** consumes packets: lane classes
  (standard / trading / atomic cross-domain / liquidation / settlement) with a
  deterministic, documented scheduling policy. The ledger is the natural starting
  point for admission, and this packet is the unit to admit.
- `TICKET-071` — host evidence for packet realism (state-root freshness, venue
  price attestation, strategy-commitment linkage).
- `x3c packet build` was deliberately not written: generating a packet from an
  `arb`/`atomic_trade` program is the not-yet-implemented chain (PHASE 37's
  generator, `TICKET-070`).
