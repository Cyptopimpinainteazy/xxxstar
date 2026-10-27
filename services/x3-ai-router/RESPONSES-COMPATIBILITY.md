# X3 AI Router — Responses API compatibility

Status: implemented and locally verified on 2026-09-30.
Scope: `services/x3-ai-router/router.py`, `config.json`, `test_router.py`.

## The bug this fixed

```text
Codex  POST /v1/responses
       -> X3 router
       -> DeepSeek
       -> 502 {"type":"no_provider_succeeded", "message":"No provider succeeded"}
```

The router treated every provider as a Chat Completions provider. A Codex
Responses request was translated and posted to `https://api.deepseek.com/chat/completions`.
DeepSeek answered `HTTP 400` on that path, the router benched it on cooldown,
and the remaining providers could not take the request either (OpenRouter was
`HTTP 402`, local Ollama is not tool-capable, the free cloud models are not
cleared for critical work). The client saw only `No provider succeeded`.

Evidence from the running service's own health table, before the fix:

```text
('deepseek',   2, ..., 'HTTP 400')
('openrouter', 2, ..., 'HTTP 402')
```

## What changed

The router now asks each provider which upstream protocol it speaks and gives it
that one. DeepSeek speaks the OpenAI Responses protocol natively, so a Codex
request is forwarded to `https://api.deepseek.com/responses` with the Responses
body intact — no Chat Completions translation on that path.

```text
Codex /v1/responses
      -> provider capability selection (protocol)
      -> responses provider  -> POST /responses         (native, un-translated)
      -> chat provider       -> POST /chat/completions  (tested translation)
```

## Supported inbound protocols

| Surface | Behaviour |
| --- | --- |
| `POST /v1/responses` | native Responses forwarding to `responses` providers; tested translation for `chat_completions` providers |
| `POST /v1/chat/completions` | unchanged Chat Completions surface |
| `GET /v1/models`, `/v1/models/x3-auto` | unchanged (routing policy aliases) |
| `GET /v1/registry`, `/v1/capabilities`, `/v1/explain`, `/v1/context`, `/v1/memory` | unchanged; `/v1/explain` now classifies the Responses body as sent |
| `/v1/embeddings`, `/v1/audio/*` | still `501`, naming the gap |

`stream=false` and `stream=true` are both implemented for `/v1/responses`.
Neither returns `501`.

## Provider protocols

Every provider declares its protocol explicitly. The router never infers one
from a base URL or a model name.

```json
"deepseek": { "base_url": "https://api.deepseek.com", "protocol": "responses",
              "protocols": ["responses", "chat_completions"],
              "model": "deepseek-flash", "api_key_env": "DEEPSEEK_API_KEY" }
```

* `protocol` — the protocol the provider speaks when it speaks one.
* `protocols` — every protocol it speaks, for a provider like DeepSeek that
  serves both surfaces.
* A provider that declares neither keeps the pre-existing `chat_completions`
  default, so an older config still routes.

| Provider | protocol(s) |
| --- | --- |
| `deepseek` | `responses`, `chat_completions` |
| `openrouter`, `direct`, `ollama`, `nemotron_lightning_free`, `nemotron_ultra_free` | `chat_completions` |

Selection rules:

* The surface's own protocol wins when the provider speaks it. A Responses
  request reaches a Responses provider natively.
* A Responses request falls back to the Chat Completions translation only for a
  provider that does not speak Responses. That translation is the existing,
  tested one, which is what keeps those providers in the failover chain.
* A Chat Completions request is never handed to a Responses-only provider.
  There is no chat -> responses translation, so the provider is skipped with a
  named reason instead of being sent a body it will reject.

## Model alias replacement

`model: "x3-auto"` (and every other policy alias) is a routing instruction. The
router always replaces it with the provider's configured model before forwarding,
in both protocols. The provider never sees an alias it does not know.

## Streaming behaviour

DeepSeek's Responses stream is semantic SSE: `event:` + `data:` lines terminated
by `response.completed`, `response.incomplete`, or `response.failed`. There is no
`data: [DONE]`. The router relays those bytes verbatim and watches for the
terminal event names; it never waits for `[DONE]`, and it preserves the SSE
headers (`Content-Type: text/event-stream`, `Cache-Control: no-cache`,
`Connection: close`).

A Chat Completions provider on the Responses surface is still adapted: its chunks
are re-emitted as `response.created` -> `response.output_item.added` ->
`response.output_text.delta` -> `...done` -> `response.completed`.

A provider that fails before producing any output (`response.failed` as the first
event) is failed over without the client seeing a half-started stream. A provider
that reports `response.failed` after output has begun has that event relayed,
is benched in provider health, and is not booked as a success.

## Tool capabilities

