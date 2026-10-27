#!/usr/bin/env python3
"""
Validator Registration & API Key Management System

Allows external validators to:
1. Register their chain/validator
2. Get API keys for Inferstructor access
3. Choose SLA tier (Basic/Pro/Enterprise)
4. Test acceleration immediately

Integrated with existing JWT authentication from docs/runbooks/getting-started/AUTHENTICATION_SETUP.md
"""

import hashlib
import json
import logging
import os
import secrets
import time
from dataclasses import dataclass, field
from datetime import datetime, timedelta
from enum import Enum
from typing import Dict, List, Optional

import jwt
from aiohttp import web
import aiohttp_cors


# From docs/runbooks/getting-started/AUTHENTICATION_SETUP.md
AUTH_SALT = "x3-chain-inferstructor-2026"  # Change in production!
JWT_SECRET = "inferstructor-jwt-secret-change-me"  # Must match .env


class SLATier(Enum):
    BASIC = "basic"
    PRO = "pro"
    ENTERPRISE = "enterprise"


@dataclass
class ValidatorCredentials:
    """Validator registration info"""
    validator_id: str
    chain: str  # "solana", "ethereum", "arbitrum", etc.
    api_key: str
    api_secret: str  # Hashed
    sla_tier: SLATier
    email: str
    created_at: float = field(default_factory=time.time)
    max_tps: int = 100_000
    enabled: bool = True
    
    # Usage tracking
    total_requests: int = 0
    total_tx: int = 0
    last_used: Optional[float] = None


class ValidatorRegistry:
    """Manages validator registrations and API keys"""
    
    def __init__(self, db_path: str = "validators.json"):
        # Resolve relative paths against the script's directory
        if not os.path.isabs(db_path):
            script_dir = os.path.dirname(os.path.abspath(__file__))
            db_path = os.path.join(script_dir, db_path)
        self.db_path = db_path
        self.validators: Dict[str, ValidatorCredentials] = {}
        self.api_key_index: Dict[str, str] = {}  # api_key -> validator_id
        self.logger = logging.getLogger("ValidatorRegistry")
        
        self._load_db()
    
    def _load_db(self):
        """Load validators from JSON file"""
        try:
            with open(self.db_path) as f:
                data = json.load(f)
                for vid, vdata in data.items():
                    # Convert sla_tier string to enum
                    vdata['sla_tier'] = SLATier(vdata['sla_tier'])
                    self.validators[vid] = ValidatorCredentials(**vdata)
                    self.api_key_index[vdata['api_key']] = vid
            self.logger.info(f"Loaded {len(self.validators)} validators")
        except FileNotFoundError:
            self.logger.info("No existing validator DB, starting fresh")
        except Exception as e:
            self.logger.error(f"Error loading DB: {e}")
    
    def _save_db(self):
        """Save validators to JSON file"""
        try:
            data = {
                vid: {
                    'validator_id': v.validator_id,
                    'chain': v.chain,
                    'api_key': v.api_key,
                    'api_secret': v.api_secret,
                    'sla_tier': v.sla_tier.value,
                    'email': v.email,
                    'created_at': v.created_at,
                    'max_tps': v.max_tps,
                    'enabled': v.enabled,
                    'total_requests': v.total_requests,
                    'total_tx': v.total_tx,
                    'last_used': v.last_used,
                }
                for vid, v in self.validators.items()
            }
            with open(self.db_path, 'w') as f:
                json.dump(data, f, indent=2)
        except Exception as e:
            self.logger.error(f"Error saving DB: {e}")
    
    def register_validator(
        self, 
        chain: str, 
        email: str, 
        sla_tier: str = "basic"
    ) -> Dict:
        """Register new validator and generate API credentials"""
        
        # Generate unique validator ID
        validator_id = f"{chain}_{secrets.token_hex(8)}"
        
        # Generate API key and secret
        api_key = f"infra_{secrets.token_urlsafe(32)}"
        api_secret = secrets.token_urlsafe(48)
        api_secret_hash = self._hash_secret(api_secret)
        
        # Determine max TPS based on SLA tier
        tier = SLATier(sla_tier)
        max_tps_map = {
            SLATier.BASIC: 100_000,
            SLATier.PRO: 1_000_000,
            SLATier.ENTERPRISE: 999_999_999,  # Unlimited
        }
        
        credentials = ValidatorCredentials(
            validator_id=validator_id,
            chain=chain,
            api_key=api_key,
            api_secret=api_secret_hash,
            sla_tier=tier,
            email=email,
            max_tps=max_tps_map[tier],
        )
        
        self.validators[validator_id] = credentials
        self.api_key_index[api_key] = validator_id
        self._save_db()
        
        self.logger.info(f"Registered validator: {validator_id} ({chain}, {sla_tier})")
        
        return {
            "validator_id": validator_id,
            "chain": chain,
            "api_key": api_key,
            "api_secret": api_secret,  # Only returned once!
            "sla_tier": sla_tier,
            "max_tps": max_tps_map[tier],
            "bridge_endpoint": "http://localhost:9999",
            "toll_booth_endpoint": "http://localhost:7000",
        }
    
    def validate_api_key(self, api_key: str, api_secret: str) -> Optional[ValidatorCredentials]:
        """Validate API key and secret"""
        validator_id = self.api_key_index.get(api_key)
        if not validator_id:
            return None
        
        validator = self.validators.get(validator_id)
        if not validator or not validator.enabled:
            return None
        
        # Verify secret
        secret_hash = self._hash_secret(api_secret)
        if secret_hash != validator.api_secret:
            return None
        
        # Update last used
        validator.last_used = time.time()
        return validator
    
    def get_validator_by_api_key(self, api_key: str) -> Optional[ValidatorCredentials]:
        """Get validator by API key (without secret validation)"""
        validator_id = self.api_key_index.get(api_key)
        if not validator_id:
            return None
        return self.validators.get(validator_id)
    
    def record_usage(self, api_key: str, requests: int = 1, tx_count: int = 0):
        """Record validator usage"""
        validator = self.get_validator_by_api_key(api_key)
        if validator:
            validator.total_requests += requests
            validator.total_tx += tx_count
            validator.last_used = time.time()
            self._save_db()
    
    def _hash_secret(self, secret: str) -> str:
        """Hash API secret with salt"""
        return hashlib.sha256(f"{secret}{AUTH_SALT}".encode()).hexdigest()
    
    def generate_jwt_token(self, validator: ValidatorCredentials) -> str:
        """Generate JWT token for validator"""
        payload = {
            "validator_id": validator.validator_id,
            "chain": validator.chain,
            "sla_tier": validator.sla_tier.value,
            "exp": datetime.utcnow() + timedelta(hours=24),
            "iat": datetime.utcnow(),
        }
        return jwt.encode(payload, JWT_SECRET, algorithm="HS256")
    
    def validate_jwt_token(self, token: str) -> Optional[Dict]:
        """Validate JWT token"""
        try:
            payload = jwt.decode(token, JWT_SECRET, algorithms=["HS256"])
            return payload
        except jwt.ExpiredSignatureError:
            self.logger.warning("Expired JWT token")
            return None
        except jwt.InvalidTokenError as e:
            self.logger.warning(f"Invalid JWT token: {e}")
            return None


