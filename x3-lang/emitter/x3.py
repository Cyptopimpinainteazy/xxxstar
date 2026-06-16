import json
from typing import Any, Dict


class ProofRequiredError(Exception):
    code = 'X3_PROOF_REQUIRED'


def _serialize_x3_payload(payload: Dict[str, Any]) -> str:
    return '0x' + json.dumps(payload, sort_keys=True).encode('utf-8').hex()


def verify_proof_bundle(payload: Dict[str, Any], proof_bundle: Dict[str, Any]) -> bool:
    """Verifier contract boundary.

    Production execution must supply a backend-produced proof bundle containing
    the payload hash, finality checkpoint, settlement root, and verifier status.
    The emitter never manufactures those values.
    """
    if not isinstance(proof_bundle, dict):
        return False
    required = {'payload_hash', 'checkpoint', 'settlement_root', 'verifier', 'finality'}
    if not required.issubset(proof_bundle):
        return False
    return proof_bundle.get('verifier', {}).get('status') == 'verified' and bool(proof_bundle.get('finality', {}).get('confirmed'))


def emit(plan: Dict[str, Any], step: Dict[str, Any], *, proof_bundle: Dict[str, Any] | None = None, dry_run: bool = False) -> Dict[str, Any]:
    amount = step.get('amount') or plan.get('from', {}).get('amount') or 0
    try:
        amount = int(float(amount))
    except Exception:
        amount = 0

    payload = {
        'action': 'settle',
        'asset': step.get('asset'),
        'amount': amount,
        'source_chain': step.get('from_ref', {}).get('chain') or step.get('chain'),
        'target_chain': step.get('to_ref', {}).get('chain') or plan.get('to', {}).get('chain'),
        'receiver': step.get('receiver') or plan.get('to', {}).get('receiver'),
        'extras': {'intent': plan.get('intent'), 'path_step': step.get('step')},
    }
    tx = {
        'type': 'bridge',
        'chain': 'x3',
        'via': step.get('via'),
        'asset': step.get('asset'),
        'amount': amount,
        'payload': payload,
        'transaction_id': f"x3-bridge-{plan.get('intent')}-{step.get('step')}",
        'transaction_bytes': _serialize_x3_payload(payload),
        'proof_required': True,
        'dry_run': dry_run,
    }
    if dry_run:
        # Dry-run mode records the payload hash but never fabricates
        # signatures, settlement roots, or finality confirmations.
        tx['proof_bundle'] = {
            'bundle_version': '0.1',
            'dry_run': True,
            'verifier': {'status': 'pending', 'id': None},
            'finality': {
                'confirmed': False,
                'checkpoint': 0,
                'source_chain': payload.get('source_chain'),
                'target_chain': payload.get('target_chain'),
            },
            'checkpoint': 0,
            'settlement_root': None,
            'payload_hash': '0x' + __import__('hashlib').sha256(_serialize_x3_payload(payload).encode()).hexdigest(),
            'signatures': [],
            'error': 'dry-run cannot produce a production proof bundle',
            'error_code': 'X3_PROOF_REQUIRED',
        }
        return tx

    if not verify_proof_bundle(payload, proof_bundle or {}):
        # Production settlement requires a real backend-produced proof
        # bundle. The emitter never manufactures signatures, roots, or
        # finality confirmations. Return a structured rejection so
        # downstream callers fail closed.
        import hashlib as _hashlib
        tx['proof_bundle'] = {
            'bundle_version': '0.1',
            'dry_run': False,
            'verifier': {'status': 'pending', 'id': None},
            'finality': {
                'confirmed': False,
                'checkpoint': 0,
                'source_chain': payload.get('source_chain'),
                'target_chain': payload.get('target_chain'),
            },
            'checkpoint': 0,
            'settlement_root': None,
            'payload_hash': '0x' + _hashlib.sha256(_serialize_x3_payload(payload).encode()).hexdigest(),
            'signatures': [],
            'error': 'missing or invalid verifier proof/finality/settlement bundle',
            'error_code': 'X3_PROOF_REQUIRED',
        }
        return tx

    tx['proof_bundle'] = proof_bundle
    return tx
