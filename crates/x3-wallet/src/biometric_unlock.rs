/// Biometric Unlock — Face ID / Fingerprint authentication for wallet access
/// Unlock wallet with biometric data, PIN fallback, session timeout
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use scale_info::TypeInfo;
use sp_std::vec::Vec;

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, Debug, PartialEq, Eq)]
pub struct BiometricProfile {
    pub id: [u8; 32],
    pub owner: [u8; 32],
    pub biometric_type: u8, // 0=fingerprint, 1=face, 2=iris
    pub template_hash: [u8; 32],
    pub pin_hash: [u8; 32],
    pub is_enabled: bool,
    pub attempts_remaining: u8,
    pub locked_until_block: u64,
    pub created_block: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, Debug, PartialEq, Eq)]
pub struct UnlockSession {
    pub id: [u8; 32],
    pub profile_id: [u8; 32],
    pub user: [u8; 32],
    pub unlocked_block: u64,
    pub expires_at_block: u64,
    pub session_key: [u8; 32],
    pub is_active: bool,
}

pub struct BiometricManager;

impl BiometricManager {
    /// Create biometric profile
    pub fn create_profile(
        owner: [u8; 32],
        biometric_type: u8,
        template_hash: [u8; 32],
        pin_hash: [u8; 32],
        current_block: u64,
    ) -> Result<BiometricProfile, &'static str> {
        if biometric_type > 2 {
            return Err("Invalid biometric type");
        }
        if template_hash == [0u8; 32] {
            return Err("Invalid template hash");
        }
        if pin_hash == [0u8; 32] {
            return Err("PIN hash required");
        }

        let mut id = [0u8; 32];
        id[0..16].copy_from_slice(&owner[0..16]);

