#!/usr/bin/env python3
"""
TPS Bridge - Connect Go TPS Tester to Inferstructor GPU Lanes

Bridges the existing Blockchain-TPS-Test-GO tool with GPU acceleration lanes.
Forwards transaction load through toll booth to appropriate lanes.
"""

import asyncio
import json
import logging
import sys
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional

import aiohttp
from aiohttp import web
import aiohttp_cors
from prometheus_client import Counter, Histogram, start_http_server

# Import validator registry for API key validation
sys.path.insert(0, str(Path(__file__).parent))
from validator_registry import ValidatorRegistry


# Metrics
transactions_received = Counter('tps_bridge_transactions_received', 'Total transactions received from Go tester')
transactions_forwarded = Counter('tps_bridge_transactions_forwarded', 'Transactions forwarded to lanes', ['lane_id'])
acceleration_latency = Histogram('tps_bridge_acceleration_latency_seconds', 'Acceleration request latency')
transactions_failed = Counter('tps_bridge_transactions_failed', 'Failed transactions', ['reason'])


@dataclass
class BridgeConfig:
    """Configuration for TPS bridge"""
    listen_port: int = 9999
    
    # REAL GPU LANES (localhost with failover)
    primary_lane_endpoint: str = "http://localhost:9001"    # GPU 0: GTX 1070
    shadow_lane_endpoint: str = "http://localhost:9002"     # GPU 1: GTX 1070
    tertiary_lane_endpoint: str = "http://localhost:9003"   # GPU 2: GTX 1070
    
    # Performance settings
    max_concurrent_requests: int = 100000
    request_timeout_ms: int = 30000
    batch_size: int = 10000
    
    # SLA tier for this test
    validator_id: str = "inferstructor_test_validator"
    sla_tier: str = "enterprise"
    api_key: str = "test_api_key_12345"


@dataclass
class AccelerationRequest:
    """Single transaction acceleration request"""
    tx_hash: str
    tx_data: bytes
    timestamp: float = field(default_factory=time.time)
    chain: str = "generic"


@dataclass
class AccelerationResponse:
    """Response from GPU lane"""
    tx_hash: str
    result: bytes
    result_hash: str
    lane_id: str
    latency_ms: float
    success: bool
    error: Optional[str] = None


