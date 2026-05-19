#!/usr/bin/env bash
# X3 Adversarial Devnet Week - Automated Attack Simulation
# Runs all 4 governance attack types with varying parameters.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$(dirname "$SCRIPT_DIR")")"

echo "=== X3 Adversarial Devnet Week ==="
echo "Running automated governance capture simulations"
echo ""

cd "$PROJECT_ROOT"

# Run simulations with different seeds
for seed in 42 137 256 512 1024; do
    echo "--- Seed: $seed ---"
    python3 -m x3_operator simulate --seed "$seed"
    echo ""
done

# Run individual attack types with stress parameters
echo "=== Whale Attack Stress (high conviction) ==="
python3 -c "
from x3_operator.governance import GovernanceSimulator
for frac in [0.20, 0.30, 0.40, 0.50]:
    sim = GovernanceSimulator(seed=42)
    r = sim.simulate_whale_attack(whale_stake_fraction=frac, whale_conviction=6)
    print(f'  Whale {frac:.0%}: {r.result.value} (defense={r.defense_triggered})')
"

echo ""
echo "=== Sybil Attack Stress (increasing accounts) ==="
python3 -c "
from x3_operator.governance import GovernanceSimulator
for n in [100, 500, 1000, 5000]:
    sim = GovernanceSimulator(seed=42)
    r = sim.simulate_sybil_attack(n_sybils=n)
    print(f'  Sybils {n}: {r.result.value} (defense={r.defense_triggered})')
"

echo ""
echo "=== Bribery Attack Stress (increasing budget) ==="
python3 -c "
from x3_operator.governance import GovernanceSimulator
for budget in [10000, 50000, 200000, 1000000]:
    sim = GovernanceSimulator(seed=42)
    r = sim.simulate_bribery_attack(bribe_budget=budget)
    print(f'  Budget {budget}: {r.result.value} (defense={r.defense_triggered})')
"

echo ""
echo "=== Speed Attack Stress (low participation) ==="
python3 -c "
from x3_operator.governance import GovernanceSimulator
for part in [0.05, 0.10, 0.20, 0.50]:
    sim = GovernanceSimulator(seed=42)
    r = sim.simulate_speed_attack(off_peak_participation=part)
    print(f'  Participation {part:.0%}: {r.result.value} (defense={r.defense_triggered})')
"

echo ""
echo "=== Adversarial Week Complete ==="
