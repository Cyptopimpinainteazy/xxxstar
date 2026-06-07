use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityCouncilConfig {
    pub minimum_approvals: u16,
    pub member_count: u16,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SecurityCouncilConfigError {
    #[error("member_count must be greater than zero")]
    EmptyCouncil,
    #[error("minimum_approvals must be greater than zero")]
    ZeroApprovals,
    #[error("minimum_approvals cannot exceed member_count")]
    InvalidThreshold,
}

impl SecurityCouncilConfig {
    pub fn validate(&self) -> Result<(), SecurityCouncilConfigError> {
        if self.member_count == 0 {
            return Err(SecurityCouncilConfigError::EmptyCouncil);
        }
        if self.minimum_approvals == 0 {
            return Err(SecurityCouncilConfigError::ZeroApprovals);
        }
        if self.minimum_approvals > self.member_count {
            return Err(SecurityCouncilConfigError::InvalidThreshold);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_threshold() {
        let cfg = SecurityCouncilConfig {
            minimum_approvals: 3,
            member_count: 5,
        };
        assert_eq!(cfg.validate(), Ok(()));
    }

    #[test]
    fn rejects_zero_members() {
        let cfg = SecurityCouncilConfig {
            minimum_approvals: 1,
            member_count: 0,
        };
        assert_eq!(
            cfg.validate(),
            Err(SecurityCouncilConfigError::EmptyCouncil)
        );
    }

    #[test]
    fn rejects_threshold_above_members() {
        let cfg = SecurityCouncilConfig {
            minimum_approvals: 6,
            member_count: 5,
        };
        assert_eq!(
            cfg.validate(),
            Err(SecurityCouncilConfigError::InvalidThreshold)
        );
    }
}
