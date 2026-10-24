# PR-Agent for X3 Atomic Star

[PR-Agent](https://github.com/The-PR-Agent/pr-agent) reviews, describes and improves pull requests.
This directory holds the local install, and [.pr_agent.toml](../../.pr_agent.toml) holds the repository
config shared by the CLI and the GitHub Action.

## Which model

DeepSeek runs the reviews; the GPUs are the fallback.

A review sends the whole diff, the file list and the repository rules in one prompt — 30k–100k
tokens for a PR of this size — and expects structured output back. That is the workload these two
cards are worst at. Measured here on PR #671 (4.2k-token diff, `fix(dex): stop inventing liquidity`):

| command | model | time | result |
| --- | --- | --- | --- |
| `review` | `qwen2.5-coder:7b` (GPU) | 16 s | rejected: output failed schema validation |
| `review` | `qwen3:8b` (GPU) | 40–54 s | valid but empty: "No major issues detected" on a correctness fix |
| `review` | `qwen3:8b` (GPU) | 73 s | same all-clear on #672 after the timeout fix below |
| `improve` | `qwen3:8b` (GPU) | 79 s | one scored suggestion with a code diff (small patch) |
| `improve` | `qwen3:8b` (GPU) | 80 s | none: YAML parse failed, self-reflection fell back to score 7, no suggestion published (#672) |
| `review` | DeepSeek | seconds | not measured — needs `DEEPSEEK_API_KEY` |

Two things this measured that changed the wrapper:

- **PR-Agent's default `ai_timeout` is 120 s, and an 8B here exceeds it.** The first #672 review
  died at exactly 120 s and was retried, wasting a minute. `--local` now passes
  `--config.ai_timeout=600` (`PR_AGENT_LOCAL_TIMEOUT` overrides); the cloud path keeps 120 s.
- **`extra_instructions` cannot raise local review quality.** The config now demands `path:line`
  findings and refuses an unqualified all-clear; `qwen3:8b` still answers
  "No major issues detected" on a diff that deletes a fabricated-liquidity fallback. The
  instruction is aimed at the DeepSeek path, where a model can follow it.

So `config.model` is `deepseek/deepseek-chat`, and `qwen3:8b` is both the `--local` model and the
last fallback. `qwen2.5-coder:7b` is no longer used: it cannot produce the review schema at all.
`--local` is worth using for `--diff`/`--stdin` work where the answer is short and structured;
treat its `review` verdicts as a first pass that found nothing, not as a gate.

## Install

```bash
services/pr-agent/install.sh          # venv at ~/.venvs/pr-agent, secrets file at ~/.config/pr-agent/env
$EDITOR ~/.config/pr-agent/env        # set DEEPSEEK_API_KEY
services/pr-agent/check.sh            # PASS/FAIL per dependency
```

`install.sh` is idempotent and never overwrites an existing secrets file. The file is `chmod 600`
and lives outside the repository, so nothing has to be added to `.gitignore`.

## Use

```bash
services/pr-agent/review.sh 672                          # review PR 672 with DeepSeek, posts a comment
services/pr-agent/review.sh 672 --dry-run                # same, prints instead of publishing
services/pr-agent/review.sh --local 672                  # same, on the local GPUs
services/pr-agent/review.sh 672 describe                 # any PR-Agent command
services/pr-agent/review.sh --local --diff x.patch improve
services/pr-agent/review.sh 672 improve --pr_code_suggestions.num_code_suggestions_per_chunk=5
git diff | services/pr-agent/review.sh --stdin review     # review uncommitted work, no PR needed
services/pr-agent/review.sh --diff change.patch review
services/pr-agent/review.sh --via-router --model x3-code 672 review
services/pr-agent/usage.sh                                # what that router run cost
```

`review` is the default command. A bare number is expanded to
`https://github.com/Cyptopimpinainteazy/xxxstar/pull/<n>`. The GitHub token comes from
`GITHUB_TOKEN` in the env file, else from `gh auth token`; `review.sh` exports it as
`GITHUB__USER_TOKEN` because that is the setting PR-Agent actually reads.

PR-Agent only reads `.pr_agent.toml` from a git provider, which plain-diff runs do not have, so
`review.sh` also passes the file as `--extra_config_url`. That keeps one config for the CLI and the
GitHub Action. `PR_AGENT_NO_EXTRA_CONFIG=1` turns it off.

## Model names

`check.sh` prints the model ids the DeepSeek key returns. The shipped default is
`deepseek/deepseek-chat` with `deepseek/deepseek-flash`, `deepseek/deepseek-v4-pro` and the local
model as fallbacks. If none of those names is accepted, set `config.model` to an id from that list.

## What the review enforces

`[pr_reviewer].extra_instructions` in [.pr_agent.toml](../../.pr_agent.toml) carries the AGENTS.md
rules a generic reviewer does not know: exact money types, no checks behind `cfg(feature = "std")`,
no `WeightInfo = ()`, atomic rollback, and no test that was weakened to pass.

## Cost accounting

A direct DeepSeek call is invisible to this repository. `--via-router` sends the request through
[x3-ai-router](../x3-ai-router/README.md) instead, using its OpenAI-compatible surface and one of
its routing policies (`x3-deep` by default; `x3-code`, `x3-fast`, `x3-local` also work). The router
records the provider, request count and `cost_usd` per day, so `usage.sh` can answer what a PR cost:

```bash
services/pr-agent/review.sh --via-router --model x3-code 672 review
services/pr-agent/usage.sh
# 2026-10-24:
#   deepseek_flash     1 req  $0.0007
#   total: $0.0007
```

Verified on this box: a routed run recorded `ollama_qwen3 1 req $0.0000 (free)` — the transport and
the accounting both work. With no `DEEPSEEK_API_KEY` the router skips DeepSeek (`credential
unavailable`) and falls through to the local model, which is why the recorded provider was Ollama.

## Verification

Two checks, deliberately different:

- `services/pr-agent/selftest.sh` — offline, deterministic, no model call. Parses the config, checks
  the scripts, drives `review.sh` against a recording shim to assert the argv PR-Agent gets
  (PR-number expansion, `--local` with its timeout, `--via-router`, `--dry-run`, override placement
  after the command name, and the three rejection paths), and runs `usage.sh` against
  [fixtures/usage.json](fixtures/usage.json) so cost formatting is covered without a router. It is
  the `pr-agent wiring` gate: `scripts/local-ci.sh --pr-agent --only pr-agent-wiring`
  (or `make local-ci-pr-agent`).
- `services/pr-agent/check.sh` — live dependencies: binary, repo config, GitHub token, local model,
  DeepSeek key. Reports PASS / FAIL / BLOCKED with the reason and exits non-zero only on FAIL.
  A missing key is BLOCKED, not a green light: it means cloud reviews cannot run.

Run `check.sh` after changing the model or endpoint, and `selftest.sh` after changing the scripts.

## GitHub Action

[.github/workflows/pr-agent.yml](../../.github/workflows/pr-agent.yml) runs the same wrapper on
`workflow_dispatch` against the self-hosted `x3` runner, because GitHub-hosted CI is billing-locked
here. A review costs a paid call and publishes a comment, so it is manual rather than per-push. Its
inputs are `pr_number`, `command`, `model`, `via_router`, `use_local` and `dry_run`
(`dry_run` defaults to true).

- Cloud: needs a `DEEPSEEK_API_KEY` repository secret; `GITHUB_TOKEN` covers PR access. The job
  fails at the key step if the secret is absent, before spending a checkout.
- `use_local: true`: skips the key requirement and runs on this runner's GPUs. This is the only way
  to exercise the job on a machine with no key.

Status: `workflow schema` and `workflow wiring` pass; the job itself is **NOT RUN** — dispatch needs
the file on the default branch, and it is not pushed yet.

## Notes

- `PR_AGENT_VENV`, `PR_AGENT_LOCAL_MODEL`, `PR_AGENT_ROUTER_MODEL`, `X3_ROUTER_BASE`,
  `OLLAMA_BASE` and `DEEPSEEK_BASE` override the defaults.
- The local model must be pulled first: `ollama pull qwen3:8b`.
- PR-Agent is licensed under Apache-2.0 (`pr-agent` 0.48.0).
