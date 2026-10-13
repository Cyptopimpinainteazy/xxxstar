//! Virtual network.
//!
//! Models the parts of a network that break atomic swaps: partitions, message
//! loss, latency and reordering. Delivery order is decided by `(due_ms, seq)`,
//! a total order, so an interleaving is reproducible rather than incidental.

use serde::Serialize;

use crate::rng::SimRng;

pub type NodeId = usize;

/// A message in flight. `op_index` points into the simulator's op table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Envelope {
    pub seq: u64,
    pub from: NodeId,
    pub to: NodeId,
    pub due_ms: u64,
    pub op_index: usize,
}

/// Counters for the evidence bundle.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct NetworkStats {
    pub sent: u64,
    pub delivered: u64,
    pub dropped: u64,
    /// Messages never sent because the sender/receiver were partitioned.
    pub partitioned: u64,
    /// Delivered out of send order.
    pub reordered: u64,
}

/// A fully deterministic virtual network.
#[derive(Debug)]
pub struct VirtualNetwork {
    nodes: usize,
    /// `link_up[from][to]` — directed, so a one-way partition is expressible.
    link_up: Vec<Vec<bool>>,
    base_latency_ms: u64,
    jitter_ms: u64,
    drop_percent: u32,
    queue: Vec<Envelope>,
    seq: u64,
    stats: NetworkStats,
}

impl VirtualNetwork {
    pub fn new(nodes: usize, base_latency_ms: u64, jitter_ms: u64, drop_percent: u32) -> Self {
        Self {
            nodes,
            link_up: vec![vec![true; nodes]; nodes],
            base_latency_ms,
            jitter_ms,
            drop_percent,
            queue: Vec::new(),
            seq: 0,
            stats: NetworkStats::default(),
        }
    }

    pub fn nodes(&self) -> usize {
        self.nodes
    }

    pub fn stats(&self) -> NetworkStats {
        self.stats
    }

    pub fn set_latency(&mut self, base_latency_ms: u64, jitter_ms: u64) {
        self.base_latency_ms = base_latency_ms;
        self.jitter_ms = jitter_ms;
    }

    pub fn set_drop_percent(&mut self, drop_percent: u32) {
        self.drop_percent = drop_percent.min(100);
    }

    pub fn set_link(&mut self, a: NodeId, b: NodeId, up: bool) {
        if a < self.nodes && b < self.nodes {
            self.link_up[a][b] = up;
            self.link_up[b][a] = up;
        }
    }

    /// Cut every directed link between the two groups.
    pub fn partition(&mut self, group_a: &[NodeId], group_b: &[NodeId]) {
        for &a in group_a {
            for &b in group_b {
                self.set_link(a, b, false);
            }
        }
    }

    /// Restore full connectivity.
    pub fn heal_all(&mut self) {
        for row in self.link_up.iter_mut() {
            for cell in row.iter_mut() {
                *cell = true;
            }
        }
    }

    pub fn link_up(&self, from: NodeId, to: NodeId) -> bool {
        from < self.nodes && to < self.nodes && self.link_up[from][to]
    }

    /// Attempt to put a message on the wire.
    ///
    /// Returns `true` if the message was accepted into the queue. A `false`
    /// means it never travelled — either partitioned or dropped — and the
    /// caller must treat the operation as "not delivered".
    pub fn send(
        &mut self,
        from: NodeId,
        to: NodeId,
        now_ms: u64,
        op_index: usize,
        rng: &mut SimRng,
    ) -> bool {
        if !self.link_up(from, to) {
            self.stats.partitioned += 1;
            return false;
        }
        if rng.chance(self.drop_percent) {
            self.stats.dropped += 1;
            return false;
        }

        let jitter = if self.jitter_ms == 0 {
            0
        } else {
            rng.next_u64() % (self.jitter_ms + 1)
        };
        self.seq += 1;
        self.queue.push(Envelope {
            seq: self.seq,
            from,
            to,
            due_ms: now_ms
                .saturating_add(self.base_latency_ms)
                .saturating_add(jitter),
            op_index,
        });
        self.stats.sent += 1;
        true
    }

    /// Remove and return every envelope due at or before `now_ms`, in
    /// `(due_ms, seq)` order.
    pub fn deliver_ready(&mut self, now_ms: u64) -> Vec<Envelope> {
        let mut ready: Vec<Envelope> = self
            .queue
            .iter()
            .filter(|env| env.due_ms <= now_ms)
            .copied()
            .collect();
        self.queue.retain(|env| env.due_ms > now_ms);
        ready.sort_by_key(|env| (env.due_ms, env.seq));

        // Count deliveries that did not follow send order.
        let mut max_seq = 0u64;
        for env in &ready {
            if env.seq < max_seq {
                self.stats.reordered += 1;
            }
            max_seq = max_seq.max(env.seq);
        }

        self.stats.delivered += ready.len() as u64;
        ready
    }

    /// Drop every in-flight message (used by a total crash of the receiving node).
    pub fn drop_in_flight_to(&mut self, node: NodeId) -> usize {
        let before = self.queue.len();
        self.queue.retain(|env| env.to != node);
        let dropped = before - self.queue.len();
        self.stats.dropped += dropped as u64;
        dropped
    }

    pub fn in_flight(&self) -> usize {
        self.queue.len()
    }
}
