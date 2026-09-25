"""
GPU Swarm Performance Optimization
GPU memory pooling, task batching, network optimization
"""

import os
import time
import logging
from typing import List, Dict, Any, Optional, Tuple
from dataclasses import dataclass, field
from collections import defaultdict
from threading import Lock, Event
from queue import Queue, PriorityQueue
import heapq

logger = logging.getLogger(__name__)


@dataclass
class MemoryBlock:
    """Represents a contiguous GPU memory block"""
    offset: int
    size: int
    allocated: bool = False
    allocated_size: int = 0
    metadata: Dict[str, Any] = field(default_factory=dict)
    timestamp: float = field(default_factory=time.time)


class GPUMemoryPool:
    """Efficient GPU memory management with pooling and defragmentation"""

    def __init__(self, total_memory_bytes: int):
        self.total_memory = total_memory_bytes
        self.allocated = 0
        self.blocks: List[MemoryBlock] = [
            MemoryBlock(0, total_memory_bytes)
        ]
        self.lock = Lock()
        self.fragmentation_threshold = 0.7
        self.stats = {
            "allocations": 0,
            "deallocations": 0,
            "defragmentations": 0,
            "peak_utilization": 0.0,
        }

    def allocate(self, size: int, metadata: Dict[str, Any] = None) -> Optional[int]:
        """
        Allocate memory block, automatically defragment if needed.
        Returns offset or None if allocation fails.
        """
        with self.lock:
            # Try first-fit allocation
            offset = self._first_fit_allocate(size, metadata)
            
            if offset is None and self._get_fragmentation() > self.fragmentation_threshold:
                # Defragment and retry
                logger.info(f"GPU memory fragmentation {self._get_fragmentation():.1%}, defragmenting...")
                self._defragment()
                offset = self._first_fit_allocate(size, metadata)
            
            if offset is not None:
                self.allocated += size
                self.stats["allocations"] += 1
                self.stats["peak_utilization"] = max(
                    self.stats["peak_utilization"],
                    self.allocated / self.total_memory
                )
                logger.debug(f"Allocated {size} bytes at offset {offset}")
            
            return offset

    def _first_fit_allocate(
        self,
        size: int,
        metadata: Dict[str, Any] = None
    ) -> Optional[int]:
        """Find first free block with sufficient size"""
        for block in self.blocks:
            if not block.allocated and block.size >= size:
                # Split block if necessary
                if block.size > size:
                    new_block = MemoryBlock(
                        block.offset + size,
                        block.size - size
                    )
                    self.blocks.insert(self.blocks.index(block) + 1, new_block)
                
                # Mark as allocated
                block.allocated = True
                block.allocated_size = size
                block.metadata = metadata or {}
                block.timestamp = time.time()
                return block.offset
        
        return None

    def deallocate(self, offset: int) -> bool:
        """Free allocated memory block"""
        with self.lock:
            for block in self.blocks:
                if block.offset == offset and block.allocated:
                    block.allocated = False
                    block.allocated_size = 0
                    self.allocated -= block.size
                    self.stats["deallocations"] += 1
                    logger.debug(f"Deallocated memory at offset {offset}")
                    return True
        
        return False

    def _defragment(self):
        """Consolidate free blocks to reduce fragmentation"""
        # Sort blocks by offset
        self.blocks.sort(key=lambda b: b.offset)
        
        # Merge adjacent free blocks
        i = 0
        while i < len(self.blocks) - 1:
            if (not self.blocks[i].allocated and
                not self.blocks[i + 1].allocated):
                # Merge blocks
                self.blocks[i].size += self.blocks[i + 1].size
                self.blocks.pop(i + 1)
            else:
                i += 1
        
        self.stats["defragmentations"] += 1
        logger.info(f"Defragmentation complete. Fragmentation: {self._get_fragmentation():.1%}")

    def _get_fragmentation(self) -> float:
        """Calculate memory fragmentation ratio"""
        if self.allocated == 0:
            return 0.0
        
        # Count number of free blocks
        free_blocks = sum(1 for b in self.blocks if not b.allocated)
        
        # Fragmentation = (free_blocks - 1) / allocated_blocks
        # Higher ratio = more fragmented
        allocated_blocks = sum(1 for b in self.blocks if b.allocated)
        
        if allocated_blocks == 0:
            return 0.0
        
        return max(0, (free_blocks - 1) / allocated_blocks)

    def get_stats(self) -> Dict[str, Any]:
        """Get memory pool statistics"""
        with self.lock:
            return {
                "total_memory": self.total_memory,
                "allocated": self.allocated,
                "free": self.total_memory - self.allocated,
                "utilization": self.allocated / self.total_memory,
                "fragmentation": self._get_fragmentation(),
                "blocks": len(self.blocks),
                "allocated_blocks": sum(1 for b in self.blocks if b.allocated),
                **self.stats
            }


