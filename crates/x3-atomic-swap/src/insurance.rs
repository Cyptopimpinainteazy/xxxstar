//! # Insurance Fund
//!
//! Protects users against relayers failing to claim within the timeout window,
//! solvers producing stale fills, and chain-reorg events that invalidate proofs.
//!
//! The fund is denominated in a canonical stablecoin (USDC) and replenished by
//! slashing proceeds and protocol fees.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

/// Reason a claim was made against the insurance fund.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InsuranceClaimReason {
    /// Relayer failed to claim before timeout.
    RelayerMissedClaim,
    /// Solver produced a stale fill that lost value.
    StaleQuoteLoss,
    /// Chain reorg invalidated a previously confirmed proof.
    ReorgInvalidatedProof,
    /// Destination chain transaction failed irrecoverably.
    DestinationTxFailure,
    /// Bridge relay failure (source locked, destination never received).
    BridgeRelayFailure,
}

impl InsuranceClaimReason {
    pub fn display_name(&self) -> &'static str {
        match self {
            InsuranceClaimReason::RelayerMissedClaim => "relayer_missed_claim",
            InsuranceClaimReason::StaleQuoteLoss => "stale_quote_loss",
            InsuranceClaimReason::ReorgInvalidatedProof => "reorg_invalidated_proof",
            InsuranceClaimReason::DestinationTxFailure => "destination_tx_failure",
            InsuranceClaimReason::BridgeRelayFailure => "bridge_relay_failure",
        }
    }
}

/// Status of an insurance claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClaimStatus {
    /// Claim filed, under review.
    Pending,
    /// Claim approved, payout processed.
    Approved,
    /// Claim rejected.
    Rejected,
}

/// An insurance claim record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsuranceClaim {
    /// Unique claim ID.
    pub claim_id: u64,
    /// Intent ID this claim relates to.
    pub intent_id: u64,
    /// Reason for the claim.
    pub reason: InsuranceClaimReason,
    /// Amount claimed.
    pub amount: u128,
    /// Address to receive the payout.
    pub beneficiary: String,
    /// Evidence supporting the claim.
    pub evidence: Vec<u8>,
    /// Current status.
    pub status: ClaimStatus,
    /// When the claim was filed.
    pub filed_at: u64,
    /// When the claim was resolved.
    pub resolved_at: Option<u64>,
}

/// The insurance fund state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsuranceFund {
    /// Current fund balance.
    pub balance: u128,
    /// Total paid out across all approved claims.
    pub total_paid_out: u128,
    /// Total received from slashing proceeds.
    pub total_received_from_slashing: u128,
    /// All claims filed against the fund.
    pub claims: BTreeMap<u64, InsuranceClaim>,
    /// Next claim ID to assign.
    pub next_claim_id: u64,
    /// Minimum balance before the fund triggers a rebalance warning.
    pub minimum_balance: u128,
}

impl InsuranceFund {
    /// Create a new insurance fund with an initial balance.
    pub fn new(initial_balance: u128) -> Self {
        Self {
            balance: initial_balance,
            total_paid_out: 0,
            total_received_from_slashing: 0,
            claims: BTreeMap::new(),
            next_claim_id: 1,
            minimum_balance: initial_balance / 10,
        }
    }

    /// Deposit slashing proceeds into the fund.
    pub fn deposit_slashing_proceeds(&mut self, amount: u128) {
        self.balance += amount;
        self.total_received_from_slashing += amount;
    }

    /// File a new insurance claim.
    ///
    /// Returns the claim ID.
    pub fn file_claim(
        &mut self,
        intent_id: u64,
        reason: InsuranceClaimReason,
        amount: u128,
        beneficiary: String,
        evidence: Vec<u8>,
    ) -> u64 {
        let claim_id = self.next_claim_id;
        self.next_claim_id += 1;

        self.claims.insert(
            claim_id,
            InsuranceClaim {
                claim_id,
                intent_id,
                reason,
                amount,
                beneficiary,
                evidence,
                status: ClaimStatus::Pending,
                filed_at: 0,
                resolved_at: None,
            },
        );
        claim_id
    }

    /// Approve a claim and pay out the beneficiary.
    ///
    /// Reduces the fund balance. Returns an error if the fund has insufficient
    /// balance — the claim remains Pending in that case.
    pub fn approve_claim(&mut self, claim_id: u64) -> Result<(), &'static str> {
        let claim = self.claims.get(&claim_id).ok_or("claim not found")?;
        if claim.status != ClaimStatus::Pending {
            return Err("claim is not in pending status");
        }
        let amount = claim.amount;
        if self.balance < amount {
            return Err("insufficient fund balance");
        }

        self.balance -= amount;
        self.total_paid_out += amount;

