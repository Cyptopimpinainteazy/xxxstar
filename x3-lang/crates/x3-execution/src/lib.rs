//! Canonical trading semantics shared by developer execution and validator WASM.
#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;
pub mod artifact;
pub mod ir;
pub mod profit;
pub mod trading;
pub mod verify;
pub mod spec {
    pub mod opcodes {
        use alloc::{
            format,
            string::{String, ToString},
            vec::Vec,
        };
        include!("../../../spec/opcodes.rs");
    }
}
