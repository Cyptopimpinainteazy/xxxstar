# X3 AI router — gap analysis against the Forge Intelligence Engine spec

Required by §55 of the Forge Intelligence Engine spec before the router is
changed. Every verdict below was checked against the code in this directory,
not against the README. Line numbers are from the commit that added this file.

## §55 — the twenty inspection points

| # | Item | Verdict | Evidence |
| --- | --- | --- | --- |
| 1 | Current router implementation | `services/x3-ai-router/router.py`, stdlib only, systemd unit `x3-ai-router.service` | `router.py:1`, `~/.config/systemd/user/x3-ai-router.service` |
| 2 | `~/.codex/config.toml` and project config | Present. `model_provider = "x3-router"`, provider block points at `http://127.0.0.1:11435/v1`, `wire_api = "responses"`. Secrets are not in the router repo. | `~/.codex/config.toml` |
| 3 | Current provider configuration | 6 providers: `deepseek`, `openrouter`, `direct`, `ollama`, `nemotron_lightning_free`, `nemotron_ultra_free`. | `config.json` |
| 4 | How `x3-auto` resolves | **Weak.** `x3-auto` is a label only. The client's `model` is accepted and discarded; `choose()` scans request text for keywords and yields `critical` or `routine`, then uses the fixed chain for that tier. | `router.py:563` `choose`, `router.py:1222` `/v1/models` |
| 5 | DeepSeek integration | Direct, `https://api.deepseek.com`, `deepseek-flash`, `DEEPSEEK_API_KEY` from the service environment, peak-rate pricing, thinking-mode tool-choice rule. | `config.json` `providers.deepseek` |
| 6 | Ollama integration | `http://127.0.0.1:11434/v1`, `qwen2.5-coder:7b`. Measured **not** tool-capable, so declared `supports_tools: false`: text-only fallback. | `config.json`, `README.md` |
| 7 | Fallback behaviour | `budget_fallback` appended after the route; critical requests cannot leave the machine to a third party. | `router.py:568` `attempt_order`, `may_serve_critical` |
| 8 | `/v1/responses` compatibility | Implemented, both streaming and non-streaming, including custom (`apply_patch`) tools. | `router.py` `serve_responses`, `ResponsesStream` |
| 9 | `/v1/chat/completions` compatibility | Implemented, streaming and non-streaming. | `router.py` `do_POST` |
| 10 | Provider health checks | Cooldown with doubling backoff, `Retry-After` honoured, `/v1/providers`. Liveness `/health` is separate from capability. | `note_provider_failure`, `provider_health` |
| 11 | Model catalog | Flat `providers` map in `config.json`. No measured profile per model. | `config.json` |
| 12 | Reasoning-effort handling | Codex's `reasoning.effort` is carried through and sent only to a provider declaring `reasoning_effort`. Forced tool choice disables DeepSeek thinking. | `apply_provider_reasoning` |
| 13 | Timeout / retry behaviour | Timeouts per provider (`timeout_seconds`, default 120). **No retries at all** — one attempt per provider, then the next provider. | `router.py:998`, `router.py:1069` |
| 14 | Logging | One sanitized line per refusal: request id, provider, status, reason, cooldown. No prompt text, no credentials. | `Router.log_diagnostic` |
| 15 | Existing caching | **None.** No semantic cache, no response cache, no prompt cache key handling. | — |
| 16 | Routing metrics | `/v1/usage`, `/v1/tasks`, `/v1/learning`, `/metrics`, `/v1/dashboard`. | `handler_for` |
| 17 | Tests before modification | 69 tests, green. | `python3 -m unittest test_router` |
| 18 | Gap analysis | This file. | — |
| 19 | Rollback configuration | Direct-DeepSeek Codex config preserved; backup at `~/.codex/config.toml.backup-before-x3-router-*`; rollback is one line. Verified live. | `~/.codex/` |
| 20 | Incremental improvement | Started below; each step is separately committed and tested. | git log |

## §56 — minimum useful release checklist

| Capability | State | Note |
| --- | --- | --- |
| OpenAI-compatible endpoint | **done** | chat completions + responses |
| DeepSeek provider | **done** | primary, on both tiers |
| Ollama provider | **done** | text-only, measured |
| Configurable external providers | **done** | `config.json` |
| `x3-auto` | **partial** | resolves, but as a fixed chain, not a policy |
| Task classification | **partial** | keyword scan producing `critical`/`routine` only; none of §2's 22 classes |
| Provider health | **done** | cooldown + capability probe |
| Bounded retries | **missing** | no retry path exists |
| Fallback | **done** | including budget fallback |
| Logging | **done** | sanitized, per request id |
| Token accounting | **done** | per provider, model, agent, day |
| Latency accounting | **partial** | per task only; no per-provider latency |
| Model capability registry | **partial** | tool-call probe only; no measured profile (§3) |
| Context compiler integration | **missing** | §7 is not built |
| Failure memory integration | **missing** | §17 is not built |

