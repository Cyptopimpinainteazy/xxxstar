//! # x3-sim — deterministic fault-injection simulator
//!
//! X3's atomic swap coordinator has been tested with unit tests that call one
//! operation at a time. That cannot express the failures that actually hurt:
//! a claim racing a refund across a partition, a duplicate delivery after a
//! restart, a persisted write that disappears.
//!
//! `x3-sim` runs the **real** `x3-cross-vm-coordinator` state machine against a
//! virtual clock, a seeded RNG, a lossy/partitionable network and a scheduled
//! fault plan. A run is pinned by its seed, and a failing run prints the exact
//! `cargo run` line that reproduces it.
//!
//! ```text
//! cargo run -p x3-sim -- --seed 948218671 --scenario partition-storm
//! ```
//!
//! The simulator never re-implements a state transition. If it did, a passing
//! run would only prove that the simulator agrees with itself. All semantics
//! come from the coordinator crate; the simulator chooses the order of calls
//! and judges the resulting state with [`invariants`].

#![deny(unsafe_code)]
#![deny(missing_debug_implementations)]

pub mod clock;
pub mod faults;
pub mod invariants;
pub mod network;
pub mod params;
pub mod rng;
pub mod sim;

pub use clock::VirtualClock;
pub use faults::{FaultEvent, FaultKind, FaultPlan};
pub use invariants::{check_session, check_sessions, Violation};
pub use network::{NetworkStats, VirtualNetwork};
pub use params::SimParams;
pub use rng::SimRng;
pub use sim::{run, run_with, Scenario, SimConfig, SimOp, SimOutcome, START_UNIX_MS};
