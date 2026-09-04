#!/usr/bin/env python3
"""
Mock Toll Booth GPU Acceleration Backend
Simulates GPU transaction acceleration for demo purposes
"""

import logging
from aiohttp import web
import hashlib
import time

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("MockTollBooth")

async def handle_accelerate(request: web.Request) -> web.Response:
    """Simulate GPU acceleration"""
    try:
        data = await request.json()
        tx_hash = data.get('tx_hash')
        tx_data = data.get('tx_data', '')
        chain = data.get('chain', 'unknown')
        validator_id = data.get('validator_id', 'unknown')
        
        logger.info(f"🚀 Accelerating {chain} tx {tx_hash} for validator {validator_id}")
        
        # Simulate GPU processing
        result = hashlib.sha256(f"{tx_data}_accelerated".encode()).hexdigest()
        result_hash = hashlib.sha256(result.encode()).hexdigest()
        
        return web.json_response({
            "success": True,
            "tx_hash": tx_hash,
            "result": result,
            "result_hash": result_hash,
            "lane_id": "gpu_lane_1",
            "chain": chain,
            "processed_at": time.time()
        })
        
    except Exception as e:
        logger.error(f"Error: {e}")
        return web.json_response({"error": str(e)}, status=500)

async def handle_health(request: web.Request) -> web.Response:
    """Health check"""
    return web.json_response({"status": "healthy", "service": "mock-toll-booth"})

async def main():
    app = web.Application()
    app.router.add_post('/accelerate', handle_accelerate)
    app.router.add_get('/health', handle_health)
    
    runner = web.AppRunner(app)
    await runner.setup()
    site = web.TCPSite(runner, '0.0.0.0', 7000)
    await site.start()
    
    logger.info("🎯 Mock Toll Booth running on http://0.0.0.0:7000")
    logger.info("Ready to accelerate transactions!")
    
    # Keep running
    await asyncio.Event().wait()

if __name__ == '__main__':
    import asyncio
    asyncio.run(main())
