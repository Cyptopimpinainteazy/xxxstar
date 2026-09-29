# X3 AI router

Start with `python3 router.py --db /path/to/usage.sqlite3`. Point an OpenAI-compatible client at `http://127.0.0.1:11435/v1`, model `x3-auto`. Set `X3_ROUTER_TOKEN` to require bearer authorization, and set `OPENROUTER_API_KEY` or `OPENAI_API_KEY` for cloud providers. `X-X3-Agent` identifies a caller for per-agent budgets. `GET /v1/usage` returns spend accounting. The service binds to loopback; put authenticated TLS in front of it for remote access.

Edit `config.json` for installed Ollama models, provider models, prices, and budgets. Prices are examples and must be set to current provider rates before relying on cost limits. A zero price means no monetary accounting for that provider. Input reservation assumes at most `max_input_tokens`; enforce input limits upstream for strict budgets. Usage is charged from provider-reported token counts. Failed requests are not charged locally even if a provider billed them.

V1 accepts nonstreaming chat completions. It does not implement semantic caching, context retrieval, verification feedback, or an interactive dashboard. Those require separate evidence and privacy policies; the service does not persist prompts or credentials.
