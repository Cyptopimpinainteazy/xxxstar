#!/usr/bin/env python3
"""
Bridged Testnet Scoreboard Generator

Generates latest.json and latest.md from test results.
Run after testnet execution to produce the scoreboard.
"""

import json
import sys
import os
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Any, Optional
from dataclasses import dataclass, asdict, field

@dataclass
class ComponentResult:
    name: str
    category: str  # infrastructure, core, bridge, monitoring, tooling
    status: str    # PASS, FAIL, SKIP, NOT_TESTED
    details: str
    latency_ms: int = 0
    error: str = ""

@dataclass
class ScenarioResult:
    name: str
    category: str  # happy_path, failure_recovery, security, invariants, stress
    status: str    # PASS, FAIL, SKIP, NOT_TESTED
    description: str
    expected: str
    actual: str
    duration_ms: int = 0
    error: str = ""
    proof_refs: List[str] = field(default_factory=list)

@dataclass
class Scoreboard:
    timestamp: str
    profile: str
    overall_status: str
    duration_seconds: int
    components: List[ComponentResult]
    scenarios: List[ScenarioResult]
    summary: Dict[str, Any]
    artifacts: Dict[str, Any] = field(default_factory=dict)

# Component definitions - what we expect to test
EXPECTED_COMPONENTS = [
    # Infrastructure
    ComponentResult("X3 Validator Network", "infrastructure", "NOT_TESTED", "4+ validators producing and finalizing blocks"),
    ComponentResult("X3 RPC", "infrastructure", "NOT_TESTED", "RPC endpoints responding to system_health, chain_getFinalizedHead"),
    ComponentResult("Bootnode", "infrastructure", "NOT_TESTED", "Bootnode accepting peer connections"),

    # Core
    ComponentResult("Relayer", "core", "NOT_TESTED", "Relayer connected, processing events, submitting proofs"),
    ComponentResult("Packet Standard", "core", "NOT_TESTED", "x3-packet-standard crate functional (commitments, replay guard, timeouts)"),
    ComponentResult("IXL Execution", "core", "NOT_TESTED", "x3-ixl crate functional (planner, interpreter, rollback, verifier)"),

    # Bridge Adapters
    ComponentResult("Ethereum Adapter", "bridge", "NOT_TESTED", "EthereumBridgeAdapter: header validation, proof generation, block/log polling"),
    ComponentResult("Solana Adapter", "bridge", "NOT_TESTED", "SolanaBridgeAdapter: header validation, proof generation, slot/log polling"),
    ComponentResult("Arbitrum Adapter", "bridge", "NOT_TESTED", "Arbitrum adapter (via Ethereum adapter or dedicated)"),
    ComponentResult("BNB/BSC Adapter", "bridge", "NOT_TESTED", "BSC adapter (via Ethereum adapter or dedicated)"),

    # Monitoring
    ComponentResult("Prometheus", "monitoring", "NOT_TESTED", "Prometheus scraping metrics from validators, relayer, RPC"),
    ComponentResult("Grafana", "monitoring", "NOT_TESTED", "Grafana dashboards accessible and showing data"),
    ComponentResult("Invariant Watcher", "monitoring", "NOT_TESTED", "Canonical supply invariant checker running and alerting"),
    ComponentResult("Alertmanager", "monitoring", "NOT_TESTED", "Alertmanager routing alerts from Prometheus rules"),

    # Tooling
    ComponentResult("Faucet", "tooling", "NOT_TESTED", "Faucet dispensing test tokens"),
    ComponentResult("Explorer/API", "tooling", "NOT_TESTED", "Block explorer and API serving chain data"),
]

