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
- **`extra_instructions` works on small focused diffs, not on real PRs.** On a four-line diff
  that injects `let fee = 0.5_f64;` into a `u128` money path, the routed local model reported
  *"Float Usage — use of f64 for a value that should be an exact integer without
  float-exemption comment"*: the AGENTS.md rule reached the model and it applied it. On the
  multi-file #671/#672 diffs the same model returned "No major issues detected". So the
  instruction is worth having, and a clean verdict from an 8B on a large PR is still not
  evidence of anything.

So `config.model` is `deepseek/deepseek-v4-pro`, and `qwen3:8b` is both the `--local` model and the
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
services/pr-agent/review.sh --dry-run 672                # same, prints instead of publishing
services/pr-agent/review.sh --local 672                  # same, on the local GPUs
services/pr-agent/review.sh 672 describe                 # any PR-Agent command
services/pr-agent/review.sh --local --diff x.patch improve
services/pr-agent/review.sh 672 improve --pr_code_suggestions.num_code_suggestions_per_chunk=5
git diff | services/pr-agent/review.sh --stdin review     # review uncommitted work, no PR needed
services/pr-agent/review.sh --diff change.patch review
services/pr-agent/review.sh --via-router --model x3-code 672 review
services/pr-agent/usage.sh                                # what that router run cost
```

Wrapper flags (`--dry-run`, `--local`, `--model`, …) go before the PR or diff. Anywhere after it,
including after the command, they are rejected: PR-Agent silently drops an argument without `=`, so
`672 review --dry-run` used to publish. `--dry-run` on a PR prints what would have been posted: with publishing off PR-Agent keeps
the result in memory and prints nothing, so [dry_run.py](dry_run.py) runs the CLI and prints it.
`review`, `describe`, `improve` and `ask` are covered.

`review` is the default command. A bare number is expanded to
`https://github.com/Cyptopimpinainteazy/xxxstar/pull/<n>`. The GitHub token comes from
`GITHUB_TOKEN` in the env file, else from `gh auth token`; `review.sh` exports it as
`GITHUB__USER_TOKEN` because that is the setting PR-Agent actually reads.

PR-Agent only reads `.pr_agent.toml` from a git provider, which plain-diff runs do not have, so
`review.sh` also passes the file as `--extra_config_url`. That keeps one config for the CLI and the
GitHub Action. `PR_AGENT_NO_EXTRA_CONFIG=1` turns it off.

## Model names

`check.sh` prints the model ids the DeepSeek key returns. On 2026-10-24 the key served
`deepseek-flash` and `deepseek-v4-pro` (not `deepseek-chat`), so the shipped default is
`deepseek/deepseek-v4-pro` with `deepseek/deepseek-flash` and the local model as fallbacks. If the
list changes, set `config.model` to an id from it.

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
- `services/pr-agent/check.sh [cloud|local|router]` — live dependencies: binary, repo config,
  GitHub token, local model, router, DeepSeek key. Reports PASS / FAIL / INFO per row and exits
  non-zero only when a dependency the *selected mode* needs has failed; the other providers are
  INFO. `cloud` is the default, so a bare `check.sh` answers "can a DeepSeek review run".

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

## Automatic reviews

[.github/workflows/pr-agent-auto.yml](../../.github/workflows/pr-agent-auto.yml) publishes on its own,
on the same `x3` runner:

| event | runs |
| --- | --- |
| PR opened, reopened, or marked ready for review | `describe`, `review`, `improve` |
| new commits pushed | `review`, `improve` (the same comments are updated, not added) |
| comment `/review`, `/improve`, `/describe` | that command |
| comment `/ask <question>` | `ask` |

Drafts, fork PRs and Dependabot PRs are skipped, and only OWNER, MEMBER or COLLABORATOR comments run
a command. The runner is self-hosted, so nothing from the PR executes on it: `pull_request_target`
runs the workflow and scripts from the default branch, the checkout pins that branch, and PR-Agent
reads the diff through the API. [on-comment.sh](on-comment.sh) parses the comment: first line only,
four commands, words after `/review` `/improve` `/describe` dropped, and an `/ask` question that
starts with `-` rejected, because PR-Agent reads `--x=y` arguments as settings overrides.

Both jobs check for the `DEEPSEEK_API_KEY` secret first, set up Python 3.12 (the runners'
`/usr/bin/python3` is 3.10, older than PR-Agent needs), and install the pinned `pr-agent` 0.48.0
that [dry_run.py](dry_run.py) is written against. CI sets `CONFIG__FALLBACK_MODELS` to
`deepseek-flash` only, so a DeepSeek outage fails the job instead of publishing a `qwen3:8b` review
under the same comment. `review` and `improve` still run when `describe` fails. A comment run and a
push run on one PR can overlap; the last to finish owns the persistent comment, and the next push
rewrites it.

Status (2026-10-24): merged in #673. The first live run, `/review` on #668, failed at install on
the runners' Python 3.10, which this change fixes. A successful live run has **not** happened yet.

Cost is roughly 30 s to 3 min and a few cents of
DeepSeek per push. `gemma4:26b` was measured as a free alternative and is not one: 417 s on #671 for
"No major issues detected", where `deepseek-v4-pro` took 49 s and flagged the `f64` rate comparison.

## Notes

- `PR_AGENT_VENV`, `PR_AGENT_LOCAL_MODEL`, `PR_AGENT_LOCAL_TIMEOUT`, `PR_AGENT_ROUTER_MODEL`,
  `X3_ROUTER_BASE`, `X3_ROUTER_TOKEN`, `OLLAMA_BASE`, `DEEPSEEK_BASE` and `PR_AGENT_MODE`
  override the defaults.

## Mode isolation

Three things `--local` and `--via-router` must not do, and how each is prevented:

- **Leave the machine.** PR-Agent walks `[model] + fallback_models`, so a `--local` run that
  failed its first attempt would have called DeepSeek. Both modes now export
  `CONFIG__FALLBACK_MODELS='[]'`, leaving the model as the only attempt. This cannot be a
  `--config` override: PR-Agent treats `fallback_models` as host-only and rejects the argument,
  but dynaconf reads it from the environment and that outranks the repo config.
- **Bypass the router's accounting.** Same mechanism; the router does its own fallback internally.
- **Be blocked by the other mode's requirements.** `check.sh` takes a mode (`cloud`, `local`,
  `router`) and only the provider that mode uses can fail the check — a rejected cloud key does
  not stop a `--local` run and a missing local model does not stop a cloud review. The workflow
  derives the mode from its inputs.

Credentials are also direction-safe: the empty `DEEPSEEK_API_KEY=` / `GITHUB_TOKEN=` placeholders
`install.sh` writes no longer blank values a runner injected into the environment.
- The local model must be pulled first: `ollama pull qwen3:8b`.
- PR-Agent is licensed under Apache-2.0 (`pr-agent` 0.48.0).
