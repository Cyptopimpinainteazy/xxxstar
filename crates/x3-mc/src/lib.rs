//! # x3-mc — probabilistic adversarial simulation engine
//!
//! Monte Carlo is not a replacement for proof, deterministic tests, fuzzing or
//! real validator testing. It is an extra search layer over the state space
//! those do not reach: it samples a scenario, runs it, judges it, and — when it
//! finds something — hands back a seed that reproduces it exactly.
//!
//! ```text
//! distributions -> run seed -> sample -> EXECUTE -> invariants -> failure?
//!                                                       |            |
//!                                                      no           yes
//!                                                       |            |
//!                                                    next run    save seed
//!                                                                   |
//!                                                             replay + minimize
//!                                                                   |
//!                                                            regression test
//! ```
//!
//! The execution step is real code. `scenarios::AtomicSettlement` drives
//! `x3_cross_vm_coordinator::SwapCoordinator` through `x3_sim::run_with` — the
//! same entry point the deterministic fixtures use, against the same
//! coordinator. The engine contributes randomness and bookkeeping, never
//! semantics.
//!
//! ## What a passing campaign does and does not mean
//!
//! A campaign that completes a million runs has sampled a million scenarios
//! from the distributions it was given. [`CampaignReport::failure_rate`]
//! reports the observed rate with an interval, and the evidence bundle records
//! what was not covered. No run count establishes that a failure is impossible.

#![deny(unsafe_code)]
#![deny(missing_debug_implementations)]

pub mod campaign;
pub mod distribution;
pub mod engine;
pub mod evidence;
pub mod minimize;
pub mod rng;
pub mod scenarios;

pub use campaign::{Campaign, CampaignMeta, SeedSpec};
pub use distribution::Distribution;
pub use engine::{
    execute_one, run_campaign, sample_params, wilson, CampaignConfig, CampaignReport, Failure,
    MonteCarloScenario, ParamSet, RunOutcome, StopReason, ViolationReport,
};
pub use evidence::EvidenceBundle;
pub use minimize::{cluster, minimize, MinimizeResult};
pub use rng::{derive_seed, McRng};
pub use scenarios::AtomicSettlement;
