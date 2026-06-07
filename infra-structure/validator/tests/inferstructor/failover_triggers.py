#!/usr/bin/env python3
"""
Failover Trigger System - Controlled Failure Injection for Inferstructor Testing

Simulates various failure scenarios to test lane failover behavior:
- GPU crashes
- Node failures  
- Network partitions
- Memory exhaustion
- Cascade failures
"""

import argparse
import asyncio
import logging
import subprocess
import time
from dataclasses import dataclass
from enum import Enum
from typing import Optional

import aiohttp
import psutil


class TriggerType(Enum):
    """Available failure triggers"""
    KILL_PRIMARY_GPU = "kill_primary_gpu"
    KILL_PRIMARY_NODE = "kill_primary_node"
    KILL_SHADOW_LANE = "kill_shadow_lane"
    INJECT_LATENCY_SPIKE = "inject_latency_spike"
    FILL_GPU_MEMORY = "fill_gpu_memory"
    PARTITION_PRIMARY_SHADOW = "partition_primary_shadow"
    CASCADE_FAILURE = "cascade_failure"
    CORRUPT_STATE = "corrupt_state"
    EXHAUST_CPU = "exhaust_cpu"
    NETWORK_DISCONNECT = "network_disconnect"


@dataclass
class TriggerConfig:
    """Configuration for a specific trigger"""
    lane_id: str
    endpoint: str
    duration_seconds: int = 0  # 0 = permanent until recovered
    intensity: float = 1.0  # 0.0-1.0 severity