        let claim = self.claims.get_mut(&claim_id).unwrap();
        claim.status = ClaimStatus::Approved;
        claim.resolved_at = Some(1);
        Ok(())
    }

    /// Reject a claim (no payout).
    pub fn reject_claim(&mut self, claim_id: u64) -> Result<(), &'static str> {
        let claim = self.claims.get_mut(&claim_id).ok_or("claim not found")?;
        if claim.status != ClaimStatus::Pending {
            return Err("claim is not in pending status");
        }
        claim.status = ClaimStatus::Rejected;
        claim.resolved_at = Some(1);
        Ok(())
    }

    /// Check if the fund is below its minimum balance.
    pub fn needs_rebalance(&self) -> bool {
        self.balance < self.minimum_balance
    }

    /// Get total pending claim amount.
    pub fn pending_claim_amount(&self) -> u128 {
        self.claims
            .values()
            .filter(|c| c.status == ClaimStatus::Pending)
            .map(|c| c.amount)
            .sum()
    }

    /// Get a summary of the fund state.
    pub fn summary(&self) -> InsuranceFundSummary {
        let pending = self
            .claims
            .values()
            .filter(|c| c.status == ClaimStatus::Pending)
            .count() as u64;
        let approved = self
            .claims
            .values()
            .filter(|c| c.status == ClaimStatus::Approved)
            .count() as u64;
        let rejected = self
            .claims
            .values()
            .filter(|c| c.status == ClaimStatus::Rejected)
            .count() as u64;

        InsuranceFundSummary {
            balance: self.balance,
            total_paid_out: self.total_paid_out,
            total_received_from_slashing: self.total_received_from_slashing,
            pending_claims: pending,
            approved_claims: approved,
            rejected_claims: rejected,
            pending_amount: self.pending_claim_amount(),
            needs_rebalance: self.needs_rebalance(),
        }
    }
}

/// Summary of the insurance fund state for dashboard display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsuranceFundSummary {
    pub balance: u128,
    pub total_paid_out: u128,
    pub total_received_from_slashing: u128,
    pub pending_claims: u64,
    pub approved_claims: u64,
    pub rejected_claims: u64,
    pub pending_amount: u128,
    pub needs_rebalance: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fund_creation() {
        let fund = InsuranceFund::new(1_000_000);
        assert_eq!(fund.balance, 1_000_000);
        assert_eq!(fund.total_paid_out, 0);
        assert_eq!(fund.minimum_balance, 100_000);
        assert!(!fund.needs_rebalance());
    }

    #[test]
    fn test_deposit_slashing() {
        let mut fund = InsuranceFund::new(1_000_000);
        fund.deposit_slashing_proceeds(50_000);
        assert_eq!(fund.balance, 1_050_000);
        assert_eq!(fund.total_received_from_slashing, 50_000);
    }

    #[test]
    fn test_file_and_approve_claim() {
        let mut fund = InsuranceFund::new(1_000_000);
        let id = fund.file_claim(
            42,
            InsuranceClaimReason::RelayerMissedClaim,
            100_000,
            "0xbeneficiary".into(),
            vec![0u8; 16],
        );
        assert_eq!(id, 1);

        fund.approve_claim(id).unwrap();
        assert_eq!(fund.balance, 900_000);
        assert_eq!(fund.total_paid_out, 100_000);

        let claim = fund.claims.get(&id).unwrap();
        assert_eq!(claim.status, ClaimStatus::Approved);
    }

    #[test]
    fn test_reject_claim() {
        let mut fund = InsuranceFund::new(1_000_000);
        let id = fund.file_claim(
            1,
            InsuranceClaimReason::StaleQuoteLoss,
            50_000,
            "0xuser".into(),
            vec![],
        );
        fund.reject_claim(id).unwrap();
        assert_eq!(fund.balance, 1_000_000); // no payout
        assert_eq!(fund.claims.get(&id).unwrap().status, ClaimStatus::Rejected);
    }

    #[test]
    fn test_insufficient_balance() {
        let mut fund = InsuranceFund::new(10_000);
        let id = fund.file_claim(
            1,
            InsuranceClaimReason::BridgeRelayFailure,
            100_000,
            "0xuser".into(),
            vec![],
        );
        let result = fund.approve_claim(id);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("insufficient"));
    }

    #[test]
    fn test_needs_rebalance() {
        let mut fund = InsuranceFund::new(1_000_000);
        assert!(!fund.needs_rebalance());

        // Pay out 950k, leaving 50k — below 100k minimum
        let id = fund.file_claim(
            1,
            InsuranceClaimReason::ReorgInvalidatedProof,
            950_000,
            "0xuser".into(),
            vec![],
        );
        fund.approve_claim(id).unwrap();
        assert!(fund.needs_rebalance());
    }

    #[test]
    fn test_summary() {
        let mut fund = InsuranceFund::new(1_000_000);
        fund.file_claim(
            1,
            InsuranceClaimReason::RelayerMissedClaim,
            100_000,
            "a".into(),
            vec![],
        );
        let id2 = fund.file_claim(
            2,
            InsuranceClaimReason::DestinationTxFailure,
            200_000,
            "b".into(),
            vec![],
        );
        fund.approve_claim(id2).unwrap();

        let summary = fund.summary();
        assert_eq!(summary.balance, 800_000);
        assert_eq!(summary.total_paid_out, 200_000);
        assert_eq!(summary.pending_claims, 1);
        assert_eq!(summary.approved_claims, 1);
        assert_eq!(summary.pending_amount, 100_000);
    }
}
