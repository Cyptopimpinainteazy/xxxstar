//! VM event system: typed events emitted during X3VM execution.
//!
//! Events are buffered during execution and flushed to the host after successful
//! commit. On rollback the buffer is discarded. This provides atomicity: events
//! are only observable if the execution succeeded.

/// An event topic: 32-byte hash.
pub type Topic = [u8; 32];

/// Raw event data (variable-length).
pub type EventData = Vec<u8>;

/// A single event emitted by VM execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VmEvent {
    /// Contract address that emitted the event.
    pub emitter: [u8; 32],
    /// Up to 4 indexed topics.
    pub topics: Vec<Topic>,
    /// Non-indexed event payload.
    pub data: EventData,
}

/// Errors from event operations.
#[derive(Debug, PartialEq, Eq)]
pub enum EventError {
    /// More than 4 topics specified (EVM-compatible limit).
    TooManyTopics,
    /// Event data payload exceeds maximum size.
    DataTooLarge,
    /// Tried to flush on rollback (should not happen in normal flow).
    FlushOnRollback,
}

/// Maximum topics per event (EVM-compatible).
pub const MAX_TOPICS: usize = 4;
/// Maximum event data size in bytes.
pub const MAX_DATA_SIZE: usize = 4_096;

/// Buffered event log for a single execution context.
pub struct EventBuffer {
    /// Events accumulated during execution.
    pending: Vec<VmEvent>,
    /// Committed events (visible to host after flush).
    committed: Vec<VmEvent>,
}

impl EventBuffer {
    pub fn new() -> Self {
        Self {
            pending: Vec::new(),
            committed: Vec::new(),
        }
    }

    /// Emit an event into the pending buffer.
    pub fn emit(
        &mut self,
        emitter: [u8; 32],
        topics: Vec<Topic>,
        data: EventData,
    ) -> Result<(), EventError> {
        if topics.len() > MAX_TOPICS {
            return Err(EventError::TooManyTopics);
        }
        if data.len() > MAX_DATA_SIZE {
            return Err(EventError::DataTooLarge);
        }
        self.pending.push(VmEvent {
            emitter,
            topics,
            data,
        });
        Ok(())
    }

    /// Commit pending events (called on successful execution).
    pub fn commit(&mut self) {
        self.committed.extend(self.pending.drain(..));
    }

    /// Discard pending events (called on rollback).
    pub fn rollback(&mut self) {
        self.pending.clear();
    }

    /// Drain all committed events for delivery to the host.
    pub fn drain_committed(&mut self) -> Vec<VmEvent> {
        core::mem::take(&mut self.committed)
    }

    /// Number of pending (uncommitted) events.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Number of committed events awaiting delivery.
    pub fn committed_count(&self) -> usize {
        self.committed.len()
    }
}

impl Default for EventBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(b: u8) -> [u8; 32] {
        [b; 32]
    }
    fn topic(b: u8) -> Topic {
        [b; 32]
    }

    #[test]
    fn test_emit_and_commit() {
        let mut buf = EventBuffer::new();
        buf.emit(addr(1), vec![topic(0xAA)], vec![1, 2, 3]).unwrap();
        assert_eq!(buf.pending_count(), 1);
        buf.commit();
        assert_eq!(buf.pending_count(), 0);
        assert_eq!(buf.committed_count(), 1);
        let events = buf.drain_committed();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, vec![1, 2, 3]);
    }

    #[test]
    fn test_rollback_discards_pending() {
        let mut buf = EventBuffer::new();
        buf.emit(addr(1), vec![], vec![0xFF]).unwrap();
        buf.rollback();
        assert_eq!(buf.pending_count(), 0);
        assert_eq!(buf.committed_count(), 0);
    }

    #[test]
    fn test_too_many_topics_rejected() {
        let mut buf = EventBuffer::new();
        let topics = vec![topic(1), topic(2), topic(3), topic(4), topic(5)];
        assert_eq!(
            buf.emit(addr(1), topics, vec![]),
            Err(EventError::TooManyTopics)
        );
    }

    #[test]
    fn test_data_too_large_rejected() {
        let mut buf = EventBuffer::new();
        let data = vec![0u8; MAX_DATA_SIZE + 1];
        assert_eq!(
            buf.emit(addr(1), vec![], data),
            Err(EventError::DataTooLarge)
        );
    }

    #[test]
    fn test_committed_events_preserved_across_rollback() {
        let mut buf = EventBuffer::new();
        buf.emit(addr(1), vec![], vec![1]).unwrap();
        buf.commit();
        buf.emit(addr(2), vec![], vec![2]).unwrap();
        buf.rollback();
        assert_eq!(buf.committed_count(), 1);
        let events = buf.drain_committed();
        assert_eq!(events[0].data, vec![1]);
    }
}