# Scenario definitions - the actual bridge flows to test
EXPECTED_SCENARIOS = [
    # Happy Path
    ScenarioResult(
        "X3 → Ethereum Atomic Swap (Happy Path)",
        "happy_path", "NOT_TESTED",
        "Lock X3 native asset, emit packet, relayer submits to Ethereum HTLC, claim on Ethereum, proof submitted back, invariant holds",
        "Atomic swap completes: X3 balance decreases, Ethereum HTLC funded, claim succeeds, X3 receives proof, canonical supply preserved",
        ""
    ),
    ScenarioResult(
        "Ethereum → X3 Refund (Happy Path)",
        "happy_path", "NOT_TESTED",
        "Lock on Ethereum, timeout expires, refund on Ethereum, proof submitted back to X3, X3 asset unlocked",
        "Refund completes: Ethereum HTLC refunded, X3 asset released, canonical supply preserved",
        ""
    ),
    ScenarioResult(
        "X3 → Solana Atomic Swap (Happy Path)",
        "happy_path", "NOT_TESTED",
        "Lock X3 native asset, emit packet, relayer submits to Solana HTLC, claim on Solana, proof submitted back, invariant holds",
        "Atomic swap completes: X3 balance decreases, Solana HTLC funded, claim succeeds, X3 receives proof, canonical supply preserved",
        ""
    ),
    ScenarioResult(
        "Solana → X3 Refund (Happy Path)",
        "happy_path", "NOT_TESTED",
        "Lock on Solana, timeout expires, refund on Solana, proof submitted back to X3, X3 asset unlocked",
        "Refund completes: Solana HTLC refunded, X3 asset released, canonical supply preserved",
        ""
    ),
    ScenarioResult(
        "X3 ↔ Arbitrum Atomic Swap (Happy Path)",
        "happy_path", "NOT_TESTED",
        "Full bridge flow via Arbitrum adapter (uses Ethereum adapter with chain_id=42161)",
        "Atomic swap completes via Arbitrum, proof submitted back, canonical supply preserved",
        ""
    ),
    ScenarioResult(
        "X3 ↔ BNB/BSC Atomic Swap (Happy Path)",
        "happy_path", "NOT_TESTED",
        "Full bridge flow via BSC adapter (uses Ethereum adapter with chain_id=56)",
        "Atomic swap completes via BSC, proof submitted back, canonical supply preserved",
        ""
    ),

    # Failure Recovery
    ScenarioResult(
        "Duplicate Proof Rejection",
        "failure_recovery", "NOT_TESTED",
        "Submit same proof twice — second submission must be rejected by replay guard",
        "Second proof submission fails with replay error, no double-credit",
        ""
    ),
    ScenarioResult(
        "Wrong-Domain Rejection",
        "failure_recovery", "NOT_TESTED",
        "Submit proof for domain A on domain B — must be rejected",
        "Cross-domain proof rejected, no unauthorized state change",
        ""
    ),
    ScenarioResult(
        "Timeout Refund E2E",
        "failure_recovery", "NOT_TESTED",
        "Lock asset, wait for timeout, execute refund on external chain, submit proof, verify X3 unlock",
        "Refund executes after timeout, X3 asset released, canonical supply preserved",
        ""
    ),
    ScenarioResult(
        "Relayer Restart During Pending Swap",
        "failure_recovery", "NOT_TESTED",
        "Start swap, kill relayer mid-flow, restart relayer — pending swap must complete",
        "Relayer recovers pending state, swap completes or refunds correctly",
        ""
    ),
    ScenarioResult(
        "RPC Failure During Proof Submission",
        "failure_recovery", "NOT_TESTED",
        "Submit proof while RPC is down — must retry and succeed when RPC recovers",
        "Proof submitted successfully after RPC recovery, no loss",
        ""
    ),
    ScenarioResult(
        "Finality Delay Handling",
        "failure_recovery", "NOT_TESTED",
        "External chain finality delayed — relayer must wait for required confirmations",
        "Relayer waits for finality threshold, does not submit premature proof",
        ""
    ),

    # Security
    ScenarioResult(
        "Replay Attack Prevention",
        "security", "NOT_TESTED",
        "Attempt to replay old packet with same (src_chain, src_port, sequence) — must be rejected",
        "ReplayGuard rejects duplicate, no state corruption",
        ""
    ),
    ScenarioResult(
        "Nonce Exhaustion / Reuse Prevention",
        "security", "NOT_TESTED",
        "Verify nonce monotonicity per sender — no reuse, no gaps that cause stuck funds",
        "NextNonce increments correctly, no nonce collision",
        ""
    ),
    ScenarioResult(
        "Packet Commitment Integrity",
        "security", "NOT_TESTED",
        "Verify packet commitment hash matches packet content — mutation detection",
        "Any packet mutation changes commitment, detected by verifier",
        ""
    ),

    # Invariants
    ScenarioResult(
        "Canonical Supply Invariant (Continuous)",
        "invariants", "NOT_TESTED",
        "After every block: locked_total == claimed_total + refunded_total + fees + pending_reservations",
        "Invariant holds at all times during test run, alert fires on violation",
        ""
    ),
    ScenarioResult(
        "Cross-Chain Collateral Invariant",
        "invariants", "NOT_TESTED",
        "external_locked >= represented + pending for each gateway route",
        "Collateral invariant holds for all active routes",
        ""
    ),
    ScenarioResult(
        "No Ghost Mint/Burn",
        "invariants", "NOT_TESTED",
        "Total supply across all domains equals canonical supply + pending",
        "No unauthorized mint/burn detected",
        ""
    ),

    # Stress
    ScenarioResult(
        "Concurrent Swaps (10 parallel)",
        "stress", "NOT_TESTED",
        "Execute 10 concurrent atomic swaps across different chains",
        "All 10 complete successfully, invariants hold",
        ""
    ),
    ScenarioResult(
        "Network Partition + Heal",
        "stress", "NOT_TESTED",
        "Partition validators, wait for heal, verify bridge state consistency",
        "After heal: finality resumes, pending swaps resolve correctly",
        ""
    ),
]

