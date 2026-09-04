# Scoreboard

The X3 Scoreboard measures readiness across 12 categories.

## Categories

| Category | Default Score | Detection Method |
|----------|--------------|------------------|
| x3-lang | 0-100 | Directory existence |
| x3-vm | 0-100 | Directory existence |
| EVM adapter | 0-100 | Directory existence |
| SVM adapter | 0-100 | Directory existence |
| BTC adapter | 0-100 | Directory existence |
| Relayer swarm | 0-100 | Directory existence |
| Proof ledger | 0-100 | Directory existence |
| Validator ops | 0-100 | Directory existence |
| Security checks | 0-100 | Directory existence |
| Test coverage | 0-100 | Directory existence |
| Workspace health | 0-100 | Git status |
| Build status | 0-100 | cargo check result |

## Scoring

- Each category scored 0-100
- Total = average of all categories
- Status: PASS (≥80), PARTIAL (≥50), FAIL (<50), BLOCKED (error)

## Scoreboard Artifacts

- `x3-proof/SCOREBOARD.json` — machine-readable
- `x3-proof/SCOREBOARD.md` — markdown report
