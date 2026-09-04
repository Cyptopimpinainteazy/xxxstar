/// Multisig Wallet Engine — M-of-N signature consensus for DAO treasuries
/// Threshold-based approvals with timelock and quorum tracking
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use scale_info::TypeInfo;
#[allow(unused_imports)]
use sp_std::vec;
use sp_std::vec::Vec;

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, Debug, PartialEq, Eq)]
pub struct MultisigWallet {
    pub id: [u8; 32],
    pub signers: Vec<[u8; 32]>, // list of authorized signers
    pub threshold: u32,         // M in M-of-N (minimum signatures required)
    pub owner: [u8; 32],
    pub created_block: u64,
    pub timelock_delay: u64, // blocks to wait before execution
    pub is_active: bool,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, Debug, PartialEq, Eq)]
pub struct MultisigProposal {
    pub id: [u8; 32],
    pub wallet_id: [u8; 32],
    pub proposer: [u8; 32],
    pub transaction_hash: [u8; 32],
    pub target: [u8; 32],
    pub value: u128,
    pub data: Vec<u8>,
    pub created_block: u64,
    pub execution_block: u64,      // earliest block this can execute
    pub signatures: Vec<[u8; 32]>, // list of signers who approved
    pub status: u8,                // 0=pending, 1=approved (threshold met), 2=executed, 3=cancelled
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, Debug, PartialEq, Eq)]
pub struct SignerApproval {
    pub signer: [u8; 32],
    pub proposal_id: [u8; 32],
    pub timestamp: u64,
    pub approval_hash: [u8; 32],
}

pub struct MultisigWalletEngine;

impl MultisigWalletEngine {
    /// Create a new multisig wallet (M-of-N)
    pub fn create_multisig(
        signers: Vec<[u8; 32]>,
        threshold: u32,
        owner: [u8; 32],
        timelock: u64,
    ) -> Result<MultisigWallet, &'static str> {
        if signers.is_empty() {
            return Err("At least 1 signer required");
        }
        if threshold == 0 || threshold as usize > signers.len() {
            return Err("Invalid threshold");
        }
        if threshold > 50 {
            return Err("Threshold too high (max 50)");
        }

        let mut id = [0u8; 32];
        for i in 0..signers.len() {
            id[i % 32] = id[i % 32].wrapping_add(signers[i][0]);
        }

