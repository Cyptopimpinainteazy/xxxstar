# Clauses the parser could not read, and a guard word that begins nothing — 2026-09-20

TICKET-046 was a decision request: the acceptance asked for the 22 clause words to
become lexer keywords, and the ledger's reasoning said that conflicts with the
language because those words are also field names. Measuring it instead of
deciding it found the defect the acceptance was reaching for.

## The class: an arm written for a word the lexer reserves

The lexer maps `swap`, `bridge`, `require`, `emit`, `use`, `mint`, `burn`,
`lock` and `release` to keywords, and `keyword_to_tok` gives each a `Tok::Kw*`
variant. An arm of the form `Tok::Ident(ref s) if s == "<one of those>"` can
therefore **never run** — and nothing in the compiler says so. Three clauses
were written that way.

## Defect 1 — `use <target> <config>` in an intent body

```
$ x3c build use_probe.x3
x3c: compile error: Parser error: unexpected clause in intent body: KwUse; expected
  one of `from`, `to`, `route`, `require`, `timeout`, `on_fail`, `use` or `on`
```

The message lists `use` among the clauses the body accepts. The match had an arm
for `Tok::Ident(...) == "use"` and no `Tok::KwUse` arm, so the clause was
unreadable. The formatter already wrote `use <target> <config>` back out, with a
comment saying "the clause is what the intent surface accepts".

After (`Tok::KwUse`):

```
$ x3c build use_full.x3                       # examples/route_fallback.x3 with `use uniswap 1`
x3c build: 424 bytes (106 ops)
$ x3c explain use_full.x3b | grep HostCall
  0003  0x61  CALL_HOST    HostCall { function: "use", args: ["uniswap", "1"] }
$ x3c run use_full.x3b
x3c run: ok — 4 asset ops, 0 bridge ops, 0 receipts, gas remaining 998734
```

## Defect 2 — the terse `finality_policy` form

```
$ x3c check fp_terse.x3     # finality_policy strict { ethereum require finalized  blocks 12 }
x3c: parsing failed: Parser error: expected '}' after finality_policy body
```

The comment above the arm documents this as "the terse form: `<chain_name>
require <mode>`". The lookahead tested `Tok::Ident(r) if r == "require"`. After
the fix: `x3c check: 1 ops, no semantic errors`. The long form
(`chain` / `requirement` / `blocks`) is unchanged and still passes.

## Defect 3 — the inline `rpc_quorum` form

```
$ x3c check rq_inline.x3    # rpc_quorum { source require 2_of_3  relayers a b c }
x3c: parsing failed: Parser error: rpc_quorum source chain: expected identifier
```

Same cause, same comment ("inline: source require N_of_M"). After the fix:
`x3c check: 1 ops, no semantic errors`.

## `CLAUSE_WORDS`: one stale entry, nothing missing

The lookahead list is a second statement of the grammar, so both directions were
measured rather than argued.

**Stale — `balance`.** One occurrence of the string in the whole parser: the
list entry. No arm dispatches on it. A guard that stops at a word which begins
nothing reads `require <kind> balance` as a guard with no subject. Removed.

**Nothing missing.** A guard can only be followed by a clause in a body that
also holds statements. Those bodies are the intent body and the `atomic swap`
body, and the fixtures cover both. The one route-step word that is not a lexer
keyword — `fallback` — cannot follow a guard: route blocks are read by a step
loop that accepts only route operations, measured by putting a `require` between
two steps:

```
$ x3c build fb_guard.x3     # route { swap …  require proof_complete  fallback { … } }
x3c: compile error: Parser error: expected route operation (swap/bridge/lock/mint/burn/release/fallback)
```

## Drift is now a test failure, in both directions

- `every_word_in_this_list_begins_a_clause` reads `CLAUSE_WORDS` out of
  `parser.rs` and fails for any word no arm dispatches. Verified load-bearing by
  re-adding `balance`: *"these words stop a guard but begin no clause in the
  parser: [balance]"*.
- The guard fixtures went from nine words to thirteen — `allow`, `on`, `use`
  (intent body) and `min_output` (swap body). The `use` fixture is the one that
  would have caught defect 1: with the identifier arm restored it panics with the
  parser's own `unexpected clause in intent body: KwUse`.
- `compiler/tests/test_keyword_clauses.rs` pins all three restored clauses. With
  the identifier lookahead put back, two of its four tests fail.

## Seven arms of the same shape remain — TICKET-113

| function | word | live twin |
|---|---|---|
| `parse_trade_stmt` | `bridge` | `Tok::KwBridge` |
| `parse_intent_clause` | `on_fail` | `Tok::KwOnFail` |
| `parse_route_step` | `swap`, `bridge`, `lock`/`mint`/`burn`/`release` | `Tok::Kw*` |
| `parse_rpc_quorum_item` | `require` | `Tok::KwRequire` |
| `parse_finality_policy_item` | the `require` half of `requirement \|\| require` | `Tok::KwRequire` |

Nothing is broken — the keyword twin does the work — but an unreachable arm is
what let the three broken clauses look correct, and `parse_route_step` currently
has two arms for `swap` with nothing saying which runs. Recorded as TICKET-113
rather than fixed here: cleanup with no user-visible effect.

## Verification

```
cd x3-lang
CARGO_TARGET_DIR=/tmp/x3lang-target-merge /tmp/x3lang-cargo.sh test --workspace   # 1233 passed / 0 failed
… clippy --workspace --all-targets -- -D warnings                                # clean
… fmt --all -- --check                                                           # clean
pytest tests/ -q                                                                 # 23 passed
bash /tmp/x3probe/sweep2.sh <x3-lang> <x3c>   # files=20 check=20 build=20 warning-free=20 run-artifact=19
scripts/local-ci.sh --only no-float-in-consensus,cargo-lockfile-locked,invariant-registry,workspace-membership
                                                                                 # all PASS
```

Test count 1228 → 1233: one for the list, four for the clauses.

## Note on the commit message of `c7dfb461a`

It was written with backticks inside a shell double-quoted argument, so three
were expanded away — `lists \`use\` among the clauses it accepts` reads as
`lists  among`, and the `HostCall` record lost the quotes around `use`,
`uniswap` and `1`. The code and the tests are correct; this report and the
ledger entry for TICKET-046 carry the intended text. Not corrected by
force-pushing: rewriting a pushed `master` while other agents are working on it
is the hazard `git cherry` exists to avoid, and a commit message is not worth
that.
