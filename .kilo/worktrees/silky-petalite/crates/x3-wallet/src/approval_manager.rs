/// Approval Manager — Transaction approval policies and spending limits
/// Set spending limits, approve transactions above threshold, rate-limit withdrawals
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use scale_info::TypeInfo;
use sp_std::vec::Vec;

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, Debug, PartialEq, Eq)]
pub struct ApprovalPolicy {
    pub id: [u8; 32],
    pub owner: [u8; 32],
    pub daily_limit: u128,
    pub daily_spent: u128,
    pub daily_reset_block: u64,
    pub requires_approval_above: u128,
    pub approval_timeout_blocks: u64,
    pub is_active: bool,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, Debug, PartialEq, Eq)]
pub struct TransactionApproval {
    pub id: [u8; 32],
    pub policy_id: [u8; 32],
    pub transaction_hash: [u8; 32],
    pub amount: u128,
    pub requester: [u8; 32],
    pub status: u8, // 0=pending, 1=approved, 2=rejected, 3=expired
    pub created_block: u64,
    pub expires_at_block: u64,
    pub approvals_received: Vec<[u8; 32]>,
}

pub struct ApprovalManager;

impl ApprovalManager {
    /// Create an approval policy for wallet
    pub fn create_policy(
        owner: [u8; 32],
        daily_limit: u128,
        approval_threshold: u128,
        approval_timeout: u64,
        current_block: u64,
    ) -> Result<ApprovalPolicy, &'static str> {
        if daily_limit == 0 {
            return Err("Daily limit must be > 0");
        }
        if approval_threshold > daily_limit {
            return Err("Approval threshold cannot exceed daily limit");
        }

        let mut id = [0u8; 32];
        id[0..16].copy_from_slice(&owner[0..16]);

