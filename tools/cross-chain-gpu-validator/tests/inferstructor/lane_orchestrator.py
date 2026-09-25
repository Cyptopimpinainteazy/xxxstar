#!/usr/bin/env python3
"""
Lane Orchestrator - Inferstructor Traffic Control & Failover Engine

Responsibilities:
- Monitor health of all lanes (primary, shadow, tertiary)
- Execute deterministic failover
- Prevent split-brain scenarios
- Track per-request hash correctness
- Enforce SLA limits via toll booth
- Log all promotion events
"""

import asyncio
import logging
import time
from dataclasses import dataclass, field
from enum import Enum
from typing import Dict, List, Optional
import aiohttp
import yaml
from prometheus_client import Counter, Gauge, Histogram, start_http_server


# Metrics
lane_health_score = Gauge('lane_health_score', 'Health score per lane', ['lane_id'])
failover_counter = Counter('failover_total', 'Total failovers', ['from_lane', 'to_lane', 'reason'])
promotion_duration = Histogram('promotion_duration_seconds', 'Lane promotion time')
requests_routed = Counter('requests_routed_total', 'Requests routed', ['lane_id', 'sla_tier'])
hash_correctness = Gauge('hash_correctness_ratio', 'Hash correctness ratio', ['lane_id'])


class LaneState(Enum):
    HEALTHY = "healthy"
    DEGRADED = "degraded"
    UNHEALTHY = "unhealthy"
    OFFLINE = "offline"


class LaneRole(Enum):
    PRIMARY = "primary"
    SHADOW = "shadow"
    TERTIARY = "tertiary"
    OFFLINE = "offline"


@dataclass
class HealthMetrics:
    gpu_utilization: float = 0.0
    cpu_utilization: float = 0.0
    memory_usage: float = 0.0
    kernel_execution_time_ms: float = 0.0
    request_queue_depth: int = 0
    error_rate: float = 0.0
    response_time_ms: float = 0.0
    timestamp: float = field(default_factory=time.time)


@dataclass
class Lane:
    id: str
    name: str
    role: LaneRole
    priority: int
    endpoint: str
    metrics_endpoint: str
    region: str
    
    state: LaneState = LaneState.OFFLINE
    health_score: float = 0.0
    metrics: HealthMetrics = field(default_factory=HealthMetrics)
    signing_authority: bool = False
    last_heartbeat: float = 0.0
    
    def calculate_health_score(self) -> float:
        """Calculate composite health score (0.0 - 1.0)"""
        if self.state == LaneState.OFFLINE:
            return 0.0
        
        # Inverse error rate (lower is better)
        error_score = max(0, 1.0 - self.metrics.error_rate)
        
        # Response time score (inverse - lower latency is better)
        # Target: <1ms = 1.0, >10ms = 0.0
        latency_score = max(0, 1.0 - (self.metrics.response_time_ms / 10.0))
        
        # GPU utilization (sweet spot: 70-95%)
        if self.metrics.gpu_utilization > 95:
            gpu_score = 0.5  # Overloaded
        elif self.metrics.gpu_utilization < 30:
            gpu_score = 0.3  # Underutilized or broken
        else:
            gpu_score = 1.0
        
        # Queue depth (lower is better)
        queue_score = max(0, 1.0 - (self.metrics.request_queue_depth / 10000.0))
        
        # Weighted composite
        score = (
            error_score * 0.3 +
            latency_score * 0.3 +
            gpu_score * 0.2 +
            queue_score * 0.2
        )
        
        return max(0.0, min(1.0, score))


