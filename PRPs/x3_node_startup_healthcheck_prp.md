name: "X3 Node Startup Health Check Script"
description: |

## Purpose
Create a repeatable, low-risk health check script for `x3-chain-master` that validates node startup prerequisites, key environment files, and critical service ports before launch.

## Core Principles
1. **Context is King**: mirror established script style in this repository
2. **Validation Loops**: include commands that are runnable locally
3. **Information Dense**: reuse existing ports, env vars, and launcher behavior
4. **Progressive Success**: verify prerequisites first, then runtime readiness checks
5. **Global rules**: follow all rules in `CLAUDE.md`

---

## Goal
Implement a new script `scripts/x3_node_healthcheck.sh` that operators can run pre-launch to catch common local configuration issues quickly.

## Why
- Reduce failed node starts due to missing binaries, env vars, or occupied ports
- Standardize a preflight check used by dev/staging/prod launch scripts
- Provide deterministic pass/fail output for runbooks and CI-like local checks

## What
Create a shell-based preflight utility that checks:
1. Required commands (`cargo`, `bash`, `curl`; optional `nc`, `lsof`)
2. Node binary existence or build path readiness
3. Launch script prerequisites (`NODE_NAME` when production mode selected)
4. Key env file presence for app integrations (`apps/*/.env.local` or ability to generate via `setup-app-env.sh`)
5. Port availability for node defaults:
   - Dev defaults from `run-dev-node.sh`: RPC `9944`, WS `9945`, P2P `30333`, Prometheus `9615`
   - Prod defaults from `run-production-node.sh`: RPC `9944`, P2P `30333`, Prometheus `9615`
6. Optional live health probe if node is already running (`/health`, metrics endpoint)

### Success Criteria
- [ ] `scripts/x3_node_healthcheck.sh` created and executable
- [ ] Script supports `--mode dev|prod` (default `dev`)
- [ ] Script reports clear PASS/WARN/FAIL summary and exits non-zero on FAIL
- [ ] Script references existing launch defaults from current scripts (no hardcoded drift)
- [ ] `README` or ops doc updated with usage command

## All Needed Context

### Documentation & References
```yaml
- file: run-dev-node.sh
  why: canonical dev ports/defaults and readiness behavior

- file: run-production-node.sh
  why: production env requirements (`NODE_NAME`) and secure defaults

- file: setup-app-env.sh
  why: expected `.env.local` generation pattern for apps

- file: validate-test-framework.sh
  why: output style (PASS/FAIL/WARN counters) and helper check functions

- file: RUN_ALL_TESTS.sh
  why: consistent terminal reporting style and summary structure

- file: NODE_REQUIREMENTS.md
  why: deterministic boot and node requirement framing

- file: DEVELOPMENT.md
  why: operational guidance around node startup and metrics

- file: CONFIG.md
  why: tiered config/ports and environment variable expectations

- file: X3_GOLIVE_CHECKLIST.md
  why: go-live gate style and operator-friendly command sequences
```

### Current Codebase tree (relevant subset)
```bash
x3-chain-master/
├── run-dev-node.sh
├── run-production-node.sh
├── setup-app-env.sh
├── validate-test-framework.sh
├── RUN_ALL_TESTS.sh
├── NODE_REQUIREMENTS.md
├── DEVELOPMENT.md
├── CONFIG.md
├── X3_GOLIVE_CHECKLIST.md
├── apps/
│   ├── explorer/
│   ├── wallet/
│   ├── dex/
│   └── x3-intelligence/
└── scripts/
```

### Desired Codebase tree
```bash
x3-chain-master/
├── scripts/
│   └── x3_node_healthcheck.sh        # preflight checks for dev/prod node startup
└── DEVELOPMENT.md or X3_DEPLOYMENT_SOP.md
    # add short usage section for the healthcheck command
```

### Known Gotchas of this codebase
```bash
# Dev launcher can auto-kill processes using ports unless --keep-ports is passed.
# Production launcher requires NODE_NAME and must not run as root.
# RPC and WS defaults differ from some Substrate defaults in docs; use repo scripts as source of truth.
# Some checks should be WARN (optional tools), not FAIL (hard requirements).
# If binary is missing, preflight should recommend build command, not auto-build by default.
```

## Implementation Blueprint

### Data models and structure
No Python models required. Bash functions for checks + summary counters.

