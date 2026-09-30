//! The campaigns.
//!
//! Each one is a thin mapping from sampled variables onto a real execution
//! path plus the invariants that decide whether the run held. A campaign that
//! cannot reach real code does not belong here — it would produce run counts
//! with nothing behind them.

pub mod atomic;

pub use atomic::AtomicSettlement;