@dataclass
class Task:
    """Represents a computation task"""
    task_id: str
    priority: int = 0
    memory_required: int = 0
    timeout_seconds: int = 300
    gpu_backend: str = "cuda"
    dependencies: List[str] = field(default_factory=list)
    timestamp: float = field(default_factory=time.time)


class TaskBatchOptimizer:
    """Optimize task execution through intelligent batching"""

    def __init__(
        self,
        batch_size: int = 32,
        batch_timeout: float = 1.0,
        parallel_batches: int = 4
    ):
        self.batch_size = batch_size
        self.batch_timeout = batch_timeout
        self.parallel_batches = parallel_batches
        self.pending_tasks: PriorityQueue = PriorityQueue()
        self.active_batches: List[List[Task]] = []
        self.lock = Lock()
        self.batch_ready_event = Event()
        self.stats = {
            "tasks_batched": 0,
            "batches_created": 0,
            "avg_batch_size": 0,
            "total_batching_delay": 0.0,
        }

    def add_task(self, task: Task):
        """Add task to batching queue"""
        # Priority: -priority (higher priority = lower heap value)
        # Then: timestamp (older tasks have priority)
        self.pending_tasks.put((-task.priority, task.timestamp, task))
        
        if self.pending_tasks.qsize() >= self.batch_size:
            self.batch_ready_event.set()

    def get_next_batch(self) -> Optional[List[Task]]:
        """Get next batch of tasks ready for execution"""
        with self.lock:
            if len(self.active_batches) >= self.parallel_batches:
                return None  # Cannot start new batch yet
        
        batch = []
        batch_created_time = time.time()
        
        # Collect tasks for batch
        while len(batch) < self.batch_size and not self.pending_tasks.empty():
            try:
                _, _, task = self.pending_tasks.get_nowait()
                batch.append(task)
            except:
                break
        
        # Wait for timeout or minimum number of tasks
        if len(batch) < self.batch_size and not self.pending_tasks.empty():
            time.sleep(self.batch_timeout)
            
            # Try to get more tasks
            while len(batch) < self.batch_size:
                try:
                    _, _, task = self.pending_tasks.get_nowait()
                    batch.append(task)
                except:
                    break
        
        if batch:
            with self.lock:
                self.active_batches.append(batch)
            
            batching_delay = time.time() - batch_created_time
            self.stats["tasks_batched"] += len(batch)
            self.stats["batches_created"] += 1
            self.stats["total_batching_delay"] += batching_delay
            self.stats["avg_batch_size"] = self.stats["tasks_batched"] / self.stats["batches_created"]
            
            logger.info(f"Creating batch with {len(batch)} tasks (delay: {batching_delay:.2f}s)")
            return batch
        
        return None

    def complete_batch(self, batch: List[Task], results: List[Any]):
        """Mark batch as complete"""
        with self.lock:
            if batch in self.active_batches:
                self.active_batches.remove(batch)

    def get_stats(self) -> Dict[str, Any]:
        """Get batching statistics"""
        with self.lock:
            return {
                "pending_tasks": self.pending_tasks.qsize(),
                "active_batches": len(self.active_batches),
                **self.stats
            }


@dataclass
class GossipMessage:
    """Message for peer gossip protocol"""
    message_id: str
    message_type: str
    payload: Any
    hop_limit: int = 64
    created_at: float = field(default_factory=time.time)


