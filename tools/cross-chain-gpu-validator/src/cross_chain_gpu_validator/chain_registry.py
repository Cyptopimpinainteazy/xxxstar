"""Chain registry for managing validators across 103+ blockchains."""

from __future__ import annotations

from typing import Iterable
import json
import os

from .chain_adapter import ChainConfig, ChainValidator, SignatureAlgorithm, HashAlgorithm


class ChainRegistry:
    """Registry mapping chain IDs to their validators and configurations."""

    def __init__(self) -> None:
        self._validators: dict[str, ChainValidator] = {}
        self._configs: dict[str, ChainConfig] = {}

    def register_chain(self, config: ChainConfig, validator: ChainValidator) -> None:
        """Register a chain with its validator."""
        if config.chain_id in self._validators:
            raise ValueError(f"Chain {config.chain_id} already registered")
        self._validators[config.chain_id] = validator
        self._configs[config.chain_id] = config

    def get_validator(self, chain_id: str) -> ChainValidator | None:
        """Get validator for a chain."""
        return self._validators.get(chain_id)

    def get_config(self, chain_id: str) -> ChainConfig | None:
        """Get configuration for a chain."""
        return self._configs.get(chain_id)

    def list_chains(self) -> Iterable[str]:
        """List all registered chain IDs."""
        return self._validators.keys()

    def chain_count(self) -> int:
        """Get number of registered chains."""
        return len(self._validators)

    def validate_enabled_chains(self, chain_ids: Iterable[str]) -> bool:
        """Check if all specified chains are registered."""
        for chain_id in chain_ids:
            if chain_id not in self._validators:
                return False
        return True


def _load_configs_from_file(path: str) -> dict[str, ChainConfig]:
    """Load chain configs from a JSON resource file.

    The file contains a list of objects with keys: chain_id, chain_name, rpc_url,
    is_evm, is_svm, supports_gpu.
    """
    try:
        base = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "resources"))
        fp = os.path.join(base, path)
        if not os.path.exists(fp):
            return {}
        with open(fp, "r", encoding="utf-8") as handle:
            items = json.load(handle)
    except Exception:
        return {}

    configs: dict[str, ChainConfig] = {}
    for item in items:
        cid = item.get("chain_id")
        if not cid:
            continue
        name = item.get("chain_name") or cid
        rpc = item.get("rpc_url")
        if not rpc:
            continue
        is_evm = bool(item.get("is_evm"))
        is_svm = bool(item.get("is_svm"))
        if is_evm:
            sig_alg = SignatureAlgorithm.SECP256K1
            hash_alg = HashAlgorithm.KECCAK256
            sig_pub = 64
            sig_sig = 65
        elif is_svm:
            sig_alg = SignatureAlgorithm.ED25519
            hash_alg = HashAlgorithm.SHA256
            sig_pub = 32
            sig_sig = 64
        else:
            # conservative default
            sig_alg = SignatureAlgorithm.SECP256K1
            hash_alg = HashAlgorithm.SHA256
            sig_pub = 33
            sig_sig = 64

        configs[cid] = ChainConfig(
            chain_id=cid,
            chain_name=name,
            rpc_url=rpc,
            sig_algorithm=sig_alg,
            hash_algorithm=hash_alg,
            sig_pubkey_size=sig_pub,
            sig_signature_size=sig_sig,
            hash_output_size=32,
            supports_gpu=bool(item.get("supports_gpu", True)),
        )

    return configs