        Ok(ApprovalPolicy {
            id,
            owner,
            daily_limit,
            daily_spent: 0,
            daily_reset_block: current_block + 7200, // ~1 day (blocks)
            requires_approval_above: approval_threshold,
            approval_timeout_blocks: approval_timeout,
            is_active: true,
        })
    }

    /// Check if transaction requires approval
    pub fn check_transaction(
        policy: &ApprovalPolicy,
        amount: u128,
        current_block: u64,
    ) -> Result<bool, &'static str> {
        if !policy.is_active {
            return Err("Policy not active");
        }

        // Reset daily limit if needed
        let (_, daily_spent) = if current_block > policy.daily_reset_block {
            (current_block + 7200, 0u128) // RESET daily tracker
        } else {
            (policy.daily_reset_block, policy.daily_spent)
        };

        // Check daily limit
        if daily_spent + amount > policy.daily_limit {
            return Err("Exceeds daily limit");
        }

        // Return whether approval is needed for this amount
        Ok(amount > policy.requires_approval_above)
    }

    /// Create approval request
    pub fn request_approval(
        policy: &ApprovalPolicy,
        tx_hash: [u8; 32],
        amount: u128,
        requester: [u8; 32],
        current_block: u64,
    ) -> Result<TransactionApproval, &'static str> {
        if !policy.is_active {
            return Err("Policy not active");
        }
        if amount == 0 {
            return Err("Amount must be > 0");
        }

        let mut id = [0u8; 32];
        id[0..16].copy_from_slice(&tx_hash[0..16]);
        id[16..24].copy_from_slice(&requester[0..8]);

        Ok(TransactionApproval {
            id,
            policy_id: policy.id,
            transaction_hash: tx_hash,
            amount,
            requester,
            status: 0, // pending
            created_block: current_block,
            expires_at_block: current_block + policy.approval_timeout_blocks,
            approvals_received: vec![],
        })
    }

    /// Approve transaction
    pub fn approve_transaction(
        approval: &mut TransactionApproval,
        approver: [u8; 32],
        current_block: u64,
    ) -> Result<(), &'static str> {
        if approval.status != 0 {
            return Err("Approval not pending");
        }
        if current_block > approval.expires_at_block {
            return Err("Approval expired");
        }
        if approval.approvals_received.contains(&approver) {
            return Err("Already approved by this address");
        }

        approval.approvals_received.push(approver);
        approval.status = 1; // approved
        Ok(())
    }

    /// Reject transaction approval
    pub fn reject_transaction(approval: &mut TransactionApproval) -> Result<(), &'static str> {
        if approval.status != 0 {
            return Err("Approval not pending");
        }

        approval.status = 2; // rejected
        Ok(())
    }

    /// Check if approval is valid
    pub fn is_approval_valid(approval: &TransactionApproval, current_block: u64) -> bool {
        approval.status == 1 && current_block <= approval.expires_at_block
    }

    /// Update daily spent amount
    pub fn update_daily_spent(
        policy: &mut ApprovalPolicy,
        amount: u128,
        current_block: u64,
    ) -> Result<(), &'static str> {
        if current_block > policy.daily_reset_block {
            policy.daily_spent = amount;
            policy.daily_reset_block = current_block + 7200;
        } else {
            policy.daily_spent += amount;
        }

        if policy.daily_spent > policy.daily_limit {
            return Err("Daily limit exceeded");
        }

        Ok(())
    }

    /// Get approval count
    pub fn get_approval_count(approval: &TransactionApproval) -> usize {
        approval.approvals_received.len()
    }

    /// Disable policy
    pub fn disable_policy(policy: &mut ApprovalPolicy) -> Result<(), &'static str> {
        if !policy.is_active {
            return Err("Policy already disabled");
        }
        policy.is_active = false;
        Ok(())
    }

    /// Re-enable policy
    pub fn enable_policy(policy: &mut ApprovalPolicy) -> Result<(), &'static str> {
        if policy.is_active {
            return Err("Policy already enabled");
        }
        policy.is_active = true;
        Ok(())
    }

    /// Check if approval expired
    pub fn is_approval_expired(approval: &TransactionApproval, current_block: u64) -> bool {
        approval.status == 3 || current_block > approval.expires_at_block
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_policy() {
        let result = ApprovalManager::create_policy([1u8; 32], 10000, 5000, 100, 0);
        assert!(result.is_ok());
        let policy = result.unwrap();
        assert_eq!(policy.daily_limit, 10000);
    }

    #[test]
    fn test_create_policy_invalid_threshold() {
        let result = ApprovalManager::create_policy([1u8; 32], 1000, 5000, 100, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_check_transaction() {
        let policy = ApprovalManager::create_policy([1u8; 32], 10000, 5000, 100, 0).unwrap();

        let result = ApprovalManager::check_transaction(&policy, 3000, 0);
        assert!(result.is_ok());
        assert!(!result.unwrap()); // doesn't need approval
    }

    #[test]
    fn test_check_transaction_needs_approval() {
        let policy = ApprovalManager::create_policy([1u8; 32], 10000, 5000, 100, 0).unwrap();

        let result = ApprovalManager::check_transaction(&policy, 7000, 0);
        assert!(result.is_ok());
        assert!(result.unwrap()); // needs approval
    }

    #[test]
    fn test_check_transaction_exceeds_limit() {
        let policy = ApprovalManager::create_policy([1u8; 32], 10000, 5000, 100, 0).unwrap();

        let result = ApprovalManager::check_transaction(&policy, 15000, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_request_approval() {
        let policy = ApprovalManager::create_policy([1u8; 32], 10000, 5000, 100, 0).unwrap();

        let result = ApprovalManager::request_approval(&policy, [2u8; 32], 7000, [3u8; 32], 0);
        assert!(result.is_ok());
        let approval = result.unwrap();
        assert_eq!(approval.status, 0);
    }

    #[test]
    fn test_approve_transaction() {
        let policy = ApprovalManager::create_policy([1u8; 32], 10000, 5000, 100, 0).unwrap();

        let mut approval =
            ApprovalManager::request_approval(&policy, [2u8; 32], 7000, [3u8; 32], 0).unwrap();

        let result = ApprovalManager::approve_transaction(&mut approval, [4u8; 32], 0);
        assert!(result.is_ok());
        assert_eq!(approval.status, 1);
    }

    #[test]
    fn test_approve_transaction_duplicate() {
        let policy = ApprovalManager::create_policy([1u8; 32], 10000, 5000, 100, 0).unwrap();

        let mut approval =
            ApprovalManager::request_approval(&policy, [2u8; 32], 7000, [3u8; 32], 0).unwrap();

        ApprovalManager::approve_transaction(&mut approval, [4u8; 32], 0).unwrap();

        let result = ApprovalManager::approve_transaction(&mut approval, [4u8; 32], 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_transaction() {
        let policy = ApprovalManager::create_policy([1u8; 32], 10000, 5000, 100, 0).unwrap();

        let mut approval =
            ApprovalManager::request_approval(&policy, [2u8; 32], 7000, [3u8; 32], 0).unwrap();

        let result = ApprovalManager::reject_transaction(&mut approval);
        assert!(result.is_ok());
        assert_eq!(approval.status, 2);
    }

    #[test]
    fn test_is_approval_valid() {
        let policy = ApprovalManager::create_policy([1u8; 32], 10000, 5000, 100, 0).unwrap();

        let mut approval =
            ApprovalManager::request_approval(&policy, [2u8; 32], 7000, [3u8; 32], 0).unwrap();

        ApprovalManager::approve_transaction(&mut approval, [4u8; 32], 0).unwrap();

        assert!(ApprovalManager::is_approval_valid(&approval, 0));
        assert!(!ApprovalManager::is_approval_valid(&approval, 101));
    }

    #[test]
    fn test_update_daily_spent() {
        let mut policy = ApprovalManager::create_policy([1u8; 32], 10000, 5000, 100, 0).unwrap();

        let result = ApprovalManager::update_daily_spent(&mut policy, 3000, 0);
        assert!(result.is_ok());
        assert_eq!(policy.daily_spent, 3000);
    }

    #[test]
    fn test_disable_and_enable_policy() {
        let mut policy = ApprovalManager::create_policy([1u8; 32], 10000, 5000, 100, 0).unwrap();

        assert!(ApprovalManager::disable_policy(&mut policy).is_ok());
        assert!(!policy.is_active);

        assert!(ApprovalManager::enable_policy(&mut policy).is_ok());
        assert!(policy.is_active);
    }

    #[test]
    fn test_get_approval_count() {
        let policy = ApprovalManager::create_policy([1u8; 32], 10000, 5000, 100, 0).unwrap();

        let mut approval =
            ApprovalManager::request_approval(&policy, [2u8; 32], 7000, [3u8; 32], 0).unwrap();

        ApprovalManager::approve_transaction(&mut approval, [4u8; 32], 0).unwrap();

        assert_eq!(ApprovalManager::get_approval_count(&approval), 1);
    }

    #[test]
    fn test_is_approval_expired() {
        let policy = ApprovalManager::create_policy([1u8; 32], 10000, 5000, 100, 0).unwrap();

        let approval =
            ApprovalManager::request_approval(&policy, [2u8; 32], 7000, [3u8; 32], 0).unwrap();

        assert!(!ApprovalManager::is_approval_expired(&approval, 0));
        assert!(ApprovalManager::is_approval_expired(&approval, 101));
    }
}
