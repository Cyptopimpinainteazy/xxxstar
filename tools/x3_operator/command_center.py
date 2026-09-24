"""
X3 Command Center Backend
~~~~~~~~~~~~~~~~~~~~~~~~~

REST API server for the operator command center.
Exposes operator status, bonding, slashing, agent supervision,
storage deals, governance simulations, and genesis ceremony
management over HTTP + WebSocket.
"""

import json
import logging
import time
from http.server import HTTPServer, BaseHTTPRequestHandler
from pathlib import Path
from urllib.parse import urlparse, parse_qs

from x3_operator.config import X3Config
from x3_operator.identity import load_operator_identity
from x3_operator.health import run_health_check
from x3_operator.bonding import BondLedger
from x3_operator.slashing import SlashingEngine, SlashEvidence, FaultType
from x3_operator.supervisor import AgentSupervisor, PolicyManifest
from x3_operator.storage import StorageRegistry, ContentID
from x3_operator.governance import GovernanceSimulator
from x3_operator.genesis import GenesisCeremony, GenesisParticipant
from x3_operator.telemetry import create_operator_metrics

logger = logging.getLogger(__name__)


class CommandCenterState:
    """Shared state for the command center API."""

    def __init__(self, data_dir: Path):
        self.data_dir = data_dir
        self.config = self._load_config()
        self.identity = load_operator_identity(data_dir)
        self.bond_ledger = self._load_bonds()
        self.slashing_engine = SlashingEngine(self.config)
        self.supervisor = AgentSupervisor(self.config)
        self.storage_registry = StorageRegistry(self.config)
        self.metrics = create_operator_metrics()
        self.start_time = time.time()

    def _load_config(self) -> X3Config:
        config_path = self.data_dir / "config.json"
        if config_path.exists():
            return X3Config.load(config_path)
        return X3Config()

    def _load_bonds(self) -> BondLedger:
        bond_path = self.data_dir / "bonds.json"
        if bond_path.exists():
            return BondLedger.load(bond_path)
        return BondLedger()