A native Responses request keeps `function` tools and freeform `custom` tools
(Codex's `apply_patch`) unchanged. `namespace` tools are flattened into their
members, as the Chat Completions translation already did.

Hosted tool types a provider has not declared are dropped and reported by name,
never forwarded as though they worked:

```text
web_search  file_search  computer_use  code_interpreter  mcp
image_generation  local_shell  shell
```

A provider opts in with `"hosted_tools": ["web_search"]`. If dropping leaves no
tools at all, a forced `tool_choice` is relaxed to `"auto"` — a forced choice
with nothing to call is a guaranteed upstream 400.

For a Chat Completions provider, a hosted tool with no chat equivalent keeps its
existing behaviour: it is refused by name (`400 unsupported_feature`) unless the
client itself disabled it. With a Responses-capable provider in the chain that
refusal no longer sinks the request — the native provider still serves it.

## Token accounting

Output bound, recognised in this order, whichever the surface uses:

```text
max_output_tokens   (Responses)
max_completion_tokens
max_tokens          (Chat Completions)
```

A request can no longer be reserved at the 4096 default while asking a provider
for more. `max_output_tokens` is validated against `max_output_tokens` config the
same way the chat fields are, and is always set on the upstream body.

Usage is normalised before it is stored: `input_tokens`/`output_tokens`
(Responses) and `prompt_tokens`/`completion_tokens` (Chat Completions) both land
in the router's internal model. Verified: a Responses answer reporting
`input_tokens=39, output_tokens=26` books 39/26, not zero.

## Failover behaviour

Provider selection, budgets, reservations, cooldowns, retries and fallback are
unchanged and shared by both protocols; only the body and URL differ per attempt.
A provider that fails does not corrupt the request handed to the next one — each
attempt is built from the same immutable plan. Verified: a `broken` provider
returning HTTP 400 is followed by a `good` provider that receives the untouched
Responses body.

## Error reporting

A `502` now carries the sanitized attempts instead of only `No provider succeeded`:

```json
{"error": {"message": "No provider succeeded", "type": "no_provider_succeeded",
           "request_id": "...",
           "attempts": ["deepseek: HTTP 400 message: unsupported field 'foo'"],
           "providers": [{"provider": "deepseek", "status": 400,
                          "reason": "HTTP 400", "message": "unsupported field 'foo'",
                          "cooldown_seconds": 60.0}]}}
```

The provider's own message is scrubbed of credential shapes (`sk-...`,
`Bearer ...`, `api_key=...`) before it reaches the response, the operator log, or
the provider-health table. Verified with an injected `sk-...` secret: the failure
body and the log both contain `[redacted]` and never the secret.

Provider health stores the sanitized detail (`last_error = "HTTP 400: unsupported
field 'foo'"`), so the next request's skip reason is diagnostic.

## Known limitations

* **Chat -> Responses translation does not exist.** A Responses-only provider is
  skipped for `/v1/chat/completions`. DeepSeek declares both protocols, so this
  does not affect it.
* **Forced `tool_choice` on DeepSeek Responses still fails.** DeepSeek's native
  `/responses` rejects `tool_choice: "required"` (or a named tool) with
  `HTTP 400 Thinking mode does not support this tool_choice`, and the Responses
  API has no accepted `thinking: {type: disabled}` knob (verified: the parameter
  is accepted but does not lift the restriction). The router forwards the choice
  verbatim rather than silently weakening it, so such a turn fails over or fails
  loudly. Codex sends `tool_choice: "auto"`, which works.
* **Hosted tools are dropped, not implemented.** `web_search` and friends are not
  proxied to a hosted-tool backend even when the provider would accept them.
* The capability probe still uses `/chat/completions`, so a provider that speaks
  only Responses reports `not probed` for tool capability rather than a verdict.
* Reasoning items DeepSeek returns (`output[].type == "reasoning"`) are relayed
  unchanged on the native path; the Chat translation still does not synthesise
  them.

## Verification

```bash
cd services/x3-ai-router
python3 -m unittest test_router          # 125 tests, OK
systemctl --user restart x3-ai-router.service
systemctl --user status x3-ai-router.service --no-pager
curl -sS --max-time 60 http://127.0.0.1:11435/v1/responses \
  -H 'Content-Type: application/json' \
  -d '{"model":"x3-auto","input":"Reply with exactly X3_ROUTER_OK","stream":false}'
```

Recorded live results are appended below.

### Non-streaming

```text
$ curl -sS --max-time 60 http://127.0.0.1:11435/v1/responses -H 'Content-Type: application/json' -d '{"model":"x3-auto","input":"Reply with exactly X3_ROUTER_OK","stream":false}'
```

### Streaming

```text
$ curl -sS -N --max-time 60 http://127.0.0.1:11435/v1/responses -H 'Content-Type: application/json' -d '{"model":"x3-auto","input":"Reply with exactly X3_ROUTER_OK","stream":true}'
```
