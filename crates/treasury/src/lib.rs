#![cfg_attr(not(feature = "std"), no_std)]

//! Treasury utilities for X3 Chain.

pub mod error;
pub mod types;

pub use error::TreasuryError;
pub use types::*;