        Ok(BiometricProfile {
            id,
            owner,
            biometric_type,
            template_hash,
            pin_hash,
            is_enabled: true,
            attempts_remaining: 5,
            locked_until_block: 0,
            created_block: current_block,
        })
    }

    /// Verify biometric match (simplified)
    pub fn verify_biometric(
        profile: &BiometricProfile,
        provided_template: [u8; 32],
    ) -> Result<bool, &'static str> {
        if !profile.is_enabled {
            return Err("Profile disabled");
        }

        Ok(provided_template == profile.template_hash)
    }

    /// Unlock with biometric
    pub fn unlock_with_biometric(
        profile: &mut BiometricProfile,
        provided_template: [u8; 32],
        current_block: u64,
    ) -> Result<UnlockSession, &'static str> {
        if !profile.is_enabled {
            return Err("Profile disabled");
        }
        if current_block < profile.locked_until_block {
            return Err("Profile temporally locked");
        }

        if provided_template != profile.template_hash {
            profile.attempts_remaining = profile.attempts_remaining.saturating_sub(1);
            if profile.attempts_remaining == 0 {
                profile.locked_until_block = current_block + 100; // lock for 100 blocks
            }
            return Err("Biometric mismatch");
        }

        profile.attempts_remaining = 5; // reset attempts

        let mut session_id = [0u8; 32];
        session_id[0..8].copy_from_slice(&profile.owner[0..8]);
        session_id[8..16].copy_from_slice(&current_block.to_le_bytes());

        Ok(UnlockSession {
            id: session_id,
            profile_id: profile.id,
            user: profile.owner,
            unlocked_block: current_block,
            expires_at_block: current_block + 300, // 5 minute session
            session_key: session_id,
            is_active: true,
        })
    }

    /// Unlock with PIN (as fallback)
    pub fn unlock_with_pin(
        profile: &mut BiometricProfile,
        provided_pin_hash: [u8; 32],
        current_block: u64,
    ) -> Result<UnlockSession, &'static str> {
        if !profile.is_enabled {
            return Err("Profile disabled");
        }
        if current_block < profile.locked_until_block {
            return Err("Profile locked");
        }

        if provided_pin_hash != profile.pin_hash {
            profile.attempts_remaining = profile.attempts_remaining.saturating_sub(1);
            if profile.attempts_remaining == 0 {
                profile.locked_until_block = current_block + 200; // lock longer for PIN
            }
            return Err("PIN incorrect");
        }

        profile.attempts_remaining = 5;

        let mut session_id = [0u8; 32];
        session_id[0..8].copy_from_slice(&profile.owner[0..8]);
        session_id[8..16].copy_from_slice(&current_block.to_le_bytes());

        Ok(UnlockSession {
            id: session_id,
            profile_id: profile.id,
            user: profile.owner,
            unlocked_block: current_block,
            expires_at_block: current_block + 300,
            session_key: session_id,
            is_active: true,
        })
    }

    /// Check if session is valid
    pub fn is_session_valid(session: &UnlockSession, current_block: u64) -> bool {
        session.is_active && current_block <= session.expires_at_block
    }

    /// Extend session (refresh)
    pub fn extend_session(
        session: &mut UnlockSession,
        current_block: u64,
    ) -> Result<(), &'static str> {
        if !session.is_active {
            return Err("Session not active");
        }

        session.expires_at_block = current_block + 300;
        Ok(())
    }

    /// Revoke session
    pub fn revoke_session(session: &mut UnlockSession) -> Result<(), &'static str> {
        if !session.is_active {
            return Err("Session not active");
        }

        session.is_active = false;
        Ok(())
    }

    /// Update PIN
    pub fn update_pin(
        profile: &mut BiometricProfile,
        old_pin_hash: [u8; 32],
        new_pin_hash: [u8; 32],
    ) -> Result<(), &'static str> {
        if old_pin_hash != profile.pin_hash {
            return Err("Old PIN incorrect");
        }
        if new_pin_hash == [0u8; 32] {
            return Err("PIN hash invalid");
        }

        profile.pin_hash = new_pin_hash;
        Ok(())
    }

    /// Disable profile
    pub fn disable_profile(profile: &mut BiometricProfile) -> Result<(), &'static str> {
        if !profile.is_enabled {
            return Err("Profile already disabled");
        }

        profile.is_enabled = false;
        Ok(())
    }

    /// Re-enable profile
    pub fn enable_profile(profile: &mut BiometricProfile) -> Result<(), &'static str> {
        if profile.is_enabled {
            return Err("Profile already enabled");
        }

        profile.is_enabled = true;
        profile.attempts_remaining = 5; // reset attempts
        Ok(())
    }

    /// Get attempts remaining
    pub fn get_attempts_remaining(profile: &BiometricProfile) -> u8 {
        profile.attempts_remaining
    }

    /// Check if profile locked
    pub fn is_profile_locked(profile: &BiometricProfile, current_block: u64) -> bool {
        current_block < profile.locked_until_block
    }

    /// Reset lockout
    pub fn reset_lockout(profile: &mut BiometricProfile) -> Result<(), &'static str> {
        profile.attempts_remaining = 5;
        profile.locked_until_block = 0;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_profile() {
        let result = BiometricManager::create_profile([1u8; 32], 0, [2u8; 32], [3u8; 32], 100);
        assert!(result.is_ok());
        let profile = result.unwrap();
        assert_eq!(profile.attempts_remaining, 5);
    }

    #[test]
    fn test_create_profile_invalid_type() {
        let result = BiometricManager::create_profile([1u8; 32], 5, [2u8; 32], [3u8; 32], 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_profile_invalid_template() {
        let result = BiometricManager::create_profile([1u8; 32], 0, [0u8; 32], [3u8; 32], 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_biometric() {
        let profile =
            BiometricManager::create_profile([1u8; 32], 0, [2u8; 32], [3u8; 32], 100).unwrap();

        let result = BiometricManager::verify_biometric(&profile, [2u8; 32]);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_unlock_with_biometric() {
        let mut profile =
            BiometricManager::create_profile([1u8; 32], 0, [2u8; 32], [3u8; 32], 100).unwrap();

        let result = BiometricManager::unlock_with_biometric(&mut profile, [2u8; 32], 100);
        assert!(result.is_ok());
        let session = result.unwrap();
        assert!(session.is_active);
    }

    #[test]
    fn test_unlock_with_biometric_wrong() {
        let mut profile =
            BiometricManager::create_profile([1u8; 32], 0, [2u8; 32], [3u8; 32], 100).unwrap();

        let result = BiometricManager::unlock_with_biometric(&mut profile, [99u8; 32], 100);
        assert!(result.is_err());
        assert_eq!(profile.attempts_remaining, 4);
    }

    #[test]
    fn test_unlock_with_pin() {
        let mut profile =
            BiometricManager::create_profile([1u8; 32], 0, [2u8; 32], [3u8; 32], 100).unwrap();

        let result = BiometricManager::unlock_with_pin(&mut profile, [3u8; 32], 100);
        assert!(result.is_ok());
    }

    #[test]
    fn test_unlock_with_pin_wrong() {
        let mut profile =
            BiometricManager::create_profile([1u8; 32], 0, [2u8; 32], [3u8; 32], 100).unwrap();

        let result = BiometricManager::unlock_with_pin(&mut profile, [99u8; 32], 100);
        assert!(result.is_err());
        assert_eq!(profile.attempts_remaining, 4);
    }

    #[test]
    fn test_is_session_valid() {
        let session = UnlockSession {
            id: [1u8; 32],
            profile_id: [2u8; 32],
            user: [3u8; 32],
            unlocked_block: 100,
            expires_at_block: 400,
            session_key: [4u8; 32],
            is_active: true,
        };

        assert!(BiometricManager::is_session_valid(&session, 200));
        assert!(!BiometricManager::is_session_valid(&session, 401));
    }

    #[test]
    fn test_extend_session() {
        let mut session = UnlockSession {
            id: [1u8; 32],
            profile_id: [2u8; 32],
            user: [3u8; 32],
            unlocked_block: 100,
            expires_at_block: 400,
            session_key: [4u8; 32],
            is_active: true,
        };

        let old_expiry = session.expires_at_block;
        let result = BiometricManager::extend_session(&mut session, 150);
        assert!(result.is_ok());
        assert!(session.expires_at_block > old_expiry);
    }

    #[test]
    fn test_revoke_session() {
        let mut session = UnlockSession {
            id: [1u8; 32],
            profile_id: [2u8; 32],
            user: [3u8; 32],
            unlocked_block: 100,
            expires_at_block: 400,
            session_key: [4u8; 32],
            is_active: true,
        };

        let result = BiometricManager::revoke_session(&mut session);
        assert!(result.is_ok());
        assert!(!session.is_active);
    }

    #[test]
    fn test_update_pin() {
        let mut profile =
            BiometricManager::create_profile([1u8; 32], 0, [2u8; 32], [3u8; 32], 100).unwrap();

        let result = BiometricManager::update_pin(&mut profile, [3u8; 32], [4u8; 32]);
        assert!(result.is_ok());
        assert_eq!(profile.pin_hash, [4u8; 32]);
    }

    #[test]
    fn test_disable_profile() {
        let mut profile =
            BiometricManager::create_profile([1u8; 32], 0, [2u8; 32], [3u8; 32], 100).unwrap();

        let result = BiometricManager::disable_profile(&mut profile);
        assert!(result.is_ok());
        assert!(!profile.is_enabled);
    }

    #[test]
    fn test_enable_profile() {
        let mut profile =
            BiometricManager::create_profile([1u8; 32], 0, [2u8; 32], [3u8; 32], 100).unwrap();

        BiometricManager::disable_profile(&mut profile).unwrap();
        assert!(!profile.is_enabled);

        let result = BiometricManager::enable_profile(&mut profile);
        assert!(result.is_ok());
        assert!(profile.is_enabled);
    }

    #[test]
    fn test_is_profile_locked() {
        let profile =
            BiometricManager::create_profile([1u8; 32], 0, [2u8; 32], [3u8; 32], 100).unwrap();

        assert!(!BiometricManager::is_profile_locked(&profile, 100));

        let mut locked = profile.clone();
        locked.locked_until_block = 200;

        assert!(BiometricManager::is_profile_locked(&locked, 150));
        assert!(!BiometricManager::is_profile_locked(&locked, 201));
    }

    #[test]
    fn test_reset_lockout() {
        let mut profile =
            BiometricManager::create_profile([1u8; 32], 0, [2u8; 32], [3u8; 32], 100).unwrap();

        profile.attempts_remaining = 0;
        profile.locked_until_block = 200;

        let result = BiometricManager::reset_lockout(&mut profile);
        assert!(result.is_ok());
        assert_eq!(profile.attempts_remaining, 5);
        assert_eq!(profile.locked_until_block, 0);
    }

    #[test]
    fn test_get_attempts_remaining() {
        let profile =
            BiometricManager::create_profile([1u8; 32], 0, [2u8; 32], [3u8; 32], 100).unwrap();

        assert_eq!(BiometricManager::get_attempts_remaining(&profile), 5);
    }
}
