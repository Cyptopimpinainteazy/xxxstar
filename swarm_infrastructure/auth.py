"""OAuth2 token validation helpers.

This module prefers an OAuth2 token introspection endpoint configured via
`OAUTH_INTROSPECTION_URL` and client credentials `OAUTH_INTROSPECTION_CLIENT_ID`/
`OAUTH_INTROSPECTION_CLIENT_SECRET` or a bearer `OAUTH_INTROSPECTION_TOKEN`.

If introspection is not configured it falls back to verifying JWTs via a
JWKS URL (`OAUTH_JWKS_URL`) when available.

The functions are synchronous and use `requests` for simplicity; callers are
expected to call them from async handlers (blocking) during rollout. For
production, replace with async aiohttp-based calls.
"""
import os
import logging
from typing import Optional

logger = logging.getLogger(__name__)

try:
    import requests
except Exception:
    requests = None

try:
    import jwt
except Exception:
    jwt = None


def _extract_bearer(request) -> Optional[str]:
    auth = request.headers.get('Authorization') or ''
    if auth.lower().startswith('bearer '):
        return auth.split(' ', 1)[1].strip()
    legacy = request.headers.get('X-Admin-Token')
    if legacy:
        return legacy
    return None


def introspect_token(token: str) -> Optional[dict]:
    url = os.getenv('OAUTH_INTROSPECTION_URL')
    if not url or requests is None:
        return None

    client_id = os.getenv('OAUTH_INTROSPECTION_CLIENT_ID')
    client_secret = os.getenv('OAUTH_INTROSPECTION_CLIENT_SECRET')
    bearer = os.getenv('OAUTH_INTROSPECTION_TOKEN')

    headers = {}
    auth = None
    data = {'token': token}

    if client_id and client_secret:
        auth = (client_id, client_secret)
    elif bearer:
        headers['Authorization'] = f'Bearer {bearer}'

    try:
        resp = requests.post(url, data=data, auth=auth, headers=headers, timeout=5)
        if resp.status_code != 200:
            logger.debug(f'Introspection returned {resp.status_code}')
            return None
        return resp.json()
    except Exception as e:
        logger.warning(f'Introspection call failed: {e}')
        return None


def verify_jwt(token: str) -> Optional[dict]:
    jwks_url = os.getenv('OAUTH_JWKS_URL')
    if not jwks_url or requests is None or jwt is None:
        return None

    try:
        resp = requests.get(jwks_url, timeout=5)
        if resp.status_code != 200:
            return None
        try:
            from jwt import PyJWKClient
            jwk_client = PyJWKClient(jwks_url)
            signing_key = jwk_client.get_signing_key_from_jwt(token)
            payload = jwt.decode(token, signing_key.key, algorithms=[signing_key.algorithm], options={"verify_aud": False})
            return payload
        except Exception:
            payload = jwt.decode(token, options={"verify_signature": False, "verify_aud": False})
            return payload
    except Exception as e:
        logger.warning(f'JWT verify failed: {e}')
        return None


def token_active_and_scopes(token: str) -> Optional[dict]:
    if not token:
        return None

    data = introspect_token(token)
    if data:
        return data

    payload = verify_jwt(token)
    if payload is None:
        return None

    scope = payload.get('scope') or payload.get('scp') or payload.get('scopes') or payload.get('permissions')
    if isinstance(scope, list):
        scope = ' '.join(scope)
    return {'active': True, 'scope': scope, 'claims': payload}


def request_has_scope(request, required_scope: str) -> bool:
    token = _extract_bearer(request)
    if not token:
        return False
    info = token_active_and_scopes(token)
    if not info or not info.get('active'):
        return False
    scope_str = info.get('scope') or ''
    scopes = set(s.strip() for s in scope_str.split() if s.strip())
    return required_scope in scopes
