//! Verdict types and helpers.

use crate::types::{VerdictOutcome, VerdictRecord};

/// Verdict analysis and reporting.
pub struct Verdict;

impl Verdict {
    /// Check if a verdict results in slashing.
    pub fn requires_slashing(verdict: &VerdictRecord) -> bool {
        verdict.outcome == VerdictOutcome::Guilty
    }

    /// Check if a verdict acquits the respondent.
    pub fn is_acquittal(verdict: &VerdictRecord) -> bool {
        verdict.outcome == VerdictOutcome::NotGuilty
    }

    /// Check if a verdict dismisses the dispute.
    pub fn is_dismissal(verdict: &VerdictRecord) -> bool {
        verdict.outcome == VerdictOutcome::InvalidDispute
    }

    /// Generate a human-readable summary of the verdict.
    pub fn summary(verdict: &VerdictRecord) -> String {
        match verdict.outcome {
            VerdictOutcome::Guilty => format!(
                "GUILTY — Dispute #{}: Respondent found in violation. Slashing enforced. Finalized at block {}.",
                verdict.dispute_id.0,
                verdict.rendered_at,
            ),
            VerdictOutcome::NotGuilty => format!(
                "NOT GUILTY — Dispute #{}: Execution replay confirmed deterministic. No action taken. Finalized at block {}.",
                verdict.dispute_id.0,
                verdict.rendered_at,
            ),
            VerdictOutcome::InvalidDispute => format!(
                "DISMISSED — Dispute #{}: Filing is invalid or malformed. No action taken. Finalized at block {}.",
                verdict.dispute_id.0,
                verdict.rendered_at,
            ),
        }
    }
}
