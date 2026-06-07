//! Concrete chain adapters. Real network integrations replace the mock
//! bodies in production builds; the trait surface stays identical.

pub mod evm;
pub mod svm;
pub mod x3vm;
