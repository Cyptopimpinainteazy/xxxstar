//! Slash Tracker — Slashing history and risk assessment
//! 
//! Maintains comprehensive slashing event records and calculates
//! risk metrics for validators.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::Result;

/// Slashing event types
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SlashingReason {
    /// Validator was offline during validation
    Offline,
    /// Equivocation (double signing)
    Equivocation,
    /// Misbehavior during consensus
    Misbehavior,
    /// Invalid block production
    InvalidBlocks,
}

/// Single slashing event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashEvent {
    pub event_id: String,
    pub validator: String,
    pub reason: SlashingReason,
    pub era: u32,
    pub amount_slashed: u128,
    pub percentage_slashed: f64,
    pub timestamp: DateTime<Utc>,
    pub recovered: bool,
}

impl SlashEvent {
    /// Days since slashing event
    pub fn days_since(&self) -> i64 {
        (Utc::now() - self.timestamp).num_days()
    }

    /// Est. recovery time in days based on typical patterns
    pub fn estimated_recovery_days(&self) -> u32 {
        match self.reason {
            SlashingReason::Offline => 14,
            SlashingReason::Equivocation => 60,
            SlashingReason::Misbehavior => 90,
            SlashingReason::InvalidBlocks => 45,
        }
    }

    /// Percent of recovery elapsed
    pub fn recovery_progress(&self) -> f64 {
        let days_elapsed = self.days_since() as u32;
        let recovery_time = self.estimated_recovery_days();
        ((days_elapsed as f64 / recovery_time as f64) * 100.0).min(100.0)
    }
}

/// Slash Tracker — Maintains slashing history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashTracker {
    events: HashMap<String, SlashEvent>,
    by_validator: HashMap<String, Vec<String>>,
    event_counter: u32,
}

impl SlashTracker {
    pub fn new() -> Self {
        SlashTracker {
            events: HashMap::new(),
            by_validator: HashMap::new(),
            event_counter: 0,
        }
    }

    /// Record slashing event
    pub fn record_slash(
        &mut self,
        validator: &str,
        reason: SlashingReason,
        era: u32,
        amount: u128,
        percentage: f64,
    ) -> String {
        self.event_counter += 1;
        let event_id = format!("slash_{}", self.event_counter);

        let event = SlashEvent {
            event_id: event_id.clone(),
            validator: validator.to_string(),
            reason,
            era,
            amount_slashed: amount,
            percentage_slashed: percentage,
            timestamp: Utc::now(),
            recovered: false,
        };

        self.events.insert(event_id.clone(), event);
        self.by_validator
            .entry(validator.to_string())
            .or_insert_with(Vec::new)
            .push(event_id.clone());

        event_id
    }

    /// Get slashing event
    pub fn get_event(&self, event_id: &str) -> Option<SlashEvent> {
        self.events.get(event_id).cloned()
    }

    /// Get slashing history for validator
    pub fn validator_slash_history(&self, validator: &str) -> Vec<SlashEvent> {
        self.by_validator
            .get(validator)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.events.get(id))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Total amount slashed for validator
    pub fn total_slashed(&self, validator: &str) -> u128 {
        self.validator_slash_history(validator)
            .iter()
            .map(|e| e.amount_slashed)
            .sum()
    }

    /// Slashing count for validator
    pub fn slash_count(&self, validator: &str) -> u32 {
        self.validator_slash_history(validator).len() as u32
    }

    /// Slashing count in last N days
    pub fn recent_slashes(&self, validator: &str, days: i64) -> Vec<SlashEvent> {
        let cutoff = Utc::now() - chrono::Duration::days(days);
        self.validator_slash_history(validator)
            .into_iter()
            .filter(|e| e.timestamp > cutoff)
            .collect()
    }

