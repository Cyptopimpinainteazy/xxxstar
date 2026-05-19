# X3 Mainnet Progress Web Dashboard

This dashboard renders auto-generated progress data from the canonical scorecard.

## Files

- `index.html` - dashboard UI
- `styles.css` - dashboard styles
- `app.js` - dashboard logic
- `data/mainnet_progress.json` - auto-generated progress data (do not edit manually)
- `data/mainnet_goals.json` - auto-generated goals data (do not edit manually)

## Regenerate Data

Run from repository root:

```bash
scripts/update_mainnet_progress_dashboard.sh
```

This reads:

- `X3_MAINNET_READINESS_SCORECARD_2026_04_22.md`
- `config/mainnet_goals_config.json` + markdown todos in relevant docs

And updates:

- `web/mainnet-progress/data/mainnet_progress.json`
- `web/mainnet-progress/data/mainnet_goals.json`
- `X3_MAINNET_PROGRESS_AUTOGEN.md`
- `X3_MAINNET_GOALS_STATUS_AUTOGEN.md`

## Goals Engine

Run goals/todo tracker generation:

```bash
python3 scripts/update_mainnet_goals_status.py
```

Optional: apply status overrides directly to markdown source checkboxes:

```bash
python3 scripts/update_mainnet_goals_status.py --apply-source
```

Optional: fail command if open actionable todos remain:

```bash
python3 scripts/update_mainnet_goals_status.py --check
```

## CI Workflow

The auto-generated data files must be kept in sync with source files. CI pipelines should:

1. **Pre-commit check** (local developer workflow):
   - Run regeneration scripts: `scripts/update_mainnet_progress_dashboard.sh` and `python3 scripts/update_mainnet_goals_status.py`
   - Verify generated outputs have changed if source files changed
   - If outputs are stale, fail with message to run regeneration locally

2. **CI validation** (automated):
   - Run both regeneration scripts
   - Compare generated outputs with committed versions
   - Fail build if outputs differ (indicates stale data was committed)
   - Record generation timestamps and source file hashes in CI logs

3. **Regeneration gates**:
   - `scripts/update_mainnet_progress_dashboard.sh` exits with code 1 if dashboard sources are stale in CI mode (set `CI=true` environment variable)
   - `scripts/update_mainnet_goals_status.py --check` exits with code 1 if goals checklist has uncommitted changes

### Example CI Job

```yaml
- name: Validate mainnet progress data
  run: |
    scripts/update_mainnet_progress_dashboard.sh
    python3 scripts/update_mainnet_goals_status.py --check
    git diff --exit-code web/mainnet-progress/data/ X3_MAINNET_PROGRESS_AUTOGEN.md X3_MAINNET_GOALS_STATUS_AUTOGEN.md
```

Goals config reference:

- `X3_MAINNET_GOALS_CONFIG_GUIDE.md`

## Run Locally

From repository root:

```bash
python3 -m http.server 8000
```

Then open:

- `http://localhost:8000/web/mainnet-progress/`

## Auto-Update

The workflow `.github/workflows/markdown-reconcile.yml` runs the generator on push and commits updated dashboard data automatically when values change.
