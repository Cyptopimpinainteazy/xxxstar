# X3 AI router

Task feedback: attach `X-X3-Task-ID`, `X-X3-Revision` (full candidate commit SHA), and `X-X3-Scope: router` to chat requests. `/v1/tasks` exposes scoped outcomes, request latency, and attributed spend. IDs bind permanently to agent/revision/scope and cannot be reused after finalization. This version records feedback; it does not automatically change routing based on that feedback.

Set a separate `X3_VERIFIER_TOKEN` on the router and only the verifier worker. On a clean committed checkout, run `python3 services/x3-ai-router/verify_task.py --repo /path/to/xxxstar --task-id TASK`. The verifier runs the fixed router test suite, checks that the checkout stayed unchanged, and submits exit codes/output digests. Builder credentials cannot post outcomes. The trusted verifier credential is a trust boundary; the server cannot independently authenticate the truth of submitted check results. A `checks_passed` result covers only this scope, not a verified merge or blockchain-wide correctness.

The bundled DeepSeek, OpenRouter GPT-4.1 Mini and direct GPT-5 prices were checked on 2026-09-29/30. Paid providers are skipped after 30 days unless `pricing_checked_on` and the rates are refreshed together. The direct provider uses `max_completion_tokens` for GPT-5. Pricing is an estimate; reconcile the dashboard with provider invoices.

## DeepSeek (the primary provider)