    /// Risk assessment (0-100, higher = more risk)
    pub fn validator_risk_score(&self, validator: &str) -> f64 {
        let history = self.validator_slash_history(validator);

        if history.is_empty() {
            return 0.0;
        }

        let mut score = 0.0;

        // History score (weight: 40%)
        let count_score = (history.len() as f64 / 10.0) * 100.0;
        score += (count_score.min(100.0)) * 0.4;

        // Recency score (weight: 30%) - more recent = higher risk
        if let Some(most_recent) = history.last() {
            let days_since = (Utc::now() - most_recent.timestamp).num_days() as f64;
            let recency = 100.0 - (days_since / 365.0 * 100.0).min(100.0);
            score += recency * 0.3;
        }

        // Severity score (weight: 30%)
        let avg_severity = history
            .iter()
            .map(|e| e.percentage_slashed)
            .sum::<f64>()
            / history.len() as f64;
        score += (avg_severity.min(100.0)) * 0.3;

        score.min(100.0)
    }

    /// Is validator risky (score > 50)?
    pub fn is_risky(&self, validator: &str) -> bool {
        self.validator_risk_score(validator) > 50.0
    }

    /// Mark slash event as recovered
    pub fn mark_recovered(&mut self, event_id: &str) -> Result<()> {
        if let Some(event) = self.events.get_mut(event_id) {
            event.recovered = true;
            Ok(())
        } else {
            Err(crate::StakingError::ValidatorNotFound)
        }
    }

    /// Get average percentage per slash
    pub fn average_slash_percentage(&self, validator: &str) -> f64 {
        let history = self.validator_slash_history(validator);
        if history.is_empty() {
            return 0.0;
        }
        history.iter().map(|e| e.percentage_slashed).sum::<f64>() / history.len() as f64
    }

    /// Pagination: Get all slashes for validator
    pub fn all_slashes(&self, validator: &str) -> Vec<SlashEvent> {
        self.validator_slash_history(validator)
    }

    /// Count unrecovered slashes
    pub fn unrecovered_count(&self, validator: &str) -> u32 {
        self.validator_slash_history(validator)
            .iter()
            .filter(|e| !e.recovered)
            .count() as u32
    }

    /// Percentage of validators with slashes
    pub fn network_slash_percentage(&self) -> f64 {
        if self.by_validator.is_empty() {
            return 0.0;
        }
        let slashed_count = self.by_validator.len();
        (slashed_count as f64 / self.by_validator.len() as f64) * 100.0
    }

    /// Slashing frequency (slashes per 365 days)
    pub fn slashing_frequency(&self, validator: &str) -> f64 {
        let history = self.validator_slash_history(validator);
        if history.is_empty() {
            return 0.0;
        }

        if history.len() == 1 {
            return 1.0;
        }

        let oldest = history.first().unwrap();
        let newest = history.last().unwrap();
        let days_span = (newest.timestamp - oldest.timestamp).num_days() as f64;

        if days_span < 1.0 {
            return history.len() as f64;
        }

        (history.len() as f64 / days_span) * 365.0
    }

    /// Total validators with slashing history
    pub fn slashed_validator_count(&self) -> u32 {
        self.by_validator.len() as u32
    }
}

impl Default for SlashTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_slash() {
        let mut tracker = SlashTracker::new();
        let event_id = tracker.record_slash(
            "val1",
            SlashingReason::Offline,
            10,
            1000,
            5.0,
        );