class CommandCenterHandler(BaseHTTPRequestHandler):
    """HTTP request handler for the command center API."""

    state: CommandCenterState = None  # Set by server

    def do_GET(self):
        parsed = urlparse(self.path)
        path = parsed.path.rstrip("/")
        params = parse_qs(parsed.query)

        routes = {
            "/api/health": self._handle_health,
            "/api/status": self._handle_status,
            "/api/identity": self._handle_identity,
            "/api/bond": self._handle_bond_status,
            "/api/agents": self._handle_agents,
            "/api/storage/deals": self._handle_storage_deals,
            "/api/metrics": self._handle_metrics,
            "/api/metrics/prometheus": self._handle_prometheus,
            "/api/config": self._handle_config,
        }

        handler = routes.get(path)
        if handler:
            handler(params)
        else:
            self._json_response(404, {"error": "not found", "path": path})

    def do_OPTIONS(self):
        """Handle CORS preflight requests."""
        self.send_response(204)
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "Content-Type, Authorization")
        self.send_header("Access-Control-Max-Age", "86400")
        self.end_headers()

    def do_POST(self):
        parsed = urlparse(self.path)
        path = parsed.path.rstrip("/")

        content_length = int(self.headers.get("Content-Length", 0))
        body = {}
        if content_length > 0:
            raw = self.rfile.read(content_length)
            try:
                body = json.loads(raw)
            except json.JSONDecodeError:
                self._json_response(400, {"error": "invalid JSON body"})
                return

        routes = {
            "/api/simulate": self._handle_simulate,
            "/api/simulate/whale": self._handle_simulate_whale,
            "/api/simulate/sybil": self._handle_simulate_sybil,
            "/api/simulate/bribery": self._handle_simulate_bribery,
            "/api/simulate/speed": self._handle_simulate_speed,
            "/api/agents/register": self._handle_register_agent,
            "/api/agents/kill": self._handle_kill_agent,
            "/api/agents/kill-switch": self._handle_kill_switch,
            "/api/storage/register": self._handle_register_storage,
            "/api/storage/propose": self._handle_propose_deal,
            "/api/genesis/run": self._handle_genesis_run,
            "/api/slash": self._handle_slash,
        }

        handler = routes.get(path)
        if handler:
            handler(body)
        else:
            self._json_response(404, {"error": "not found", "path": path})

    # GET handlers

    def _handle_health(self, params):
        role = params.get("role", [None])[0]
        require_gpu = role == "gpu" if role else False
        report = run_health_check(require_gpu=require_gpu)
        data = {
            "overall": report.overall.value,
            "checks": [
                {
                    "name": c.name,
                    "status": c.status.value,
                    "value": c.value,
                    "required": c.required,
                    "message": c.message,
                }
                for c in report.checks
            ],
            "recommended_roles": report.recommended_roles,
        }
        self._json_response(200, data)

    def _handle_status(self, params):
        state = self.state
        uptime = time.time() - state.start_time
        op_id = state.identity.operator_id if state.identity else "unknown"
        role = state.identity.role if state.identity else "unknown"

        bond = None
        if state.identity:
            bond_rec = state.bond_ledger.get_bond(state.identity.operator_id)
            if bond_rec:
                bond = bond_rec.to_dict()

        data = {
            "operator_id": op_id,
            "role": role,
            "network": state.config.chain.network_phase.value,
            "rpc_url": state.config.chain.rpc_url,
            "uptime_seconds": round(uptime, 1),
            "bond": bond,
            "agents": state.supervisor.status_summary(),
        }
        self._json_response(200, data)

    def _handle_identity(self, params):
        if not self.state.identity:
            self._json_response(404, {"error": "not initialized"})
            return
        ident = self.state.identity
        data = {
            "operator_id": ident.operator_id,
            "pubkey": ident.pubkey,
            "hardware_fingerprint": ident.hardware_fingerprint,
            "role": ident.role,
            "created_at": ident.created_at,
        }
        self._json_response(200, data)

    def _handle_bond_status(self, params):
        if not self.state.identity:
            self._json_response(404, {"error": "not initialized"})
            return
        bond = self.state.bond_ledger.get_bond(self.state.identity.operator_id)
        if bond:
            self._json_response(200, bond.to_dict())
        else:
            self._json_response(200, {"status": "unbonded"})

    def _handle_agents(self, params):
        data = self.state.supervisor.status_summary()
        data["agents"] = [
            {
                "agent_id": r.agent_id,
                "operator_id": r.operator_id,
                "state": r.state.value,
                "pid": r.pid,
                "violations": r.violations,
                "call_count": r.call_count,
            }
            for r in self.state.supervisor.agents.values()
        ]
        self._json_response(200, data)

    def _handle_storage_deals(self, params):
        deals = list(self.state.storage_registry.deals.values())
        data = {
            "count": len(deals),
            "deals": [
                {
                    "deal_id": d.deal_id,
                    "client_id": d.client_id,
                    "provider_id": d.provider_id,
                    "cid": d.content.cid[:24],
                    "size_bytes": d.content.size_bytes,
                    "status": d.status.value,
                    "proof_count": d.proof_count,
                    "fault_count": d.fault_count,
                }
                for d in deals
            ],
        }
        self._json_response(200, data)

    def _handle_metrics(self, params):
        self._json_response(200, json.loads(self.state.metrics.export_json()))

    def _handle_prometheus(self, params):
        self.send_response(200)
        self.send_header("Content-Type", "text/plain; charset=utf-8")
        self.end_headers()
        self.wfile.write(self.state.metrics.export_prometheus().encode())

    def _handle_config(self, params):
        self._json_response(200, self.state.config.to_dict())

    # POST handlers

    def _handle_simulate(self, body):
        seed = body.get("seed", 42)
        sim = GovernanceSimulator(seed=seed)
        reports = sim.run_full_suite()
        self._json_response(200, {
            "reports": [r.to_dict() for r in reports],
            "summary": sim.summary(),
        })

    def _handle_simulate_whale(self, body):
        seed = body.get("seed", 42)
        sim = GovernanceSimulator(seed=seed)
        report = sim.simulate_whale_attack(
            whale_stake_fraction=body.get("whale_stake_fraction", 0.34),
        )
        self._json_response(200, report.to_dict())

    def _handle_simulate_sybil(self, body):
        seed = body.get("seed", 42)
        sim = GovernanceSimulator(seed=seed)
        report = sim.simulate_sybil_attack(
            n_sybils=body.get("n_sybils", 500),
        )
        self._json_response(200, report.to_dict())

    def _handle_simulate_bribery(self, body):
        seed = body.get("seed", 42)
        sim = GovernanceSimulator(seed=seed)
        report = sim.simulate_bribery_attack(
            bribe_budget=body.get("bribe_budget", 50000),
        )
        self._json_response(200, report.to_dict())

    def _handle_simulate_speed(self, body):
        seed = body.get("seed", 42)
        sim = GovernanceSimulator(seed=seed)
        report = sim.simulate_speed_attack()
        self._json_response(200, report.to_dict())

    def _handle_register_agent(self, body):
        agent_id = body.get("agent_id")
        operator_id = body.get("operator_id", "")
        if not agent_id:
            self._json_response(400, {"error": "agent_id required"})
            return
        if not operator_id and self.state.identity:
            operator_id = self.state.identity.operator_id

        policy = PolicyManifest(
            allowed_endpoints=body.get("allowed_endpoints", []),
            max_memory_mb=body.get("max_memory_mb", 512),
            max_cpu_percent=body.get("max_cpu_percent", 25.0),
        )
        try:
            record = self.state.supervisor.register_agent(agent_id, operator_id, policy)
            self._json_response(200, {
                "agent_id": record.agent_id,
                "state": record.state.value,
                "policy_hash": record.policy_hash,
            })
        except ValueError as e:
            self._json_response(400, {"error": str(e)})

    def _handle_kill_agent(self, body):
        agent_id = body.get("agent_id")
        reason = body.get("reason", "manual kill")
        if not agent_id:
            self._json_response(400, {"error": "agent_id required"})
            return
        try:
            self.state.supervisor.kill_agent(agent_id, reason)
            self._json_response(200, {"status": "killed", "agent_id": agent_id})
        except KeyError as e:
            self._json_response(404, {"error": str(e)})

    def _handle_kill_switch(self, body):
        self.state.supervisor.arm_kill_switch()
        self._json_response(200, {"status": "kill_switch_armed"})

    def _handle_register_storage(self, body):
        provider_id = body.get("provider_id")
        capacity = body.get("capacity_bytes", 0)
        if not provider_id or capacity <= 0:
            self._json_response(400, {"error": "provider_id and capacity_bytes required"})
            return
        self.state.storage_registry.register_provider(provider_id, capacity)
        self._json_response(200, {"status": "registered", "provider_id": provider_id})

    def _handle_propose_deal(self, body):
        try:
            content = ContentID(
                cid=body.get("cid", ""),
                size_bytes=body.get("size_bytes", 0),
                checksum_sha256=body.get("checksum", ""),
            )
            deal = self.state.storage_registry.propose_deal(
                client_id=body["client_id"],
                provider_id=body["provider_id"],
                content=content,
                duration_seconds=body.get("duration_seconds", 86400),
            )
            self._json_response(200, {
                "deal_id": deal.deal_id,
                "status": deal.status.value,
                "total_cost": deal.total_cost(),
            })
        except (KeyError, ValueError) as e:
            self._json_response(400, {"error": str(e)})

    def _handle_genesis_run(self, body):
        ceremony = GenesisCeremony(self.state.config)
        validators = body.get("validators", [
            "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
            "5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty",
            "5FLSigC9HGRKVhB9FiEo4Y3koPsNmBmLJbpXg2mp1hXcS59Y",
        ])
        balances = {v: 1_000_000_000_000_000 for v in validators}
        genesis = ceremony.configure_genesis(
            chain_id=body.get("chain_id", "x3-devnet"),
            chain_name=body.get("chain_name", "X3 Devnet"),
            initial_validators=validators,
            initial_balances=balances,
            sudo_key=validators[0],
        )
        for i, v in enumerate(validators):
            ceremony.add_participant(GenesisParticipant(
                operator_id=f"op-{i:04d}", pubkey=v, role="validator", stake=balances[v],
            ))
        ceremony.collect_attestations()
        frozen_hash = ceremony.freeze_genesis()
        passed, errors = ceremony.verify_genesis()
        spec_path = self.state.data_dir / "chain-spec.json"
        ceremony.generate_chain_spec(spec_path)
        dry_ok, issues = ceremony.dry_run()
        anchor = ceremony.anchor_hash()

        self._json_response(200, {
            "frozen_hash": frozen_hash,
            "verification": {"passed": passed, "errors": errors},
            "dry_run": {"passed": dry_ok, "issues": issues},
            "anchor": anchor,
            "spec_path": str(spec_path),
            "status": ceremony.status(),
        })

    def _handle_slash(self, body):
        operator_id = body.get("operator_id")
        fault_type = body.get("fault_type")
        if not operator_id or not fault_type:
            self._json_response(400, {"error": "operator_id and fault_type required"})
            return

        bond = self.state.bond_ledger.get_bond(operator_id)
        if not bond:
            self._json_response(404, {"error": f"no bond for {operator_id}"})
            return

        evidence = SlashEvidence(
            fault_type=FaultType(fault_type),
            operator_id=operator_id,
            block_number=body.get("block_number", 0),
            timestamp=time.time(),
            description=body.get("description", ""),
            reporter_id=body.get("reporter_id", "api"),
        )
        verdict = self.state.slashing_engine.evaluate(
            evidence, bond.effective_stake(),
            confidence=body.get("confidence", 1.0),
        )
        self.state.bond_ledger.apply_slash(operator_id, verdict.slash_amount, fault_type)
        self._json_response(200, verdict.to_dict())

    def _json_response(self, status: int, data: dict):
        payload = json.dumps(data, indent=2, default=str).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(payload)))
        self.send_header("Access-Control-Allow-Origin", "*")
        self.end_headers()
        self.wfile.write(payload)

    def log_message(self, format, *args):
        logger.info(format, *args)


def run_server(host: str = "0.0.0.0", port: int = 8900, data_dir: str = "~/.x3_operator"):
    """Start the command center HTTP server."""
    data_path = Path(data_dir).expanduser()
    state = CommandCenterState(data_path)

    CommandCenterHandler.state = state

    server = HTTPServer((host, port), CommandCenterHandler)
    logger.info("Command Center running on http://%s:%d", host, port)
    print(f"X3 Command Center running on http://{host}:{port}")
    print(f"Data dir: {data_path}")
    print(f"Operator: {state.identity.operator_id if state.identity else 'not initialized'}")
    print("Press Ctrl+C to stop.\n")

    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\nShutting down command center...")
        server.shutdown()


if __name__ == "__main__":
    import argparse
    from x3_operator.telemetry import setup_structured_logging

    parser = argparse.ArgumentParser(description="X3 Command Center")
    parser.add_argument("--host", default="0.0.0.0")
    parser.add_argument("--port", type=int, default=8900)
    parser.add_argument("--data-dir", default="~/.x3_operator")
    args = parser.parse_args()

    setup_structured_logging(level="INFO")
    run_server(args.host, args.port, args.data_dir)
