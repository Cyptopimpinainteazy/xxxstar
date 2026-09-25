"""
X3 Operator CLI
~~~~~~~~~~~~~~~

Main entry point for operator management.

Commands:
    doctor    - Run hardware preflight checks
    init      - Initialize operator identity and config
    bond      - Bond stake for a role
    start     - Start the operator daemon
    status    - Show operator status
    simulate  - Run governance capture simulation
    genesis   - Run genesis ceremony steps
    exit-op   - Begin graceful exit (unbonding)
"""

import argparse
import logging
import time
from pathlib import Path

from .config import OperatorRole, NetworkPhase, X3Config
from .health import HealthStatus, run_health_check
from .identity import generate_operator_identity, load_operator_identity
from .bonding import BondLedger, BondStatus
from .slashing import SlashingEngine
from .supervisor import AgentSupervisor, PolicyManifest
from .storage import StorageRegistry
from .governance import GovernanceSimulator
from .genesis import GenesisCeremony, GenesisParticipant, GenesisConfig
from .telemetry import setup_structured_logging, create_operator_metrics

logger = logging.getLogger(__name__)

DATA_DIR = Path.home() / ".x3_operator"
CONFIG_PATH = DATA_DIR / "config.json"
IDENTITY_PATH = DATA_DIR / "identity.json"
BOND_PATH = DATA_DIR / "bonds.json"


def cmd_doctor(args: argparse.Namespace):
    """Run hardware preflight checks."""
    print("=== X3 Operator Doctor ===\n")

    require_gpu = args.role == "gpu" if args.role else False
    report = run_health_check(
        min_disk_gb=args.min_disk or 100,
        min_ram_gb=args.min_ram or 8,
        min_cpu_cores=args.min_cpu or 4,
        require_gpu=require_gpu,
    )

    for check in report.checks:
        symbol = {"PASS": "+", "WARN": "!", "FAIL": "X"}[check.status.value]
        print(f"  [{symbol}] {check.name}: {check.value} (required: {check.required})")
        if check.message:
            print(f"      {check.message}")

    print(f"\nOverall: {report.overall.value}")
    if report.recommended_roles:
        print(f"Recommended roles: {', '.join(report.recommended_roles)}")

    return 0 if report.overall != HealthStatus.FAIL else 1


def cmd_init(args: argparse.Namespace):
    """Initialize operator identity and config."""
    print("=== X3 Operator Init ===\n")

    DATA_DIR.mkdir(parents=True, exist_ok=True)

    # Config
    role = OperatorRole(args.role) if args.role else OperatorRole.VALIDATOR
    phase = NetworkPhase(args.network) if args.network else NetworkPhase.DEVNET

    config = X3Config()
    config.chain.network_phase = phase
    config.chain.rpc_url = args.rpc_url or config.chain.rpc_url
    errors = config.validate()
    if errors:
        for e in errors:
            print(f"  Config error: {e}")
        return 1
    config.save(CONFIG_PATH)
    print(f"  Config saved to {CONFIG_PATH}")

    # Identity
    identity = generate_operator_identity(role, DATA_DIR)
    print(f"  Operator ID: {identity.operator_id}")
    print(f"  Role: {identity.role}")
    print(f"  Hardware fingerprint: {identity.hardware_fingerprint[:32]}...")
    print(f"  Identity saved to {IDENTITY_PATH}")

    return 0


def cmd_bond(args: argparse.Namespace):
    """Bond stake for the operator role."""
    print("=== X3 Bond ===\n")

    config = X3Config.load(CONFIG_PATH)
    identity = load_operator_identity(DATA_DIR)

    ledger = BondLedger()
    if BOND_PATH.exists():
        ledger = BondLedger.load(BOND_PATH)

    amount = args.amount
    role = OperatorRole(identity.role)

    try:
        record = ledger.create_bond(identity.operator_id, role, amount, config)
        # In real deployment this would submit an extrinsic
        record = ledger.confirm_bond(identity.operator_id)
        ledger.save(BOND_PATH)
        print(f"  Bond created and confirmed")
        print(f"  Amount: {record.amount}")
        print(f"  Status: {record.status.value}")
        print(f"  TX Hash: {record.tx_hash[:32]}...")
    except (ValueError, KeyError) as e:
        print(f"  Error: {e}")
        return 1

    return 0


