# Ralph setup for X3‑Chain repo

This directory contains the Ralph agent used to execute the project PRDs.

## Files

* `ralph.sh` – agent runner script (wrapped from the upstream project).
* `prompt.md` – instructions fed to the AI tool each iteration.
* `ralph.conf` – optional configuration overrides (see below).
* `prd.json` – the JSON‑formatted PRD (generated from markdown).
* `progress.txt` – log of previous runs.

### Supported tools

The script now understands four providers:

  * `amp` – Amp CLI (requires paid credits for non‑interactive use)
  * `claude` – Claude Code
  * `ollama` – local Ollama CLI/model (no credits needed)
  * `openrouter` – cloud provider via OpenRouter API key

Use the `TOOL` variable in `ralph.conf` or `--tool` CLI argument to pick one.

For **ollama**, set the `MODEL` variable to specify which local model to run
(e.g. `codellama`, `mistral`, `llama2:7b`). Ollama must be installed and
reachable on the path.

For **openrouter**, export `OPENROUTER_API_KEY` in your environment and
optionally override the default `OPENROUTER_MODEL` in `ralph.conf`.

## Configuration

You can tweak Ralph without editing `ralph.sh` by creating or editing
`ralph.conf` in the same directory.  Supported variables:

```bash
TOOL        # "amp" or "claude" (default amp)
MAX_ITERATIONS
PRD_FILE    # path to prd.json (default is this directory)
PROGRESS_FILE
```

Example `ralph.conf`:

```bash
# use Claude Code instead of Amp
TOOL="claude"

# increase loop limit
MAX_ITERATIONS=20

# work against the complete project PRD
PRD_FILE="/home/lojak/Desktop/x3-chain-master/prd.json"
```

Any setting you add to the config file will override the built-in default.

## Generating prd.json

Ralph operates on a JSON version of your PRD.  To convert a markdown
file (`docs/planning-artifacts/docs/planning-artifacts/PRD_COMPLETE_PROJECT.md` or any other), use the Amp/Claude skill:

```bash
# with Amp CLI installed
amp run "/ralph convert /absolute/path/to/docs/planning-artifacts/docs/planning-artifacts/PRD_COMPLETE_PROJECT.md"
# result is saved as prd.json in this directory
```

If you prefer manual editing, simply create a JSON object matching the
`prd.json.example` format and save it as `prd.json`.

## Running Ralph

```bash
cd /home/lojak/Desktop/x3-chain-master
./scripts/ralph/ralph.sh        # uses config if present
```

You can also specify tool or iterations on the command line; config values
are overridden by CLI args.

## Tips

* Keep a copy of `docs/planning-artifacts/docs/planning-artifacts/PRD_COMPLETE_PROJECT.md` open during execution; the
  agent will often reference story details when committing.
* Commit `ralph.conf` to your repository if you want team members to share
  the same settings.
* When pushing to GitHub, make sure `prd.json` is also committed so remote
  CI jobs can reproduce the sequence.