class TPSBridge:
    def __init__(self, config: BridgeConfig, registry: Optional[ValidatorRegistry] = None):
        self.config = config
        self.logger = logging.getLogger("TPSBridge")
        
        # Validator registry for API key auth
        self.registry = registry or ValidatorRegistry()
        
        self.request_semaphore = asyncio.Semaphore(config.max_concurrent_requests)
        self.session: Optional[aiohttp.ClientSession] = None
        
        # Load balancing counter (round-robin across GPUs)
        self.lane_roundrobin_counter = 0
        self.lane_lock = asyncio.Lock()
        
        # All lane endpoints for batch distribution
        self.all_lane_endpoints = [
            ("primary", config.primary_lane_endpoint),
            ("shadow", config.shadow_lane_endpoint),
            ("tertiary", config.tertiary_lane_endpoint),
        ]
        
        # Stats
        self.stats = {
            "total_received": 0,
            "total_forwarded": 0,
            "total_failed": 0,
            "start_time": time.time(),
        }
    
    async def start(self):
        """Start the bridge server"""
        self.logger.info(f"Starting TPS Bridge on port {self.config.listen_port}")
        
        # Start metrics server
        start_http_server(8002)
        
        # Create HTTP session
        self.session = aiohttp.ClientSession(
            timeout=aiohttp.ClientTimeout(total=self.config.request_timeout_ms / 1000.0)
        )
        
        # Setup HTTP server for Go TPS tester to send transactions
        app = web.Application(client_max_size=100 * 1024 * 1024)  # 100MB for huge batches
        
        # Configure CORS
        cors = aiohttp_cors.setup(app, defaults={
            "*": aiohttp_cors.ResourceOptions(
                allow_credentials=True,
                expose_headers="*",
                allow_headers="*",
                allow_methods="*"
            )
        })
        
        # Add routes with CORS
        cors.add(app.router.add_post('/accelerate', self.handle_acceleration_request))
        cors.add(app.router.add_post('/accelerate/batch', self.handle_batch_request))
        cors.add(app.router.add_post('/accelerate/gpu-batch', self.handle_gpu_batch_request))
        cors.add(app.router.add_get('/stats', self.handle_stats_request))
        cors.add(app.router.add_get('/health', self.handle_health_request))
        
        runner = web.AppRunner(app)
        await runner.setup()
        
        site = web.TCPSite(runner, '0.0.0.0', self.config.listen_port)
        await site.start()
        
        self.logger.info(f"TPS Bridge running on http://0.0.0.0:{self.config.listen_port}")
        self.logger.info("Ready to receive transactions from Go TPS tester")
        
        # Keep running
        await asyncio.Event().wait()
    
    async def handle_acceleration_request(self, request: web.Request) -> web.Response:
        """Handle single transaction acceleration request"""
        try:
            # Validate API key
            api_key = request.headers.get('X-API-Key')
            if not api_key:
                transactions_failed.labels(reason='missing_api_key').inc()
                return web.json_response(
                    {"error": "Missing X-API-Key header"},
                    status=401
                )
            
            validator = self.registry.get_validator_by_api_key(api_key)
            if not validator or not validator.enabled:
                transactions_failed.labels(reason='invalid_api_key').inc()
                return web.json_response(
                    {"error": "Invalid or disabled API key"},
                    status=401
                )
            
            data = await request.json()
            
            tx_hash = data.get('tx_hash')
            tx_data = bytes.fromhex(data.get('tx_data', ''))
            chain = data.get('chain', 'generic')
            
            if not tx_hash or not tx_data:
                transactions_failed.labels(reason='invalid_request').inc()
                return web.json_response(
                    {"error": "Missing tx_hash or tx_data"},
                    status=400
                )
            
            transactions_received.inc()
            self.stats["total_received"] += 1
            
            # Accelerate through GPU lane
            response = await self.accelerate_transaction(
                AccelerationRequest(
                    tx_hash=tx_hash,
                    tx_data=tx_data,
                    chain=chain
                )
            )
            
            if response.success:
                transactions_forwarded.labels(lane_id=response.lane_id).inc()
                self.stats["total_forwarded"] += 1
                
                # Record usage
                self.registry.record_usage(api_key, requests=1, tx_count=1)
                
                return web.json_response({
                    "success": True,
                    "tx_hash": response.tx_hash,
                    "result": response.result.hex(),
                    "result_hash": response.result_hash,
                    "lane_id": response.lane_id,
                    "latency_ms": response.latency_ms,
                    "validator_id": validator.validator_id,
                })
            else:
                transactions_failed.labels(reason='acceleration_failed').inc()
                self.stats["total_failed"] += 1
                
                return web.json_response({
                    "success": False,
                    "error": response.error,
                }, status=500)
                
        except Exception as e:
            self.logger.error(f"Error handling request: {e}")
            transactions_failed.labels(reason='exception').inc()
            return web.json_response({"error": str(e)}, status=500)
    
    async def handle_batch_request(self, request: web.Request) -> web.Response:
        """Handle batch acceleration request"""
        try:
            # Validate API key
            api_key = request.headers.get('X-API-Key')
            if not api_key:
                transactions_failed.labels(reason='missing_api_key').inc()
                return web.json_response(
                    {"error": "Missing X-API-Key header"},
                    status=401
                )
            
            validator = self.registry.get_validator_by_api_key(api_key)
            if not validator or not validator.enabled:
                transactions_failed.labels(reason='invalid_api_key').inc()
                return web.json_response(
                    {"error": "Invalid or disabled API key"},
                    status=401
                )
            
            data = await request.json()
            transactions = data.get('transactions', [])
            
            if not transactions:
                return web.json_response({"error": "Empty batch"}, status=400)
            
            # Convert to AccelerationRequest objects
            requests = [
                AccelerationRequest(
                    tx_hash=tx['tx_hash'],
                    tx_data=bytes.fromhex(tx['tx_data']),
                    chain=tx.get('chain', 'generic')
                )
                for tx in transactions
            ]
            
            transactions_received.inc(len(requests))
            self.stats["total_received"] += len(requests)
            
            # Accelerate in parallel
            tasks = [self.accelerate_transaction(req) for req in requests]
            responses = await asyncio.gather(*tasks, return_exceptions=True)
            
            # Build response
            results = []
            for resp in responses:
                if isinstance(resp, AccelerationResponse) and resp.success:
                    transactions_forwarded.labels(lane_id=resp.lane_id).inc()
                    self.stats["total_forwarded"] += 1
                    results.append({
                        "success": True,
                        "tx_hash": resp.tx_hash,
                        "result_hash": resp.result_hash,
                        "lane_id": resp.lane_id,
                        "latency_ms": resp.latency_ms,
                    })
                else:
                    transactions_failed.labels(reason='acceleration_failed').inc()
                    self.stats["total_failed"] += 1
                    results.append({
                        "success": False,
                        "error": str(resp) if isinstance(resp, Exception) else resp.error
                    })
            
            # Record batch usage
            success_count = sum(1 for r in results if r.get('success'))
            self.registry.record_usage(api_key, requests=1, tx_count=success_count)
            
            return web.json_response({
                "results": results,
                "validator_id": validator.validator_id,
                "total": len(results),
                "successful": success_count,
            })
            
        except Exception as e:
            self.logger.error(f"Error handling batch: {e}")
            return web.json_response({"error": str(e)}, status=500)
    
    async def accelerate_transaction(self, req: AccelerationRequest) -> AccelerationResponse:
        """Accelerate a single transaction through GPU lanes with ROUND-ROBIN LOAD BALANCING + failover"""
        start_time = time.time()
        
        async with self.request_semaphore:
            # Define all lanes
            all_lanes = [
                ("primary", self.config.primary_lane_endpoint),
                ("shadow", self.config.shadow_lane_endpoint),
                ("tertiary", self.config.tertiary_lane_endpoint)
            ]
            
            # ROUND-ROBIN: Pick starting lane based on counter, then try others as failover
            async with self.lane_lock:
                start_idx = self.lane_roundrobin_counter % len(all_lanes)
                self.lane_roundrobin_counter += 1
            
            # Reorder lanes to start with round-robin pick, then failover to others
            endpoints = all_lanes[start_idx:] + all_lanes[:start_idx]
            
            last_error = None
            
            for lane_name, endpoint in endpoints:
                try:
                    # Forward to GPU lane
                    async with self.session.post(
                        f"{endpoint}/accelerate",
                        json={
                            "validator_id": req.validator_id if hasattr(req, 'validator_id') else "bridge",
                            "sla_tier": "enterprise",
                            "tx_hash": req.tx_hash,
                            "tx_data": req.tx_data.hex(),
                            "chain": req.chain,
                        },
                        timeout=aiohttp.ClientTimeout(total=self.config.request_timeout_ms / 1000.0)
                    ) as resp:
                        latency = (time.time() - start_time) * 1000  # ms
                        acceleration_latency.observe(latency / 1000.0)
                        
                        if resp.status != 200:
                            last_error = f"HTTP {resp.status} from {lane_name}"
                            self.logger.warning(f"⚠️  {lane_name} lane returned {resp.status}, trying next...")
                            continue
                        
                        data = await resp.json()
                        
                        if not data.get('success', True):
                            last_error = data.get('error', 'Unknown error')
                            self.logger.warning(f"⚠️  {lane_name} lane failed: {last_error}, trying next...")
                            continue
                        
                        # Success!
                        self.logger.info(f"✅ Accelerated via {lane_name} lane in {latency:.2f}ms")
                        
                        return AccelerationResponse(
                            tx_hash=req.tx_hash,
                            result=bytes.fromhex(data.get('result', '')),
                            result_hash=data.get('result_hash', ''),
                            lane_id=data.get('lane_id', lane_name),
                            latency_ms=latency,
                            success=True
                        )
                        
                except asyncio.TimeoutError:
                    last_error = "Timeout"
                    self.logger.warning(f"⚠️  {lane_name} lane timeout, trying next...")
                    continue
                except Exception as e:
                    last_error = str(e)
                    self.logger.warning(f"⚠️  {lane_name} lane error: {e}, trying next...")
                    continue
            
            # All lanes failed
            latency = (time.time() - start_time) * 1000
            self.logger.error(f"❌ All GPU lanes failed: {last_error}")
            
            return AccelerationResponse(
                tx_hash=req.tx_hash,
                result=b'',
                result_hash='',
                lane_id='all_failed',
                latency_ms=latency,
                success=False,
                error=f"All lanes failed: {last_error}"
            )
    
    async def handle_gpu_batch_request(self, request: web.Request) -> web.Response:
        """Handle massive batch: splits across all GPU lanes for max throughput"""
        try:
            api_key = request.headers.get('X-API-Key')
            if not api_key:
                return web.json_response({"error": "Missing X-API-Key header"}, status=401)
            
            validator = self.registry.get_validator_by_api_key(api_key)
            if not validator or not validator.enabled:
                return web.json_response({"error": "Invalid or disabled API key"}, status=401)
            
            data = await request.json()
            transactions = data.get('transactions', [])
            chain = data.get('chain', 'generic')
            
            if not transactions:
                return web.json_response({"error": "Empty batch"}, status=400)
            
            total = len(transactions)
            self.stats["total_received"] += total
            transactions_received.inc(total)
            
            # Split transactions across GPU lanes (round-robin chunks)
            num_lanes = len(self.all_lane_endpoints)
            chunk_size = (total + num_lanes - 1) // num_lanes
            
            tasks = []
            for i, (lane_name, endpoint) in enumerate(self.all_lane_endpoints):
                chunk = transactions[i * chunk_size:(i + 1) * chunk_size]
                if chunk:
                    tasks.append(self._send_batch_to_lane(lane_name, endpoint, chunk, chain))
            
            # Execute all GPU batches in parallel
            lane_results = await asyncio.gather(*tasks, return_exceptions=True)
            
            total_success = 0
            total_failed = 0
            lane_stats = {}
            
            for lr in lane_results:
                if isinstance(lr, Exception):
                    total_failed += chunk_size
                    continue
                total_success += lr.get('successful', 0)
                lane_batch_size = lr.get('batch_size', lr.get('successful', 0))
                total_failed += lane_batch_size - lr.get('successful', 0)
                lane_id = lr.get('lane_id', 'unknown')
                lane_stats[lane_id] = lr.get('successful', 0)
            
            self.stats["total_forwarded"] += total_success
            self.stats["total_failed"] += total_failed
            
            self.registry.record_usage(api_key, requests=1, tx_count=total_success)
            
            return web.json_response({
                "success": True,
                "total": total,
                "successful": total_success,
                "failed": total_failed,
                "validator_id": validator.validator_id,
                "lane_distribution": lane_stats,
            })
            
        except Exception as e:
            self.logger.error(f"GPU batch error: {e}")
            return web.json_response({"error": str(e)}, status=500)
    
    async def _send_batch_to_lane(self, lane_name: str, endpoint: str, transactions: list, chain: str) -> dict:
        """Send a batch of transactions to a specific GPU lane"""
        try:
            async with self.session.post(
                f"{endpoint}/accelerate/batch",
                json={"transactions": transactions, "chain": chain},
                timeout=aiohttp.ClientTimeout(total=self.config.request_timeout_ms / 1000.0)
            ) as resp:
                if resp.status != 200:
                    return {"successful": 0, "batch_size": len(transactions), "lane_id": lane_name, "results": []}
                return await resp.json()
        except Exception as e:
            self.logger.warning(f"{lane_name} batch failed: {e}")
            return {"successful": 0, "batch_size": len(transactions), "lane_id": lane_name, "results": []}
    
    async def handle_stats_request(self, request: web.Request) -> web.Response:
        """Return current stats"""
        elapsed = time.time() - self.stats["start_time"]
        tps = self.stats["total_forwarded"] / elapsed if elapsed > 0 else 0
        
        return web.json_response({
            "total_received": self.stats["total_received"],
            "total_forwarded": self.stats["total_forwarded"],
            "total_failed": self.stats["total_failed"],
            "uptime_seconds": elapsed,
            "current_tps": tps,
        })
    
    async def handle_health_request(self, request: web.Request) -> web.Response:
        """Health check endpoint"""
        return web.json_response({"status": "healthy", "service": "tps-bridge"})

async def main():
    logging.basicConfig(
        level=logging.INFO,
        format='%(asctime)s [%(levelname)s] %(name)s: %(message)s'
    )
    
    # Load validator registry for API key validation
    registry = ValidatorRegistry()
    
    config = BridgeConfig()
    bridge = TPSBridge(config, registry=registry)
    
    try:
        await bridge.start()
    except KeyboardInterrupt:
        logging.info("Shutting down TPS Bridge")


if __name__ == "__main__":
    asyncio.run(main())