class FailoverTrigger:
    def __init__(self):
        self.logger = logging.getLogger("FailoverTrigger")
        self.active_triggers: dict[str, asyncio.Task] = {}
        
    async def execute_trigger(self, trigger: TriggerType, config: TriggerConfig):
        """Execute a specific failure trigger"""
        self.logger.info(f"Executing trigger: {trigger.value} on {config.lane_id}")
        
        trigger_map = {
            TriggerType.KILL_PRIMARY_GPU: self._kill_gpu_process,
            TriggerType.KILL_PRIMARY_NODE: self._kill_node_process,
            TriggerType.KILL_SHADOW_LANE: self._kill_node_process,
            TriggerType.INJECT_LATENCY_SPIKE: self._inject_network_latency,
            TriggerType.FILL_GPU_MEMORY: self._fill_gpu_memory,
            TriggerType.PARTITION_PRIMARY_SHADOW: self._partition_network,
            TriggerType.CASCADE_FAILURE: self._cascade_failure,
            TriggerType.CORRUPT_STATE: self._corrupt_state,
            TriggerType.EXHAUST_CPU: self._exhaust_cpu,
            TriggerType.NETWORK_DISCONNECT: self._disconnect_network,
        }
        
        handler = trigger_map.get(trigger)
        if not handler:
            raise ValueError(f"Unknown trigger: {trigger}")
        
        result = await handler(config)
        self.logger.info(f"Trigger {trigger.value} completed: {result}")
        return result
    
    async def _kill_gpu_process(self, config: TriggerConfig) -> dict:
        """Simulate GPU crash by killing GPU worker processes"""
        self.logger.warning(f"Killing GPU processes on {config.lane_id}")
        
        try:
            # Send kill signal to lane
            async with aiohttp.ClientSession() as session:
                async with session.post(
                    f"{config.endpoint}/control/kill-gpu",
                    json={"intensity": config.intensity},
                    timeout=aiohttp.ClientTimeout(total=5)
                ) as resp:
                    if resp.status == 200:
                        return {"success": True, "message": "GPU process killed"}
                    else:
                        return {"success": False, "error": f"HTTP {resp.status}"}
                        
        except Exception as e:
            self.logger.error(f"Failed to kill GPU: {e}")
            # Fallback: try killing via process name
            try:
                subprocess.run(
                    ["pkill", "-9", "-f", f"ccgv.*{config.lane_id}"],
                    check=False,
                    capture_output=True
                )
                return {"success": True, "message": "GPU process killed via pkill"}
            except Exception as e2:
                return {"success": False, "error": str(e2)}
    
    async def _kill_node_process(self, config: TriggerConfig) -> dict:
        """Kill entire node process"""
        self.logger.warning(f"Killing node process on {config.lane_id}")
        
        try:
            # Gracefully request shutdown first
            async with aiohttp.ClientSession() as session:
                async with session.post(
                    f"{config.endpoint}/control/shutdown",
                    timeout=aiohttp.ClientTimeout(total=2)
                ) as resp:
                    if resp.status == 200:
                        return {"success": True, "message": "Node shutdown requested"}
        except:
            pass  # Expected if node dies
        
        # Force kill if still alive after 2 seconds
        await asyncio.sleep(2)
        try:
            subprocess.run(
                ["pkill", "-9", "-f", f"lane.*{config.lane_id}"],
                check=False
            )
            return {"success": True, "message": "Node force killed"}
        except Exception as e:
            return {"success": False, "error": str(e)}
    
    async def _inject_network_latency(self, config: TriggerConfig) -> dict:
        """Inject network latency using tc (traffic control)"""
        self.logger.warning(f"Injecting network latency on {config.lane_id}")
        
        # Calculate latency in ms based on intensity
        latency_ms = int(config.intensity * 1000)  # Up to 1000ms
        
        try:
            # Extract interface from endpoint (assume eth0 for now)
            interface = "eth0"
            
            # Add latency using tc
            subprocess.run([
                "tc", "qdisc", "add", "dev", interface,
                "root", "netem", "delay", f"{latency_ms}ms"
            ], check=True, capture_output=True)
            
            # Auto-remove after duration if specified
            if config.duration_seconds > 0:
                await asyncio.sleep(config.duration_seconds)
                subprocess.run([
                    "tc", "qdisc", "del", "dev", interface, "root"
                ], check=False)
                
            return {
                "success": True,
                "message": f"Added {latency_ms}ms latency",
                "interface": interface
            }
            
        except subprocess.CalledProcessError as e:
            return {"success": False, "error": str(e), "stderr": e.stderr.decode()}
        except Exception as e:
            return {"success": False, "error": str(e)}
    
    async def _fill_gpu_memory(self, config: TriggerConfig) -> dict:
        """Fill GPU memory to trigger VRAM exhaustion"""
        self.logger.warning(f"Filling GPU memory on {config.lane_id}")
        
        try:
            async with aiohttp.ClientSession() as session:
                # Request lane to allocate large GPU buffers
                async with session.post(
                    f"{config.endpoint}/control/stress-gpu-memory",
                    json={
                        "fill_percent": config.intensity * 100,
                        "duration_seconds": config.duration_seconds
                    },
                    timeout=aiohttp.ClientTimeout(total=5)
                ) as resp:
                    if resp.status == 200:
                        data = await resp.json()
                        return {"success": True, **data}
                    else:
                        return {"success": False, "error": f"HTTP {resp.status}"}
                        
        except Exception as e:
            return {"success": False, "error": str(e)}
    
    async def _partition_network(self, config: TriggerConfig) -> dict:
        """Create network partition between primary and shadow"""
        self.logger.warning(f"Creating network partition for {config.lane_id}")
        
        try:
            # Use iptables to block traffic
            # This assumes we know the peer IP addresses
            # In production, read from config
            
            # Block outgoing traffic to shadow
            subprocess.run([
                "iptables", "-A", "OUTPUT", "-d", "10.0.2.0/24", "-j", "DROP"
            ], check=True)
            
            # Block incoming traffic from shadow
            subprocess.run([
                "iptables", "-A", "INPUT", "-s", "10.0.2.0/24", "-j", "DROP"
            ], check=True)
            
            # Auto-remove after duration
            if config.duration_seconds > 0:
                await asyncio.sleep(config.duration_seconds)
                # Flush rules
                subprocess.run(["iptables", "-F"], check=False)
            
            return {"success": True, "message": "Network partition created"}
            
        except subprocess.CalledProcessError as e:
            return {"success": False, "error": str(e)}
    
    async def _cascade_failure(self, config: TriggerConfig) -> dict:
        """Trigger cascade failure (kill primary + shadow simultaneously)"""
        self.logger.warning("Triggering cascade failure - killing primary AND shadow")
        
        # Kill primary first
        primary_config = TriggerConfig(
            lane_id="primary",
            endpoint="http://10.0.1.10:9000"
        )
        result_primary = await self._kill_node_process(primary_config)
        
        # Wait 100ms then kill shadow
        await asyncio.sleep(0.1)
        
        shadow_config = TriggerConfig(
            lane_id="shadow",
            endpoint="http://10.0.2.10:9000"
        )
        result_shadow = await self._kill_node_process(shadow_config)
        
        return {
            "success": True,
            "primary": result_primary,
            "shadow": result_shadow,
            "message": "Cascade failure triggered - both lanes killed"
        }
    
    async def _corrupt_state(self, config: TriggerConfig) -> dict:
        """Corrupt internal state to trigger validation failures"""
        self.logger.warning(f"Corrupting state on {config.lane_id}")
        
        try:
            async with aiohttp.ClientSession() as session:
                async with session.post(
                    f"{config.endpoint}/control/corrupt-state",
                    json={"corruption_level": config.intensity},
                    timeout=aiohttp.ClientTimeout(total=5)
                ) as resp:
                    if resp.status == 200:
                        return {"success": True, "message": "State corrupted"}
                    else:
                        return {"success": False, "error": f"HTTP {resp.status}"}
                        
        except Exception as e:
            return {"success": False, "error": str(e)}
    
    async def _exhaust_cpu(self, config: TriggerConfig) -> dict:
        """Exhaust CPU to slow down processing"""
        self.logger.warning(f"Exhausting CPU on {config.lane_id}")
        
        try:
            # Use stress-ng if available
            cpu_count = psutil.cpu_count()
            workers = int(cpu_count * config.intensity)
            
            proc = subprocess.Popen([
                "stress-ng", "--cpu", str(workers),
                "--timeout", f"{config.duration_seconds}s"
            ], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            
            return {
                "success": True,
                "message": f"CPU stress started with {workers} workers",
                "pid": proc.pid
            }
            
        except FileNotFoundError:
            # Fallback: create busy loops
            return {"success": False, "error": "stress-ng not installed"}
        except Exception as e:
            return {"success": False, "error": str(e)}
    
    async def _disconnect_network(self, config: TriggerConfig) -> dict:
        """Completely disconnect network interface"""
        self.logger.warning(f"Disconnecting network on {config.lane_id}")
        
        try:
            interface = "eth0"
            subprocess.run(["ip", "link", "set", interface, "down"], check=True)
            
            # Auto-reconnect after duration
            if config.duration_seconds > 0:
                await asyncio.sleep(config.duration_seconds)
                subprocess.run(["ip", "link", "set", interface, "up"], check=False)
                
            return {"success": True, "message": f"Interface {interface} disconnected"}
            
        except subprocess.CalledProcessError as e:
            return {"success": False, "error": str(e)}


async def main():
    parser = argparse.ArgumentParser(description="Inferstructor Failover Trigger System")
    parser.add_argument(
        "--trigger",
        required=True,
        choices=[t.value for t in TriggerType],
        help="Type of failure to trigger"
    )
    parser.add_argument(
        "--lane",
        default="primary",
        help="Lane ID to target (default: primary)"
    )
    parser.add_argument(
        "--endpoint",
        default="http://10.0.1.10:9000",
        help="Lane endpoint URL"
    )
    parser.add_argument(
        "--duration",
        type=int,
        default=0,
        help="Duration in seconds (0 = permanent)"
    )
    parser.add_argument(
        "--intensity",
        type=float,
        default=1.0,
        help="Failure intensity 0.0-1.0 (default: 1.0)"
    )
    
    args = parser.parse_args()
    
    logging.basicConfig(
        level=logging.INFO,
        format='%(asctime)s [%(levelname)s] %(name)s: %(message)s'
    )
    
    trigger_system = FailoverTrigger()
    
    config = TriggerConfig(
        lane_id=args.lane,
        endpoint=args.endpoint,
        duration_seconds=args.duration,
        intensity=args.intensity
    )
    
    trigger_type = TriggerType(args.trigger)
    
    result = await trigger_system.execute_trigger(trigger_type, config)
    
    print(f"\nTrigger Result:")
    print(f"  Trigger: {trigger_type.value}")
    print(f"  Lane: {config.lane_id}")
    print(f"  Success: {result.get('success', False)}")
    if 'message' in result:
        print(f"  Message: {result['message']}")
    if 'error' in result:
        print(f"  Error: {result['error']}")


if __name__ == "__main__":
    asyncio.run(main())
