//! Packet Module - Network packet handling

use crate::error::TurbineResult;
use bytes::{Buf, BufMut};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;

/// Packet data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Packet {
    /// Packet destination
    pub addr: String,
    /// Packet data
    pub data: Vec<u8>,
    /// Metadata
    pub meta: PacketMeta,
}

/// Packet metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketMeta {
    pub size: usize,
    pub timestamp: u64,
    pub slot: Option<u64>,
    pub shred_index: Option<u32>,
}

impl Packet {
    /// Create new packet
    pub fn new(addr: String, data: Vec<u8>, slot: Option<u64>, shred_index: Option<u32>) -> Self {
        let meta = PacketMeta {
            size: data.len(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            slot,
            shred_index,
        };

        Self { addr, data, meta }
    }

    /// Serialize packet to bytes
    pub fn serialize(&self) -> TurbineResult<Vec<u8>> {
        let mut buf = Vec::new();

        // Write address length and address
        let addr_bytes = self.addr.as_bytes();
        buf.put_u32(addr_bytes.len() as u32);
        buf.put_slice(addr_bytes);

        // Write data length and data
        buf.put_u32(self.data.len() as u32);
        buf.put_slice(&self.data);

        // Write metadata
        buf.put_u64(self.meta.size as u64);
        buf.put_u64(self.meta.timestamp);

        if let Some(slot) = self.meta.slot {
            buf.put_u8(1);
            buf.put_u64(slot);
        } else {
            buf.put_u8(0);
        }

        if let Some(index) = self.meta.shred_index {
            buf.put_u8(1);
            buf.put_u32(index);
        } else {
            buf.put_u8(0);
        }

        Ok(buf)
    }

    /// Deserialize packet from bytes
    pub fn deserialize(data: &[u8]) -> TurbineResult<Self> {
        let mut buf = data;

        // Read address
        let addr_len = buf.get_u32() as usize;
        let addr = String::from_utf8_lossy(&buf[..addr_len]).to_string();
        buf.advance(addr_len);

        // Read data
        let data_len = buf.get_u32() as usize;
        let data = buf[..data_len].to_vec();
        buf.advance(data_len);

        // Read metadata
        let size = buf.get_u64() as usize;
        let timestamp = buf.get_u64();

        let slot = if buf.get_u8() == 1 {
            Some(buf.get_u64())
        } else {
            None
        };
        let shred_index = if buf.get_u8() == 1 {
            Some(buf.get_u32())
        } else {
            None
        };

        Ok(Self {
            addr,
            data,
            meta: PacketMeta {
                size,
                timestamp,
                slot,
                shred_index,
            },
        })
    }
}

/// Packet pool for memory management
pub struct PacketPool {
    pool: Arc<Mutex<VecDeque<Packet>>>,
    max_size: usize,
}

impl PacketPool {
    /// Create new packet pool
    pub fn new(max_size: usize) -> Self {
        Self {
            pool: Arc::new(Mutex::new(VecDeque::new())),
            max_size,
        }
    }

    /// Get a packet from the pool
    pub fn get(
        &self,
        addr: String,
        data: Vec<u8>,
        slot: Option<u64>,
        shred_index: Option<u32>,
    ) -> Packet {
        let mut pool = self.pool.lock();

        if let Some(mut packet) = pool.pop_front() {
            packet.addr = addr;
            let data_len = data.len();
            packet.data = data;
            packet.meta = PacketMeta {
                size: data_len,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64,
                slot,
                shred_index,
            };
            packet
        } else {
            Packet::new(addr, data, slot, shred_index)
        }
    }

    /// Return a packet to the pool
    pub fn put(&self, packet: Packet) {
        let mut pool = self.pool.lock();
        if pool.len() < self.max_size {
            pool.push_back(packet);
        }
    }

    /// Get pool size
    pub fn size(&self) -> usize {
        self.pool.lock().len()
    }
}

impl Default for PacketPool {
    fn default() -> Self {
        Self::new(1000)
    }
}
