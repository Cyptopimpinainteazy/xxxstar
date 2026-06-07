/// Social Recovery Manager — Guardian-based account recovery (ERC-4337 model)
/// Designate 3 guardians who can collectively recover your wallet
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use scale_info::TypeInfo;
#[allow(unused_imports)]
use sp_std::vec;
use sp_std::vec::Vec;

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, Debug, PartialEq, Eq)]
pub struct GuardianAccount {
    pub id: [u8; 32],
    pub owner: [u8; 32],
    pub guardians: Vec<[u8; 32]>, // list of recovery guardians
    pub required_guardians: u32,  // threshold (usually m-of-n)
    pub recovery_delay_blocks: u64,
    pub is_active: bool,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, Debug, PartialEq, Eq)]
pub struct RecoveryRequest {
    pub id: [u8; 32],
    pub account_id: [u8; 32],
    pub new_owner: [u8; 32],
    pub approvals: Vec<[u8; 32]>, // guardians who approved recovery
    pub created_block: u64,
    pub executable_block: u64, // after delay, can execute
    pub status: u8,            // 0=pending, 1=approved, 2=executed, 3=cancelled
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, Debug, PartialEq, Eq)]
pub struct GuardianApproval {
    pub guardian: [u8; 32],
    pub recovery_request_id: [u8; 32],
    pub approval_timestamp: u64,
    pub approval_hash: [u8; 32],
}

pub struct SocialRecoveryManager;

impl SocialRecoveryManager {
    /// Initialize social recovery with guardians
    pub fn create_recovery_account(
        owner: [u8; 32],
        guardians: Vec<[u8; 32]>,
        required: u32,
        delay_blocks: u64,
    ) -> Result<GuardianAccount, &'static str> {
        if guardians.is_empty() {
            return Err("At least 1 guardian required");
        }
        if required == 0 || required as usize > guardians.len() {
            return Err("Invalid threshold");
        }
        if required > 20 {
            return Err("Too many guardians required");
        }
        if guardians.len() > 30 {
            return Err("Max 30 guardians allowed");
        }

        let mut id = [0u8; 32];
        id[0..16].copy_from_slice(&owner[0..16]);

