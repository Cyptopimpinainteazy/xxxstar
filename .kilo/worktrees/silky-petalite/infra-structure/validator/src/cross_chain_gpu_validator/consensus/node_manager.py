"""Validator Node Manager — deploy, monitor and manage full-node/validator processes.

Manages full validator nodes across chains (EVM geth/reth, SVM solana-validator,
Cosmos gaiad, Substrate polkadot/x3-node).  Each node is tracked as a
``ValidatorNode`` with health, sync state, and lifecycle management.

Architecture
────────────
  NodeManager
    ├── node_registry (chain_id → ValidatorNode)
    ├── process supervisor (systemd / bare metal)
    ├── sync monitor (block height + peer tracking)
    └── auto-restart on crash with circuit-breaker backoff

Bare Metal
──────────
  Nodes are managed as systemd services or direct processes.
  No Docker.  No containers.  Full bare metal.
"""

from __future__ import annotations

import json
import logging
import os
import signal
import subprocess
import threading
import time
from dataclasses import dataclass, field
from enum import Enum, auto
from pathlib import Path
from typing import Any, Callable

logger = logging.getLogger("x3.consensus.node_manager")


# ─── Enums ───────────────────────────────────────────────────


class ChainClientType(Enum):
    """Supported chain client binaries."""

    # EVM
    GETH = "geth"
    RETH = "reth"
    NETHERMIND = "nethermind"
    BESU = "besu"
    ERIGON = "erigon"

    # SVM (Solana)
    SOLANA_VALIDATOR = "solana-validator"
    JITO_VALIDATOR = "jito-validator"
    FIREDANCER = "firedancer"

    # Cosmos
    GAIAD = "gaiad"
    OSMOSIS = "osmosisd"

    # Substrate
    POLKADOT = "polkadot"
    X3_NODE = "x3-node"

    # WASM runtimes
    WASMTIME = "wasmtime"


class NodeRole(Enum):
    """Role of a validator node."""

    FULL_VALIDATOR = "full_validator"     # Active consensus participant
    SENTRY = "sentry"                     # Public-facing sentry node
    ARCHIVE = "archive"                   # Full archival node
    LIGHT = "light"                       # Light client
    RPC = "rpc"                           # Dedicated RPC endpoint
    BACKUP = "backup"                     # Cold standby


class NodeStatus(Enum):
    """Lifecycle status of a node."""

    STOPPED = "stopped"
    STARTING = "starting"
    SYNCING = "syncing"
    HEALTHY = "healthy"
    DEGRADED = "degraded"
    CRASHED = "crashed"
    RESTARTING = "restarting"
    DECOMMISSIONED = "decommissioned"


# ─── Data Structures ────────────────────────────────────────


@dataclass
class ValidatorNode:
    """A managed validator/full-node instance."""

    chain_id: str
    client_type: ChainClientType
    role: NodeRole
    status: NodeStatus = NodeStatus.STOPPED

    # Process management
    pid: int | None = None
    binary_path: str = ""
    data_dir: str = ""
    config_file: str = ""
    args: list[str] = field(default_factory=list)

    # Network
    p2p_port: int = 30303
    rpc_port: int = 8545
    ws_port: int = 8546
    listen_address: str = "0.0.0.0"
    external_ip: str = ""
    bootnodes: list[str] = field(default_factory=list)

    # Sync state
    block_height: int = 0
    network_height: int = 0
    peer_count: int = 0
    sync_progress: float = 0.0
    last_block_time: float = 0.0

    # Validator state
    validator_address: str = ""
    validator_pubkey: str = ""
    is_signing: bool = False
    missed_blocks: int = 0
    total_blocks_produced: int = 0

    # Metadata
    started_at: float = 0.0
    restarts: int = 0
    max_restarts: int = 10
    restart_cooldown_seconds: float = 30.0
    last_restart_at: float = 0.0

    @property
    def is_synced(self) -> bool:
        """True if within 5 blocks of network height."""
        if self.network_height == 0:
            return False
        return (self.network_height - self.block_height) <= 5

    @property
    def uptime_seconds(self) -> float:
        if self.started_at == 0.0:
            return 0.0
        return time.time() - self.started_at

    def to_dict(self) -> dict:
        return {
            "chain_id": self.chain_id,
            "client_type": self.client_type.value,
            "role": self.role.value,
            "status": self.status.value,
            "pid": self.pid,
            "binary_path": self.binary_path,
            "p2p_port": self.p2p_port,
            "rpc_port": self.rpc_port,
            "block_height": self.block_height,
            "network_height": self.network_height,
            "peer_count": self.peer_count,
            "sync_progress": round(self.sync_progress, 4),
            "is_synced": self.is_synced,
            "is_signing": self.is_signing,
            "missed_blocks": self.missed_blocks,
            "total_blocks_produced": self.total_blocks_produced,
            "validator_address": self.validator_address,
            "uptime_seconds": round(self.uptime_seconds, 1),
            "restarts": self.restarts,
        }


