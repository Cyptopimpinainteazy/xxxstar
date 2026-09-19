//! # Orchestra — the multi-layered agent system
//!
//! The Orchestra is the agent layer of X3 Chain. It is built out of three
//! cooperating tiers:
//!
//! - **On-chain agents** ([`agent::on_chain`]) execute tasks and are bound to
//!   the protocol: they cannot act outside [`score::Commandment`]s.
//! - **Off-chain agents** ([`agent::off_chain`]) sit on a read-only snapshot of
//!   chain state, run jury duty, adversarial audit and stress scenarios, and
//!   never write to chain state themselves.
//! - **Juries** ([`jury`]) decide the fate of major tasks through anonymous
//!   commit-reveal voting, with rotation ([`rotation`]) deciding who sits on
//!   which jury, and the scrap yard ([`scrap`]) retiring agents that fall out
//!   of alignment.
//!
//! The rules those tiers are held to are the ten immutable commandments in
//! [`score`]. They are constants: no agent, jury or proposal can alter them.
//!
//! Every decision boundary writes to [`audit`], which is the on-chain log of
//! record for what an agent did and why.
//!
//! ## What this crate does not do
//!
//! The Orchestra does not ship a compute backend. [`task::TaskExecutor`] takes
//! a [`task::TaskDispatcher`] from its caller and refuses to report an execution
//! it did not perform; there is no path in this crate that reports a task as
//! executed without a dispatcher behind it. Likewise
//! [`agent::off_chain::OffChainAgent::record_stress_result`] records an outcome
//! the caller measured — it does not invent one.

#![deny(unsafe_code)]

pub mod agent;
pub mod audit;
pub mod jury;
pub mod rotation;
pub mod score;
pub mod scrap;
pub mod task;