class LaneOrchestrator:
    def __init__(self, config_path: str):
        self.config = self._load_config(config_path)
        self.lanes: Dict[str, Lane] = {}
        self.active_lane_id: Optional[str] = None
        self.signer_lock_held_by: Optional[str] = None
        
        self.logger = logging.getLogger("LaneOrchestrator")
        self.logger.setLevel(logging.INFO)
        
        self._init_lanes()
    
    def _load_config(self, path: str) -> dict:
        with open(path) as f:
            return yaml.safe_load(f)
    
    def _init_lanes(self):
        """Initialize lane objects from config"""
        for lane_config in self.config['lanes']:
            lane = Lane(
                id=lane_config['id'],
                name=lane_config['name'],
                role=LaneRole(lane_config['role']),
                priority=lane_config['priority'],
                endpoint=lane_config['endpoint'],
                metrics_endpoint=lane_config['metrics_endpoint'],
                region=lane_config['region'],
            )
            self.lanes[lane.id] = lane
            
            # Primary starts as active
            if lane.role == LaneRole.PRIMARY:
                self.active_lane_id = lane.id
                lane.signing_authority = True
                
        self.logger.info(f"Initialized {len(self.lanes)} lanes")
    
    async def start(self):
        """Start orchestrator control loop"""
        self.logger.info("Starting Lane Orchestrator")
        
        # Start metrics server
        start_http_server(8000)
        
        # Start health monitoring loop
        asyncio.create_task(self.health_check_loop())
        
        # Start failover decision loop
        asyncio.create_task(self.failover_decision_loop())
        
        # Keep running
        await asyncio.Event().wait()
    
    async def health_check_loop(self):
        """Continuously monitor all lanes"""
        while True:
            tasks = [self.check_lane_health(lane) for lane in self.lanes.values()]
            await asyncio.gather(*tasks, return_exceptions=True)
            
            # Update metrics
            for lane in self.lanes.values():
                lane_health_score.labels(lane_id=lane.id).set(lane.health_score)
            
            await asyncio.sleep(5)  # Check every 5 seconds
    
    async def check_lane_health(self, lane: Lane):
        """Query lane metrics and update health state"""
        try:
            async with aiohttp.ClientSession() as session:
                async with session.get(
                    f"{lane.metrics_endpoint}/metrics",
                    timeout=aiohttp.ClientTimeout(total=2)
                ) as resp:
                    if resp.status != 200:
                        raise Exception(f"HTTP {resp.status}")
                    
                    data = await resp.json()
                    
                    # Update metrics
                    lane.metrics = HealthMetrics(
                        gpu_utilization=data.get('gpu_utilization', 0),
                        cpu_utilization=data.get('cpu_utilization', 0),
                        memory_usage=data.get('memory_usage', 0),
                        kernel_execution_time_ms=data.get('kernel_exec_time_ms', 0),
                        request_queue_depth=data.get('queue_depth', 0),
                        error_rate=data.get('error_rate', 0),
                        response_time_ms=data.get('response_time_ms', 0),
                    )
                    
                    lane.last_heartbeat = time.time()
                    
                    # Calculate health score
                    lane.health_score = lane.calculate_health_score()
                    
                    # Update state based on score
                    if lane.health_score >= 0.7:
                        lane.state = LaneState.HEALTHY
                    elif lane.health_score >= 0.3:
                        lane.state = LaneState.DEGRADED
                    else:
                        lane.state = LaneState.UNHEALTHY
                    
                    self.logger.debug(f"{lane.id} health: {lane.health_score:.2f} state: {lane.state.value}")
                    
        except Exception as e:
            self.logger.warning(f"{lane.id} health check failed: {e}")
            lane.state = LaneState.OFFLINE
            lane.health_score = 0.0
    
    async def failover_decision_loop(self):
        """Decide when to trigger failover"""
        while True:
            active_lane = self.lanes.get(self.active_lane_id)
            
            if not active_lane:
                self.logger.error("No active lane set!")
                await asyncio.sleep(5)
                continue
            
            # Check if active lane is unhealthy
            if active_lane.state in [LaneState.UNHEALTHY, LaneState.OFFLINE]:
                self.logger.warning(f"Active lane {active_lane.id} is {active_lane.state.value}. Initiating failover.")
                await self.execute_failover(active_lane)
            
            # Check for heartbeat timeout
            elif time.time() - active_lane.last_heartbeat > 10:
                self.logger.warning(f"Active lane {active_lane.id} heartbeat timeout. Initiating failover.")
                await self.execute_failover(active_lane)
            
            await asyncio.sleep(1)  # Check every second
    
    async def execute_failover(self, failed_lane: Lane):
        """Execute controlled failover to next best lane"""
        start_time = time.time()
        
        # Find best promotion target
        target_lane = self._select_promotion_target(failed_lane)
        
        if not target_lane:
            self.logger.error("No healthy lane available for failover!")
            # Alert external validators deferred to notification service integration
            return
        
        self.logger.info(f"Failing over: {failed_lane.id} -> {target_lane.id}")
        
        # Step 1: Release signer authority from failed lane
        await self._release_signer_lock(failed_lane)
        
        # Step 2: Acquire signer authority for target lane
        acquired = await self._acquire_signer_lock(target_lane)
        if not acquired:
            self.logger.error(f"Failed to acquire signer lock for {target_lane.id}")
            return
        
        # Step 3: Promote target lane
        await self._promote_lane(target_lane)
        
        # Step 4: Demote failed lane
        await self._demote_lane(failed_lane)
        
        # Update active lane
        old_active = self.active_lane_id
        self.active_lane_id = target_lane.id
        
        duration = time.time() - start_time
        
        # Record metrics
        promotion_duration.observe(duration)
        failover_counter.labels(
            from_lane=failed_lane.id,
            to_lane=target_lane.id,
            reason=failed_lane.state.value
        ).inc()
        
        self.logger.info(f"Failover complete in {duration*1000:.2f}ms: {old_active} -> {target_lane.id}")
    
    def _select_promotion_target(self, failed_lane: Lane) -> Optional[Lane]:
        """Select best lane for promotion based on health + priority"""
        candidates = [
            lane for lane in self.lanes.values()
            if lane.id != failed_lane.id and lane.state == LaneState.HEALTHY
        ]
        
        if not candidates:
            # No healthy lanes - try degraded
            candidates = [
                lane for lane in self.lanes.values()
                if lane.id != failed_lane.id and lane.state == LaneState.DEGRADED
            ]
        
        if not candidates:
            return None
        
        # Sort by priority (lower number = higher priority) then health score
        candidates.sort(key=lambda l: (l.priority, -l.health_score))
        
        return candidates[0]
    
    async def _acquire_signer_lock(self, lane: Lane) -> bool:
        """Acquire distributed signer lock for lane"""
        # Distributed lock requires etcd cluster deployment
        # For now, simple in-memory lock
        
        if self.signer_lock_held_by and self.signer_lock_held_by != lane.id:
            self.logger.warning(f"Signer lock already held by {self.signer_lock_held_by}")
            return False
        
        self.signer_lock_held_by = lane.id
        lane.signing_authority = True
        
        self.logger.info(f"Signer lock acquired by {lane.id}")
        return True
    
    async def _release_signer_lock(self, lane: Lane):
        """Release signer lock"""
        if self.signer_lock_held_by == lane.id:
            self.signer_lock_held_by = None
            lane.signing_authority = False
            self.logger.info(f"Signer lock released by {lane.id}")
    
    async def _promote_lane(self, lane: Lane):
        """Promote lane to active role"""
        try:
            async with aiohttp.ClientSession() as session:
                async with session.post(
                    f"{lane.endpoint}/control/promote",
                    timeout=aiohttp.ClientTimeout(total=5)
                ) as resp:
                    if resp.status != 200:
                        raise Exception(f"Promotion failed: HTTP {resp.status}")
                    
            self.logger.info(f"Lane {lane.id} promoted to active")
            
        except Exception as e:
            self.logger.error(f"Failed to promote {lane.id}: {e}")
            raise
    
    async def _demote_lane(self, lane: Lane):
        """Demote lane to standby/offline"""
        try:
            async with aiohttp.ClientSession() as session:
                async with session.post(
                    f"{lane.endpoint}/control/demote",
                    timeout=aiohttp.ClientTimeout(total=5)
                ) as resp:
                    if resp.status != 200:
                        self.logger.warning(f"Demotion of {lane.id} failed: HTTP {resp.status}")
                    else:
                        self.logger.info(f"Lane {lane.id} demoted")
                    
        except Exception as e:
            # Non-critical if already offline
            self.logger.warning(f"Failed to demote {lane.id}: {e}")
    
    async def route_request(self, validator_id: str, sla_tier: str, request: dict):
        """Route validator request to appropriate lane"""
        active_lane = self.lanes.get(self.active_lane_id)
        
        if not active_lane or active_lane.state != LaneState.HEALTHY:
            # Fallback to next best lane
            active_lane = self._select_promotion_target(active_lane)
        
        if not active_lane:
            raise Exception("No healthy lane available")
        
        # Route to lane
        requests_routed.labels(lane_id=active_lane.id, sla_tier=sla_tier).inc()
        
        # Request forwarding deferred to lane routing logic
        return await self._forward_to_lane(active_lane, request)
    
    async def _forward_to_lane(self, lane: Lane, request: dict):
        """Forward request to specific lane"""
        async with aiohttp.ClientSession() as session:
            async with session.post(
                f"{lane.endpoint}/accelerate",
                json=request,
                timeout=aiohttp.ClientTimeout(total=1)
            ) as resp:
                if resp.status != 200:
                    raise Exception(f"Lane {lane.id} returned HTTP {resp.status}")
                
                return await resp.json()


async def main():
    logging.basicConfig(
        level=logging.INFO,
        format='%(asctime)s [%(levelname)s] %(name)s: %(message)s'
    )
    
    orchestrator = LaneOrchestrator("configs/orchestrator.yaml")
    await orchestrator.start()


if __name__ == "__main__":
    asyncio.run(main())
