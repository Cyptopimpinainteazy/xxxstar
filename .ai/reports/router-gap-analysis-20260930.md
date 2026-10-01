# X3 AI router — gap analysis (IMMEDIATE ROUTER MISSION, steps 1–19)

Date: 2026-09-30
Base revision: 4ec2fdc66 (router hardening landed this session)
Author: Codex session audit; every claim below is from code, live state, or a test run.

## 1. Current router implementation

- `services/x3-ai-router/router.py`, 2,640 lines, Python standard library only.
- `services/x3-ai-router/test_router.py`, 2,342 lines.
- Live service: `systemctl --user status x3-ai-router.service` → active, PID 16494,
  port 11435, started 2026-09-30 21:43:54 MDT, journal shows `POST /v1/responses`
  200s served through DeepSeek.

## 2–3. Client and provider configuration

`~/.codex/config.toml` (read redacted; no secrets printed or stored):

- `model_provider = "x3-router"`, `model = "x3-auto"`, `model_reasoning_effort = "max"`,
  `web_search = "disabled"`.
- `[model_providers.x3-router] base_url = http://127.0.0.1:11435/v1`, `wire_api = "responses"`.

`services/x3-ai-router/config.json` provider census:

| provider | protocol | notes |
| --- | --- | --- |
| deepseek_flash | responses | primary; thinking-mode rules (`disable_when_tool_choice_forced`) |
| deepseek_pro | responses | escalation |
| openrouter | chat_completions | needs `OPENROUTER_API_KEY`; live 402 (credits exhausted) |
| ollama | chat_completions | local |
| nemotron_lightning_free / nemotron_ultra_free | chat_completions | free endpoints, opt-in via `X3_ENABLE_FREE_CLOUD` |
| direct | chat_completions | `OPENAI_API_KEY`; live 401 (credential unavailable) |

## 4. How `x3-auto` resolves

Policy, not a hard-coded chain: tier `auto`, order
`deepseek_flash → deepseek_pro → openrouter → ollama → nemotron_lightning_free → direct`,
then `budget_fallback`. Critical classifications use `x3-security`
(`deepseek_flash → deepseek_pro → direct`); third-party providers are never
cleared for critical work. `POST /v1/explain` returns the chosen policy, order,
classification and blast radius without spending a token.

## 5–7. DeepSeek / Ollama / fallback

- DeepSeek: Responses API, tool calling, thinking-mode consequences handled
  (reasoning tokens billed as output; forced tool_choice switches thinking off).
- Ollama: chat_completions provider; `x3-local` policy is ollama-only.
- Fallback: `RETRYABLE_STATUS` bounded retries, per-provider cooldowns,
  capability checks (a provider that is not tool-capable is skipped for tool
  turns), reservation-based budgets, and a documented `budget_fallback`.

## 8–9. Wire compatibility

- `POST /v1/responses` (non-stream and streaming; Codex bridge), `POST /v1/chat/completions`.
- Unsupported OpenAI surfaces (`/v1/embeddings`, `/v1/audio`) return 501 rather
  than pseudo-success. Responses `input` accepts both the string and item-list forms.

## 10–16. Ops surface

- Health: `/v1/providers` (failures, cooldown, last_error), capability probes with
  a TTL cache, `/v1/registry`, `/v1/models`, `/v1/tasks`, `/v1/usage`,
  `/v1/dashboard`, `/v1/learning`, `/metrics` (Prometheus text).
- Accounting: tokens, estimated cost, per-model latency samples, revision-bound
  task feedback with `verified_patch_rate`; reservations and orphan reconciliation.
- Reasoning effort: forwarded only where the provider declares support; stripped otherwise.
- Logging: `request_id`, provider, status, cooldown, reason; provider errors are
  scrubbed of credentials before any log line or client body (`SECRET_PATTERNS`).
- Caching: capability-verdict cache only. There is **no response cache**.
- `/v1/learning` currently answers `"routing_mode": "fixed"`: feedback is
  recorded, and the router does not silently change routing from it.

## 17. Tests before modification

`cd services/x3-ai-router && python3 -m unittest test_router` → **126 tests, OK**
(66.05s, run 2026-09-30 in this session, outside the sandbox so the test HTTP
server can bind). The 17 tests added by the landed hardening cover protocol
capability, sanitized provider errors, Responses string input, usage
normalization, and the rule that no failure path prints a credential.

## 18. Gap analysis — v1 checklist (§56) vs reality

Present: OpenAI-compatible endpoint; DeepSeek; Ollama; configurable providers;
x3-auto; task classification; provider health; bounded retries; fallback;
logging; token accounting; latency accounting; capability registry; context
compiler integration (`/v1/explain` pulls Forge context); failure-memory
integration; per-agent budgets.

Absent (v2 items, in the spec's order):

1. Semantic/response caching — nothing exists; a safe first form is exact-match
   only, non-tool, temperature-0 turns, keyed by full body hash + policy +
   provider/model, default OFF.
2. Model racing for hard problems — absent.
3. Adaptive routing from measured outcomes — deliberately absent: feedback is
   recorded but routing is fixed; making it adaptive reverses a documented
   safety decision and needs its own review.
4. Test-aware routing — absent.
5. Distributed local inference — absent.

Live operational findings from this audit:

- openrouter is over its credit limit (`HTTP 402`) and direct has no usable
  credential (`HTTP 401`); the router correctly degrades to DeepSeek and serves.
  These are operator/billing states, not router defects.

## 19. Rollback configuration

`config.json` and `router.py` are versioned in git; the live service reads the
worktree. Landing order was: tests first (126 OK), commit `4ec2fdc66`, and the
service was restarted only by the operator. `git revert 4ec2fdc66` restores the
previous router.

## 20. Incremental plan (next bounded steps)

1. (done this session) Land hardening with tests.
2. Response cache behind `cache.mode = "off" | "exact"`, default off, with a
   gate that a cached body is byte-identical to the live response for a fixture.
3. Backpressure/observability: expose per-provider 401/402 states in `/v1/providers`
   as operator actions (already partially present via `last_error`).
4. Only after the above: model racing, adaptive routing — each behind an
   explicit config flag with measurement before/after.

VERDICT: router v1 is implemented and tested for this scope. v2 items above are
the remaining gaps; none of them block the router's current live use.
