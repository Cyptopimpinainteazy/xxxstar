#!/usr/bin/env python3
"""
Metrics Dashboard - Real-time monitoring for Inferstructor 300× Test

Collects metrics from:
- Lane orchestrator
- TPS bridge
- GPU lanes (primary, shadow, tertiary)
- Toll booth
- Go TPS tester

Provides:
- Real-time TPS monitoring
- Failover event tracking
- Lane health visualization
- Latency distribution
- Success rate metrics
"""

import asyncio
import json
import logging
import time
from collections import deque
from dataclasses import dataclass, field
from datetime import datetime
from typing import Deque, Dict, List

import aiohttp
from aiohttp import web
from prometheus_client import start_http_server


@dataclass
class MetricSnapshot:
    """Point-in-time metrics snapshot"""
    timestamp: float
    tps: float
    total_tx: int
    latency_p50: float
    latency_p95: float
    latency_p99: float
    success_rate: float
    active_lane: str
    failover_count: int
    gpu_utilization: Dict[str, float] = field(default_factory=dict)


class MetricsCollector:
    def __init__(self, config_path: str = "configs/metrics_dashboard.yaml"):
        self.logger = logging.getLogger("MetricsCollector")
        
        # Endpoints to scrape
        self.endpoints = {
            "orchestrator": "http://localhost:8000/metrics",
            "tps_bridge": "http://localhost:8002/metrics",
            "primary_lane": "http://10.0.1.10:9091/metrics",
            "shadow_lane": "http://10.0.2.10:9092/metrics",
            "tertiary_lane": "http://10.1.1.10:9093/metrics",
            "toll_booth": "http://toll-booth:7001/metrics",
        }
        
        # Rolling window of metrics (last 1000 samples)
        self.history: Deque[MetricSnapshot] = deque(maxlen=1000)
        
        # Failover events
        self.failover_events: List[dict] = []
        
        # Current aggregated metrics
        self.current_metrics = {
            "tps": 0.0,
            "total_tx": 0,
            "success_rate": 100.0,
            "active_lane": "primary",
            "failover_count": 0,
            "uptime_seconds": 0.0,
        }
        
        self.start_time = time.time()
    
    async def start(self):
        """Start metrics collection"""
        self.logger.info("Starting Metrics Dashboard")
        
        # Start HTTP API for dashboard
        app = web.Application()
        app.router.add_get('/api/current', self.handle_current_metrics)
        app.router.add_get('/api/history', self.handle_history)
        app.router.add_get('/api/failovers', self.handle_failover_events)
        app.router.add_get('/api/lanes', self.handle_lane_status)
        app.router.add_get('/', self.handle_dashboard_ui)
        
        runner = web.AppRunner(app)
        await runner.setup()
        site = web.TCPSite(runner, '0.0.0.0', 8080)
        await site.start()
        
        self.logger.info("Dashboard UI: http://localhost:8080")
        
        # Start collection loop
        asyncio.create_task(self.collection_loop())
        
        await asyncio.Event().wait()
    
    async def collection_loop(self):
        """Continuously collect metrics"""
        while True:
            try:
                await self.collect_metrics()
                await asyncio.sleep(1)  # Collect every second
            except Exception as e:
                self.logger.error(f"Error collecting metrics: {e}")
                await asyncio.sleep(5)
    
    async def collect_metrics(self):
        """Collect metrics from all endpoints"""
        async with aiohttp.ClientSession() as session:
            # Collect from all endpoints in parallel
            tasks = {
                name: self.scrape_endpoint(session, url)
                for name, url in self.endpoints.items()
            }
            
            results = {}
            for name, task in tasks.items():
                try:
                    results[name] = await task
                except Exception as e:
                    self.logger.debug(f"Failed to scrape {name}: {e}")
                    results[name] = {}
            
            # Aggregate metrics
            snapshot = self.aggregate_metrics(results)
            self.history.append(snapshot)
            
            # Update current metrics
            self.current_metrics.update({
                "tps": snapshot.tps,
                "total_tx": snapshot.total_tx,
                "success_rate": snapshot.success_rate,
                "active_lane": snapshot.active_lane,
                "failover_count": snapshot.failover_count,
                "uptime_seconds": time.time() - self.start_time,
            })
            
            # Log progress every 10 seconds
            if int(time.time()) % 10 == 0:
                self.logger.info(
                    f"TPS: {snapshot.tps:.2f} | "
                    f"Total TX: {snapshot.total_tx} | "
                    f"Success: {snapshot.success_rate:.2f}% | "
                    f"Lane: {snapshot.active_lane}"
                )
    
    async def scrape_endpoint(self, session: aiohttp.ClientSession, url: str) -> dict:
        """Scrape metrics from single endpoint"""
        try:
            async with session.get(url, timeout=aiohttp.ClientTimeout(total=2)) as resp:
                if resp.status == 200:
                    # Try JSON first
                    try:
                        return await resp.json()
                    except:
                        # Fall back to Prometheus format parsing
                        text = await resp.text()
                        return self.parse_prometheus_metrics(text)
                return {}
        except:
            return {}
    
    def parse_prometheus_metrics(self, text: str) -> dict:
        """Parse Prometheus format metrics"""
        metrics = {}
        for line in text.split('\n'):
            if line.startswith('#') or not line.strip():
                continue
            parts = line.split()
            if len(parts) >= 2:
                key = parts[0]
                value = parts[1]
                try:
                    metrics[key] = float(value)
                except:
                    pass
        return metrics
    
    def aggregate_metrics(self, results: Dict[str, dict]) -> MetricSnapshot:
        """Aggregate collected metrics into snapshot"""
        
        # Extract TPS from bridge
        bridge_stats = results.get("tps_bridge", {})
        tps = bridge_stats.get("current_tps", 0.0)
        total_tx = bridge_stats.get("total_forwarded", 0)
        
        # Success rate
        total_sent = bridge_stats.get("total_received", 1)
        total_failed = bridge_stats.get("total_failed", 0)
        success_rate = ((total_sent - total_failed) / total_sent * 100) if total_sent > 0 else 100.0
        
        # Lane health
        primary_health = results.get("primary_lane", {}).get("lane_health_score", 0)
        shadow_health = results.get("shadow_lane", {}).get("lane_health_score", 0)
        
        # Determine active lane
        active_lane = "primary"
        if primary_health < 0.3 and shadow_health > 0.5:
            active_lane = "shadow"
        elif primary_health < 0.3 and shadow_health < 0.3:
            active_lane = "tertiary"
        
        # Failover count
        orchestrator_metrics = results.get("orchestrator", {})
        failover_count = int(orchestrator_metrics.get("failover_total", 0))
        
        # GPU utilization
        gpu_util = {
            "primary": results.get("primary_lane", {}).get("gpu_utilization", 0),
            "shadow": results.get("shadow_lane", {}).get("gpu_utilization", 0),
            "tertiary": results.get("tertiary_lane", {}).get("gpu_utilization", 0),
        }
        
        # Latency (placeholder - would need histogram parsing)
        latency_p50 = 0.5
        latency_p95 = 0.8
        latency_p99 = 1.0
        
        return MetricSnapshot(
            timestamp=time.time(),
            tps=tps,
            total_tx=total_tx,
            latency_p50=latency_p50,
            latency_p95=latency_p95,
            latency_p99=latency_p99,
            success_rate=success_rate,
            active_lane=active_lane,
            failover_count=failover_count,
            gpu_utilization=gpu_util,
        )
    
    async def handle_current_metrics(self, request: web.Request) -> web.Response:
        """API: Current metrics"""
        return web.json_response(self.current_metrics)
    
    async def handle_history(self, request: web.Request) -> web.Response:
        """API: Historical metrics"""
        # Get last N samples
        count = int(request.query.get('count', 100))
        history_data = [
            {
                "timestamp": s.timestamp,
                "tps": s.tps,
                "total_tx": s.total_tx,
                "success_rate": s.success_rate,
                "active_lane": s.active_lane,
            }
            for s in list(self.history)[-count:]
        ]
        return web.json_response(history_data)
    
    async def handle_failover_events(self, request: web.Request) -> web.Response:
        """API: Failover events"""
        return web.json_response(self.failover_events)
    
    async def handle_lane_status(self, request: web.Request) -> web.Response:
        """API: Lane status"""
        latest = self.history[-1] if self.history else None
        if not latest:
            return web.json_response({"error": "No data yet"}, status=503)
        
        return web.json_response({
            "primary": {
                "gpu_utilization": latest.gpu_utilization.get("primary", 0),
                "active": latest.active_lane == "primary",
            },
            "shadow": {
                "gpu_utilization": latest.gpu_utilization.get("shadow", 0),
                "active": latest.active_lane == "shadow",
            },
            "tertiary": {
                "gpu_utilization": latest.gpu_utilization.get("tertiary", 0),
                "active": latest.active_lane == "tertiary",
            },
        })
    
    async def handle_dashboard_ui(self, request: web.Request) -> web.Response:
        """Serve dashboard UI"""
        html = """
<!DOCTYPE html>
<html>
<head>
    <title>Inferstructor 300× Test Dashboard</title>
    <style>
        body { font-family: monospace; background: #000; color: #0f0; padding: 20px; }
        .metric { font-size: 24px; margin: 10px 0; }
        .label { color: #0a0; }
        .value { color: #0f0; font-weight: bold; }
        .alert { color: #f00; }
        .lane { padding: 10px; margin: 10px; border: 1px solid #0f0; display: inline-block; }
        .active { background: #050; }
    </style>
</head>
<body>
    <h1>🚀 Inferstructor 300× Solana Test</h1>
    
    <div class="metric">
        <span class="label">TPS:</span>
        <span class="value" id="tps">0</span>
    </div>
    
    <div class="metric">
        <span class="label">Total TX:</span>
        <span class="value" id="total_tx">0</span>
    </div>
    
    <div class="metric">
        <span class="label">Success Rate:</span>
        <span class="value" id="success_rate">100%</span>
    </div>
    
    <div class="metric">
        <span class="label">Uptime:</span>
        <span class="value" id="uptime">0s</span>
    </div>
    
    <h2>Lanes</h2>
    <div id="lanes">
        <div class="lane" id="lane_primary">Primary</div>
        <div class="lane" id="lane_shadow">Shadow</div>
        <div class="lane" id="lane_tertiary">Tertiary</div>
    </div>
    
    <h2>Target: 19.5M TPS (300× Solana)</h2>
    <div class="metric">
        <span class="label">Progress:</span>
        <span class="value" id="progress">0%</span>
    </div>
    
    <script>
        async function update() {
            const resp = await fetch('/api/current');
            const data = await resp.json();
            
            document.getElementById('tps').textContent = data.tps.toFixed(2);
            document.getElementById('total_tx').textContent = data.total_tx.toLocaleString();
            document.getElementById('success_rate').textContent = data.success_rate.toFixed(2) + '%';
            document.getElementById('uptime').textContent = data.uptime_seconds.toFixed(0) + 's';
            
            // Calculate progress toward 300× Solana (19.5M TPS)
            const target = 19500000;
            const progress = (data.tps / target * 100).toFixed(2);
            document.getElementById('progress').textContent = progress + '%';
            
            // Update lane status
            ['primary', 'shadow', 'tertiary'].forEach(lane => {
                const elem = document.getElementById('lane_' + lane);
                if (data.active_lane === lane) {
                    elem.className = 'lane active';
                } else {
                    elem.className = 'lane';
                }
            });
        }
        
        setInterval(update, 1000);
        update();
    </script>
</body>
</html>
        """
        return web.Response(text=html, content_type='text/html')


async def main():
    logging.basicConfig(
        level=logging.INFO,
        format='%(asctime)s [%(levelname)s] %(name)s: %(message)s'
    )
    
    collector = MetricsCollector()
    await collector.start()


if __name__ == "__main__":
    asyncio.run(main())