        Ok(MultisigWallet {
            id,
            signers,
            threshold,
            owner,
            created_block: 0,
            timelock_delay: timelock,
            is_active: true,
        })
    }

    /// Propose a transaction for multisig approval
    pub fn propose_transaction(
        wallet: &MultisigWallet,
        proposer: [u8; 32],
        tx_hash: [u8; 32],
        target: [u8; 32],
        value: u128,
        data: &[u8],
        current_block: u64,
    ) -> Result<MultisigProposal, &'static str> {
        if !wallet.is_active {
            return Err("Wallet not active");
        }

        // Verify proposer is a signer
        if !wallet.signers.contains(&proposer) {
            return Err("Proposer not authorized");
        }

        let mut id = [0u8; 32];
        id[0..16].copy_from_slice(&tx_hash[0..16]);
        id[16..32].copy_from_slice(&wallet.id[16..32]);

        Ok(MultisigProposal {
            id,
            wallet_id: wallet.id,
            proposer,
            transaction_hash: tx_hash,
            target,
            value,
            data: data.to_vec(),
            created_block: current_block,
            execution_block: current_block + wallet.timelock_delay,
            signatures: vec![proposer], // proposer auto-approves
            status: 0,                  // pending
        })
    }

    /// Signer approves a proposal
    pub fn sign_proposal(
        wallet: &MultisigWallet,
        proposal: &mut MultisigProposal,
        signer: [u8; 32],
        approval_hash: [u8; 32],
        current_block: u64,
    ) -> Result<SignerApproval, &'static str> {
        if !wallet.is_active {
            return Err("Wallet not active");
        }

        // Verify signer is authorized
        if !wallet.signers.contains(&signer) {
            return Err("Signer not authorized");
        }

        // Check if already signed
        if proposal.signatures.contains(&signer) {
            return Err("Signer already approved");
        }

        proposal.signatures.push(signer);

        // Check if threshold met
        if proposal.signatures.len() as u32 >= wallet.threshold {
            proposal.status = 1; // approved
        }

        Ok(SignerApproval {
            signer,
            proposal_id: proposal.id,
            timestamp: current_block,
            approval_hash,
        })
    }

    /// Check if proposal has reached threshold
    pub fn is_approved(wallet: &MultisigWallet, proposal: &MultisigProposal) -> bool {
        proposal.signatures.len() as u32 >= wallet.threshold && proposal.status == 1
    }

    /// Check if proposal can be executed (passed timelock)
    pub fn can_execute(
        proposal: &MultisigProposal,
        current_block: u64,
    ) -> Result<bool, &'static str> {
        if proposal.status != 1 {
            return Err("Proposal not approved");
        }
        Ok(current_block >= proposal.execution_block)
    }

    /// Execute an approved proposal
    pub fn execute_proposal(
        proposal: &mut MultisigProposal,
        current_block: u64,
    ) -> Result<(), &'static str> {
        if proposal.status != 1 {
            return Err("Proposal not approved");
        }
        if current_block < proposal.execution_block {
            return Err("Timelock not elapsed");
        }

        proposal.status = 2; // executed
        Ok(())
    }

    /// Cancel a proposal
    pub fn cancel_proposal(
        proposal: &mut MultisigProposal,
        caller: [u8; 32],
    ) -> Result<(), &'static str> {
        if proposal.status >= 2 {
            return Err("Cannot cancel executed/cancelled proposal");
        }

        // Only proposer or owner can cancel
        if caller != proposal.proposer {
            return Err("Not authorized to cancel");
        }

        proposal.status = 3;
        Ok(())
    }

    /// Add a new signer to multisig
    pub fn add_signer(
        wallet: &mut MultisigWallet,
        new_signer: [u8; 32],
        accessor: [u8; 32],
    ) -> Result<(), &'static str> {
        // Only owner can add signers
        if accessor != wallet.owner {
            return Err("Only owner can add signers");
        }

        if wallet.signers.len() >= 20 {
            return Err("Max signers reached (20)");
        }

        if wallet.signers.contains(&new_signer) {
            return Err("Signer already exists");
        }

        wallet.signers.push(new_signer);
        Ok(())
    }

    /// Remove a signer from multisig
    pub fn remove_signer(
        wallet: &mut MultisigWallet,
        signer_to_remove: [u8; 32],
        accessor: [u8; 32],
    ) -> Result<(), &'static str> {
        if accessor != wallet.owner {
            return Err("Only owner can remove signers");
        }

        if !wallet.signers.contains(&signer_to_remove) {
            return Err("Signer not found");
        }

        if wallet.signers.len() as u32 <= wallet.threshold {
            return Err("Cannot remove signer - would fall below threshold");
        }

        wallet.signers.retain(|s| s != &signer_to_remove);
        Ok(())
    }

    /// Get signature count for proposal
    pub fn get_signature_count(proposal: &MultisigProposal) -> usize {
        proposal.signatures.len()
    }

    /// Get remaining votes needed for approval
    pub fn votes_needed(wallet: &MultisigWallet, proposal: &MultisigProposal) -> u32 {
        let needed = wallet.threshold as i64 - proposal.signatures.len() as i64;
        if needed <= 0 {
            0
        } else {
            needed as u32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_multisig() {
        let signers = vec![[1u8; 32], [2u8; 32], [3u8; 32]];
        let result = MultisigWalletEngine::create_multisig(signers, 2, [0u8; 32], 10);
        assert!(result.is_ok());
        let wallet = result.unwrap();
        assert_eq!(wallet.signers.len(), 3);
        assert_eq!(wallet.threshold, 2);
    }

    #[test]
    fn test_create_multisig_threshold_too_high() {
        let signers = vec![[1u8; 32], [2u8; 32]];
        let result = MultisigWalletEngine::create_multisig(signers, 3, [0u8; 32], 10);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_multisig_empty_signers() {
        let signers = vec![];
        let result = MultisigWalletEngine::create_multisig(signers, 1, [0u8; 32], 10);
        assert!(result.is_err());
    }

    #[test]
    fn test_propose_transaction() {
        let signers = vec![[1u8; 32], [2u8; 32], [3u8; 32]];
        let wallet = MultisigWalletEngine::create_multisig(signers, 2, [0u8; 32], 10).unwrap();

        let result = MultisigWalletEngine::propose_transaction(
            &wallet, [1u8; 32], [42u8; 32], [99u8; 32], 1000, b"data", 100,
        );
        assert!(result.is_ok());
        let proposal = result.unwrap();
        assert_eq!(proposal.signatures.len(), 1); // auto-signed by proposer
    }

    #[test]
    fn test_propose_transaction_not_signer() {
        let signers = vec![[1u8; 32], [2u8; 32]];
        let wallet = MultisigWalletEngine::create_multisig(signers, 2, [0u8; 32], 10).unwrap();

        let result = MultisigWalletEngine::propose_transaction(
            &wallet, [99u8; 32], [42u8; 32], [99u8; 32], 1000, b"data", 100,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_sign_proposal() {
        let signers = vec![[1u8; 32], [2u8; 32], [3u8; 32]];
        let wallet = MultisigWalletEngine::create_multisig(signers, 2, [0u8; 32], 10).unwrap();

        let mut proposal = MultisigWalletEngine::propose_transaction(
            &wallet, [1u8; 32], [42u8; 32], [99u8; 32], 1000, b"data", 100,
        )
        .unwrap();

        let result =
            MultisigWalletEngine::sign_proposal(&wallet, &mut proposal, [2u8; 32], [88u8; 32], 101);
        assert!(result.is_ok());
        assert_eq!(proposal.signatures.len(), 2);
        assert_eq!(proposal.status, 1); // approved (threshold met)
    }

    #[test]
    fn test_sign_proposal_duplicate() {
        let signers = vec![[1u8; 32], [2u8; 32]];
        let wallet = MultisigWalletEngine::create_multisig(signers, 2, [0u8; 32], 10).unwrap();

        let mut proposal = MultisigWalletEngine::propose_transaction(
            &wallet, [1u8; 32], [42u8; 32], [99u8; 32], 1000, b"data", 100,
        )
        .unwrap();

        let _ =
            MultisigWalletEngine::sign_proposal(&wallet, &mut proposal, [2u8; 32], [88u8; 32], 101);

        let result =
            MultisigWalletEngine::sign_proposal(&wallet, &mut proposal, [1u8; 32], [88u8; 32], 102);
        assert!(result.is_err());
    }

    #[test]
    fn test_can_execute_timelock() {
        let signers = vec![[1u8; 32], [2u8; 32]];
        let wallet = MultisigWalletEngine::create_multisig(signers, 2, [0u8; 32], 10).unwrap();

        let mut proposal = MultisigWalletEngine::propose_transaction(
            &wallet, [1u8; 32], [42u8; 32], [99u8; 32], 1000, b"data", 100,
        )
        .unwrap();

        MultisigWalletEngine::sign_proposal(&wallet, &mut proposal, [2u8; 32], [88u8; 32], 101)
            .unwrap();

        // Can't execute before timelock
        assert!(MultisigWalletEngine::can_execute(&proposal, 105).is_ok());
        assert!(!MultisigWalletEngine::can_execute(&proposal, 105).unwrap());

        // Can execute after timelock
        assert!(MultisigWalletEngine::can_execute(&proposal, 120).unwrap());
    }

    #[test]
    fn test_execute_proposal() {
        let signers = vec![[1u8; 32], [2u8; 32]];
        let wallet = MultisigWalletEngine::create_multisig(signers, 2, [0u8; 32], 10).unwrap();

        let mut proposal = MultisigWalletEngine::propose_transaction(
            &wallet, [1u8; 32], [42u8; 32], [99u8; 32], 1000, b"data", 100,
        )
        .unwrap();

        MultisigWalletEngine::sign_proposal(&wallet, &mut proposal, [2u8; 32], [88u8; 32], 101)
            .unwrap();

        let result = MultisigWalletEngine::execute_proposal(&mut proposal, 120);
        assert!(result.is_ok());
        assert_eq!(proposal.status, 2);
    }

    #[test]
    fn test_cancel_proposal() {
        let signers = vec![[1u8; 32], [2u8; 32]];
        let wallet = MultisigWalletEngine::create_multisig(signers, 2, [0u8; 32], 10).unwrap();

        let mut proposal = MultisigWalletEngine::propose_transaction(
            &wallet, [1u8; 32], [42u8; 32], [99u8; 32], 1000, b"data", 100,
        )
        .unwrap();

        let result = MultisigWalletEngine::cancel_proposal(&mut proposal, [1u8; 32]);
        assert!(result.is_ok());
        assert_eq!(proposal.status, 3);
    }

    #[test]
    fn test_add_signer() {
        let signers = vec![[1u8; 32], [2u8; 32]];
        let mut wallet = MultisigWalletEngine::create_multisig(signers, 2, [0u8; 32], 10).unwrap();

        let result = MultisigWalletEngine::add_signer(&mut wallet, [3u8; 32], [0u8; 32]);
        assert!(result.is_ok());
        assert_eq!(wallet.signers.len(), 3);
    }

    #[test]
    fn test_add_signer_not_owner() {
        let signers = vec![[1u8; 32], [2u8; 32]];
        let mut wallet = MultisigWalletEngine::create_multisig(signers, 2, [0u8; 32], 10).unwrap();

        let result = MultisigWalletEngine::add_signer(&mut wallet, [3u8; 32], [99u8; 32]);
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_signer() {
        let signers = vec![[1u8; 32], [2u8; 32], [3u8; 32]];
        let mut wallet = MultisigWalletEngine::create_multisig(signers, 2, [0u8; 32], 10).unwrap();

        let result = MultisigWalletEngine::remove_signer(&mut wallet, [3u8; 32], [0u8; 32]);
        assert!(result.is_ok());
        assert_eq!(wallet.signers.len(), 2);
    }

    #[test]
    fn test_votes_needed() {
        let signers = vec![[1u8; 32], [2u8; 32], [3u8; 32]];
        let wallet = MultisigWalletEngine::create_multisig(signers, 3, [0u8; 32], 10).unwrap();

        let proposal = MultisigWalletEngine::propose_transaction(
            &wallet, [1u8; 32], [42u8; 32], [99u8; 32], 1000, b"data", 100,
        )
        .unwrap();

        assert_eq!(MultisigWalletEngine::votes_needed(&wallet, &proposal), 2);
    }

    #[test]
    fn test_get_signature_count() {
        let signers = vec![[1u8; 32], [2u8; 32]];
        let wallet = MultisigWalletEngine::create_multisig(signers, 2, [0u8; 32], 10).unwrap();

        let proposal = MultisigWalletEngine::propose_transaction(
            &wallet, [1u8; 32], [42u8; 32], [99u8; 32], 1000, b"data", 100,
        )
        .unwrap();

        assert_eq!(MultisigWalletEngine::get_signature_count(&proposal), 1);
    }

    #[test]
    fn test_is_approved() {
        let signers = vec![[1u8; 32], [2u8; 32]];
        let wallet = MultisigWalletEngine::create_multisig(signers, 2, [0u8; 32], 10).unwrap();

        let mut proposal = MultisigWalletEngine::propose_transaction(
            &wallet, [1u8; 32], [42u8; 32], [99u8; 32], 1000, b"data", 100,
        )
        .unwrap();

        assert!(!MultisigWalletEngine::is_approved(&wallet, &proposal));

        MultisigWalletEngine::sign_proposal(&wallet, &mut proposal, [2u8; 32], [88u8; 32], 101)
            .unwrap();

        assert!(MultisigWalletEngine::is_approved(&wallet, &proposal));
    }
}
