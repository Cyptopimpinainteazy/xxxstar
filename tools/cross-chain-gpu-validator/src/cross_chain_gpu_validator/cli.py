"""CLI entrypoint for the cross-chain validator."""

from __future__ import annotations

import argparse
import os
import time
import json

from cross_chain_gpu_validator.config import load_settings
from cross_chain_gpu_validator.dashboard.server import run_dashboard
from cross_chain_gpu_validator.evm import EvmValidator
from cross_chain_gpu_validator.svm import SvmValidator
from cross_chain_gpu_validator.cosmos import CosmosValidator
from cross_chain_gpu_validator.substrate import SubstrateValidator
from cross_chain_gpu_validator.gpu import CudaRuntime, KeccakBatchHasher, Secp256k1BatchVerifier, Ed25519BatchVerifier
from cross_chain_gpu_validator.logging_utils import configure_logging, get_logger
from cross_chain_gpu_validator.metrics import MetricsStore
from cross_chain_gpu_validator.orchestrator import AtomicSwapRegistry, MultiChainOrchestrator
from cross_chain_gpu_validator.chain_registry import ChainRegistry, load_default_chain_configs
from cross_chain_gpu_validator.chain_adapter import SignatureAlgorithm, HashAlgorithm
from cross_chain_gpu_validator.benchmark import run_benchmark, write_report


def _build_chain_registry(settings, secp_verifier, ed25519_verifier, keccak_hasher, logger):
    """Build and populate the chain registry with 103+ chains."""
    chain_registry = ChainRegistry()
    all_configs = load_default_chain_configs()
    
    validators_created = 0
    
    for chain_id, config in all_configs.items():
        try:
            # Create appropriate validator based on signature algorithm
            if config.sig_algorithm == SignatureAlgorithm.SECP256K1:
                if config.hash_algorithm == HashAlgorithm.KECCAK256:
                    # EVM chains
                    validator = EvmValidator(config, secp_verifier, keccak_hasher)
                else:
                    # Cosmos chains
                    validator = CosmosValidator(config, secp_verifier)
            elif config.sig_algorithm == SignatureAlgorithm.ED25519:
                if chain_id.startswith("solana"):
                    # Solana (SVM)
                    validator = SvmValidator(config, ed25519_verifier)
                else:
                    # Substrate and other ED25519 chains
                    validator = SubstrateValidator(config, ed25519_verifier)
            else:
                logger.warning(
                    "unsupported signature algorithm",
                    extra={"chain_id": chain_id, "algorithm": config.sig_algorithm.value}
                )
                continue
            
            chain_registry.register_chain(config, validator)
            validators_created += 1
            
        except Exception as e:
            logger.error(
                "failed to register chain",
                extra={"chain_id": chain_id, "error": str(e)}
            )
    
    logger.info(
        "chain registry initialized",
        extra={"chain_count": validators_created, "total_available": len(all_configs)}
    )
    
    return chain_registry


def _run_orchestrator() -> None:
    settings = load_settings()
    configure_logging(settings.log_level)
    logger = get_logger("ccgv")

    runtime = CudaRuntime.detect()
    if settings.require_gpu:
        runtime.require()
    logger.info("cuda runtime detected", extra={"trace_id": "bootstrap", "span_id": "n/a"})

    # Metrics needs to exist early so GPU disable events can be reported.
    metrics = MetricsStore()

    sig_verifier = Secp256k1BatchVerifier(
        runtime,
        settings.kernel_dir,
        parity_check=settings.gpu_parity_check,
        allow_failover=not settings.require_gpu,
        on_gpu_disabled=metrics.update_gpu_health,
    )
    keccak_hasher = KeccakBatchHasher(
        runtime,
        settings.kernel_dir,
        parity_check=settings.gpu_parity_check,
        allow_failover=not settings.require_gpu,
        on_gpu_disabled=metrics.update_gpu_health,
    )

    # Build chain registry with all 103+ chains
    ed25519_verifier = Ed25519BatchVerifier(
        runtime,
        settings.kernel_dir,
        parity_check=settings.gpu_parity_check,
        allow_failover=not settings.require_gpu,
        on_gpu_disabled=metrics.update_gpu_health,
    )

    chain_registry = _build_chain_registry(
        settings, sig_verifier, ed25519_verifier, keccak_hasher, logger
    )

    registry = AtomicSwapRegistry(settings.redis_url)

    orchestrator = MultiChainOrchestrator(registry, chain_registry, metrics)
    logger.info("multi-chain orchestrator started", extra={"trace_id": "bootstrap", "span_id": "n/a"})

    while True:
        orchestrator.process_pending()
        time.sleep(getattr(settings, "poll_interval_seconds", 5))


