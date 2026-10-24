"""Authenticated local volunteer job dispatch; no network listener or GPU runtime.

This bridge checks a signed lease, verifies the exact manifest bytes, consumes
its nonce durably, and only then runs a fixed CPU smoke test. Do not expose
this module as a public execution service.
"""
import json
from lease_auth import LeaseVerifier
from safe_benchmark import execute

MAX_MANIFEST_BYTES = 1024

def execute_signed_lease(verifier: LeaseVerifier, lease: dict, signature: str,
                         worker_id: str, manifest_bytes: bytes, *,
                         consent: bool, revoked: bool = False,
                         stop_requested: bool = False, now=None):
    if not consent or revoked or stop_requested:
        raise PermissionError("worker not accepting jobs")
    if not isinstance(manifest_bytes, bytes) or len(manifest_bytes) > MAX_MANIFEST_BYTES:
        raise ValueError("invalid manifest size")
    # Fail closed: exactly one workload; no arbitrary shell, Python, URLs,
    # paths, model files or remote artifacts.
    try:
        manifest = json.loads(manifest_bytes)
    except (ValueError, UnicodeDecodeError) as exc:
        raise ValueError("invalid manifest") from exc
    if manifest != {"kind": "fixed_cpu_smoke", "version": 1}:
        raise ValueError("workload not allowlisted")
    verifier.accept(lease, signature, worker_id, lease["job_id"], manifest_bytes, now=now)
    return execute(manifest, consent=consent, revoked=revoked)
