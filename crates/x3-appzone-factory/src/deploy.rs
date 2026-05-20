//! Deploy: converts a template + parameter override set into a deployment
//! request and validates it before it is submitted on-chain.

use alloc::string::String;
use alloc::vec::Vec;
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use scale_info::TypeInfo;
use sp_core::H256;

use crate::templates::{Param, TemplateError, TemplateId};

/// A validated, ready-to-submit AppZone deployment request.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct DeployRequest {
    /// Template this zone is based on.
    pub template_id: TemplateId,
    /// Human-readable zone name.
    pub zone_name: String,
    /// Merged parameter set (required + optional overrides).
    pub params: Vec<Param>,
    /// Blake2-256 hash of `(template_id ‖ zone_name ‖ encoded_params)`.
    /// Computed deterministically so the chain can verify the request.
    pub commitment: H256,
}

/// Deployment errors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeployError {
    Template(TemplateError),
    /// Zone name is empty.
    EmptyZoneName,
    /// Zone name exceeds 64 bytes.
    ZoneNameTooLong,
}

impl From<TemplateError> for DeployError {
    fn from(e: TemplateError) -> Self {
        DeployError::Template(e)
    }
}

pub struct Deployer;

impl Deployer {
    /// Validate inputs and build a `DeployRequest`.
    ///
    /// Does **not** submit anything on-chain; that is the caller's
    /// responsibility.
    pub fn build(
        template_id: TemplateId,
        zone_name: String,
        params: Vec<Param>,
    ) -> Result<DeployRequest, DeployError> {
        if zone_name.is_empty() {
            return Err(DeployError::EmptyZoneName);
        }
        if zone_name.len() > 64 {
            return Err(DeployError::ZoneNameTooLong);
        }

        let commitment = Self::compute_commitment(&template_id, &zone_name, &params);

        Ok(DeployRequest {
            template_id,
            zone_name,
            params,
            commitment,
        })
    }

    fn compute_commitment(template_id: &TemplateId, zone_name: &str, params: &[Param]) -> H256 {
        use parity_scale_codec::Encode;
        // Domain-separated blake2_256: tag ‖ template_id ‖ zone_name ‖ params
        let mut input: Vec<u8> = b"x3:appzone:deploy:v1:".to_vec();
        input.extend_from_slice(template_id);
        input.extend_from_slice(zone_name.as_bytes());
        input.extend_from_slice(&params.encode());
        H256(sp_core::blake2_256(&input))
    }
}
