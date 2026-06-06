/// Privacy Mixing — Stealth addresses & transaction mixing for privacy
/// Mix transactions, use stealth addresses, break on-chain traceability
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use sp_std::vec::Vec;

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct MixingPool {
    pub id: [u8; 32],
    pub pool_size: u128,
    pub participant_count: u32,
    pub mix_denomination: u128, // all deposits must match this
    pub is_active: bool,
    pub created_block: u64,
    pub last_mixed_block: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct MixingTransaction {
    pub id: [u8; 32],
    pub pool_id: [u8; 32],
    pub depositor: [u8; 32],
    pub deposit_amount: u128,
    pub stealth_address: [u8; 32],
    pub withdrawal_address: [u8; 32],
    pub is_mixed: bool,
    pub is_withdrawn: bool,
    pub deposit_block: u64,
    pub min_withdrawal_block: u64, // must wait to prevent tracing
}

pub struct PrivacyMixer;

impl PrivacyMixer {
    /// Create a new mixing pool
    pub fn create_pool(denomination: u128, current_block: u64) -> Result<MixingPool, &'static str> {
        if denomination == 0 {
            return Err("Denomination must be > 0");
        }

        let mut id = [0u8; 32];
        // Use lower 8 bytes of denomination (u128 → 16 bytes)
        id[0..8].copy_from_slice(&denomination.to_le_bytes()[0..8]);
        id[8..16].copy_from_slice(&current_block.to_le_bytes());

        Ok(MixingPool {
            id,
            pool_size: 0,
            participant_count: 0,
            mix_denomination: denomination,
            is_active: true,
            created_block: current_block,
            last_mixed_block: current_block,
        })
    }

    /// Deposit to mixing pool
    pub fn deposit_to_pool(
        pool: &mut MixingPool,
        amount: u128,
        depositor: [u8; 32],
        current_block: u64,
    ) -> Result<MixingTransaction, &'static str> {
        if !pool.is_active {
            return Err("Pool not active");
        }
        if amount != pool.mix_denomination {
            return Err("Amount does not match pool denomination");
        }

        // Generate stealth address deterministically
        let mut stealth_addr = [0u8; 32];
        stealth_addr[0..8].copy_from_slice(&depositor[0..8]);
        stealth_addr[8..16].copy_from_slice(&current_block.to_le_bytes());

        let mut tx_id = [0u8; 32];
        tx_id[0..8].copy_from_slice(&pool.id[0..8]);
        tx_id[8..16].copy_from_slice(&depositor[8..16]);

        pool.pool_size += amount;
        pool.participant_count += 1;

        Ok(MixingTransaction {
            id: tx_id,
            pool_id: pool.id,
            depositor,
            deposit_amount: amount,
            stealth_address: stealth_addr,
            withdrawal_address: [0u8; 32],
            is_mixed: false,
            is_withdrawn: false,
            deposit_block: current_block,
            min_withdrawal_block: current_block + 1000, // wait 1000 blocks (~3 days)
        })
    }

    /// Mark transaction as mixed (after coinjoin)
    pub fn mark_mixed(tx: &mut MixingTransaction, current_block: u64) -> Result<(), &'static str> {
        if tx.is_mixed {
            return Err("Already mixed");
        }
        if current_block < tx.deposit_block + 50 {
            return Err("Mixing delay not elapsed");
        }

        tx.is_mixed = true;
        Ok(())
    }

    /// Withdraw from pool using stealth address
    pub fn withdraw_from_pool(
        tx: &mut MixingTransaction,
        withdrawal_addr: [u8; 32],
        current_block: u64,
    ) -> Result<(), &'static str> {
        if !tx.is_mixed {
            return Err("Transaction not yet mixed");
        }
        if tx.is_withdrawn {
            return Err("Already withdrawn");
        }
        if current_block < tx.min_withdrawal_block {
            return Err("Minimum withdrawal delay not elapsed");
        }

        tx.withdrawal_address = withdrawal_addr;
        tx.is_withdrawn = true;
        Ok(())
    }

    /// Track mixing rounds participated
    pub fn get_mixing_rounds_participated(pool: &MixingPool, current_block: u64) -> u32 {
        // Each 100 blocks = 1 mixing round
        ((current_block - pool.created_block) / 100) as u32
    }

    /// Check pool security (minimum participants before mixing)
    pub fn can_execute_mix(pool: &MixingPool) -> bool {
        pool.participant_count >= 10 && pool.is_active
    }

    /// Anonymity set size
    pub fn get_anonymity_set_size(pool: &MixingPool) -> u32 {
        pool.participant_count
    }

    /// Get pool concentration (helps measure privacy leak)
    pub fn get_pool_concentration(pool: &MixingPool, user_deposit: u128) -> u32 {
        if pool.pool_size == 0 {
            return 0;
        }
        // concentration = (user_deposit / pool_size) * 1000
        ((user_deposit as u32 * 1000) / (pool.pool_size as u32)).min(1000)
    }

    /// Deactivate pool
    pub fn deactivate_pool(pool: &mut MixingPool) -> Result<(), &'static str> {
        if !pool.is_active {
            return Err("Pool already inactive");
        }
        pool.is_active = false;
        Ok(())
    }

    /// Reactivate pool
    pub fn reactivate_pool(pool: &mut MixingPool) -> Result<(), &'static str> {
        if pool.is_active {
            return Err("Pool already active");
        }
        pool.is_active = true;
        Ok(())
    }

    /// Get transaction status
    pub fn get_transaction_status(tx: &MixingTransaction) -> Vec<u8> {
        if tx.is_withdrawn {
            b"withdrawn".to_vec()
        } else if tx.is_mixed {
            b"mixed".to_vec()
        } else {
            b"pending".to_vec()
        }
    }

    /// Calculate privacy score (0-100, higher is better)
    pub fn calculate_privacy_score(
        tx: &MixingTransaction,
        pool: &MixingPool,
        current_block: u64,
    ) -> u8 {
        let mut score = 50u8; // base score

        if tx.is_mixed {
            score += 20; // mixed bonus
        }

        if tx.is_withdrawn {
            score += 15; // withdrawn from mixed pool bonus
        }

        let blocks_elapsed = (current_block - tx.deposit_block).min(1000) as u32;
        let time_bonus = ((blocks_elapsed / 10).min(15)) as u8;
        score = (score + time_bonus).min(100);

        // Reduce score based on pool concentration
        let concentration = Self::get_pool_concentration(pool, tx.deposit_amount);
        let concentration_penalty = (concentration / 100) as u8;
        score = score.saturating_sub(concentration_penalty);

        score
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_pool() {
        let result = PrivacyMixer::create_pool(1000, 100);
        assert!(result.is_ok());
        let pool = result.unwrap();
        assert_eq!(pool.mix_denomination, 1000);
        assert!(pool.is_active);
    }

    #[test]
    fn test_create_pool_zero_denom() {
        let result = PrivacyMixer::create_pool(0, 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_deposit_to_pool() {
        let mut pool = PrivacyMixer::create_pool(1000, 100).unwrap();

        let result = PrivacyMixer::deposit_to_pool(&mut pool, 1000, [1u8; 32], 100);
        assert!(result.is_ok());
        assert_eq!(pool.participant_count, 1);
        assert_eq!(pool.pool_size, 1000);
    }

    #[test]
    fn test_deposit_to_pool_wrong_amount() {
        let mut pool = PrivacyMixer::create_pool(1000, 100).unwrap();

        let result = PrivacyMixer::deposit_to_pool(&mut pool, 500, [1u8; 32], 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_mark_mixed() {
        let mut pool = PrivacyMixer::create_pool(1000, 100).unwrap();
        let mut tx = PrivacyMixer::deposit_to_pool(&mut pool, 1000, [1u8; 32], 100).unwrap();

        let result = PrivacyMixer::mark_mixed(&mut tx, 150);
        assert!(result.is_ok());
        assert!(tx.is_mixed);
    }

    #[test]
    fn test_mark_mixed_too_early() {
        let mut pool = PrivacyMixer::create_pool(1000, 100).unwrap();
        let mut tx = PrivacyMixer::deposit_to_pool(&mut pool, 1000, [1u8; 32], 100).unwrap();

        let result = PrivacyMixer::mark_mixed(&mut tx, 120);
        assert!(result.is_err());
    }

    #[test]
    fn test_withdraw_from_pool() {
        let mut pool = PrivacyMixer::create_pool(1000, 100).unwrap();
        let mut tx = PrivacyMixer::deposit_to_pool(&mut pool, 1000, [1u8; 32], 100).unwrap();

        PrivacyMixer::mark_mixed(&mut tx, 150).unwrap();
        let result = PrivacyMixer::withdraw_from_pool(&mut tx, [2u8; 32], 1100);
        assert!(result.is_ok());
        assert!(tx.is_withdrawn);
    }

    #[test]
    fn test_withdraw_too_early() {
        let mut pool = PrivacyMixer::create_pool(1000, 100).unwrap();
        let mut tx = PrivacyMixer::deposit_to_pool(&mut pool, 1000, [1u8; 32], 100).unwrap();

        PrivacyMixer::mark_mixed(&mut tx, 150).unwrap();
        let result = PrivacyMixer::withdraw_from_pool(&mut tx, [2u8; 32], 200);
        assert!(result.is_err());
    }

    #[test]
    fn test_can_execute_mix() {
        let mut pool = PrivacyMixer::create_pool(1000, 100).unwrap();

        assert!(!PrivacyMixer::can_execute_mix(&pool));

        for i in 0..10 {
            PrivacyMixer::deposit_to_pool(&mut pool, 1000, [i as u8; 32], 100).unwrap();
        }

        assert!(PrivacyMixer::can_execute_mix(&pool));
    }

    #[test]
    fn test_get_anonymity_set_size() {
        let mut pool = PrivacyMixer::create_pool(1000, 100).unwrap();

        for i in 0..5 {
            PrivacyMixer::deposit_to_pool(&mut pool, 1000, [i as u8; 32], 100).unwrap();
        }

        assert_eq!(PrivacyMixer::get_anonymity_set_size(&pool), 5);
    }

    #[test]
    fn test_get_pool_concentration() {
        let mut pool = PrivacyMixer::create_pool(1000, 100).unwrap();
        PrivacyMixer::deposit_to_pool(&mut pool, 1000, [1u8; 32], 100).unwrap();

        let concentration = PrivacyMixer::get_pool_concentration(&pool, 1000);
        assert_eq!(concentration, 1000); // 100% of pool
    }

    #[test]
    fn test_deactivate_pool() {
        let mut pool = PrivacyMixer::create_pool(1000, 100).unwrap();

        let result = PrivacyMixer::deactivate_pool(&mut pool);
        assert!(result.is_ok());
        assert!(!pool.is_active);
    }

    #[test]
    fn test_reactivate_pool() {
        let mut pool = PrivacyMixer::create_pool(1000, 100).unwrap();
        PrivacyMixer::deactivate_pool(&mut pool).unwrap();

        let result = PrivacyMixer::reactivate_pool(&mut pool);
        assert!(result.is_ok());
        assert!(pool.is_active);
    }

    #[test]
    fn test_get_transaction_status() {
        let mut pool = PrivacyMixer::create_pool(1000, 100).unwrap();
        let tx = PrivacyMixer::deposit_to_pool(&mut pool, 1000, [1u8; 32], 100).unwrap();

        let status = PrivacyMixer::get_transaction_status(&tx);
        assert_eq!(status, b"pending".to_vec());
    }

    #[test]
    fn test_get_mixing_rounds_participated() {
        let pool = PrivacyMixer::create_pool(1000, 100).unwrap();

        let rounds = PrivacyMixer::get_mixing_rounds_participated(&pool, 500);
        assert_eq!(rounds, 4); // (500-100)/100
    }

    #[test]
    fn test_calculate_privacy_score() {
        let mut pool = PrivacyMixer::create_pool(1000, 100).unwrap();
        let mut tx = PrivacyMixer::deposit_to_pool(&mut pool, 1000, [1u8; 32], 100).unwrap();

        let score = PrivacyMixer::calculate_privacy_score(&tx, &pool, 100);
        assert!(score >= 0 && score <= 100);

        PrivacyMixer::mark_mixed(&mut tx, 150).unwrap();
        let score_after_mix = PrivacyMixer::calculate_privacy_score(&tx, &pool, 150);
        assert!(score_after_mix > score);
    }
}
