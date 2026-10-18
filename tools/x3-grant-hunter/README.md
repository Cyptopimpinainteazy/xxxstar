# X3 GPU Grant Hunter MVP
Python 3.10+, standard library only. Local metadata seed catalog, eligibility gate, draft generator, and safe queue. **No live provider provisioning or automatic application submission.** URLs are reference sources, not verified award offers.

```bash
python3 hunter.py discover --company forming
python3 hunter.py discover --company forming --fetch  # optional network reachability checks, not eligibility validation
python3 hunter.py draft hf-community
python3 hunter.py allocate x3-inference-benchmark
python3 -m unittest discover -s tests -v
```

The `allocate` command creates a pending local job only; it cannot claim GPUs or bypass quotas. Add an approved provider adapter with authentication, quota inspection, API-specific job execution and cost guardrails before enabling real dispatch. Never commit credentials. Use a daily cron/systemd timer for `discover`; sources are static until an API-backed discovery adapter is added. Human review is mandatory for applications and paid-resource creation.

## Deep scavenger extension
`opportunities-extra.json` adds verified leads (not automatically awarded capacity).
`community.py` registers **owner-consented** nodes and matches VRAM/workload permission. No network scans, untrusted job execution, or automatic dispatch.

```bash
python3 community.py register --node-id donated-1070 --gpu 'GTX 1070' --vram 8 --owner-consent --workloads inference benchmark --max-hours 2
python3 community.py match --vram 6 --workload benchmark
```

Next production steps: authenticated opt-in agent, sandboxed allowlisted jobs, signed results, GPU memory/time limits, per-node revoke, and a zero-billing provider adapter.

## GrantOps pipeline (new)

Local-only grant opportunity CRM; no automatic applications, email, purchases or provider enrollment.

```bash
python3 grantops.py import leads.json
python3 grantops.py list --company forming
python3 grantops.py move example researching
python3 grantops.py move example submitted --approval-record 'approved-by-owner-and-submission-receipt-reference'
python3 -m unittest discover -s tests -v
```

`leads.json` is a JSON array of records with `id`, `name`, `official_url` (HTTPS), `category` (gpu/cloud/blockchain/security/research/other), and optional `amount_usd`, `deadline` (YYYY-MM-DD), `entity_required`, `notes`. Source URLs and amounts are **user-supplied leads, not automatically verified**. Scores are heuristic prioritization, never eligibility determinations. The `move` command only updates local records; it does not submit applications. Human approval and evidence must be verified externally; an approval reference string alone is not proof.

## Discovery engine v3

`discovery.py` fetches **operator-approved JSON feeds** over HTTPS. It does not scrape arbitrary websites or claim to support a particular provider's API without an adapter. It requires a public-resolving hostname on an explicit allowlist, rejects redirects, limits payload size, and times out network calls. These are basic safeguards, not a complete defense against DNS rebinding; only configure trusted provider domains.

Create `sources.json` (no default external feeds are assumed):

```json
{"sources": [{"url": "https://YOUR-TRUSTED-PROVIDER.example/opportunities.json", "allowed_hosts": ["your-trusted-provider.example"]}]}
```

The endpoint must return a JSON array of records in the `grantops.py import` schema. This is an **adapter contract**, not a claim that any grant agency publishes this exact endpoint.

```bash
python3 discovery.py sources.json                   # dry-run, no database mutation
python3 discovery.py sources.json --import-reviewed # import as unverified discovered leads
python3 grantops.py list --company forming
python3 -m unittest discover -s tests -v
```

No automatic applications, email, spending, or eligibility assertions. For Grants.gov, NSF, cloud credits and other sources, implement and test source-specific parsers against their published APIs/terms before enabling them. Do not use unattended scraping or treat search results as verified awards.
