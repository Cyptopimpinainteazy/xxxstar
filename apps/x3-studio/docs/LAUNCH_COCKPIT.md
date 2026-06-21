# Launch Cockpit

## Status: ✅ FULL

### Testnet Readiness Checks
| Check | Method |
|-------|--------|
| Builds pass | Proof Mode exit code |
| Tests pass | Proof Mode exit code |
| Scoreboard generated | Score exists |
| Scanner run | Findings available |

### Mainnet Readiness Checks
| Check | Method |
|-------|--------|
| No critical findings | Scanner severity filter |
| Testnet gate passed | Testnet checks ≥ 75% |
| Score ≥ 80% | Scoreboard total |
| Build clean | Build exit code |

### Command Buttons
- **Run Testnet Gate** — `cargo check && cargo test`
- **Run Mainnet Gate** — Full verification pipeline

### Readiness Percentage
- Testnet: percentage of testnet checks passed
- Mainnet: percentage of mainnet checks passed
- Visual progress bars with color coding