class NetworkOptimizer:
    """Optimize network communication with batching and compression"""

    def __init__(self, compression_threshold: int = 1024):
        self.compression_threshold = compression_threshold
        self.message_buffer: Dict[str, List[Any]] = defaultdict(list)
        self.compression_enabled = True
        self.stats = {
            "messages_sent": 0,
            "messages_batched": 0,
            "bytes_saved": 0,
        }
        self.lock = Lock()

    def queue_message(self, peer_id: str, message: GossipMessage):
        """Queue message for batching"""
        with self.lock:
            self.message_buffer[peer_id].append(message)

    def should_flush(self, peer_id: str) -> bool:
        """Determine if buffer should be flushed"""
        # Check if buffer has messages and either:
        # 1. Contains critical message, or
        # 2. Buffer is getting large, or
        # 3. Time threshold exceeded
        with self.lock:
            if not self.message_buffer[peer_id]:
                return False
            
            buffer_size = sum(
                len(str(m).encode())
                for m in self.message_buffer[peer_id]
            )
            
            return buffer_size > self.compression_threshold

    def get_messages_to_send(self, peer_id: str) -> Optional[bytes]:
        """Get batched messages for peer"""
        with self.lock:
            messages = self.message_buffer.pop(peer_id, [])
            
            if not messages:
                return None
            
            # Serialize messages
            import json
            serialized = json.dumps([
                {
                    "id": m.message_id,
                    "type": m.message_type,
                    "payload": m.payload,
                    "hop_limit": m.hop_limit
                }
                for m in messages
            ]).encode()
            
            # Compress if beneficial
            if self.compression_enabled and len(serialized) > self.compression_threshold:
                import gzip
                compressed = gzip.compress(serialized)
                if len(compressed) < len(serialized):
                    self.stats["bytes_saved"] += len(serialized) - len(compressed)
                    serialized = compressed
            
            self.stats["messages_sent"] += len(messages)
            self.stats["messages_batched"] += 1
            
            return serialized

    def get_stats(self) -> Dict[str, Any]:
        """Get network optimization statistics"""
        with self.lock:
            return {
                "buffered_peers": len(self.message_buffer),
                **self.stats
            }


class PerformanceOptimizationEngine:
    """Central engine coordinating all performance optimizations"""

    def __init__(self, gpu_memory_bytes: int = 40 * 1024 * 1024 * 1024):  # 40GB default
        self.memory_pool = GPUMemoryPool(gpu_memory_bytes)
        self.batch_optimizer = TaskBatchOptimizer()
        self.network_optimizer = NetworkOptimizer()
        self.stats = {}

    def optimize_task_execution(self) -> Optional[List[Task]]:
        """Get next optimized batch of tasks"""
        batch = self.batch_optimizer.get_next_batch()
        
        if batch:
            # Allocate required memory for batch
            total_memory = sum(t.memory_required for t in batch)
            offset = self.memory_pool.allocate(
                total_memory,
                metadata={"batch_ids": [t.task_id for t in batch]}
            )
            
            if offset is None:
                logger.error(f"Failed to allocate memory for batch {batch}")
                return None
        
        return batch

    def get_comprehensive_stats(self) -> Dict[str, Any]:
        """Get stats from all optimization components"""
        return {
            "memory_pool": self.memory_pool.get_stats(),
            "batch_optimizer": self.batch_optimizer.get_stats(),
            "network_optimizer": self.network_optimizer.get_stats(),
        }


# Singleton instance
_engine = None

def get_performance_engine(gpu_memory_bytes: int = None) -> PerformanceOptimizationEngine:
    """Get or create the performance optimization engine"""
    global _engine
    if _engine is None:
        _engine = PerformanceOptimizationEngine(gpu_memory_bytes or 40 * 1024 * 1024 * 1024)
    return _engine


__all__ = [
    'GPUMemoryPool',
    'TaskBatchOptimizer',
    'NetworkOptimizer',
    'PerformanceOptimizationEngine',
    'get_performance_engine',
    'MemoryBlock',
    'Task',
]