# ─── Node Manager ───────────────────────────────────────────


class NodeManager:
    """Manages lifecycle of validator nodes across chains.

    Parameters
    ----------
    state_dir : str
        Directory for node state persistence (default: /var/lib/x3/nodes).
    on_node_crash : callable
        Invoked when a node crashes: ``fn(chain_id, ValidatorNode)``.
    on_sync_complete : callable
        Invoked when a node finishes syncing: ``fn(chain_id, ValidatorNode)``.
    monitor_interval : float
        Seconds between health-check cycles (default 10).
    """

    def __init__(
        self,
        state_dir: str = "/var/lib/x3/nodes",
        on_node_crash: Callable[[str, ValidatorNode], None] | None = None,
        on_sync_complete: Callable[[str, ValidatorNode], None] | None = None,
        monitor_interval: float = 10.0,
    ) -> None:
        self._state_dir = Path(state_dir)
        self._state_dir.mkdir(parents=True, exist_ok=True)

        self._nodes: dict[str, ValidatorNode] = {}
        self._lock = threading.Lock()
        self._stop = threading.Event()
        self._monitor_thread: threading.Thread | None = None
        self._monitor_interval = monitor_interval

        self._on_crash = on_node_crash
        self._on_sync = on_sync_complete

    # ─── Registration ────────────────────────────────────────

    def register_node(
        self,
        chain_id: str,
        client_type: ChainClientType,
        role: NodeRole = NodeRole.FULL_VALIDATOR,
        binary_path: str = "",
        data_dir: str = "",
        config_file: str = "",
        args: list[str] | None = None,
        p2p_port: int = 30303,
        rpc_port: int = 8545,
        ws_port: int = 8546,
        validator_address: str = "",
        validator_pubkey: str = "",
        bootnodes: list[str] | None = None,
        external_ip: str = "",
    ) -> ValidatorNode:
        """Register a new validator node for management."""

        # Auto-detect binary if not provided
        if not binary_path:
            binary_path = self._detect_binary(client_type)

        if not data_dir:
            data_dir = str(self._state_dir / chain_id / "data")

        node = ValidatorNode(
            chain_id=chain_id,
            client_type=client_type,
            role=role,
            binary_path=binary_path,
            data_dir=data_dir,
            config_file=config_file,
            args=args or [],
            p2p_port=p2p_port,
            rpc_port=rpc_port,
            ws_port=ws_port,
            validator_address=validator_address,
            validator_pubkey=validator_pubkey,
            bootnodes=bootnodes or [],
            external_ip=external_ip,
        )

        with self._lock:
            self._nodes[chain_id] = node

        logger.info(
            "Registered node chain=%s client=%s role=%s",
            chain_id, client_type.value, role.value,
        )
        self._persist_state()
        return node

    def register_multiple_chains(self, chains: list[str], client_type: ChainClientType = ChainClientType.GETH, role: NodeRole = NodeRole.FULL_VALIDATOR) -> None:
        """Register validator nodes for multiple chains."""
        for chain_id in chains:
            self.register_node(
                chain_id=chain_id,
                client_type=client_type,
                role=role,
                p2p_port=30303 + len(self._nodes),  # Incremental ports
                rpc_port=8545 + len(self._nodes),
                ws_port=8546 + len(self._nodes),
            )
        logger.info(f"Registered {len(chains)} new chains")

    def unregister_node(self, chain_id: str) -> None:
        """Remove a node from management (stops it if running)."""
        with self._lock:
            node = self._nodes.pop(chain_id, None)
        if node and node.pid:
            self._stop_process(node)
        self._persist_state()

    # ─── Lifecycle ───────────────────────────────────────────

    def start_node(self, chain_id: str) -> bool:
        """Start a registered validator node process."""
        with self._lock:
            node = self._nodes.get(chain_id)
            if not node:
                logger.error("Node not registered: %s", chain_id)
                return False
            if node.status not in (NodeStatus.STOPPED, NodeStatus.CRASHED):
                logger.warning("Node %s already running (status=%s)", chain_id, node.status.value)
                return False

        node.status = NodeStatus.STARTING

        cmd = self._build_command(node)
        logger.info("Starting node %s: %s", chain_id, " ".join(cmd))

        try:
            # Ensure data directory exists
            Path(node.data_dir).mkdir(parents=True, exist_ok=True)

            # Launch as background process
            log_path = self._state_dir / chain_id / "node.log"
            log_path.parent.mkdir(parents=True, exist_ok=True)
            log_file = open(log_path, "a")

            proc = subprocess.Popen(
                cmd,
                stdout=log_file,
                stderr=subprocess.STDOUT,
                preexec_fn=os.setsid,
            )

            node.pid = proc.pid
            node.started_at = time.time()
            node.status = NodeStatus.SYNCING
            logger.info("Node %s started, pid=%d", chain_id, proc.pid)

            self._persist_state()
            return True

        except FileNotFoundError:
            node.status = NodeStatus.CRASHED
            logger.error("Binary not found for %s: %s", chain_id, node.binary_path)
            return False
        except Exception as exc:
            node.status = NodeStatus.CRASHED
            logger.error("Failed to start %s: %s", chain_id, exc)
            return False

    def stop_node(self, chain_id: str) -> bool:
        """Gracefully stop a running node."""
        with self._lock:
            node = self._nodes.get(chain_id)
            if not node:
                return False
        return self._stop_process(node)

    def restart_node(self, chain_id: str) -> bool:
        """Restart a node (stop + start)."""
        self.stop_node(chain_id)
        time.sleep(2)
        return self.start_node(chain_id)

    # ─── Monitoring ──────────────────────────────────────────

    def start_monitoring(self) -> None:
        """Start the background health monitor thread."""
        if self._monitor_thread and self._monitor_thread.is_alive():
            return
        self._stop.clear()
        self._monitor_thread = threading.Thread(
            target=self._monitor_loop, daemon=True, name="x3-node-monitor",
        )
        self._monitor_thread.start()
        logger.info("Node monitor started (interval=%.1fs)", self._monitor_interval)

    def stop_monitoring(self) -> None:
        """Stop the background monitor."""
        self._stop.set()
        if self._monitor_thread:
            self._monitor_thread.join(timeout=10)

    def _monitor_loop(self) -> None:
        """Continuously check node health and sync status."""
        while not self._stop.wait(self._monitor_interval):
            with self._lock:
                nodes = list(self._nodes.values())

            for node in nodes:
                if node.status in (NodeStatus.STOPPED, NodeStatus.DECOMMISSIONED):
                    continue
                self._check_node(node)

    def _check_node(self, node: ValidatorNode) -> None:
        """Check health of a single node."""
        # Check if process is still running
        if node.pid:
            try:
                os.kill(node.pid, 0)  # Signal 0 = check existence
            except ProcessLookupError:
                logger.warning("Node %s (pid=%d) died", node.chain_id, node.pid)
                node.status = NodeStatus.CRASHED
                node.pid = None
                if self._on_crash:
                    self._on_crash(node.chain_id, node)
                self._maybe_restart(node)
                return
            except PermissionError:
                pass  # Process alive, we just can't signal it

        # Query node RPC for sync status
        try:
            self._update_sync_status(node)
        except Exception as exc:
            logger.debug("RPC check failed for %s: %s", node.chain_id, exc)
            if node.status == NodeStatus.HEALTHY:
                node.status = NodeStatus.DEGRADED

    def _update_sync_status(self, node: ValidatorNode) -> None:
        """Query the node's RPC to update block height and peer count."""
        import urllib.request

        rpc_url = f"http://127.0.0.1:{node.rpc_port}"

        if node.client_type in (
            ChainClientType.GETH, ChainClientType.RETH,
            ChainClientType.NETHERMIND, ChainClientType.BESU,
            ChainClientType.ERIGON,
        ):
            # EVM JSON-RPC
            self._query_evm_status(node, rpc_url)

        elif node.client_type in (
            ChainClientType.SOLANA_VALIDATOR, ChainClientType.JITO_VALIDATOR,
            ChainClientType.FIREDANCER,
        ):
            self._query_solana_status(node, rpc_url)

        elif node.client_type in (ChainClientType.GAIAD, ChainClientType.OSMOSIS):
            self._query_cosmos_status(node, rpc_url)

        elif node.client_type in (ChainClientType.POLKADOT, ChainClientType.X3_NODE):
            self._query_substrate_status(node, rpc_url)

        # Update overall status
        if node.is_synced and node.peer_count > 0:
            if node.status != NodeStatus.HEALTHY:
                node.status = NodeStatus.HEALTHY
                if self._on_sync:
                    self._on_sync(node.chain_id, node)
                logger.info("Node %s synced at height %d", node.chain_id, node.block_height)
        elif node.status == NodeStatus.HEALTHY and not node.is_synced:
            node.status = NodeStatus.SYNCING

    def _query_evm_status(self, node: ValidatorNode, rpc_url: str) -> None:
        """EVM eth_syncing + eth_blockNumber + net_peerCount."""
        import urllib.request

        def _rpc(method: str) -> Any:
            payload = json.dumps({
                "jsonrpc": "2.0", "method": method, "params": [], "id": 1,
            }).encode()
            req = urllib.request.Request(
                rpc_url, data=payload,
                headers={"Content-Type": "application/json"},
            )
            with urllib.request.urlopen(req, timeout=5) as resp:
                return json.loads(resp.read())["result"]

        # Block height
        result = _rpc("eth_blockNumber")
        if result:
            node.block_height = int(result, 16)

        # Sync status
        sync = _rpc("eth_syncing")
        if sync is False:
            node.sync_progress = 1.0
        elif isinstance(sync, dict):
            current = int(sync.get("currentBlock", "0x0"), 16)
            highest = int(sync.get("highestBlock", "0x1"), 16)
            node.sync_progress = current / max(highest, 1)
            node.network_height = highest

        # Peers
        peers = _rpc("net_peerCount")
        if peers:
            node.peer_count = int(peers, 16)

    def _query_solana_status(self, node: ValidatorNode, rpc_url: str) -> None:
        """Solana getSlot + getClusterNodes."""
        import urllib.request

        def _rpc(method: str, params: list | None = None) -> Any:
            payload = json.dumps({
                "jsonrpc": "2.0", "method": method,
                "params": params or [], "id": 1,
            }).encode()
            req = urllib.request.Request(
                rpc_url, data=payload,
                headers={"Content-Type": "application/json"},
            )
            with urllib.request.urlopen(req, timeout=5) as resp:
                return json.loads(resp.read())["result"]

        slot = _rpc("getSlot")
        if slot:
            node.block_height = slot

        cluster = _rpc("getClusterNodes")
        if cluster:
            node.peer_count = len(cluster)

    def _query_cosmos_status(self, node: ValidatorNode, rpc_url: str) -> None:
        """Cosmos /status endpoint."""
        import urllib.request

        url = f"{rpc_url}/status"
        req = urllib.request.Request(url, headers={"Accept": "application/json"})
        with urllib.request.urlopen(req, timeout=5) as resp:
            data = json.loads(resp.read())

        sync_info = data.get("result", {}).get("sync_info", {})
        node.block_height = int(sync_info.get("latest_block_height", 0))
        node.peer_count = int(data.get("result", {}).get("n_peers", 0))
        catching_up = sync_info.get("catching_up", False)
        node.sync_progress = 0.5 if catching_up else 1.0

    def _query_substrate_status(self, node: ValidatorNode, rpc_url: str) -> None:
        """Substrate system_health + system_syncState."""
        import urllib.request

        def _rpc(method: str) -> Any:
            payload = json.dumps({
                "jsonrpc": "2.0", "method": method, "params": [], "id": 1,
            }).encode()
            req = urllib.request.Request(
                rpc_url, data=payload,
                headers={"Content-Type": "application/json"},
            )
            with urllib.request.urlopen(req, timeout=5) as resp:
                return json.loads(resp.read())["result"]

        health = _rpc("system_health")
        if health:
            node.peer_count = health.get("peers", 0)

        sync = _rpc("system_syncState")
        if sync:
            node.block_height = sync.get("currentBlock", 0)
            highest = sync.get("highestBlock", 0)
            node.network_height = highest
            if highest > 0:
                node.sync_progress = node.block_height / highest

    # ─── Process Management ──────────────────────────────────

    def _build_command(self, node: ValidatorNode) -> list[str]:
        """Build the command to launch a node."""
        cmd = [node.binary_path]

        if node.client_type in (ChainClientType.GETH, ChainClientType.RETH):
            cmd.extend([
                "--datadir", node.data_dir,
                "--port", str(node.p2p_port),
                "--http", "--http.port", str(node.rpc_port),
                "--ws", "--ws.port", str(node.ws_port),
                "--http.addr", node.listen_address,
            ])
            if node.bootnodes:
                cmd.extend(["--bootnodes", ",".join(node.bootnodes)])
            if node.role == NodeRole.ARCHIVE:
                cmd.extend(["--gcmode", "archive", "--syncmode", "full"])

        elif node.client_type in (
            ChainClientType.SOLANA_VALIDATOR, ChainClientType.JITO_VALIDATOR,
        ):
            cmd.extend([
                "--ledger", node.data_dir,
                "--rpc-port", str(node.rpc_port),
                "--dynamic-port-range", f"{node.p2p_port}-{node.p2p_port + 10}",
            ])
            if node.validator_pubkey:
                cmd.extend(["--identity", node.validator_pubkey])

        elif node.client_type in (ChainClientType.GAIAD, ChainClientType.OSMOSIS):
            cmd.extend(["start", "--home", node.data_dir])
            if node.rpc_port != 26657:
                cmd.extend(["--rpc.laddr", f"tcp://0.0.0.0:{node.rpc_port}"])

        elif node.client_type in (ChainClientType.POLKADOT, ChainClientType.X3_NODE):
            cmd.extend([
                "--base-path", node.data_dir,
                "--port", str(node.p2p_port),
                "--rpc-port", str(node.rpc_port),
                "--rpc-external",
                "--validator",
            ])
            if node.bootnodes:
                for bn in node.bootnodes:
                    cmd.extend(["--bootnodes", bn])

        # Config file override
        if node.config_file:
            cmd.extend(["--config", node.config_file])

        # Extra args
        cmd.extend(node.args)
        return cmd

    def _stop_process(self, node: ValidatorNode) -> bool:
        """Gracefully stop a node process."""
        if not node.pid:
            node.status = NodeStatus.STOPPED
            return True

        try:
            # Send SIGTERM first
            os.kill(node.pid, signal.SIGTERM)
            logger.info("Sent SIGTERM to %s (pid=%d)", node.chain_id, node.pid)

            # Wait up to 30 seconds for graceful shutdown
            for _ in range(30):
                time.sleep(1)
                try:
                    os.kill(node.pid, 0)
                except ProcessLookupError:
                    break
            else:
                # Force kill
                logger.warning("Node %s didn't stop, sending SIGKILL", node.chain_id)
                os.kill(node.pid, signal.SIGKILL)

        except ProcessLookupError:
            pass
        except Exception as exc:
            logger.error("Error stopping %s: %s", node.chain_id, exc)
            return False

        node.pid = None
        node.status = NodeStatus.STOPPED
        self._persist_state()
        return True

    def _maybe_restart(self, node: ValidatorNode) -> None:
        """Auto-restart a crashed node with backoff."""
        if node.restarts >= node.max_restarts:
            logger.error(
                "Node %s exceeded max restarts (%d), decommissioning",
                node.chain_id, node.max_restarts,
            )
            node.status = NodeStatus.DECOMMISSIONED
            return

        now = time.time()
        cooldown = node.restart_cooldown_seconds * (2 ** min(node.restarts, 6))
        if (now - node.last_restart_at) < cooldown:
            logger.info(
                "Node %s in cooldown (%.0fs remaining)",
                node.chain_id, cooldown - (now - node.last_restart_at),
            )
            return

        node.restarts += 1
        node.last_restart_at = now
        node.status = NodeStatus.RESTARTING
        logger.info("Auto-restarting %s (attempt %d/%d)", node.chain_id, node.restarts, node.max_restarts)
        self.start_node(node.chain_id)

    # ─── Utility ─────────────────────────────────────────────

    def _detect_binary(self, client_type: ChainClientType) -> str:
        """Try to find the binary on PATH."""
        import shutil
        path = shutil.which(client_type.value)
        if path:
            return path
        # Common install locations
        for prefix in ["/usr/local/bin", "/usr/bin", os.path.expanduser("~/.local/bin")]:
            candidate = os.path.join(prefix, client_type.value)
            if os.path.isfile(candidate):
                return candidate
        return client_type.value  # Will fail at start time if not found

    def _persist_state(self) -> None:
        """Save node registry to disk."""
        state_file = self._state_dir / "node_registry.json"
        with self._lock:
            data = {k: v.to_dict() for k, v in self._nodes.items()}
        try:
            state_file.write_text(json.dumps(data, indent=2))
        except Exception as exc:
            logger.debug("Failed to persist state: %s", exc)

    def load_state(self) -> None:
        """Load node registry from disk (for recovery after reboot)."""
        state_file = self._state_dir / "node_registry.json"
        if not state_file.exists():
            return
        try:
            data = json.loads(state_file.read_text())
            logger.info("Loaded %d nodes from state file", len(data))
            # Nodes will be in STOPPED state — caller must start them
        except Exception as exc:
            logger.warning("Failed to load state: %s", exc)

    # ─── Accessors ───────────────────────────────────────────

    def get_node(self, chain_id: str) -> ValidatorNode | None:
        with self._lock:
            return self._nodes.get(chain_id)

    def get_all_nodes(self) -> dict[str, ValidatorNode]:
        with self._lock:
            return dict(self._nodes)

    def get_status_summary(self) -> dict:
        """Get a summary of all nodes for dashboard display."""
        with self._lock:
            nodes = list(self._nodes.values())

        total = len(nodes)
        by_status = {}
        by_chain = {}
        for n in nodes:
            by_status[n.status.value] = by_status.get(n.status.value, 0) + 1
            by_chain[n.chain_id] = n.to_dict()

        return {
            "total_nodes": total,
            "by_status": by_status,
            "nodes": by_chain,
        }