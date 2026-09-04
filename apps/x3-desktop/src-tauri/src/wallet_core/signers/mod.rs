use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct ProcessConfig {
    pub max_memory_mb: u32,
    pub allow_swap: bool,
    pub lock_memory: bool, // mlockall
    pub disable_core_dumps: bool,
}

pub trait IsolatedSigner {
    fn derive_address(&self, path: &str) -> Result<String, SignerError>;
    
    // Crucial rule: preimage only allowed IF accompanied by valid attestation
    fn sign_intent(&self, preimage: &super::ipc::IntentDraft, attestation: &super::ipc::Attestation) -> Result<String, SignerError>;
    
    // Strict bytes signing ONLY IF the tx exactly matches the approved intent
    fn sign_tx(&self, canonical_tx_bytes: &[u8], intent_id: &str) -> Result<String, SignerError>;
    
    fn get_capabilities(&self) -> super::ipc::SignerCaps;
}

#[derive(Debug)]
pub enum SignerError {
    AttestationInvalid,
    AttestationExpired,
    IntentMismatch,
    HardwareDisconnected,
    EnclaveLocked,
    CryptoError(String)
}

// Memory lock syscall stubs (linux)
#[cfg(target_os = "linux")]
pub fn secure_memory_init() -> Result<(), String> {
    unsafe {
        // libc::mlockall(libc::MCL_CURRENT | libc::MCL_FUTURE);
        // libc::setrlimit(libc::RLIMIT_CORE, &libc::rlimit { rlim_cur: 0, rlim_max: 0 });
    }
    Ok(())
}

pub mod evm;

pub mod svm;
pub mod common;
pub mod btc;