`deepseek` is listed first on both the routine and critical routes and reads `DEEPSEEK_API_KEY` from the service environment. Per the [DeepSeek API docs](https://api-docs.deepseek.com/quick_start/pricing) the OpenAI-format base URL is `https://api.deepseek.com`, the current model names are `deepseek-flash` and `deepseek-v4-pro`, and both the Responses API and tool calling are supported. `deepseek-flash` is configured; the older `deepseek-v4-flash` alias is retired and is not used here.

Prices are the **peak** rates (input $0.30/M, output $1.20/M); off-peak is half. Reservations are upper bounds, so the peak rate is the safe one to configure. DeepSeek is a paid third party and is accounted as one: it is never marked `free_model`, and its spend lands in `/v1/usage` like any other paid provider.

DeepSeek runs **thinking mode by default**, which has two consequences the router handles rather than hides:

- Reasoning tokens are billed as output tokens, and they count against `max_tokens`. A small output bound can therefore be spent entirely on reasoning and return empty content.
- Thinking mode **rejects a named or required `tool_choice`** with HTTP 400 (`Thinking mode does not support this tool_choice`). For a provider that declares `thinking.disable_when_tool_choice_forced`, the router switches thinking off for exactly those turns and leaves every other turn on the provider default. Verified live against `deepseek-flash`.

Routine requests try DeepSeek first, then OpenRouter, then local Ollama, then the two explicitly free NVIDIA models, then the remaining paid providers. A provider the operator has not opted into is skipped quietly, and a provider that is not declared tool-capable is skipped for any request carrying tools. To opt in to the free cloud endpoints, set `X3_ENABLE_FREE_CLOUD=1` and `OPENROUTER_API_KEY`. They have rate/availability limits and must retain a `:free` model ID with zero prices. NVIDIA warns that its free endpoints log prompts for product improvement; never send secrets or confidential code through them. For a local-only setup, remove cloud names from `routes.routine`. Critical requests never use the free models, not even through `budget_fallback` (see below).

Start with `python3 router.py` (or `deploy/install.sh`, below). The usage database defaults to `$X3_ROUTER_DB`, else `$XDG_DATA_HOME/x3-router/usage.sqlite3` (with `XDG_DATA_HOME` defaulting to `~/.local/share`); its directory is created and it runs in WAL mode with a 30 s busy timeout. `--host` / `X3_ROUTER_HOST` binds a non-loopback address and is refused unless `X3_ROUTER_TOKEN` is set. Point an OpenAI-compatible client at `http://127.0.0.1:11435/v1`, model `x3-auto`. Set `X3_ROUTER_TOKEN` to require bearer authorization, and set `OPENROUTER_API_KEY` or `OPENAI_API_KEY` for cloud providers. `X-X3-Agent` identifies a caller for per-agent budgets. `GET /v1/usage` returns spend accounting. The service binds to loopback; put authenticated TLS in front of it for remote access.

Edit `config.json` for installed Ollama models, provider models, prices, and budgets. Prices are examples and must be set to current provider rates before relying on cost limits. Paid providers with zero prices are skipped. Requests whose JSON exceeds `max_input_tokens` UTF-8 bytes are rejected as a conservative input bound. SQLite reservations enforce the configured budgets across concurrent workers; a provider returning no usage is charged the reserved estimate. Failed requests are not charged locally even if a provider billed them. Provider-side hidden tokens or prices that change without a config update can still produce a higher actual bill.

## Request validation

## Routing intelligence

`x3-auto` used to be a label over one fixed chain per tier. It is now a
policy decision, and the decision is auditable without spending a token:

```bash
curl -s -X POST http://127.0.0.1:11435/v1/explain -H 'Content-Type: application/json' \
  -d '{"messages":[{"role":"user","content":"Fix the consensus finality bug"}]}'
```

```json
{"policy":"x3-security","tier":"critical",
 "provider_order":["deepseek","direct","ollama"],
 "classification":{"task_class":"CONSENSUS","risk":"critical",
   "blast_radius":["consensus","finality"],"complexity":"medium",
   "verification":["unit","integration","local-ci","audit"],"parallelizable":"low"}}
```

### Task classification

A deterministic keyword classifier, not a model call: asking a model which
model to use would add the cost, the latency and the nondeterminism the router
exists to manage. It names one of the 23 classes in the Forge spec and
estimates complexity, risk, blast radius, context requirement, verification
requirement and parallelizability.

It scores the **task text only** — user, assistant and tool messages. Codex
sends ~17KB of instructions that mention security, consensus and testing in the
abstract, and scoring those would classify every request as everything.

### Logical models are routing policies

`GET /v1/models` lists them; each one is an ordered provider preference, not a
model name:

| Logical model | Order | Tier |
| --- | --- | --- |
| `x3-auto` | class decides | derived |
| `x3-fast` | ollama → deepseek → openrouter | routine |
| `x3-code` | deepseek → openrouter → ollama | routine |
| `x3-deep` | deepseek → openrouter → direct | routine |
| `x3-security` | deepseek → direct | critical |
| `x3-review` | deepseek → openrouter | routine |
| `x3-local` | ollama only | routine |

A client that sends one of these as its `model` is giving a routing
instruction and is obeyed. Anything else is the alias Codex sends, resolved
from the class. Capability, health, budget and privacy checks all still apply
to whichever provider the policy names.

**A critical classification is a floor, not a ceiling.** A policy cannot
downgrade it: a request containing consensus or settlement terms stays critical
even when its task class would route to the cheap chain, because that check is
what keeps such code off third-party providers.

### Measured capability registry

`GET /v1/registry` reports what each provider and model has actually done
rather than what its config claims: attempts, failures, failure rate, retries,
retry rate, average latency, tokens, cost, and the verified-patch rate joined
from the task feedback table. Latency is recorded for **failed** attempts too —
a provider that is fast when it works and slow when it times out is a
different routing proposition from one that is uniformly slow, and averaging
only the successes hides exactly that.

### Bounded retries

A provider is retried on a status that means "try again" (`408, 409, 425, 429,
500, 502, 503, 504`) or a transport error, up to `retry_attempts` (default 1)
with exponential backoff capped by `retry_backoff_max_ms`. A `400`, `401` or a
malformed body is not retried: repeating an identical request produces the
identical answer, so a retry only spends money. On a stream, a retry is legal
only **before the first byte reaches the client** — after that the client
already has half an answer and gets a failure event instead.

### Context compiler integration (§7)

The router reaches the Forge context compiler, which answers the other half of
a task's question: not which model, but what it should read.

```bash
curl -s "http://127.0.0.1:11435/v1/context?q=where+is+atomic+settlement+enforced"
curl -s -X POST http://127.0.0.1:11435/v1/explain -H 'Content-Type: application/json' \
  -d '{"context":true,"messages":[{"role":"user","content":"fix the refund race"}]}'
```

```json
{"policy":"x3-security","tier":"critical","classification":{"task_class":"CROSS_CHAIN"},
 "context":{"elapsed_ms":13517.4,"package":{"index_commit":"893261786e1c…",
   "included":[{"path":"crates/x3-atomic-swap/src/cairo_vm_htlc.rs","sha256":"…",
                "estimated_tokens":4125}]}}}
```

The package is the compiler's own, provenance included, so a selection can be
checked rather than trusted. The router supplies the query — the same task text
the classifier scores, with system and developer messages excluded — and does
not reimplement any of the selection.

`context: true` is opt-in on `/v1/explain` because compiling is a subprocess
reading a large index: ~13s for a narrow query, ~31s for a broad one. It never
runs on the routing path. `budget` and `max_files` pass through to the
compiler. An unconfigured, failing, slow or non-JSON compiler is reported as
`502 context_unavailable` naming the reason — never as an empty package.

### Failure memory integration (§17–19)

`tools/x3-forge/failure_memory.py` records failures, successes and dead ends as
one append-only log. Append-only because a memory that rewrites itself cannot
be audited, and this one is evidence: "we already tried this and it failed" is
only useful if the record cannot quietly change.

The router writes to it and reads from it:

* a submitted task outcome becomes an entry — a failure carrying the failing
  check and the provider that produced it, or a success (§18: "also record what
  worked");
* `GET /v1/memory?q=<task>` searches it, which is §17's "before debugging:
  search failure memory";
* `POST /v1/explain {"memory": true}` returns the routing decision *and* the
  known failures for the task, so a known problem is not rediscovered.

```json
{"policy":"x3-deep","tier":"routine",
 "classification":{"task_class":"DEBUGGING","verification":["unit","reproduction"]},
 "memory":{"matches":[{"fingerprint":"8105f8488a1de5a6",
   "component":"crates/cross-vm-coordinator",
   "error":"abort after complete refunded both legs at line 412",
   "fix":"route abort through the same transition table"}]}}
```

The fingerprint normalises hex addresses and reported line/column positions, so
repeats of one bug collapse into one row with a sighting count — twelve
thousand failures become three root causes, which is the difference between a
usable memory and a log nobody reads. `kind` is part of the fingerprint: the
same error text as a dead end and as a failure are different knowledge.

**Memory is advisory.** The verification result is committed before the memory
write is attempted, and a memory outage is reported (`"recorded": false`),
never raised: losing a memory entry must not undo a passed check.

A reservation is an upper bound on one call, so the request must not be able to spend more than the reservation covers. Two shapes used to get through:

- **A second output parameter.** The estimate read `max_tokens` and fell back to 4096, so a request that set `max_completion_tokens` instead — which is what GPT-5 on the direct provider requires — was reserved at the default and billed for whatever it asked. Both parameters are now validated against `max_output_tokens`, and the estimate uses whichever one the client set.
- **Multiple completions.** `n` and `best_of` multiply the completions a provider bills for while the estimate assumed one. Anything other than `1` is refused with `400`.

`max_output_tokens` (32768), `default_max_output_tokens` (4096), `reservation_ttl_seconds` (900), `provider_cooldown_seconds` (60) and `provider_cooldown_max_seconds` (3600) are config knobs.

## Running out of money degrades the model, it does not stop the work

An exhausted budget used to end the request: `reserve()` returned nothing and
the router answered `429` without trying anything else. Two things were wrong
with that. A zero-cost provider could never get a reservation once spend passed
the ceiling, so the free local model was blocked by a paid API being over
budget — "stop spending" became "stop working". And a paid provider being
unavailable was treated as fatal even when a free one could answer.

`budget_fallback` is now appended after the configured route. It is only
reached when nothing better answered. Providers that cannot bill need no
reservation at all. If every provider in the chain *and* the fallback fails,
the request gets a `429` with `type: budget_exceeded` and an `attempts` list
naming each refusal.

The fallback cannot clear a provider for *critical* work. A critical request
may only be served by a provider with `critical_allowed: true`, or by one that
carries no credentials and therefore talks to a model on this machine
(`may_serve_critical` in `router.py`). Being listed in `budget_fallback` is not
clearance: the default list ends with the free cloud models, whose operator
logs prompts. A critical request whose cleared providers are unavailable fails
closed and names the refusal, instead of silently leaving the machine.

The default fallback is local Ollama first, then the two free cloud models, so
routine code stays on the machine when it can and only leaves it if the local
model cannot answer. Set `budget_fallback` to `[]` to restore hard-stop
behaviour.
Which model actually answered is visible in the response's `model` field and in
`/v1/usage`; a fallback answer is not distinguished from a paid one beyond
that, so watch for the local model name in accounting if you need to know how
often the downgrade happened.

## Provider cooldowns

A provider that fails is skipped for a doubling delay, capped, with `Retry-After` from an HTTP error taking precedence when the provider sends one. Without this, every request in turn paid the timeout of an endpoint that was already down. A success clears the record. `GET /v1/providers` shows consecutive failures and the remaining cooldown, and the skip reason is reported in the `502` body's `attempts`.

Every failure also carries `request_id`, and each refusal is logged on one
sanitized line — request id, provider, HTTP status, reason and remaining
cooldown. No prompt text, headers or credential values are logged, and no
failing client is handed that detail beyond the provider's own status.

## Agent requests need a tool-capable provider

A request carrying `tools` (or legacy `functions`) needs a provider that can
call them. A text-only model answers in prose and the agent waits for a tool
call that can never arrive, so a provider must declare `"supports_tools": true`
to be handed an agent request; otherwise it is skipped and the reason is
reported. A plain text request to the same provider still works.

### The local model is not a tool-capable fallback (measured, not assumed)

`qwen2.5-coder:7b` reports `capabilities: ["completion", "tools", "insert"]`
from `POST /api/show`, and an earlier revision of this file declared it
tool-capable on that basis. That was wrong. Asked to call a tool — with
`tool_choice: "required"`, three times — the model returned the call as
**prose** and never populated `tool_calls`:

    tool_calls: null
    content: '{"name": "exec_command", "arguments": {"cmd": "ls -1"}}'

The other installed models were no better: `huihui_ai/qwen3-abliterated:1.7b`
returned empty content, and `leonardoba500/deepseek-v41-uncensored` was over its
monthly quota and could not be tested at all. So the router is correct to refuse
rather than to reinterpret: treating that JSON as an executable tool call is
exactly the failure the Responses translation exists to prevent. `ollama` is
therefore declared `supports_tools: false`, and an agent request routed to it
fails closed with
`ollama: model is not declared tool-capable` while plain text work still
succeeds locally. A local agent fallback needs a model that genuinely emits tool
calls; installing one is a model choice, not a router change.

### Capability probes

A declaration is a claim, so a provider may opt into verification with
`"tool_probe": true`. The probe asks the provider for a required tool call and
accepts only a real `tool_calls` array. Configuration may grant a capability;
only evidence may take it away:

| probe result | effect |
| --- | --- |
| genuine `tool_calls` | provider stays usable for agent requests |
| text with no `tool_calls` | provider is refused for agent requests, named in the failure |
| probe could not run | reported as unknown; a working declaration is not revoked |

Probes run once in the background at startup — a slow provider never holds up
the listener — and the verdict is cached per provider and model for
`capability_probe_ttl_seconds` (3600). `GET /v1/capabilities` reports the
cached verdicts; `?probe=1` re-checks on demand. The report separates the
declaration from the verdict, so `declared_tools: true` next to
`probed_tools: false` is visible rather than papered over. Live at the time of
writing: `deepseek-flash` returned a genuine tool call, `qwen2.5-coder:7b` did
not.

`/health` remains a liveness check only. It does not mean an agent route works;
`/v1/capabilities` is where that question is answered.

### Reasoning effort

Codex sends `reasoning: {"effort": ...}`. It is carried through translation and
sent only to a provider declaring `"reasoning_effort": true`, so the field is
not sprayed at every other provider in the chain. DeepSeek accepts the
parameter alongside both thinking modes, including the forced-tool-choice turns
where thinking is switched off, so the effort the operator selected is not
silently dropped.

`max_tokens` is pinned on every upstream call, even when the client sent no
output bound, so the request can never be billed for more than the reservation
covers. Raising `default_max_output_tokens` is the lever if a model needs a
longer answer than the 4096 default.

A client that disconnects mid-stream is ordinary, not an incident: the
reservation is released, whatever the provider already produced is charged, no
traceback is printed, and the request is recorded with reason `client
disconnected`. Codex does this on most turns once it has the item it wanted.

## Crash recovery

A reservation is deleted only by `finish`, which runs in the request thread. If the router died between reserving budget and calling the provider, nothing deleted the row: `reserved_usd` grew all day and the budget was consumed by requests that were not running. Reservations now carry `created_at`, and startup reclaims any older than `reservation_ttl_seconds`. The TTL is longer than any provider timeout, so a live request is never reclaimed. `reconciled_orphans` appears in the snapshot, on `/metrics` and in the dashboard.

## Client compatibility

Two wire protocols are served, over the same provider chain, budgets, cooldowns
and fallback:

- **Chat Completions** at `/v1/chat/completions`, streaming and non-streaming.
  `tools`, `tool_choice`, `functions`, `response_format`, `stop`, `temperature`
  and `seed` are forwarded unchanged.
- **Responses** at `/v1/responses`, streaming and non-streaming. This is not a
  passthrough: no provider this router talks to speaks the Responses protocol,
  so requests are translated to Chat Completions and the answer is translated
  back. `instructions` becomes a system message, `input` items become
  messages / assistant tool calls / tool results, function tools are flattened
  out of the Responses shape (namespaced tools are flattened too), and the
  stream is re-emitted as
  `response.created` → `response.output_item.added` →
  `response.output_text.delta` / `response.function_call_arguments.delta` →
  `...done` → `response.completed`.

### What Codex actually sends

`codex-cli 0.159.2` was captured against a local endpoint rather than guessed at.
A trivial prompt arrives as ~56 KB with `instructions`, `input`, `tool_choice`,
`parallel_tool_calls`, `reasoning`, `include`, `text`, `store` and `client_metadata`,
and a tool set of ten entries in four different shapes:

| shape | example | how it is carried |
| --- | --- | --- |
| `function` | `exec_command`, `view_image`, `update_goal` | flattened to a Chat Completions function tool |
| `custom` | `apply_patch` (freeform, Lark grammar) | one required `input` string argument, lifted back out as a `custom_tool_call` |
| `namespace` | `collaboration` | members flattened, since chat has no namespace |
| `web_search` | `external_web_access: false` | dropped, because the client itself disabled it |

The `custom` case is the one that matters for editing: Chat Completions has no
freeform tool type, so the freeform body travels as a single `input` string and
is rebuilt as a `custom_tool_call` item (`call_id`, `name`, `input`) on the way
back. The client's own schema validation is unaffected; only the grammar is lost.
A `web_search` with `external_web_access: true`, or any tool type with no chat
equivalent (`computer_use`, `file_search`, `image_generation`, `code_interpreter`,
`mcp`, `local_shell`, `shell`), is refused with `400 unsupported_feature` naming
the tool. Nothing is silently dropped.

### Failure semantics

The stream reports what actually happened:

- `response.completed` only when the upstream reached `data: [DONE]` or a finish
  reason. Reading past `[DONE]` to wait for the socket to close used to hang the
  handler on a keep-alive connection until the client gave up.
- `response.incomplete` (with `incomplete_details`) when the upstream stopped on
  `length`, or ended without a terminal event.
- `response.failed` when the upstream errored after the headers were sent. An
  interrupted answer is never dressed up as a completed one.

`response.completed` carries the real `usage` and the model that answered
(`deepseek-flash`, not the `x3-auto` alias the client sent).

The Responses endpoint is what makes Codex usable with this router: Codex
accepts only `wire_api = "responses"` for a custom provider. It was verified
with real `codex exec` runs, including a tool round trip — `exec_command`
reached the shell, its stdout came back as a `function_call_output`, and the
model answered from it — and an `apply_patch` round trip that created a file on
disk through the custom-tool path.

The router chooses the model, so the client's `model` is accepted and ignored.
`GET /v1/models` lists `x3-auto` and `GET /v1/models/x3-auto` serves it, which
is what clients probe before their first call. `/v1/embeddings` and
`/v1/audio/*` still answer `501` naming the gap rather than a `404` that reads
as a wrong base URL.

`max_input_tokens` bounds the request body in UTF-8 bytes and must exceed what
the client actually sends. Codex sends ~94 KB for a trivial prompt, because the
system prompt, tool schemas and project instructions all ride along; the
default is 200000. A client rejected with `413` is usually this bound, not the
provider's context window.

## Operational notes

Open `/v1/dashboard` for a local spend dashboard or scrape `/metrics` for Prometheus. When `X3_ROUTER_TOKEN` is set, the dashboard accepts HTTP Basic username `x3` and that token as password; API clients can keep using bearer auth. Both views require authentication and show daily spend, reservations, and completed requests.

Streaming requests ask providers for a usage event. When usage is unavailable, the reserved estimate is charged. A provider may be retried before the first SSE event; a broken partial stream closes without switching models.

Still missing, in the order they matter for relying on this with X3 agents:

- **Verified escalation.** Fallback reacts to provider failures, not to patches that fail their checks. The feedback work records which patches pass, but nothing routes on it yet.
- **Privacy controls.** There is no per-task local-only / trusted-cloud / public-code route enforcement. `routes.routine` is a single global list (the critical tier refuses third-party providers, but nothing enforces a policy per task).
- **Context retrieval.** No repo index or context packets; prompts carry their own context, which is why a Codex turn arrives with ~19K input tokens.
- **Operational security.** One router token, no per-agent credentials; the usage database is an unencrypted SQLite file.

The service does not persist prompts or credentials.

## Dual local GPU workers (x3gpu1)

Two independent Ollama processes, one per card, each pinned by **UUID**
(`nvidia-smi` lists the GTX 1070 as 0 and the RTX as 1; CUDA's default
fastest-first order is the reverse, so a bare index lands on the wrong card):

| Worker | Port | GPU | Unit | Providers |
| --- | --- | --- | --- | --- |
| A | 127.0.0.1:11434 | RTX 2060 SUPER | system `ollama.service` + `deploy/ollama-rtx.conf` | `ollama`, `ollama_qwen3` (worker `gpu-rtx`) |
| B | 127.0.0.1:11436 | GTX 1070 | user `ollama-worker-b.service` | `ollama_gtx`, `ollama_gtx_qwen3` (worker `gpu-gtx`) |

Each worker runs one slot (`OLLAMA_NUM_PARALLEL=1`: parallel slots multiply
the KV cache and push qwen3:8b off an 8 GB card) with flash attention and a
q8_0 KV cache. Providers that share a `worker` share its `max_in_flight`
slots; when a worker is full, `attempt_order` moves its providers to the back,
so a second concurrent request goes to the other card instead of queueing.
The slot is claimed atomically when the provider is chosen. A dead worker
goes on the usual exponential cooldown and its requests fail over to the
other card, then to DeepSeek if `DEEPSEEK_API_KEY` is set.

Policies: `x3-code` → RTX first; `x3-review` → GTX first; `x3-fast` → GTX
coder first; `x3-local` → both cards only. `GET /health` reports `in_flight`
per worker.

Install (user services survive logout/reboot only when linger is enabled; the
script warns if it is not, and does not enable it):

```bash
services/x3-ai-router/deploy/install.sh            # router + GTX worker (user units)
services/x3-ai-router/deploy/install.sh --system   # also pin ollama.service to the RTX (sudo)
```

The units in `deploy/` are templates: the installer fills in this checkout's
path and the `ollama` binary on `PATH`, so it works from any clone. It also
creates `~/.config/x3-router/providers.d/`, where GPU nodes' registrations
(`scripts/x3-cluster`) go; each `<node>.json` there is loaded at start-up,
type-checked, and appended after the configured providers. A registered
provider never serves critical requests: it may run on another machine, so
having no API key is not evidence that the model is local.

Secrets go in `~/.config/x3-router/env` (mode 600). Live check + benchmark on
the real cards (stops/starts worker B to prove failover), evidence written to
`audit-artifacts/gpu-ai/`:

```bash
python3 services/x3-ai-router/live_dual_gpu_check.py --iterations 5
```