def cmd_start(args: argparse.Namespace):
    """Start the operator daemon."""
    print("=== X3 Operator Start ===\n")

    config = X3Config.load(CONFIG_PATH)
    identity = load_operator_identity(DATA_DIR)

    metrics = create_operator_metrics()
    start_time = time.time()

    # Verify bond exists
    if BOND_PATH.exists():
        ledger = BondLedger.load(BOND_PATH)
        bond = ledger.get_bond(identity.operator_id)
        if bond and bond.status == BondStatus.BONDED:
            print(f"  Bond active: {bond.amount} ({bond.effective_stake()} effective)")
            metrics.gauge("x3_operator_bond_amount").set_value(bond.amount)
            metrics.gauge("x3_operator_effective_stake").set_value(bond.effective_stake())
        else:
            print("  WARNING: No active bond. Operator will run in limited mode.")
    else:
        print("  WARNING: No bond ledger found.")

    print(f"  Operator ID: {identity.operator_id}")
    print(f"  Role: {identity.role}")
    print(f"  Network: {config.chain.network_phase.value}")
    print(f"  RPC: {config.chain.rpc_url}")

    # Main loop
    print("\n  Operator running. Press Ctrl+C to stop.\n")
    try:
        heartbeat_interval = config.health.heartbeat_interval_seconds
        while True:
            metrics.counter("x3_operator_heartbeats_total").increment()
            uptime = time.time() - start_time
            metrics.gauge("x3_operator_uptime_seconds").set_value(uptime)

            if args.metrics_port:
                # Log metrics periodically
                if int(uptime) % 60 == 0:
                    logger.info("metrics: %s", metrics.export_json())

            time.sleep(heartbeat_interval)
    except KeyboardInterrupt:
        print("\n  Shutting down...")
        uptime = time.time() - start_time
        print(f"  Total uptime: {uptime:.0f}s")

    return 0


def cmd_status(args: argparse.Namespace):
    """Show operator status."""
    print("=== X3 Operator Status ===\n")

    if not IDENTITY_PATH.exists():
        print("  Not initialized. Run 'x3-operator init' first.")
        return 1

    identity = load_operator_identity(DATA_DIR)
    print(f"  Operator ID: {identity.operator_id}")
    print(f"  Role: {identity.role}")

    if CONFIG_PATH.exists():
        config = X3Config.load(CONFIG_PATH)
        print(f"  Network: {config.chain.network_phase.value}")
        print(f"  RPC: {config.chain.rpc_url}")

    if BOND_PATH.exists():
        ledger = BondLedger.load(BOND_PATH)
        bond = ledger.get_bond(identity.operator_id)
        if bond:
            print(f"  Bond: {bond.amount} (effective: {bond.effective_stake()})")
            print(f"  Bond status: {bond.status.value}")
            print(f"  Slashed total: {bond.slash_total}")
        else:
            print("  Bond: none")

    return 0