### list of tasks to be completed to fulfill the PRP in order
```yaml
Task 1:
CREATE scripts/x3_node_healthcheck.sh:
  - Add strict mode: `set -euo pipefail`
  - Add ANSI colors + PASS/WARN/FAIL counters
  - Add helper functions: `check_command`, `check_file`, `check_port_free`, `warn`, `fail`, `pass`
  - Add argument parser for `--mode dev|prod` and optional `--strict`

Task 2:
IMPLEMENT prerequisite checks:
  - Commands: cargo, bash, curl as required
  - Optional commands: nc, lsof, netstat as warning-only
  - Binary: `target/release/x3-chain-node` existence check
  - For prod mode: enforce `NODE_NAME` present (fail if missing)

Task 3:
IMPLEMENT environment and app config checks:
  - Validate `apps/explorer/.env.local`, `apps/wallet/.env.local`, `apps/dex/.env.local`, `apps/x3-intelligence/.env.local`
  - If missing, provide fix hint: `./setup-app-env.sh`

Task 4:
IMPLEMENT port and live health checks:
  - Check dev/prod default ports from launch scripts
  - Report whether each port is free/occupied
  - If occupied, report PID/process when available
  - Optional probe:
    - `curl http://127.0.0.1:9944/health` (warning if unavailable)
    - metrics endpoint on `9615` (warning if unavailable)

Task 5:
DOCUMENT usage:
  - Add small section to DEVELOPMENT.md (or X3_DEPLOYMENT_SOP.md)
  - Include examples:
    - `bash scripts/x3_node_healthcheck.sh`
    - `bash scripts/x3_node_healthcheck.sh --mode prod`

Task 6:
VALIDATE and finalize:
  - run shell syntax check
  - run script in dev and prod modes
  - ensure exit codes behave correctly
```

### Per task pseudocode (critical details)
```bash
# Task 1/2 pseudocode
MODE="dev"
STRICT=false

parse_args() {
  # --mode dev|prod, --strict
}

check_command() {
  local cmd="$1" required="$2"
  if command -v "$cmd" >/dev/null 2>&1; then pass "Found $cmd";
  elif [[ "$required" == "required" ]]; then fail "Missing $cmd";
  else warn "Missing optional $cmd"; fi
}

# Task 4 pseudocode
check_port_free() {
  local port="$1" label="$2"
  # Use lsof if present, fallback to ss/netstat; occupied -> warn or fail per mode/strict
}

# Final summary
if [[ "$FAIL" -gt 0 ]]; then exit 1; fi
if [[ "$STRICT" == true && "$WARN" -gt 0 ]]; then exit 2; fi
exit 0
```

### Integration Points
```yaml
SCRIPTS:
  - new: scripts/x3_node_healthcheck.sh
  - reference: run-dev-node.sh, run-production-node.sh, setup-app-env.sh

DOCS:
  - update: DEVELOPMENT.md (preferred) OR X3_DEPLOYMENT_SOP.md

OPERATIONS:
  - preflight command should be runnable before ./run-dev-node.sh or ./run-production-node.sh
```

## Validation Loop

### Level 1: Syntax & shell quality
```bash
bash -n scripts/x3_node_healthcheck.sh
```

### Level 2: Functional checks
```bash
# dev mode
bash scripts/x3_node_healthcheck.sh --mode dev

# prod mode (without NODE_NAME should fail)
bash scripts/x3_node_healthcheck.sh --mode prod || true

# prod mode with NODE_NAME should pass prerequisites
NODE_NAME=validator-local bash scripts/x3_node_healthcheck.sh --mode prod
```

### Level 3: Integration sanity with existing launchers
```bash
# run preflight then launcher
bash scripts/x3_node_healthcheck.sh --mode dev
# optional next action
# ./run-dev-node.sh --keep-ports
```

## Final validation Checklist
- [ ] Script exists and is executable
- [ ] Dev mode returns 0 on healthy environment
- [ ] Prod mode enforces `NODE_NAME`
- [ ] Ports checked using defaults aligned with launch scripts
- [ ] Missing app env files produce actionable guidance
- [ ] Documentation updated with command examples

---

## Anti-Patterns to Avoid
- ❌ Don’t auto-kill ports in healthcheck (that belongs to launcher logic)
- ❌ Don’t auto-build binaries during preflight (report and suggest command instead)
- ❌ Don’t fail on optional tooling that has fallbacks
- ❌ Don’t duplicate mismatched port constants; source from existing launcher defaults
- ❌ Don’t silently pass failures—always summarize clearly with exit code

## Confidence Score
9/10 — bounded shell-script scope, clear existing patterns, and deterministic validation commands.
