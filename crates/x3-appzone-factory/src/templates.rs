//! AppZone templates.
//!
//! A template is a named, versioned descriptor for an application zone.
//! Templates are the canonical unit of AppZone deployment: a deployer
//! selects a template, optionally overrides parameters, and the factory
//! produces a `DeployRequest`.

use alloc::string::String;
use alloc::vec::Vec;
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use scale_info::TypeInfo;

/// Unique template identifier (max 64 bytes to bound encoding cost).
pub type TemplateId = [u8; 32];

/// Template parameter key-value pair.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct Param {
    pub key: String,
    pub value: String,
}

/// An AppZone template definition.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct Template {
    /// 32-byte template fingerprint (typically blake2_256 of the canonical
    /// template descriptor).
    pub id: TemplateId,
    /// Human-readable name (informational, not used for uniqueness).
    pub name: String,
    /// Semver string (e.g. "0.4.0").
    pub version: String,
    /// Required parameters.  Deployers must supply values for all of these.
    pub required_params: Vec<String>,
    /// Optional parameters with defaults.
    pub optional_params: Vec<Param>,
}

impl Template {
    /// Validate that `params` covers all required fields.
    pub fn validate_params(&self, params: &[Param]) -> Result<(), TemplateError> {
        let supplied: alloc::collections::BTreeSet<&str> =
            params.iter().map(|p| p.key.as_str()).collect();
        for req in &self.required_params {
            if !supplied.contains(req.as_str()) {
                return Err(TemplateError::MissingRequiredParam(req.clone()));
            }
        }
        Ok(())
    }
}

/// Template-layer errors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TemplateError {
    /// A required parameter was not supplied.
    MissingRequiredParam(String),
    /// Template not found in the registry.
    NotFound,
    /// Template id collision.
    AlreadyExists,
}

/// In-memory template catalogue.
///
/// In production this would be backed by on-chain storage via the registry
/// pallet; here it lives in a `BTreeMap` for ease of testing.
#[derive(Default)]
pub struct TemplateCatalogue {
    templates: alloc::collections::BTreeMap<TemplateId, Template>,
}

impl TemplateCatalogue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, t: Template) -> Result<(), TemplateError> {
        if self.templates.contains_key(&t.id) {
            return Err(TemplateError::AlreadyExists);
        }
        self.templates.insert(t.id, t);
        Ok(())
    }

    pub fn get(&self, id: &TemplateId) -> Option<&Template> {
        self.templates.get(id)
    }

    pub fn len(&self) -> usize {
        self.templates.len()
    }

    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
    }
}