        let event = tracker.get_event(&event_id);
        assert!(event.is_some());
    }

    #[test]
    fn test_slash_event_days_since() {
        let event = SlashEvent {
            event_id: "slash_1".to_string(),
            validator: "val1".to_string(),
            reason: SlashingReason::Offline,
            era: 10,
            amount_slashed: 1000,
            percentage_slashed: 5.0,
            timestamp: Utc::now() - chrono::Duration::days(3),
            recovered: false,
        };

        assert_eq!(event.days_since(), 3);
    }

    #[test]
    fn test_estimated_recovery_time() {
        let offline_event = SlashEvent {
            event_id: "s1".to_string(),
            validator: "v1".to_string(),
            reason: SlashingReason::Offline,
            era: 0,
            amount_slashed: 0,
            percentage_slashed: 0.0,
            timestamp: Utc::now(),
            recovered: false,
        };
        assert_eq!(offline_event.estimated_recovery_days(), 14);

        let equivocation_event = SlashEvent {
            event_id: "s2".to_string(),
            validator: "v2".to_string(),
            reason: SlashingReason::Equivocation,
            era: 0,
            amount_slashed: 0,
            percentage_slashed: 0.0,
            timestamp: Utc::now(),
            recovered: false,
        };
        assert_eq!(equivocation_event.estimated_recovery_days(), 60);
    }

    #[test]
    fn test_validator_slash_history() {
        let mut tracker = SlashTracker::new();
        tracker.record_slash("val1", SlashingReason::Offline, 10, 1000, 5.0);
        tracker.record_slash("val1", SlashingReason::Offline, 20, 1500, 7.0);
        tracker.record_slash("val2", SlashingReason::Equivocation, 15, 2000, 10.0);

        let val1_history = tracker.validator_slash_history("val1");
        assert_eq!(val1_history.len(), 2);

        let val2_history = tracker.validator_slash_history("val2");
        assert_eq!(val2_history.len(), 1);
    }

    #[test]
    fn test_total_slashed() {
        let mut tracker = SlashTracker::new();
        tracker.record_slash("val1", SlashingReason::Offline, 10, 1000, 5.0);
        tracker.record_slash("val1", SlashingReason::Offline, 20, 2000, 10.0);

        assert_eq!(tracker.total_slashed("val1"), 3000);
    }

    #[test]
    fn test_risk_score() {
        let mut tracker = SlashTracker::new();
        tracker.record_slash("val1", SlashingReason::Offline, 10, 1000, 5.0);
        tracker.record_slash("val1", SlashingReason::Offline, 20, 1000, 5.0);

        let risk = tracker.validator_risk_score("val1");
        assert!(risk > 0.0);
    }

    #[test]
    fn test_is_risky() {
        let mut tracker = SlashTracker::new();
        tracker.record_slash("val1", SlashingReason::Equivocation, 10, 1000, 50.0);
        tracker.record_slash("val1", SlashingReason::Equivocation, 20, 1000, 50.0);

        assert!(tracker.is_risky("val1"));
    }

    #[test]
    fn test_mark_recovered() {
        let mut tracker = SlashTracker::new();
        let event_id = tracker.record_slash("val1", SlashingReason::Offline, 10, 1000, 5.0);

        tracker.mark_recovered(&event_id).unwrap();
        let event = tracker.get_event(&event_id).unwrap();
        assert!(event.recovered);
    }

    #[test]
    fn test_recent_slashes() {
        let mut tracker = SlashTracker::new();
        tracker.record_slash("val1", SlashingReason::Offline, 10, 1000, 5.0);

        let recent = tracker.recent_slashes("val1", 1);
        assert_eq!(recent.len(), 1);

        let old_cutoff = tracker.recent_slashes("val1", 99999);
        assert_eq!(old_cutoff.len(), 1);
    }

    #[test]
    fn test_slashing_frequency() {
        let mut tracker = SlashTracker::new();
        tracker.record_slash("val1", SlashingReason::Offline, 1, 1000, 5.0);

        let frequency = tracker.slashing_frequency("val1");
        assert!(frequency > 0.0);
    }

    #[test]
    fn test_unrecovered_count() {
        let mut tracker = SlashTracker::new();
        let id1 = tracker.record_slash("val1", SlashingReason::Offline, 10, 1000, 5.0);
        let id2 = tracker.record_slash("val1", SlashingReason::Offline, 20, 1000, 5.0);

        assert_eq!(tracker.unrecovered_count("val1"), 2);

        tracker.mark_recovered(&id1).unwrap();
        assert_eq!(tracker.unrecovered_count("val1"), 1);
    }
}