def cmd_simulate(args: argparse.Namespace):
    """Run governance capture simulation."""
    print("=== X3 Governance Capture Simulation ===\n")

    seed = args.seed or 42
    sim = GovernanceSimulator(seed=seed)

    if args.attack:
        attack_map = {
            "whale": sim.simulate_whale_attack,
            "sybil": sim.simulate_sybil_attack,
            "bribery": sim.simulate_bribery_attack,
            "speed": sim.simulate_speed_attack,
        }
        fn = attack_map.get(args.attack)
        if fn is None:
            print(f"  Unknown attack: {args.attack}")
            return 1
        report = fn()
        _print_sim_report(report)
    else:
        # Run full suite
        reports = sim.run_full_suite()
        for report in reports:
            _print_sim_report(report)
            print()

        summary = sim.summary()
        print("=== Summary ===")
        print(f"  Resilience score: {summary['resilience_score']:.0%}")
        print(f"  Defended: {summary['defended']}/{summary['total_simulations']}")
        print(f"  Captured: {summary['captured']}/{summary['total_simulations']}")

    return 0


def _print_sim_report(report):
    symbol = {"captured": "X", "defended": "+", "partial": "!"}[report.result.value]
    print(f"  [{symbol}] {report.attack_type.value.upper()} attack: {report.result.value}")
    print(f"      Voters: {report.num_voters} | Attacker stake: {report.attacker_fraction:.1%}")
    print(f"      Aye: {report.proposal.aye_power} | Nay: {report.proposal.nay_power}")
    print(f"      Turnout: {report.proposal.total_turnout} / {report.proposal.quorum_required} quorum")
    if report.defense_triggered:
        print(f"      Defense: {report.defense_details}")
    for f in report.findings:
        print(f"      → {f}")


def cmd_genesis(args: argparse.Namespace):
    """Run genesis ceremony."""
    print("=== X3 Genesis Ceremony ===\n")

    config = X3Config.load(CONFIG_PATH) if CONFIG_PATH.exists() else X3Config()

    ceremony = GenesisCeremony(config)

    # Configure
    validators = args.validators.split(",") if args.validators else [
        "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
        "5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty",
        "5FLSigC9HGRKVhB9FiEo4Y3koPsNmBmLJbpXg2mp1hXcS59Y",
    ]
    balances = {v: 1000000000000000 for v in validators}  # 1000 X3 each

    genesis = ceremony.configure_genesis(
        chain_id=args.chain_id or "x3-devnet",
        chain_name=args.chain_name or "X3 Devnet",
        initial_validators=validators,
        initial_balances=balances,
        sudo_key=validators[0] if validators else "",
    )
    print(f"  Genesis configured: {genesis.chain_id}")

    # Add participants
    for i, v in enumerate(validators):
        ceremony.add_participant(GenesisParticipant(
            operator_id=f"operator-{i:04d}",
            pubkey=v,
            role="validator",
            stake=balances[v],
        ))
    print(f"  {len(validators)} participants added")

    # Attestations
    n = ceremony.collect_attestations()
    print(f"  {n} attestations collected")

    # Freeze
    frozen_hash = ceremony.freeze_genesis()
    print(f"  Frozen hash: {frozen_hash}")

    # Verify
    passed, errors = ceremony.verify_genesis()
    print(f"  Verification: {'PASSED' if passed else 'FAILED'}")
    for e in errors:
        print(f"    ERROR: {e}")

    # Generate spec
    spec_path = DATA_DIR / "chain-spec.json"
    ceremony.generate_chain_spec(spec_path)
    print(f"  Chain spec: {spec_path}")

    # Dry run
    dry_passed, issues = ceremony.dry_run()
    print(f"  Dry run: {'PASSED' if dry_passed else f'{len(issues)} issues'}")
    for issue in issues:
        print(f"    ISSUE: {issue}")

    # Anchor
    if args.anchor:
        anchor = ceremony.anchor_hash()
        print(f"  Anchored: {anchor['anchor_hash'][:32]}...")

    print("\n  Status:")
    status = ceremony.status()
    for step in status["steps"]:
        mark = "x" if step["completed"] else " "
        print(f"    [{mark}] {step['name']}: {step['result'] or 'pending'}")

    return 0


