# X3 AI router

Start with `python3 router.py --db /path/to/usage.sqlite3`. Point an OpenAI-compatible client at `http://127.0.0.1:11435/v1`, model `x3-auto`. Set `X3_ROUTER_TOKEN` to require bearer authorization, and set `OPENROUTER_API_KEY` or `OPENAI_API_KEY` for cloud providers. `X-X3-Agent` identifies a caller for per-agent budgets. `GET /v1/usage` returns spend accounting. The service binds to loopback; put authenticated TLS in front of it for remote access.

Edit `config.json` for installed Ollama models, provider models, prices, and budgets. Prices are examples and must be set to current provider rates before relying on cost limits. Paid providers with zero prices are skipped. Requests whose JSON exceeds `max_input_tokens` UTF-8 bytes are rejected as a conservative input bound. SQLite reservations enforce the configured budgets across concurrent workers; a provider returning no usage is charged the reserved estimate. Failed requests are not charged locally even if a provider billed them. Provider-side hidden tokens or prices that change without a config update can still produce a higher actual bill.

V1 accepts nonstreaming chat completions. It does not implement semantic caching, context retrieval, verification feedback, or an interactive dashboard. Those require separate evidence and privacy policies; the service does not persist prompts or credentials.