class ValidatorRegistrationAPI:
    """HTTP API for validator registration"""
    
    def __init__(self, registry: ValidatorRegistry):
        self.registry = registry
        self.logger = logging.getLogger("ValidatorAPI")
    
    async def handle_register(self, request: web.Request) -> web.Response:
        """POST /api/validators/register"""
        try:
            data = await request.json()
            
            chain = data.get('chain')
            email = data.get('email')
            sla_tier = data.get('sla_tier', 'basic')
            
            if not chain or not email:
                return web.json_response(
                    {"error": "Missing required fields: chain, email"},
                    status=400
                )
            
            # Register validator
            credentials = self.registry.register_validator(chain, email, sla_tier)
            
            # Generate JWT token
            validator = self.registry.get_validator_by_api_key(credentials['api_key'])
            jwt_token = self.registry.generate_jwt_token(validator)
            credentials['jwt_token'] = jwt_token
            
            return web.json_response({
                "success": True,
                "message": "Validator registered successfully!",
                "credentials": credentials,
                "next_steps": [
                    "Save your API key and secret securely",
                    "Test connection: curl -H 'X-API-Key: <api_key>' http://localhost:9999/health",
                    "Start testing: see docs/VALIDATOR_QUICKSTART.md",
                ]
            }, status=201)
            
        except Exception as e:
            self.logger.error(f"Registration error: {e}")
            return web.json_response(
                {"error": str(e)},
                status=500
            )
    
    async def handle_login(self, request: web.Request) -> web.Response:
        """POST /api/validators/login (authenticate with API key/secret)"""
        try:
            data = await request.json()
            
            api_key = data.get('api_key')
            api_secret = data.get('api_secret')
            
            if not api_key or not api_secret:
                return web.json_response(
                    {"error": "Missing api_key or api_secret"},
                    status=400
                )
            
            # Validate credentials
            validator = self.registry.validate_api_key(api_key, api_secret)
            if not validator:
                return web.json_response(
                    {"error": "Invalid credentials"},
                    status=401
                )
            
            # Generate JWT token
            jwt_token = self.registry.generate_jwt_token(validator)
            
            return web.json_response({
                "success": True,
                "token": jwt_token,
                "validator": {
                    "id": validator.validator_id,
                    "chain": validator.chain,
                    "sla_tier": validator.sla_tier.value,
                    "max_tps": validator.max_tps,
                },
                "endpoints": {
                    "bridge": "http://localhost:9999",
                    "toll_booth": "http://localhost:7000",
                    "dashboard": "http://localhost:8080",
                }
            })
            
        except Exception as e:
            self.logger.error(f"Login error: {e}")
            return web.json_response({"error": str(e)}, status=500)
    
    async def handle_validate_token(self, request: web.Request) -> web.Response:
        """GET /api/validators/validate (validate JWT token)"""
        auth_header = request.headers.get('Authorization', '')
        
        if not auth_header.startswith('Bearer '):
            return web.json_response({"error": "Missing Bearer token"}, status=401)
        
        token = auth_header[7:]  # Remove "Bearer "
        
        payload = self.registry.validate_jwt_token(token)
        if not payload:
            return web.json_response({"error": "Invalid or expired token"}, status=401)
        
        return web.json_response({
            "valid": True,
            "validator_id": payload['validator_id'],
            "chain": payload['chain'],
            "sla_tier": payload['sla_tier'],
        })
    
    async def handle_get_stats(self, request: web.Request) -> web.Response:
        """GET /api/validators/stats (get validator usage stats)"""
        auth_header = request.headers.get('Authorization', '')
        
        if not auth_header.startswith('Bearer '):
            return web.json_response({"error": "Missing Bearer token"}, status=401)
        
        token = auth_header[7:]
        payload = self.registry.validate_jwt_token(token)
        
        if not payload:
            return web.json_response({"error": "Invalid token"}, status=401)
        
        validator = self.registry.validators.get(payload['validator_id'])
        if not validator:
            return web.json_response({"error": "Validator not found"}, status=404)
        
        return web.json_response({
            "validator_id": validator.validator_id,
            "chain": validator.chain,
            "sla_tier": validator.sla_tier.value,
            "max_tps": validator.max_tps,
            "usage": {
                "total_requests": validator.total_requests,
                "total_tx": validator.total_tx,
                "last_used": validator.last_used,
            },
            "status": "enabled" if validator.enabled else "disabled",
        })
    
    async def handle_list_validators(self, request: web.Request) -> web.Response:
        """GET /api/validators/list (admin only)"""
        # In production, check admin JWT token
        
        validators_list = [
            {
                "validator_id": v.validator_id,
                "chain": v.chain,
                "sla_tier": v.sla_tier.value,
                "created_at": v.created_at,
                "total_requests": v.total_requests,
                "total_tx": v.total_tx,
                "enabled": v.enabled,
            }
            for v in self.registry.validators.values()
        ]
        
        return web.json_response({
            "total": len(validators_list),
            "validators": validators_list,
        })


