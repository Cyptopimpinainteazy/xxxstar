"""
X3 Operator Identity
~~~~~~~~~~~~~~~~~~~~

Hardware-bound operator identity with deterministic ID generation.
Keys never leave this module.
"""

import hashlib
import json
import logging
import os
import platform
import secrets
import time
from dataclasses import dataclass, asdict
from pathlib import Path
from typing import Optional

logger = logging.getLogger(__name__)


@dataclass
class HardwareAttestation:
    """Machine fingerprint for hardware-bound identity."""
    hostname: str
    os_name: str
    os_version: str
    arch: str
    cpu_count: int
    machine_id: str  # /etc/machine-id or equivalent

    def fingerprint(self) -> str:
        """Deterministic hardware fingerprint hash."""
        raw = json.dumps(asdict(self), sort_keys=True)
        return hashlib.sha256(raw.encode()).hexdigest()


@dataclass
class OperatorIdentity:
    """On-chain operator identity."""
    operator_id: str
    pubkey: str
    hardware_fingerprint: str
    role: str
    created_at: float
    nonce: str  # anti-replay

    def to_registration_hash(self) -> str:
        """Hash for on-chain registration."""
        raw = json.dumps({
            "operator_id": self.operator_id,
            "pubkey": self.pubkey,
            "hardware_fingerprint": self.hardware_fingerprint,
            "role": self.role,
            "nonce": self.nonce,
        }, sort_keys=True)
        return hashlib.blake2b(raw.encode(), digest_size=32).hexdigest()


def _get_machine_id() -> str:
    """Read machine ID from OS, or generate a persistent one."""
    machine_id_paths = [
        "/etc/machine-id",
        "/var/lib/dbus/machine-id",
    ]
    for path in machine_id_paths:
        try:
            return Path(path).read_text().strip()
        except (FileNotFoundError, PermissionError):
            continue
    # Fallback: generate and persist
    fallback = Path.home() / ".x3-operator" / "machine-id"
    if fallback.exists():
        return fallback.read_text().strip()
    mid = secrets.token_hex(16)
    fallback.parent.mkdir(parents=True, exist_ok=True)
    fallback.write_text(mid)
    return mid


def collect_hardware_attestation() -> HardwareAttestation:
    """Collect current machine's hardware attestation."""
    uname = platform.uname()
    return HardwareAttestation(
        hostname=uname.node,
        os_name=uname.system,
        os_version=uname.release,
        arch=uname.machine,
        cpu_count=os.cpu_count() or 1,
        machine_id=_get_machine_id(),
    )


def generate_operator_identity(
    role,
    data_dir = "~/.x3-operator",
) -> OperatorIdentity:
    """Generate a new operator identity bound to this machine.

    Identity is deterministic: same machine + role = same operator_id.
    Keypair is generated fresh and stored locally.
    """
    data_path = Path(str(data_dir)).expanduser()
    data_path.mkdir(parents=True, exist_ok=True)

    hw = collect_hardware_attestation()
    fingerprint = hw.fingerprint()

    # Deterministic operator ID from hardware + role
    role_str = role.value if hasattr(role, 'value') else str(role)
    id_input = f"{fingerprint}:{role_str}"
    operator_id = hashlib.blake2b(id_input.encode(), digest_size=16).hexdigest()

    # Generate keypair (Ed25519 simulation - in production use ed25519-dalek via FFI)
    seed = secrets.token_hex(32)
    pubkey = hashlib.sha256(seed.encode()).hexdigest()

    identity = OperatorIdentity(
        operator_id=operator_id,
        pubkey=pubkey,
        hardware_fingerprint=fingerprint,
        role=role_str,
        created_at=time.time(),
        nonce=secrets.token_hex(8),
    )

    # Persist identity
    identity_path = data_path / "identity.json"
    identity_path.write_text(json.dumps(asdict(identity), indent=2))

    # Persist key (encrypted in production)
    key_path = data_path / "operator.key"
    key_path.write_text(seed)
    key_path.chmod(0o600)

    logger.info("Generated operator identity: %s (role=%s)", operator_id, role_str)
    return identity


def load_operator_identity(data_dir = "~/.x3-operator") -> Optional[OperatorIdentity]:
    """Load existing operator identity from disk."""
    identity_path = Path(str(data_dir)).expanduser()
    if identity_path.is_dir():
        identity_path = identity_path / "identity.json"
    if not identity_path.exists():
        return None
    data = json.loads(identity_path.read_text())
    return OperatorIdentity(**data)