def calculate_summary(components: List[ComponentResult], scenarios: List[ScenarioResult]) -> Dict[str, Any]:
    total_comp = len([c for c in components if c.status != "NOT_TESTED"])
    passed_comp = len([c for c in components if c.status == "PASS"])
    failed_comp = len([c for c in components if c.status == "FAIL"])

    total_scen = len([s for s in scenarios if s.status != "NOT_TESTED"])
    passed_scen = len([s for s in scenarios if s.status == "PASS"])
    failed_scen = len([s for s in scenarios if s.status == "FAIL"])

    # Weighted readiness: components 40%, scenarios 60%
    comp_score = (passed_comp / total_comp * 100) if total_comp > 0 else 0
    scen_score = (passed_scen / total_scen * 100) if total_scen > 0 else 0
    readiness = round(comp_score * 0.4 + scen_score * 0.6, 1)

    # Overall status
    if failed_comp > 0 or failed_scen > 0:
        overall = "FAIL"
    elif total_comp == len(components) and total_scen == len(scenarios):
        overall = "PARTIAL"
    elif passed_comp == len(components) and passed_scen == len(scenarios):
        overall = "PASS"
    else:
        overall = "PARTIAL"

    return {
        "total_components": len(components),
        "passed_components": passed_comp,
        "failed_components": failed_comp,
        "total_scenarios": len(scenarios),
        "passed_scenarios": passed_scen,
        "failed_scenarios": failed_scen,
        "readiness_percentage": readiness,
        "overall_status": overall
    }

def generate_json(scoreboard: Scoreboard, output_path: Path):
    """Generate latest.json"""
    data = {
        "timestamp": scoreboard.timestamp,
        "profile": scoreboard.profile,
        "overall_status": scoreboard.overall_status,
        "duration_seconds": scoreboard.duration_seconds,
        "components": [asdict(c) for c in scoreboard.components],
        "scenarios": [asdict(s) for s in scoreboard.scenarios],
        "summary": scoreboard.summary,
        "artifacts": scoreboard.artifacts
    }
    with open(output_path, 'w') as f:
        json.dump(data, f, indent=2)
    print(f"✓ Generated {output_path}")

def generate_markdown(scoreboard: Scoreboard, output_path: Path):
    """Generate latest.md"""
    lines = [
        f"# Bridged Testnet Scoreboard",
        f"",
        f"**Profile:** {scoreboard.profile}  ",
        f"**Timestamp:** {scoreboard.timestamp}  ",
        f"**Duration:** {scoreboard.duration_seconds}s  ",
        f"**Overall Status:** `{scoreboard.overall_status}`  ",
        f"**Readiness:** {scoreboard.summary['readiness_percentage']}%  ",
        f"",
        f"---",
        f"",
        f"## Summary",
        f"",
        f"| Metric | Count |",
        f"|--------|-------|",
        f"| Total Components | {scoreboard.summary['total_components']} |",
        f"| ✅ Passed Components | {scoreboard.summary['passed_components']} |",
        f"| ❌ Failed Components | {scoreboard.summary['failed_components']} |",
        f"| Total Scenarios | {scoreboard.summary['total_scenarios']} |",
        f"| ✅ Passed Scenarios | {scoreboard.summary['passed_scenarios']} |",
        f"| ❌ Failed Scenarios | {scoreboard.summary['failed_scenarios']} |",
        f"",
        f"---",
        f"",
        f"## Components",
        f"",
        f"| Component | Category | Status | Details | Latency | Error |",
        f"|-----------|----------|--------|---------|---------|-------|",
    ]

    for c in scoreboard.components:
        status_emoji = {"PASS": "✅", "FAIL": "❌", "SKIP": "⏭️", "NOT_TESTED": "❓"}.get(c.status, "❓")
        error_short = c.error[:50] + "..." if len(c.error) > 50 else c.error
        lines.append(f"| {c.name} | {c.category} | {status_emoji} {c.status} | {c.details} | {c.latency_ms}ms | {error_short} |")

    lines.extend([
        f"",
        f"---",
        f"",
        f"## Scenarios",
        f"",
        f"| Scenario | Category | Status | Description | Expected | Actual | Duration | Error |",
        f"|----------|----------|--------|-------------|----------|--------|----------|-------|",
    ])

    for s in scoreboard.scenarios:
        status_emoji = {"PASS": "✅", "FAIL": "❌", "SKIP": "⏭️", "NOT_TESTED": "❓"}.get(s.status, "❓")
        error_short = s.error[:50] + "..." if len(s.error) > 50 else s.error
        actual_short = s.actual[:60] + "..." if len(s.actual) > 60 else s.actual
        lines.append(f"| {s.name} | {s.category} | {status_emoji} {s.status} | {s.description[:60]}... | {s.expected[:60]}... | {actual_short} | {s.duration_ms}ms | {error_short} |")

    lines.extend([
        f"",
        f"---",
        f"",
        f"## Artifacts",
        f"",
    ])

    if scoreboard.artifacts:
        for k, v in scoreboard.artifacts.items():
            if isinstance(v, list):
                lines.append(f"- **{k}**: {', '.join(v) if v else '(none)'}")
            else:
                lines.append(f"- **{k}**: {v}")
    else:
        lines.append("(no artifacts recorded)")

    with open(output_path, 'w') as f:
        f.write("\n".join(lines))
    print(f"✓ Generated {output_path}")

