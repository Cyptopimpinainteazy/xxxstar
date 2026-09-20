# `emit` / host calls, and the formatter that deleted annotations — 2026-09-20

Two instructions the language could compile and not execute, and one tool that
silently removed them from a program. Measured before and after, on the same
commands.

## Defect 1 — `EMIT` and `CALL_HOST` had no payload record

**Repro.** `fn main() { emit TransferDone(1); }`

| | before | after |
|---|---|---|
| `x3c build` | 96 bytes, 24 ops | 40 bytes, 10 ops |
| `x3c explain` | `0x60 EMIT TransferDone:{"arg0": "Literal(Int { value: 1, base: Decimal, suffix: None })"}` | `0x60 EMIT EmitEvent { name: "TransferDone", fields: [("arg0", "1")] }` |
| `x3c run` | `VM error: Panic("X3_VERIFY_FAILED: InvalidOperand(12)")` | `x3c run: ok — 0 asset ops, 0 bridge ops, 0 receipts, gas remaining 999860` |

**Cause.** `emit_operation` wrote the payload by hand:
`format!("{name}:{args:?}")`, and `decode_capability_payload` ended
`_ => Err(InvalidOpcode(opcode))` with no arm for `0x60` or `0x61`. The verifier
routes every payload opcode it has no earlier rule for through that decoder, so
the two instructions the emitter wrote were the two the verifier refused. The
hand-written string was also the compiler's Rust `Debug` output — the AST it
happens to hold, not a record any host could read.

**Every producer affected** (all five now build *and* run):

| source | record |
|---|---|
| `emit TransferDone(1);` | `EmitEvent { name: "TransferDone", fields: [("arg0", "1")] }` |
| `custom_thing(1, 2);` | `HostCall { function: "custom_thing", args: ["1", "2"] }` |
| `subscription keeper: 100, 30 { … }` | `HostCall { function: "charge_subscription", args: ["keeper", "100", "30"] }` |
| `@subscribe(TransferDone)` | `HostCall { function: "subscribe_event", args: ["TransferDone"] }` |
| `@sponsor` | `HostCall { function: "deduct_sponsor_fee", args: [] }` |
| `diff(10, 4);` | `HostCall { function: "diff", args: ["10", "4"] }` |

**Fix.** `CapabilityPayload::EmitEvent` and `CapabilityPayload::HostCall`, with
encode and decode arms for `0x60`/`0x61`; `emit_operation` routes both through
`emit_payload_op`, the same shared encoder every other payload opcode uses;
`x3c explain` decodes both; the emitter's `format!("{:?}")` for an event's
arguments became `expression_to_string`, the renderer every other payload field
uses.

The executor keeps passing raw payload bytes to `evm_call` / `svm_call`, so the
host-visible boundary is unchanged — what the bytes *say* is now a record the
shared decoder reads back.

## Defect 2 — no arm in the executor's dispatcher

Adding the variants made `dispatch_host_opcode`'s match non-exhaustive. It does
not serve either instruction (the execution loop has its own arms, which hand
the payload to the EVM/SVM backends), so the arm refuses by name
(`X3_HOST_OPCODE_MISROUTED`) rather than answering from whichever arm looked
closest.

## Defect 3 — a subscription's period was read and dropped

`subscription keeper: 100, 30 { … }` lowered to
`charge_subscription("keeper", "100")`. `period_blocks` was parsed,
stored on `SubscriptionDecl`, and never used: the one fact that makes a
subscription periodic was the one fact the host could not see. It is the third
argument now.

## Defect 4 — `x3c fmt` deleted every annotation

`X3Formatter` had no notion of annotations: `format_function` wrote `async fn …`
with nothing above it, and `format_agent` likewise.

| | before | after |
|---|---|---|
| `x3c fmt examples/events_and_host_calls.x3` | `@subscribe`/`@sponsor` gone | kept |
| `x3c build` of the formatted file | 108 bytes, 27 ops — two `CALL_HOST` records missing | 172 bytes, 43 ops |
| `cmp` against the original artifact | differ | identical |

No file in the corpus carried an annotation, so the corpus round-trip test that
compares compiled artifacts before and after formatting could not see it. The
new example is the first corpus file that uses the feature, and the round-trip
test caught the defect on its first run — which is the argument for an example
per feature rather than only tests.

`format_subscription` was dropping the period too, turning `, 30` into a
subscription charged every block.

## Deleted: `Annotation::Subscription`

Nothing constructs it (`rg 'Annotation::Subscription'` → one lowering arm, one
name-table row, no constructor), and the parser refuses `@subscription` because
the lexer reserves the word for the item form (TICKET-111). It is what forced the
formatter to choose between dropping it and writing text the parser rejects, so
it is gone: the formatter's renderer is total over annotations that exist.

## Verification

```
cd x3-lang
CARGO_TARGET_DIR=/tmp/x3lang-target-merge /tmp/x3lang-cargo.sh test --workspace   # 1227 passed / 0 failed
… clippy --workspace --all-targets -- -D warnings                                # clean
… fmt --all -- --check                                                           # clean
pytest tests/ -q                                                                 # 23 passed
bash /tmp/x3probe/sweep2.sh <x3-lang> <x3c>   # files=20 check=20 build=20 warning-free=20 run-artifact=19
```

Test count went 1218 → 1227: 5 end-to-end (build → verify → **run** → read the
record back out of the artifact), 3 verifier (empty name refused, empty function
refused, the old hand-written bytes refused rather than misread), 1 formatter
(all 20 annotation spellings formatted and re-parsed).

The end-to-end tests run the artifact rather than checking the lowering or the
bytes: both halves of the broken version were internally fine, and the defect was
that they disagreed. They cannot compile against the old code — they name the new
records — so the before/after proof is the `x3c` transcript above, taken with the
same source and the same command on either side of the change.

## `scripts/local-ci.sh` — 4 gates red, none from this change

`test x3-lang` and `test x3-lang python` PASS in the same run. The four failures
are environmental, each by its own message:

| gate | failure | cause |
|---|---|---|
| `nested workspaces` (10 tests) | `error binding to 127.0.0.1:0: Operation not permitted` | sandbox denies `bind()` |
| `test node` (1 test) | `ephemeral tcp port should be available: PermissionDenied` | same |
| `js sdk tests` (35 tests) | `connect EPERM 127.0.0.1` | same |
| `clippy runtime rc1` | `smallvec compiled by an incompatible version of rustc` (1.98.1 vs 1.90.0) | shared `target/` holds artifacts from two toolchains |

Not fixed here: the first three need a sandbox that permits loopback, and the
fourth needs a `cargo clean` of a cache this change never wrote to. `workspace
check` and `clippy workspace` — the root workspace's own gates — both PASS.

## Still open

- The unclaimed-call fallback and `diff` are recorded as named host calls, which
  is the IR's existing untyped-call form; a typed record per operation (the way
  `VenueOrder` and `RebalanceTarget` were done) would be a new opcode and so a
  version bump.
- An event's arguments are strings; nothing yet binds them to the parameters an
  event declares, because the language has no event declaration to bind them
  against.
- The four local-ci gates above need an environment that permits loopback, or a
  decision to mark them known-red.
