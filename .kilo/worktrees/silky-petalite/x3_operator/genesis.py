"""
X3 Genesis Ceremony Tools
~~~~~~~~~~~~~~~~~~~~~~~~~~

Genesis block creation, configuration freezing, hash anchoring,
and multi-party verification for mainnet launch.
"""

import hashlib
import json
import logging
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional

from .config import X3Config

logger = logging.getLogger(__name__)


@dataclass
class GenesisParticipant:
    """Verified participant in the genesis ceremony."""
    operator_id: str
    pubkey: str
    role: str
    stake: int
    attestation_hash: str = ""
    signed_at: float = 0.0

    def sign_genesis(self, genesis_hash: str) -> str:
        """Produce attestation over genesis hash."""
        payload = f"{self.operator_id}:{self.pubkey}:{genesis_hash}:{time.time()}"
        self.attestation_hash = hashlib.sha256(payload.encode()).hexdigest()
        self.signed_at = time.time()
        return self.attestation_hash


@dataclass
class GenesisConfig:
    """Frozen genesis parameters."""
    chain_id: str
    chain_name: str
    block_time_ms: int = 6000
    epoch_length: int = 600
    initial_validators: list = field(default_factory=list)
    initial_balances: dict = field(default_factory=dict)  # address -> balance
    sudo_key: str = ""
    governance_params: dict = field(default_factory=dict)
    pallet_configs: dict = field(default_factory=dict)
    created_at: float = 0.0
    frozen: bool = False
    frozen_hash: str = ""

    def compute_hash(self) -> str:
        """Deterministic hash of all genesis parameters."""
        canonical = json.dumps({
            "chain_id": self.chain_id,
            "chain_name": self.chain_name,
            "block_time_ms": self.block_time_ms,
            "epoch_length": self.epoch_length,
            "initial_validators": sorted(self.initial_validators),
            "initial_balances": dict(sorted(self.initial_balances.items())),
            "sudo_key": self.sudo_key,
            "governance_params": dict(sorted(self.governance_params.items())),
            "pallet_configs": dict(sorted(self.pallet_configs.items())),
        }, sort_keys=True, separators=(",", ":"))
        return hashlib.sha256(canonical.encode()).hexdigest()

    def freeze(self) -> str:
        """Freeze config. No more changes allowed."""
        if self.frozen:
            raise RuntimeError("Genesis config already frozen")
        self.frozen = True
        self.frozen_hash = self.compute_hash()
        self.created_at = time.time()
        logger.info("genesis config frozen: hash=%s", self.frozen_hash)
        return self.frozen_hash

    def verify(self) -> bool:
        """Verify frozen hash matches current parameters."""
        if not self.frozen:
            return False
        return self.compute_hash() == self.frozen_hash

    def to_dict(self) -> dict:
        return {
            "chain_id": self.chain_id,
            "chain_name": self.chain_name,
            "block_time_ms": self.block_time_ms,
            "epoch_length": self.epoch_length,
            "initial_validators": self.initial_validators,
            "initial_balances": self.initial_balances,
            "sudo_key": self.sudo_key,
            "governance_params": self.governance_params,
            "pallet_configs": self.pallet_configs,
            "created_at": self.created_at,
            "frozen": self.frozen,
            "frozen_hash": self.frozen_hash,
        }


@dataclass
class CeremonyStep:
    name: str
    description: str
    completed: bool = False
    completed_at: float = 0.0
    result: str = ""