def create_empty_scoreboard(profile: str = "local-full") -> Scoreboard:
    """Create a scoreboard with all components/scenarios in NOT_TESTED state"""
    now = datetime.utcnow().isoformat() + "Z"
    summary = calculate_summary(EXPECTED_COMPONENTS, EXPECTED_SCENARIOS)
    return Scoreboard(
        timestamp=now,
        profile=profile,
        overall_status="PARTIAL",
        duration_seconds=0,
        components=EXPECTED_COMPONENTS.copy(),
        scenarios=EXPECTED_SCENARIOS.copy(),
        summary=summary,
        artifacts={}
    )

def load_results_from_dir(results_dir: Path) -> tuple:
    """Load component and scenario results from JSON files in results_dir"""
    components = []
    scenarios = []

    for f in results_dir.glob("component_*.json"):
        with open(f) as fp:
            data = json.load(fp)
            components.append(ComponentResult(**data))

    for f in results_dir.glob("scenario_*.json"):
        with open(f) as fp:
            data = json.load(fp)
            scenarios.append(ScenarioResult(**data))

    return components, scenarios

def main():
    import argparse
    parser = argparse.ArgumentParser(description="Generate bridged testnet scoreboard")
    parser.add_argument("--profile", default="local-full", choices=["local-full", "local-minimal", "vps", "public"])
    parser.add_argument("--results-dir", type=Path, help="Directory with component_*.json and scenario_*.json results")
    parser.add_argument("--duration", type=int, default=0, help="Test duration in seconds")
    parser.add_argument("--output-dir", type=Path, default=Path("/home/x3star/Desktop/xxxstar-main/reports/bridged-testnet"))
    parser.add_argument("--init", action="store_true", help="Create empty scoreboard template")

    args = parser.parse_args()

    output_dir = args.output_dir
    output_dir.mkdir(parents=True, exist_ok=True)

    if args.init:
        scoreboard = create_empty_scoreboard(args.profile)
    else:
        # Start with expected structure
        scoreboard = create_empty_scoreboard(args.profile)
        scoreboard.duration_seconds = args.duration

        # Override with actual results if provided
        if args.results_dir and args.results_dir.exists():
            comp_results, scen_results = load_results_from_dir(args.results_dir)
            if comp_results:
                scoreboard.components = comp_results
            if scen_results:
                scoreboard.scenarios = scen_results

        scoreboard.summary = calculate_summary(scoreboard.components, scoreboard.scenarios)
        scoreboard.overall_status = scoreboard.summary["overall_status"]

    # Write outputs
    generate_json(scoreboard, output_dir / "latest.json")
    generate_markdown(scoreboard, output_dir / "latest.md")

    # Print summary
    print(f"\n{'='*60}")
    print(f"BRIDGED TESTNET SCOREBOARD: {scoreboard.overall_status}")
    print(f"{'='*60}")
    print(f"Readiness: {scoreboard.summary['readiness_percentage']}%")
    print(f"Components: {scoreboard.summary['passed_components']}/{scoreboard.summary['total_components']} passed")
    print(f"Scenarios:  {scoreboard.summary['passed_scenarios']}/{scoreboard.summary['total_scenarios']} passed")
    print(f"{'='*60}")

    if scoreboard.overall_status == "FAIL":
        sys.exit(1)

if __name__ == "__main__":
    main()