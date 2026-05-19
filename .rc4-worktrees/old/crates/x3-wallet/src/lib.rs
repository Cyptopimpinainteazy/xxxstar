#![cfg_attr(not(feature = "std"), no_std)]
#![allow(unused, dead_code, deprecated)]

use sp_std::vec;

// x3-wallet: Multi-featured wallet management system
// Provides institutional-grade wallet security, DeFi integration, and institutional features

pub mod address_book;
pub mod approval_manager;
pub mod biometric_unlock;
pub mod defi_tracker;
pub mod hardware_wallet;
pub mod multisig_wallet;
pub mod privacy_mixing;
pub mod social_recovery;
pub mod token_manager;
pub mod transaction_signer;

// Re-export key types
pub use address_book::{AddressBook, ContactInfo};
pub use approval_manager::{ApprovalPolicy, TransactionApproval};
pub use biometric_unlock::{BiometricProfile, UnlockSession};
pub use defi_tracker::{BorrowPosition, DeFiPortfolio, LPPosition, StakingPosition};
pub use hardware_wallet::{DeviceInfo, HardwareSignature, HardwareSigningRequest, HardwareWallet};
pub use multisig_wallet::{MultisigProposal, MultisigWallet, SignerApproval};
pub use privacy_mixing::{MixingPool, MixingTransaction};
pub use social_recovery::{GuardianAccount, GuardianApproval, RecoveryRequest};
pub use token_manager::{Token, TokenBalance, TokenWhitelist};
pub use transaction_signer::{SigningRequest, SigningTransaction, TransactionSignature};
