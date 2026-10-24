//! One-use authenticated volunteer leases.
//! This module is deliberately not wired to the legacy shell-command worker.

use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lease {
    pub job_id: String,
    pub worker_id: String,
    pub nonce: String,
    pub expires_unix_seconds: u64,
    pub manifest_digest: [u8; 32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaseError {
    Expired,
    WrongWorker,
    WrongJob,
    WrongManifest,
    Replayed,
    EmptyNonce,
}

#[derive(Default)]
pub struct LeaseGuard {
    consumed: HashSet<String>,
}

impl LeaseGuard {
    pub fn accept(
        &mut self,
        lease: &Lease,
        worker_id: &str,
        job_id: &str,
        manifest_digest: &[u8; 32],
        now_unix_seconds: u64,
    ) -> Result<(), LeaseError> {
        if lease.nonce.is_empty() { return Err(LeaseError::EmptyNonce); }
        if lease.worker_id != worker_id { return Err(LeaseError::WrongWorker); }
        if lease.job_id != job_id { return Err(LeaseError::WrongJob); }
        if lease.manifest_digest != *manifest_digest { return Err(LeaseError::WrongManifest); }
        if now_unix_seconds >= lease.expires_unix_seconds { return Err(LeaseError::Expired); }
        // Atomic within this single process. Distributed deployments require
        // a persistent transactional nonce store and authenticated signatures.
        if !self.consumed.insert(lease.nonce.clone()) {
            return Err(LeaseError::Replayed);
        }
        Ok(())
    }
}

pub fn unix_seconds() -> Result<u64, std::time::SystemTimeError> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn lease() -> Lease {
        Lease { job_id: "job-1".into(), worker_id: "node-1".into(),
            nonce: "unique-1".into(), expires_unix_seconds: 100,
            manifest_digest: [7; 32] }
    }
    #[test] fn accepts_once_and_rejects_replay() {
        let mut g = LeaseGuard::default();
        assert_eq!(g.accept(&lease(), "node-1", "job-1", &[7; 32], 99), Ok(()));
        assert_eq!(g.accept(&lease(), "node-1", "job-1", &[7; 32], 99), Err(LeaseError::Replayed));
    }
    #[test] fn rejects_expiry_and_wrong_identity() {
        let mut g = LeaseGuard::default();
        assert_eq!(g.accept(&lease(), "node-1", "job-1", &[7; 32], 100), Err(LeaseError::Expired));
        assert_eq!(g.accept(&lease(), "node-2", "job-1", &[7; 32], 10), Err(LeaseError::WrongWorker));
        assert_eq!(g.accept(&lease(), "node-1", "job-2", &[7; 32], 10), Err(LeaseError::WrongJob));
    }
    #[test] fn rejects_wrong_manifest_and_empty_nonce() {
        let mut g = LeaseGuard::default();
        assert_eq!(g.accept(&lease(), "node-1", "job-1", &[8; 32], 10), Err(LeaseError::WrongManifest));
        let mut l = lease(); l.nonce.clear();
        assert_eq!(g.accept(&l, "node-1", "job-1", &[7; 32], 10), Err(LeaseError::EmptyNonce));
    }
}