async def start_registration_service(port: int = 7001):
    """Start validator registration API service"""
    logging.basicConfig(
        level=logging.INFO,
        format='%(asctime)s [%(levelname)s] %(name)s: %(message)s'
    )
    
    registry = ValidatorRegistry()
    api = ValidatorRegistrationAPI(registry)
    
    app = web.Application()
    
    # Configure CORS
    cors = aiohttp_cors.setup(app, defaults={
        "*": aiohttp_cors.ResourceOptions(
            allow_credentials=True,
            expose_headers="*",
            allow_headers="*",
            allow_methods="*"
        )
    })
    
    # Registration & auth endpoints
    cors.add(app.router.add_post('/api/validators/register', api.handle_register))
    cors.add(app.router.add_post('/api/validators/login', api.handle_login))
    cors.add(app.router.add_get('/api/validators/validate', api.handle_validate_token))
    cors.add(app.router.add_get('/api/validators/stats', api.handle_get_stats))
    cors.add(app.router.add_get('/api/validators/list', api.handle_list_validators))
    
    # Health check
    async def health(request):
        return web.json_response({"status": "healthy", "service": "validator-registry"})
    
    cors.add(app.router.add_get('/health', health))
    
    runner = web.AppRunner(app)
    await runner.setup()
    
    site = web.TCPSite(runner, '0.0.0.0', port)
    await site.start()
    
    logging.info(f"🚀 Validator Registration API running on http://0.0.0.0:{port}")
    logging.info(f"📝 Register: POST http://localhost:{port}/api/validators/register")
    logging.info(f"🔐 Login: POST http://localhost:{port}/api/validators/login")
    
    # Keep running
    import asyncio
    await asyncio.Event().wait()


if __name__ == "__main__":
    import asyncio
    asyncio.run(start_registration_service())