        Ok(GuardianAccount {
            id,
            owner,
            guardians,
            required_guardians: required,
            recovery_delay_blocks: delay_blocks,
            is_active: true,
        })
    }

    /// Owner adds a new guardian
    pub fn add_guardian(
        account: &mut GuardianAccount,
        new_guardian: [u8; 32],
        owner: [u8; 32],
    ) -> Result<(), &'static str> {
        if owner != account.owner {
            return Err("Only owner can add guardians");
        }
        if account.guardians.contains(&new_guardian) {
            return Err("Guardian already exists");
        }
        if account.guardians.len() >= 30 {
            return Err("Max guardians reached");
        }

        account.guardians.push(new_guardian);
        Ok(())
    }

    /// Owner removes a guardian
    pub fn remove_guardian(
        account: &mut GuardianAccount,
        guardian: [u8; 32],
        owner: [u8; 32],
    ) -> Result<(), &'static str> {
        if owner != account.owner {
            return Err("Only owner can remove guardians");
        }
        if !account.guardians.contains(&guardian) {
            return Err("Guardian not found");
        }
        if account.guardians.len() as u32 <= account.required_guardians {
            return Err("Cannot remove guardian - would fall below threshold");
        }

        account.guardians.retain(|g| g != &guardian);
        Ok(())
    }

    /// Initiate recovery with new owner
    pub fn initiate_recovery(
        account: &GuardianAccount,
        new_owner: [u8; 32],
        current_block: u64,
    ) -> Result<RecoveryRequest, &'static str> {
        if !account.is_active {
            return Err("Account not active");
        }
        if new_owner == account.owner {
            return Err("New owner same as current");
        }

        let mut id = [0u8; 32];
        id[0..16].copy_from_slice(&account.id[0..16]);
        id[16..32].copy_from_slice(&new_owner[16..32]);

        Ok(RecoveryRequest {
            id,
            account_id: account.id,
            new_owner,
            approvals: vec![],
            created_block: current_block,
            executable_block: current_block + account.recovery_delay_blocks,
            status: 0, // pending
        })
    }

    /// Guardian approves recovery
    pub fn guardian_approve_recovery(
        account: &GuardianAccount,
        recovery: &mut RecoveryRequest,
        guardian: [u8; 32],
        approval_hash: [u8; 32],
        current_block: u64,
    ) -> Result<GuardianApproval, &'static str> {
        if !account.is_active {
            return Err("Account not active");
        }
        if !account.guardians.contains(&guardian) {
            return Err("Not an authorized guardian");
        }
        if recovery.status != 0 {
            return Err("Recovery not pending");
        }
        if recovery.approvals.contains(&guardian) {
            return Err("Guardian already approved");
        }

        recovery.approvals.push(guardian);

        // Check if threshold met
        if recovery.approvals.len() as u32 >= account.required_guardians {
            recovery.status = 1; // approved
        }

        Ok(GuardianApproval {
            guardian,
            recovery_request_id: recovery.id,
            approval_timestamp: current_block,
            approval_hash,
        })
    }

    /// Verify recovery is approved
    pub fn is_recovery_approved(account: &GuardianAccount, recovery: &RecoveryRequest) -> bool {
        recovery.approvals.len() as u32 >= account.required_guardians && recovery.status == 1
    }

    /// Check if recovery can be executed (delay passed)
    pub fn can_execute_recovery(
        recovery: &RecoveryRequest,
        current_block: u64,
    ) -> Result<bool, &'static str> {
        if recovery.status != 1 {
            return Err("Recovery not approved");
        }
        Ok(current_block >= recovery.executable_block)
    }

    /// Execute recovery - new owner takes control
    pub fn execute_recovery(
        account: &mut GuardianAccount,
        recovery: &mut RecoveryRequest,
        current_block: u64,
    ) -> Result<(), &'static str> {
        if recovery.status != 1 {
            return Err("Recovery not approved");
        }
        if current_block < recovery.executable_block {
            return Err("Recovery delay not elapsed");
        }

        account.owner = recovery.new_owner;
        recovery.status = 2; // executed
        Ok(())
    }

    /// Cancel recovery request
    pub fn cancel_recovery(
        recovery: &mut RecoveryRequest,
        caller: [u8; 32],
        account_owner: [u8; 32],
    ) -> Result<(), &'static str> {
        if caller != account_owner && !recovery.approvals.contains(&caller) {
            return Err("Not authorized to cancel");
        }
        if recovery.status >= 2 {
            return Err("Recovery already executed/cancelled");
        }

        recovery.status = 3;
        Ok(())
    }

    /// Get guardians for account
    pub fn get_guardians(account: &GuardianAccount) -> Vec<[u8; 32]> {
        account.guardians.clone()
    }

    /// Get current approval count
    pub fn get_approval_count(recovery: &RecoveryRequest) -> usize {
        recovery.approvals.len()
    }

    /// Check if guardian already approved
    pub fn has_guardian_approved(recovery: &RecoveryRequest, guardian: [u8; 32]) -> bool {
        recovery.approvals.contains(&guardian)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_recovery_account() {
        let guardians = vec![[1u8; 32], [2u8; 32], [3u8; 32]];
        let result = SocialRecoveryManager::create_recovery_account([0u8; 32], guardians, 2, 100);
        assert!(result.is_ok());
        let account = result.unwrap();
        assert_eq!(account.guardians.len(), 3);
        assert_eq!(account.required_guardians, 2);
    }

    #[test]
    fn test_create_recovery_invalid_threshold() {
        let guardians = vec![[1u8; 32], [2u8; 32]];
        let result = SocialRecoveryManager::create_recovery_account([0u8; 32], guardians, 3, 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_add_guardian() {
        let guardians = vec![[1u8; 32], [2u8; 32]];
        let mut account =
            SocialRecoveryManager::create_recovery_account([0u8; 32], guardians, 2, 100).unwrap();

        let result = SocialRecoveryManager::add_guardian(&mut account, [3u8; 32], [0u8; 32]);
        assert!(result.is_ok());
        assert_eq!(account.guardians.len(), 3);
    }

    #[test]
    fn test_add_guardian_not_owner() {
        let guardians = vec![[1u8; 32]];
        let mut account =
            SocialRecoveryManager::create_recovery_account([0u8; 32], guardians, 1, 100).unwrap();

        let result = SocialRecoveryManager::add_guardian(&mut account, [99u8; 32], [99u8; 32]);
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_guardian() {
        let guardians = vec![[1u8; 32], [2u8; 32], [3u8; 32]];
        let mut account =
            SocialRecoveryManager::create_recovery_account([0u8; 32], guardians, 2, 100).unwrap();

        let result = SocialRecoveryManager::remove_guardian(&mut account, [3u8; 32], [0u8; 32]);
        assert!(result.is_ok());
        assert_eq!(account.guardians.len(), 2);
    }

    #[test]
    fn test_initiate_recovery() {
        let guardians = vec![[1u8; 32], [2u8; 32]];
        let account =
            SocialRecoveryManager::create_recovery_account([0u8; 32], guardians, 2, 100).unwrap();

        let result = SocialRecoveryManager::initiate_recovery(&account, [99u8; 32], 1000);
        assert!(result.is_ok());
        let recovery = result.unwrap();
        assert_eq!(recovery.status, 0);
        assert_eq!(recovery.executable_block, 1100);
    }

    #[test]
    fn test_initiate_recovery_same_owner() {
        let guardians = vec![[1u8; 32]];
        let account =
            SocialRecoveryManager::create_recovery_account([0u8; 32], guardians, 1, 100).unwrap();

        let result = SocialRecoveryManager::initiate_recovery(&account, [0u8; 32], 1000);
        assert!(result.is_err());
    }

    #[test]
    fn test_guardian_approve_recovery() {
        let guardians = vec![[1u8; 32], [2u8; 32]];
        let account =
            SocialRecoveryManager::create_recovery_account([0u8; 32], guardians, 2, 100).unwrap();

        let mut recovery =
            SocialRecoveryManager::initiate_recovery(&account, [99u8; 32], 1000).unwrap();

        let result = SocialRecoveryManager::guardian_approve_recovery(
            &account,
            &mut recovery,
            [1u8; 32],
            [88u8; 32],
            1001,
        );
        assert!(result.is_ok());
        assert_eq!(recovery.approvals.len(), 1);
        assert_eq!(recovery.status, 0); // not yet approved (need 2)
    }

    #[test]
    fn test_guardian_approve_recovery_threshold() {
        let guardians = vec![[1u8; 32], [2u8; 32]];
        let account =
            SocialRecoveryManager::create_recovery_account([0u8; 32], guardians, 2, 100).unwrap();

        let mut recovery =
            SocialRecoveryManager::initiate_recovery(&account, [99u8; 32], 1000).unwrap();

        SocialRecoveryManager::guardian_approve_recovery(
            &account,
            &mut recovery,
            [1u8; 32],
            [88u8; 32],
            1001,
        )
        .unwrap();

        SocialRecoveryManager::guardian_approve_recovery(
            &account,
            &mut recovery,
            [2u8; 32],
            [88u8; 32],
            1002,
        )
        .unwrap();

        assert_eq!(recovery.approvals.len(), 2);
        assert_eq!(recovery.status, 1); // approved
    }

    #[test]
    fn test_can_execute_recovery_delay() {
        let guardians = vec![[1u8; 32], [2u8; 32]];
        let account =
            SocialRecoveryManager::create_recovery_account([0u8; 32], guardians, 2, 100).unwrap();

        let mut recovery =
            SocialRecoveryManager::initiate_recovery(&account, [99u8; 32], 1000).unwrap();

        SocialRecoveryManager::guardian_approve_recovery(
            &account,
            &mut recovery,
            [1u8; 32],
            [88u8; 32],
            1001,
        )
        .unwrap();
        SocialRecoveryManager::guardian_approve_recovery(
            &account,
            &mut recovery,
            [2u8; 32],
            [88u8; 32],
            1002,
        )
        .unwrap();

        assert!(!SocialRecoveryManager::can_execute_recovery(&recovery, 1050).unwrap());
        assert!(SocialRecoveryManager::can_execute_recovery(&recovery, 1100).unwrap());
    }

    #[test]
    fn test_execute_recovery() {
        let guardians = vec![[1u8; 32], [2u8; 32]];
        let mut account =
            SocialRecoveryManager::create_recovery_account([0u8; 32], guardians, 2, 100).unwrap();

        let mut recovery =
            SocialRecoveryManager::initiate_recovery(&account, [99u8; 32], 1000).unwrap();

        SocialRecoveryManager::guardian_approve_recovery(
            &account,
            &mut recovery,
            [1u8; 32],
            [88u8; 32],
            1001,
        )
        .unwrap();
        SocialRecoveryManager::guardian_approve_recovery(
            &account,
            &mut recovery,
            [2u8; 32],
            [88u8; 32],
            1002,
        )
        .unwrap();

        let result = SocialRecoveryManager::execute_recovery(&mut account, &mut recovery, 1100);
        assert!(result.is_ok());
        assert_eq!(account.owner, [99u8; 32]);
        assert_eq!(recovery.status, 2);
    }

    #[test]
    fn test_cancel_recovery() {
        let guardians = vec![[1u8; 32]];
        let mut account =
            SocialRecoveryManager::create_recovery_account([0u8; 32], guardians, 1, 100).unwrap();

        let mut recovery =
            SocialRecoveryManager::initiate_recovery(&account, [99u8; 32], 1000).unwrap();

        let result = SocialRecoveryManager::cancel_recovery(&mut recovery, [0u8; 32], [0u8; 32]);
        assert!(result.is_ok());
        assert_eq!(recovery.status, 3);
    }

    #[test]
    fn test_has_guardian_approved() {
        let guardians = vec![[1u8; 32], [2u8; 32]];
        let account =
            SocialRecoveryManager::create_recovery_account([0u8; 32], guardians, 2, 100).unwrap();

        let mut recovery =
            SocialRecoveryManager::initiate_recovery(&account, [99u8; 32], 1000).unwrap();

        assert!(!SocialRecoveryManager::has_guardian_approved(
            &recovery, [1u8; 32]
        ));

        SocialRecoveryManager::guardian_approve_recovery(
            &account,
            &mut recovery,
            [1u8; 32],
            [88u8; 32],
            1001,
        )
        .unwrap();

        assert!(SocialRecoveryManager::has_guardian_approved(
            &recovery, [1u8; 32]
        ));
    }

    #[test]
    fn test_get_approval_count() {
        let guardians = vec![[1u8; 32], [2u8; 32]];
        let account =
            SocialRecoveryManager::create_recovery_account([0u8; 32], guardians, 2, 100).unwrap();

        let mut recovery =
            SocialRecoveryManager::initiate_recovery(&account, [99u8; 32], 1000).unwrap();

        SocialRecoveryManager::guardian_approve_recovery(
            &account,
            &mut recovery,
            [1u8; 32],
            [88u8; 32],
            1001,
        )
        .unwrap();

        assert_eq!(SocialRecoveryManager::get_approval_count(&recovery), 1);
    }

    #[test]
    fn test_is_recovery_approved() {
        let guardians = vec![[1u8; 32], [2u8; 32]];
        let account =
            SocialRecoveryManager::create_recovery_account([0u8; 32], guardians, 2, 100).unwrap();

        let mut recovery =
            SocialRecoveryManager::initiate_recovery(&account, [99u8; 32], 1000).unwrap();

        assert!(!SocialRecoveryManager::is_recovery_approved(
            &account, &recovery
        ));

        SocialRecoveryManager::guardian_approve_recovery(
            &account,
            &mut recovery,
            [1u8; 32],
            [88u8; 32],
            1001,
        )
        .unwrap();
        SocialRecoveryManager::guardian_approve_recovery(
            &account,
            &mut recovery,
            [2u8; 32],
            [88u8; 32],
            1002,
        )
        .unwrap();

        assert!(SocialRecoveryManager::is_recovery_approved(
            &account, &recovery
        ));
    }
}