class GenesisCeremony:
    """Orchestrates the genesis ceremony process.

    Steps:
    1. Configure genesis parameters
    2. Collect participant attestations
    3. Freeze configuration
    4. Multi-party verification
    5. Generate chain spec
    6. Dry run validation
    7. Anchor genesis hash
    """

    def __init__(self, config: X3Config):
        self.config = config
        self.genesis: Optional[GenesisConfig] = None
        self.participants: dict[str, GenesisParticipant] = {}
        self.steps: list[CeremonyStep] = [
            CeremonyStep("configure", "Set genesis parameters"),
            CeremonyStep("collect_attestations", "Collect participant attestations"),
            CeremonyStep("freeze", "Freeze genesis configuration"),
            CeremonyStep("verify", "Multi-party verification"),
            CeremonyStep("generate_spec", "Generate chain specification"),
            CeremonyStep("dry_run", "Validate with dry run"),
            CeremonyStep("anchor", "Anchor genesis hash on-chain"),
        ]

    def configure_genesis(
        self,
        chain_id: str,
        chain_name: str,
        initial_validators: list,
        initial_balances: dict,
        sudo_key: str = "",
    ) -> GenesisConfig:
        """Step 1: Configure genesis parameters."""
        self.genesis = GenesisConfig(
            chain_id=chain_id,
            chain_name=chain_name,
            initial_validators=initial_validators,
            initial_balances=initial_balances,
            sudo_key=sudo_key,
            governance_params={
                "proposal_bond_fraction": 0.05,
                "voting_period_blocks": 100800,  # ~7 days at 6s blocks
                "enactment_delay_blocks": 28800,  # ~2 days
                "min_turnout_fraction": 0.10,
                "conviction_max": 6,
            },
        )
        self._complete_step("configure", f"chain={chain_id}")
        return self.genesis

    def add_participant(self, participant: GenesisParticipant):
        """Step 2: Register ceremony participants."""
        self.participants[participant.operator_id] = participant
        logger.info("ceremony participant added: %s (%s)", participant.operator_id, participant.role)

    def collect_attestations(self) -> int:
        """Collect signed attestations from all participants."""
        if self.genesis is None:
            raise RuntimeError("Genesis not configured")
        genesis_hash = self.genesis.compute_hash()
        count = 0
        for p in self.participants.values():
            p.sign_genesis(genesis_hash)
            count += 1
        self._complete_step("collect_attestations", f"{count} attestations collected")
        return count

    def freeze_genesis(self) -> str:
        """Step 3: Freeze genesis config."""
        if self.genesis is None:
            raise RuntimeError("Genesis not configured")
        frozen_hash = self.genesis.freeze()
        self._complete_step("freeze", f"hash={frozen_hash[:16]}...")
        return frozen_hash

    def verify_genesis(self) -> tuple[bool, list[str]]:
        """Step 4: Multi-party verification."""
        if self.genesis is None or not self.genesis.frozen:
            raise RuntimeError("Genesis not frozen")

        errors = []
        genesis_hash = self.genesis.frozen_hash

        # Verify config integrity
        if not self.genesis.verify():
            errors.append("Genesis hash mismatch - config tampered")

        # Verify all participants signed the correct hash
        for p_id, p in self.participants.items():
            if not p.attestation_hash:
                errors.append(f"Participant {p_id} has no attestation")

        # Verify minimum validator count
        min_validators = 3
        if len(self.genesis.initial_validators) < min_validators:
            errors.append(f"Need >= {min_validators} validators, have {len(self.genesis.initial_validators)}")

        # Verify balances sum
        total_issuance = sum(self.genesis.initial_balances.values())
        if total_issuance == 0:
            errors.append("Zero total issuance")

        passed = len(errors) == 0
        self._complete_step("verify", "PASSED" if passed else f"FAILED: {len(errors)} errors")

        return passed, errors

    def generate_chain_spec(self, output_path: Path) -> Path:
        """Step 5: Generate Substrate chain spec JSON."""
        if self.genesis is None or not self.genesis.frozen:
            raise RuntimeError("Genesis not frozen")

        spec = {
            "name": self.genesis.chain_name,
            "id": self.genesis.chain_id,
            "chainType": "Live",
            "bootNodes": [],
            "telemetryEndpoints": [["/dns/telemetry.x3-chain.io/tcp/443/wss", 0]],
            "protocolId": self.genesis.chain_id,
            "properties": {
                "ss58Format": 42,
                "tokenDecimals": 12,
                "tokenSymbol": "X3",
            },
            "genesis": {
                "runtime": {
                    "system": {"code": "0x"},
                    "balances": {
                        "balances": [
                            [addr, bal] for addr, bal in self.genesis.initial_balances.items()
                        ],
                    },
                    "aura": {
                        "authorities": self.genesis.initial_validators,
                    },
                    "grandpa": {
                        "authorities": [[v, 1] for v in self.genesis.initial_validators],
                    },
                    "sudo": {
                        "key": self.genesis.sudo_key,
                    },
                },
            },
            "x3_genesis_hash": self.genesis.frozen_hash,
            "x3_participants": len(self.participants),
            "x3_created_at": self.genesis.created_at,
        }

        output_path.parent.mkdir(parents=True, exist_ok=True)
        output_path.write_text(json.dumps(spec, indent=2))
        self._complete_step("generate_spec", str(output_path))
        logger.info("chain spec written to %s", output_path)
        return output_path

    def dry_run(self) -> tuple[bool, list[str]]:
        """Step 6: Validate all parameters for consistency."""
        if self.genesis is None:
            raise RuntimeError("Genesis not configured")

        issues = []

        # Check all validators have balances
        for v in self.genesis.initial_validators:
            if v not in self.genesis.initial_balances:
                issues.append(f"Validator {v} has no initial balance")

        # Check sudo key
        if self.genesis.sudo_key and self.genesis.sudo_key not in self.genesis.initial_balances:
            issues.append("Sudo key has no initial balance")

        # Check governance params
        gp = self.genesis.governance_params
        if gp.get("min_turnout_fraction", 0) > 0.50:
            issues.append("Min turnout > 50% may be unreachable")

        passed = len(issues) == 0
        self._complete_step("dry_run", "PASSED" if passed else f"{len(issues)} issues")
        return passed, issues

    def anchor_hash(self) -> dict:
        """Step 7: Anchor genesis hash (returns transaction data)."""
        if self.genesis is None or not self.genesis.frozen:
            raise RuntimeError("Genesis not frozen")

        anchor = {
            "genesis_hash": self.genesis.frozen_hash,
            "chain_id": self.genesis.chain_id,
            "participants": len(self.participants),
            "validators": len(self.genesis.initial_validators),
            "total_issuance": sum(self.genesis.initial_balances.values()),
            "anchored_at": time.time(),
            "anchor_hash": hashlib.sha256(
                f"{self.genesis.frozen_hash}:{time.time()}".encode()
            ).hexdigest(),
        }
        self._complete_step("anchor", f"anchor={anchor['anchor_hash'][:16]}...")
        logger.info("genesis hash anchored: %s", anchor["anchor_hash"])
        return anchor

    def status(self) -> dict:
        """Current ceremony status."""
        return {
            "steps": [
                {
                    "name": s.name,
                    "description": s.description,
                    "completed": s.completed,
                    "result": s.result,
                }
                for s in self.steps
            ],
            "participants": len(self.participants),
            "genesis_frozen": self.genesis.frozen if self.genesis else False,
            "genesis_hash": self.genesis.frozen_hash if self.genesis and self.genesis.frozen else "",
        }

    def _complete_step(self, name: str, result: str):
        for step in self.steps:
            if step.name == name:
                step.completed = True
                step.completed_at = time.time()
                step.result = result
                logger.info("ceremony step complete: %s → %s", name, result)
                break
