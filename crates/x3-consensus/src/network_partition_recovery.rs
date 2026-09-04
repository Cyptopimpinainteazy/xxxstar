//! Network partition recovery: view-change protocol
//!
//! If 1/3+ validators disconnect, trigger view-change to rotate leadership.
//! Chain resumes with new leader after recovering live validators.

use std::collections::{HashMap, HashSet};
use parking_lot::RwLock;
use std::sync::Arc;

/// View change request
#[derive(Clone, Debug)]
pub struct ViewChangeRequest {
    /// Current view number
    pub current_view: u32,
    /// Requested new view
    pub new_view: u32,
    /// Validator requesting change
    pub validator: String,
    /// Signatures from validators agreeing to change
    pub signatures: Vec<String>,
    /// Round when this change was triggered
    pub triggered_round: u32,
}

impl ViewChangeRequest {
    pub fn new(current_view: u32, validator: String) -> Self {
        Self {
            current_view,
            new_view: current_view + 1,
            validator,
            signatures: Vec::new(),
            triggered_round: 0,
        }
    }

    /// Check if supermajority (2/3) has signed
    pub fn has_quorum(&self, total_validators: u32) -> bool {
        let required = (total_validators * 2) / 3 + 1;
        self.signatures.len() >= required as usize
    }
}

/// Partition recovery state machine
#[derive(Clone, Debug)]
pub enum PartitionState {
    /// Normal operation
    Normal,
    /// Waiting for view change quorum
    ViewChangeInProgress,
    /// Candidate leader elected
    NewLeaderElected(String),
    /// Recovery complete
    Recovered,
}

/// Network partition detector
#[derive(Clone)]
pub struct PartitionDetector {
    /// Total validators in consensus
    total_validators: u32,
    /// Minimum validators for liveness (2/3 + 1)
    min_live_validators: u32,
    /// Current online validators
    online_validators: Arc<RwLock<HashSet<String>>>,
    /// Last heartbeat from each validator
    last_heartbeat: Arc<RwLock<HashMap<String, u64>>>,
    /// Current partition state
    state: Arc<RwLock<PartitionState>>,
    /// Current view
    current_view: Arc<RwLock<u32>>,
    /// Heartbeat timeout in milliseconds
    heartbeat_timeout_ms: u64,
}

impl PartitionDetector {
    pub fn new(total_validators: u32, heartbeat_timeout_ms: u64) -> Self {
        let min_live = (total_validators * 2) / 3 + 1;

        Self {
            total_validators,
            min_live_validators: min_live,
            online_validators: Arc::new(RwLock::new(HashSet::new())),
            last_heartbeat: Arc::new(RwLock::new(HashMap::new())),
            state: Arc::new(RwLock::new(PartitionState::Normal)),
            current_view: Arc::new(RwLock::new(0)),
            heartbeat_timeout_ms,
        }
    }

    /// Record heartbeat from validator
    pub fn record_heartbeat(&self, validator: String, timestamp_ms: u64) {
        let mut online = self.online_validators.write();
        online.insert(validator.clone());

        let mut heartbeats = self.last_heartbeat.write();
        heartbeats.insert(validator, timestamp_ms);

        self.check_liveness(timestamp_ms);
    }

    /// Check if enough validators are online
    fn check_liveness(&self, current_time_ms: u64) {
        let mut heartbeats = self.last_heartbeat.write();
        let mut online = self.online_validators.write();

        // Remove offline validators (heartbeat timeout)
        online.retain(|v| {
            if let Some(last) = heartbeats.get(v) {
                current_time_ms - last < self.heartbeat_timeout_ms
            } else {
                false
            }
        });

        let online_count = online.len() as u32;

        if online_count < self.min_live_validators {
            // Partition detected
            let mut state = self.state.write();
            *state = PartitionState::ViewChangeInProgress;
        } else if online_count >= self.min_live_validators {
            // Network recovered
            let mut state = self.state.write();
            *state = PartitionState::Recovered;
        }
    }

    /// Get current state
    pub fn get_state(&self) -> PartitionState {
        self.state.read().clone()
    }

    /// Get online validator count
    pub fn online_count(&self) -> u32 {
        self.online_validators.read().len() as u32
    }

    /// Get list of online validators
    pub fn get_online_validators(&self) -> Vec<String> {
        self.online_validators.read().iter().cloned().collect()
    }

    /// Trigger view change
    pub fn initiate_view_change(&self) -> ViewChangeRequest {
        let current = *self.current_view.read();
        let req = ViewChangeRequest::new(current, "self".to_string());

        let mut state = self.state.write();
        *state = PartitionState::ViewChangeInProgress;

        req
    }