## What this means for the next increment

The v1 release is missing four things and partial on four. The order below
follows the spec: classification feeds routing, routing needs measured
profiles to choose from, retries need latency data to bound, and the memories
plug in afterwards.

1. **Task classifier** (§2) — classify a request into the spec's classes and
   estimate complexity, risk, blast radius, context requirement, verification
   requirement and parallelizability.
2. **Logical models as routing policies** (§1) — `x3-auto`, `x3-fast`,
   `x3-code`, `x3-deep`, `x3-security`, `x3-review`, `x3-local` resolve to
   chains chosen from the classification and the registry, instead of one
   fixed chain per tier.
3. **Capability registry** (§3) — measured per provider and model: requests,
   failures, average latency, retry rate, cost, and the verified-patch rate
   the existing task feedback already records.
4. **Latency accounting** — record per-provider latency so (3) has data.
5. **Bounded retries** (§50) — retry a provider a bounded number of times on a
   transient failure, with backoff, distinguishing retryable from terminal.

Deliberately not in this increment: the context compiler (§7), the repository
graph (§6), semantic caching (§15), model racing (§5) and the memories
(§17–19). Those are larger and need their own data sources; claiming them here
would be the documentation-not-implementation failure the X3 rules forbid.

## Constraint carried forward

`x3-router` is the configuration Codex actually uses. Every change is
verified against a live Codex session, and the direct-DeepSeek rollback stays
one line away at all times.

---

## Status after the first increment

Implemented, tested and live-verified in the commit that follows this file:

| Was | Now |
| --- | --- |
| `x3-auto` resolved to one fixed chain per tier | resolved from the task class through a named policy; `x3-fast`, `x3-code`, `x3-deep`, `x3-security`, `x3-review`, `x3-local` are real routing policies |
| keyword scan producing `critical`/`routine` | §2 classifier: 23 classes plus complexity, risk, blast radius, context, verification and parallelizability estimates |
| no per-provider latency | recorded per attempt, failures included |
| no measured model profile | `GET /v1/registry` with attempts, failure rate, retry rate, average latency, cost and verified-patch rate |
| no retries at all | §50 bounded retries on retryable statuses and transport errors, with backoff, never after the first streamed byte |
| no way to ask why | `GET`/`POST /v1/explain` returns the decision without calling a provider |
| no context compiler integration | `POST /v1/context` and `explain {"context": true}` reach `tools/x3-forge/context.py` and return its provenance-carrying package |
| no failure memory integration | `tools/x3-forge/failure_memory.py` stores failures, successes and dead ends; task outcomes write to it, `GET /v1/memory` and `explain {"memory": true}` read it |

Verified live: a consensus request resolves to `x3-security` / critical with
blast radius `[consensus, finality, settlement]`; a README request resolves to
`x3-fast` / routine; a cross-chain request returns a 5-file context package
compiled from the real index at commit `89326178`; Codex still reads a local
file end to end; the registry reports real measurements (`deepseek-flash`, 2
attempts, 0% failures, 1160.7ms average).

§6 and §7 were not built here. They already exist as `tools/x3-forge/index.py`
(a provenance-carrying repository index) and `tools/x3-forge/context.py` (the
context compiler, with its own suite). This increment wires them to the router
rather than writing a second, divergent copy — which is what §7 integration
means once the canonical implementation is already in the tree.

§56 is now complete.

Failure memory lives in `tools/x3-forge/failure_memory.py`, beside `index.py`
and `context.py`, because §17-19 describe Forge-side knowledge and that is
where Forge tooling already lives. It was not put inside the router: the router
sees task outcomes but not the failing evidence, so a store there would be in
the wrong layer and would still need the Forge tooling to read it.

One behaviour worth stating: §17 asks for a failure memory, §18 for a success
memory and §19 for a dead-end memory. They are one append-only log with a
`kind` field rather than three stores, because the fields are the same fields
and three stores would drift. `kind` is part of the fingerprint, so the same
error text recorded as a dead end and as a failure stays two pieces of
knowledge.

### Known cost, stated rather than hidden

Compiling a context package takes ~13s for a narrow query and ~31s for a broad
one, because the compiler reads an 79MB index. It is an explicit call and never
on the routing path, and `--budget`/`--max-files` pass through. If it becomes
hot, the fix belongs in the index (a sharded or pre-digested form), not in the
router.
