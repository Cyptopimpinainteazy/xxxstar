//! X3 Chain Primitives
//!
//! Shared types and traits used across the X3 Chain runtime and pallets.

#![cfg_attr(not(feature = "std"), no_std)]

pub mod keyring;

pub use keyring::*;