def load_default_chain_configs() -> dict[str, ChainConfig]:
    """Load default chain configurations for all 103 chains.

    This includes mainnet and testnet chains across all major ecosystems:
    - Ethereum (EVM): mainnet, Sepolia, Holesky, etc.
    - Solana (SVM): mainnet-beta, testnet, devnet
    - Cosmos (Tendermint): Hub, Osmosis, Juno, etc.
    - Substrate: Polkadot, Kusama, etc.
    - Layer 2s: Arbitrum, Optimism, zk-EVM, Starknet, etc.
    - Other L1s: TON, Aptos, Sui, etc.
    """

    configs = {}

    # EVM Chains (secp256k1 + keccak256)
    evm_chains = [
        ("ethereum", "Ethereum", "https://eth.llamarpc.com", 43114),
        ("ethereum-sepolia", "Ethereum Sepolia", "https://eth-sepolia.g.alchemy.com/v2/demo", 11155111),
        ("arbitrum", "Arbitrum One", "https://arb1.arbitrum.io/rpc", 42161),
        ("arbitrum-sepolia", "Arbitrum Sepolia", "https://sepolia-rollup.arbitrum.io/rpc", 421614),
        ("optimism", "Optimism", "https://mainnet.optimism.io", 10),
        ("optimism-sepolia", "Optimism Sepolia", "https://sepolia.optimism.io", 11155420),
        ("polygon", "Polygon", "https://polygon-rpc.com", 137),
        ("polygon-mumbai", "Polygon Mumbai", "https://rpc-mumbai.maticvigil.com", 80001),
        ("base", "Base", "https://mainnet.base.org", 8453),
        ("base-sepolia", "Base Sepolia", "https://sepolia.base.org", 84532),
        ("avalanche", "Avalanche C-Chain", "https://api.avax.network/ext/bc/C/rpc", 43114),
        ("bsc", "BSC", "https://bsc-dataseed.bnbchain.org", 56),
        ("bsc-testnet", "BSC Testnet", "https://data-seed-prebsc-1-b7b35ded05051b650cc006c6c7b24ef4.prylabs.net:8545", 97),
        ("fantom", "Fantom", "https://rpc.ftm.tools", 250),
        ("celo", "Celo", "https://forno.celo.org", 42220),
        ("gnosis", "Gnosis", "https://rpc.gnosischain.com", 100),
        ("linea", "Linea", "https://rpc.linea.build", 59144),
        ("scroll", "Scroll", "https://rpc.scroll.io", 534352),
        ("zksync", "zkSync Era", "https://mainnet.era.zksync.io", 324),
        ("zkfair", "zkFair", "https://rpc.zkfair.io", 42766),
        ("manta", "Manta", "https://pacific-rpc.manta.network/http", 169),
        ("moonbeam", "Moonbeam", "https://rpc.api.moonbeam.network", 1284),
        ("moonriver", "Moonriver", "https://moonriver.public.blastapi.io", 1285),
        ("astar", "Astar", "https://evm.astar.network", 592),
        ("shiden", "Shiden", "https://evm.shiden.astar.network", 336),
        ("harmony", "Harmony One", "https://api.harmony.one", 1666600000),
        ("metis", "Metis Andromeda", "https://andromeda.metis.io", 1088),
        ("filecoin", "Filecoin", "https://api.node.glif.io", 314),
    ]

    for chain_id, name, rpc, _chain_num in evm_chains:
        configs[chain_id] = ChainConfig(
            chain_id=chain_id,
            chain_name=name,
            rpc_url=rpc,
            sig_algorithm=SignatureAlgorithm.SECP256K1,
            hash_algorithm=HashAlgorithm.KECCAK256,
            sig_pubkey_size=64,
            sig_signature_size=65,
            hash_output_size=32,
            supports_gpu=True,
        )

    # Solana (Ed25519 + SHA256)
    solana_chains = [
        ("solana", "Solana Mainnet", "https://api.mainnet-beta.solana.com", None),
        ("solana-devnet", "Solana Devnet", "https://api.devnet.solana.com", None),
        ("solana-testnet", "Solana Testnet", "https://api.testnet.solana.com", None),
    ]

    for chain_id, name, rpc, _ in solana_chains:
        configs[chain_id] = ChainConfig(
            chain_id=chain_id,
            chain_name=name,
            rpc_url=rpc,
            sig_algorithm=SignatureAlgorithm.ED25519,
            hash_algorithm=HashAlgorithm.SHA256,
            sig_pubkey_size=32,
            sig_signature_size=64,
            hash_output_size=32,
            supports_gpu=True,
        )

    # Cosmos/Tendermint chains (secp256k1 + SHA256)
    cosmos_chains = [
        ("cosmos-hub", "Cosmos Hub", "https://rest.cosmos.directory/cosmoshub", None),
        ("osmosis", "Osmosis", "https://rest.cosmos.directory/osmosis", None),
        ("juno", "Juno", "https://rest.cosmos.directory/juno", None),
        ("stargaze", "Stargaze", "https://rest.cosmos.directory/stargaze", None),
        ("akash", "Akash", "https://rest.cosmos.directory/akash", None),
        ("sentinel", "Sentinel", "https://rest.cosmos.directory/sentinel", None),
        ("persistence", "Persistence", "https://rest.cosmos.directory/persistence", None),
        ("sei", "Sei", "https://rest.cosmos.directory/sei", None),
        ("thorchain", "THORChain", "https://rest.cosmos.directory/thorchain", None),
        ("evmos", "Evmos", "https://rest.cosmos.directory/evmos", None),
    ]

    for chain_id, name, rpc, _ in cosmos_chains:
        configs[chain_id] = ChainConfig(
            chain_id=chain_id,
            chain_name=name,
            rpc_url=rpc,
            sig_algorithm=SignatureAlgorithm.SECP256K1,
            hash_algorithm=HashAlgorithm.SHA256,
            sig_pubkey_size=33,  # compressed
            sig_signature_size=64,
            hash_output_size=32,
            supports_gpu=True,
        )

    # Substrate chains (SR25519, inherent to Substrate)
    substrate_chains = [
        ("polkadot", "Polkadot", "wss://rpc.polkadot.io", None),
        ("kusama", "Kusama", "wss://kusama-rpc.polkadot.io", None),
    ]

    for chain_id, name, rpc, _ in substrate_chains:
        configs[chain_id] = ChainConfig(
            chain_id=chain_id,
            chain_name=name,
            rpc_url=rpc,
            sig_algorithm=SignatureAlgorithm.ED25519,  # Substrate uses SR25519 (variant of Ed25519)
            hash_algorithm=HashAlgorithm.SHA256,
            sig_pubkey_size=32,
            sig_signature_size=64,
            hash_output_size=32,
            supports_gpu=True,
        )

    # Other L1s and important chains
    other_chains = [
        ("ton", "TON", "https://toncenter.com/api/v2", None),
        ("aptos", "Aptos", "https://mainnet.aptoslabs.com/v1", None),
        ("sui", "Sui", "https://fullnode.mainnet.sui.io", None),
        ("near", "NEAR", "https://rpc.mainnet.near.org", None),
        ("flow", "Flow", "https://rest-mainnet.onflow.org", None),
        ("icp", "Internet Computer", "https://icp-api.io", None),
        ("cardano", "Cardano", "https://cardano-mainnet.blockfrost.io/api/v0", None),
        ("tezos", "Tezos", "https://mainnet.tezos.marigold.dev", None),
        ("ripple", "XRP Ledger", "https://xrplcluster.com", None),
        ("algorand", "Algorand", "https://mainnet-idx.algonode.cloud", None),
    ]

    for chain_id, name, rpc, _ in other_chains:
        configs[chain_id] = ChainConfig(
            chain_id=chain_id,
            chain_name=name,
            rpc_url=rpc,
            sig_algorithm=SignatureAlgorithm.ED25519,
            hash_algorithm=HashAlgorithm.SHA256,
            sig_pubkey_size=32,
            sig_signature_size=64,
            hash_output_size=32,
            supports_gpu=True,
        )

    # Merge in canonical resource-backed configs last so generated chain data
    # can override built-ins without dropping non-EVM families that are not yet
    # present in the generated export.
    file_configs = _load_configs_from_file("chains.json")
    configs.update(file_configs)

    return configs
