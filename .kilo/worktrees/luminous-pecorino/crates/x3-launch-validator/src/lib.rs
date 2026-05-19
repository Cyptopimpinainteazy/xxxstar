//! # X3 Adversarial World Launch Checklist Validator
//!
//! Validates all pre-launch, launch-day, and post-launch conditions from the
//! X3 Master Architecture & Launch Spec (vΩ-1.0).
//!
//! ## Usage
//!
//! ```bash
//! cargo run -p x3-launch-validator -- --check all
//! cargo run -p x3-launch-validator -- --check pre-launch
//! cargo run -p x3-launch-validator -- --check launch-day
//! cargo run -p x3-launch-validator -- --check failure-conditions
//! ```

pub mod checklist;
pub mod checks;
pub mod reporter;

pub use checklist::{CheckItem, CheckPhase, CheckResult, LaunchChecklist};
pub use reporter::ChecklistReporter;