    /// Process view change votes
    pub fn vote_view_change(&self, request: &mut ViewChangeRequest, voter: String) {
        request.signatures.push(voter);
    }

    /// Complete view change and elect new leader
    pub fn complete_view_change(&self, request: ViewChangeRequest) -> Option<String> {
        if !request.has_quorum(self.total_validators) {
            return None;
        }

        // Elect new leader deterministically (e.g., request.new_view % validator_count)
        let online = self.online_validators.read();
        let online_vec: Vec<_> = online.iter().cloned().collect();

        if online_vec.is_empty() {
            return None;
        }

        let leader_idx = (request.new_view as usize) % online_vec.len();
        let new_leader = online_vec[leader_idx].clone();

        let mut view = self.current_view.write();
        *view = request.new_view;

        let mut state = self.state.write();
        *state = PartitionState::NewLeaderElected(new_leader.clone());

        Some(new_leader)
    }
}

/// View change orchestrator (in consensus engine)
#[derive(Clone)]
pub struct ViewChangeOrchestrator {
    detector: PartitionDetector,
    pending_requests: Arc<RwLock<Vec<ViewChangeRequest>>>,
}

impl ViewChangeOrchestrator {
    pub fn new(detector: PartitionDetector) -> Self {
        Self {
            detector,
            pending_requests: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Process incoming heartbeat and trigger recovery if needed
    pub fn process_heartbeat(&self, validator: String, timestamp_ms: u64) -> Option<String> {
        self.detector.record_heartbeat(validator, timestamp_ms);

        let state = self.detector.get_state();
        match state {
            PartitionState::ViewChangeInProgress => {
                let request = self.detector.initiate_view_change();
                let mut pending = self.pending_requests.write();
                pending.push(request);
                None
            }
            PartitionState::NewLeaderElected(leader) => Some(leader),
            PartitionState::Recovered => {
                // Clear pending requests
                let mut pending = self.pending_requests.write();
                pending.clear();
                None
            }
            PartitionState::Normal => None,
        }
    }

    /// Get partition status
    pub fn get_partition_status(&self) -> (u32, u32) {
        (self.detector.online_count(), self.detector.total_validators)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_partition_detector_creation() {
        let detector = PartitionDetector::new(10, 5000);
        assert_eq!(detector.online_count(), 0);
        assert_eq!(detector.min_live_validators, 7); // 2/3 of 10 = 6.66 → 7
    }

    #[test]
    fn test_heartbeat_recording() {
        let detector = PartitionDetector::new(10, 5000);

        detector.record_heartbeat("validator1".to_string(), 1000);
        assert_eq!(detector.online_count(), 1);
    }

    #[test]
    fn test_partition_detection() {
        let detector = PartitionDetector::new(10, 5000);

        // Add 6 live validators (need 7 for 2/3)
        for i in 0..6 {
            detector.record_heartbeat(format!("validator{}", i), 1000);
        }

        // Check: not enough for liveness
        detector.check_liveness(2000);
        match detector.get_state() {
            PartitionState::ViewChangeInProgress => (),
            _ => panic!("Should detect partition"),
        }
    }

    #[test]
    fn test_recovery_from_partition() {
        let detector = PartitionDetector::new(10, 5000);

        for i in 0..6 {
            detector.record_heartbeat(format!("validator{}", i), 1000);
        }

        // Add 7th validator
        detector.record_heartbeat("validator6".to_string(), 1000);

        match detector.get_state() {
            PartitionState::Recovered => (),
            _ => panic!("Should be recovered"),
        }
    }

    #[test]
    fn test_view_change_request_quorum() {
        let mut req = ViewChangeRequest::new(1, "validator0".to_string());

        for i in 0..7 {
            req.signatures.push(format!("validator{}", i));
        }

        assert!(req.has_quorum(10)); // 7/10 > 2/3
    }

    #[test]
    fn test_view_change_orchestrator() {
        let detector = PartitionDetector::new(10, 5000);
        let orch = ViewChangeOrchestrator::new(detector);

        for i in 0..7 {
            orch.process_heartbeat(format!("validator{}", i), 1000);
        }

        let (online, total) = orch.get_partition_status();
        assert_eq!(online, 7);
        assert_eq!(total, 10);
    }

    #[test]
    fn test_view_change_election() {
        let detector = PartitionDetector::new(5, 5000);

        for i in 0..4 {
            detector.record_heartbeat(format!("validator{}", i), 1000);
        }

        let mut req = ViewChangeRequest::new(0, "validator0".to_string());
        for i in 0..3 {
            req.signatures.push(format!("validator{}", i));
        }
        req.signatures.push("validator0".to_string());

        let leader = detector.complete_view_change(req);
        assert!(leader.is_some());
    }
}
