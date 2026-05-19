//! Vote aggregation — tallying sealed votes, computing outcomes.

use super::session::JurySession;
use super::voting::Tally;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Result of aggregating votes for all tasks in a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationResult {
    /// Session ID.
    pub session_id: String,
    /// Per-task results.
    pub task_results: Vec<TaskVoteResult>,
    /// Overall session statistics.
    pub stats: AggregationStats,
    /// When aggregation was performed.
    pub aggregated_at: DateTime<Utc>,
}

/// Vote result for a single task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskVoteResult {
    /// Task ID.
    pub task_id: String,
    /// The tally.
    pub tally: Tally,
    /// Whether the task passed.
    pub approved: bool,
}

/// Aggregation statistics across the session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationStats {
    /// Total tasks voted on.
    pub total_tasks: usize,
    /// Tasks approved.
    pub tasks_approved: usize,
    /// Tasks rejected.
    pub tasks_rejected: usize,
    /// Average participation rate.
    pub avg_participation: f64,
}

/// The vote aggregator — computes outcomes from sealed ballot boxes.
pub struct VoteAggregator;

impl VoteAggregator {
    /// Aggregate all votes in a jury session.
    ///
    /// This method takes the sealed vote counts (yes/no) for each task.
    /// In a real system, these counts come from the off-chain execution
    /// environment which tallies commitments without revealing individual votes.
    pub fn aggregate(
        session: &mut JurySession,
        vote_counts: &[(String, u32, u32)], // (task_id, yes_count, no_count)
    ) -> AggregationResult {
        let mut task_results = Vec::new();
        let mut approved_count = 0;
        let mut total_participation = 0u32;
        let total_members = session.members.len() as u32;

        for (task_id, yes_count, no_count) in vote_counts {
            if let Some(ballot) = session.get_ballot_mut(task_id) {
                let tally = ballot.tally_from_sealed(*yes_count, *no_count);
                if tally.approved {
                    approved_count += 1;
                }
                total_participation += tally.total_cast;

                task_results.push(TaskVoteResult {
                    task_id: task_id.clone(),
                    tally,
                    approved: tally.approved,
                });
            }
        }

        let total_tasks = task_results.len();
        let avg_participation = if total_tasks > 0 && total_members > 0 {
            total_participation as f64 / (total_tasks as f64 * total_members as f64)
        } else {
            0.0
        };

        AggregationResult {
            session_id: session.session_id.clone(),
            task_results,
            stats: AggregationStats {
                total_tasks,
                tasks_approved: approved_count,
                tasks_rejected: total_tasks - approved_count,
                avg_participation,
            },
            aggregated_at: Utc::now(),
        }
    }

    /// Apply aggregation results to the task queue — approve or reject tasks.
    pub fn apply_to_queue(
        result: &AggregationResult,
        queue: &mut crate::task::TaskQueue,
    ) {
        for task_result in &result.task_results {
            if task_result.approved {
                queue.jury_approve(&task_result.task_id);
            } else {
                queue.jury_reject(&task_result.task_id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jury::session::{JurySession, SessionConfig};
    use crate::agent::identity::OrchestraSection;

    fn make_session_with_tasks() -> JurySession {
        let mut session = JurySession::new(
            "test-session".into(),
            SessionConfig {
                min_jury_size: 3,
                ..Default::default()
            },
        );

        session.add_member(1, OrchestraSection::Strings, false).unwrap();
        session.add_member(2, OrchestraSection::Brass, true).unwrap();
        session.add_member(3, OrchestraSection::Percussion, false).unwrap();

        session.add_task("task-001".into());
        session.add_task("task-002".into());

        session.start_voting().unwrap();
        session
    }

    #[test]
    fn aggregate_majority_yes() {
        let mut session = make_session_with_tasks();
        session.close_voting().unwrap();

        let vote_counts = vec![
            ("task-001".into(), 2, 1), // 2 yes, 1 no → approved
            ("task-002".into(), 1, 2), // 1 yes, 2 no → rejected
        ];

        let result = VoteAggregator::aggregate(&mut session, &vote_counts);

        assert_eq!(result.task_results.len(), 2);
        assert!(result.task_results[0].approved);
        assert!(!result.task_results[1].approved);
        assert_eq!(result.stats.tasks_approved, 1);
        assert_eq!(result.stats.tasks_rejected, 1);
    }

    #[test]
    fn aggregate_all_approved() {
        let mut session = make_session_with_tasks();
        session.close_voting().unwrap();

        let vote_counts = vec![
            ("task-001".into(), 3, 0),
            ("task-002".into(), 2, 1),
        ];

        let result = VoteAggregator::aggregate(&mut session, &vote_counts);
        assert_eq!(result.stats.tasks_approved, 2);
        assert_eq!(result.stats.tasks_rejected, 0);
    }
}
