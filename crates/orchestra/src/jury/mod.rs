//! Jury subsystem — session management, anonymous voting, rotation, aggregation.

pub mod aggregation;
pub mod rotation;
pub mod session;
pub mod voting;

pub use aggregation::{AggregationResult, VoteAggregator};
pub use rotation::JuryRotation;
pub use session::{JurySession, SessionConfig, SessionStatus};
pub use voting::{BallotBox, RevealedVote, Vote};