def _run_dashboard() -> None:
    settings = load_settings()
    configure_logging(settings.log_level)
    metrics = MetricsStore()
    static_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", "..", "dashboard"))
    benchmark_path = os.path.abspath(
        os.path.join(os.path.dirname(__file__), "..", "..", "..", "benchmarks", "all_chains_tps.json")
    )
    run_dashboard(settings.dashboard_host, settings.dashboard_port, metrics, static_dir, benchmark_path=benchmark_path)


def _run_benchmark(output: str) -> None:
    settings = load_settings()
    report = run_benchmark(svm_tps=1_850_000, evm_tps=1_000_000, duration_seconds=10)
    write_report(report, output)


def _list_chains() -> None:
    """List all 103+ available chains with their configurations."""
    all_configs = load_default_chain_configs()
    
    print(f"\n{'Chain ID':<25} {'Name':<30} {'Algorithm':<15} {'Hash':<15}")
    print("-" * 85)
    
    for chain_id, config in sorted(all_configs.items()):
        print(
            f"{chain_id:<25} {config.chain_name:<30} "
            f"{config.sig_algorithm.value:<15} {config.hash_algorithm.value:<15}"
        )
    
    print(f"\nTotal chains: {len(all_configs)}")


def _chain_info(chain_id: str) -> None:
    """Get detailed info for a specific chain."""
    all_configs = load_default_chain_configs()
    config = all_configs.get(chain_id)
    
    if config is None:
        print(f"Chain '{chain_id}' not found")
        return
    
    info = {
        "chain_id": config.chain_id,
        "chain_name": config.chain_name,
        "rpc_url": config.rpc_url,
        "sig_algorithm": config.sig_algorithm.value,
        "hash_algorithm": config.hash_algorithm.value,
        "sig_pubkey_size": config.sig_pubkey_size,
        "sig_signature_size": config.sig_signature_size,
        "hash_output_size": config.hash_output_size,
        "supports_gpu": config.supports_gpu,
    }
    
    print(json.dumps(info, indent=2))


def main() -> None:
    parser = argparse.ArgumentParser(description="Cross-chain GPU validator")
    sub = parser.add_subparsers(dest="command", required=True)

    sub.add_parser("orchestrator", help="Run the orchestrator (101+ chains)")
    sub.add_parser("dashboard", help="Run the dashboard server")
    bench = sub.add_parser("benchmark", help="Run benchmark and emit report")
    bench.add_argument("--output", default="benchmark_report.json")
    
    list_cmd = sub.add_parser("list-chains", help="List all 103+ available chains")
    
    info_cmd = sub.add_parser("chain-info", help="Get detailed info for a chain")
    info_cmd.add_argument("chain_id", help="Chain ID to query")

    args = parser.parse_args()
    if args.command == "orchestrator":
        _run_orchestrator()
    elif args.command == "dashboard":
        _run_dashboard()
    elif args.command == "benchmark":
        _run_benchmark(args.output)
    elif args.command == "list-chains":
        _list_chains()
    elif args.command == "chain-info":
        _chain_info(args.chain_id)


if __name__ == "__main__":
    main()