def cmd_exit_op(args: argparse.Namespace):
    """Begin graceful exit (unbonding)."""
    print("=== X3 Operator Exit ===\n")

    if not BOND_PATH.exists():
        print("  No bond ledger found.")
        return 1

    config = X3Config.load(CONFIG_PATH)
    identity = load_operator_identity(DATA_DIR)
    ledger = BondLedger.load(BOND_PATH)

    bond = ledger.get_bond(identity.operator_id)
    if not bond:
        print("  No active bond.")
        return 1

    if bond.status != BondStatus.BONDED:
        print(f"  Bond state is {bond.status.value}, cannot exit.")
        return 1

    try:
        ledger.start_unbonding(identity.operator_id, config)
        ledger.save(BOND_PATH)
        delay = config.bonding.unbonding_delay_seconds
        print(f"  Unbonding started. Completion in {delay}s ({delay/3600:.1f} hours)")
    except ValueError as e:
        print(f"  Error: {e}")
        return 1

    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="x3-operator",
        description="X3 Network Operator CLI",
    )
    parser.add_argument("--data-dir", type=Path, default=DATA_DIR, help="Data directory")
    parser.add_argument("--log-level", default="INFO", help="Log level")
    parser.add_argument("--json-logs", action="store_true", help="JSON log format")

    sub = parser.add_subparsers(dest="command", required=True)

    # doctor
    doc = sub.add_parser("doctor", help="Run hardware preflight checks")
    doc.add_argument("--role", choices=["validator", "gpu", "storage", "relayer"])
    doc.add_argument("--min-disk", type=int)
    doc.add_argument("--min-ram", type=int)
    doc.add_argument("--min-cpu", type=int)

    # init
    init = sub.add_parser("init", help="Initialize operator")
    init.add_argument("--role", default="validator", choices=["validator", "gpu", "storage", "relayer"])
    init.add_argument("--network", default="devnet", choices=["devnet", "testnet", "mainnet"])
    init.add_argument("--rpc-url", help="RPC endpoint URL")

    # bond
    bond = sub.add_parser("bond", help="Bond stake")
    bond.add_argument("amount", type=int, help="Amount to bond (planck)")

    # start
    start = sub.add_parser("start", help="Start operator daemon")
    start.add_argument("--metrics-port", type=int, help="Prometheus metrics port")

    # status
    sub.add_parser("status", help="Show operator status")

    # simulate
    sim = sub.add_parser("simulate", help="Governance capture simulation")
    sim.add_argument("--attack", choices=["whale", "sybil", "bribery", "speed"])
    sim.add_argument("--seed", type=int, default=42)

    # genesis
    gen = sub.add_parser("genesis", help="Genesis ceremony")
    gen.add_argument("--chain-id", default="x3-devnet")
    gen.add_argument("--chain-name", default="X3 Devnet")
    gen.add_argument("--validators", help="Comma-separated validator SS58 addresses")
    gen.add_argument("--anchor", action="store_true", help="Anchor genesis hash")

    # exit
    sub.add_parser("exit-op", help="Begin graceful exit")

    return parser


def main(argv: list = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)

    # Apply data dir
    global DATA_DIR, CONFIG_PATH, IDENTITY_PATH, BOND_PATH
    DATA_DIR = args.data_dir
    CONFIG_PATH = DATA_DIR / "config.json"
    IDENTITY_PATH = DATA_DIR / "identity.json"
    BOND_PATH = DATA_DIR / "bonds.json"

    setup_structured_logging(
        level=args.log_level,
        log_dir=DATA_DIR / "logs" if args.json_logs else None,
        json_format=args.json_logs,
    )

    dispatch = {
        "doctor": cmd_doctor,
        "init": cmd_init,
        "bond": cmd_bond,
        "start": cmd_start,
        "status": cmd_status,
        "simulate": cmd_simulate,
        "genesis": cmd_genesis,
        "exit-op": cmd_exit_op,
    }

    handler = dispatch.get(args.command)
    if handler is None:
        parser.print_help()
        return 1

    return handler(args)
