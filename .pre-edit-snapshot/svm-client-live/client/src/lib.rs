//! # X3 SVM live broadcaster (Solana devnet HTLC)
//!
//! Real, default-OFF broadcaster for X3 Atomic Swap HTLC on Solana devnet.
//! It builds, signs, and submits `CreateHtlc` / `ClaimHtlc` / `RefundHtlc`
//! transactions against the X3 `x3_atomic_swap` BPF program and returns the
//! genuine on-chain signature via `RpcClient`.
//!
//! This is the real counterpart to the in-memory `programs/svm` client
//! example. It mirrors the on-chain instruction layout documented in
//! `programs/svm/x3_atomic_swap/src/lib.rs`:
//!
//! | Tag | Instruction |
//! |-----|-------------|
//! | 0   | `CreateHtlc` |
//! | 1   | `ClaimHtlc` |
//! | 2   | `RefundHtlc` |
//!
//! # Security model — key is NEVER inlined
//!
//! The fee payer's keypair is read from a file path supplied by the caller
//! (e.g. a path resolved by a secret-store reference), never embedded in this
//! crate or passed on a command line. Broadcasting is driven purely by an
//! explicit [`SvmLiveConfig`]; there is **no** ambient auto-broadcast path.
//!
//! # Deployment
//!
//! The BPF program itself is deployed with
//! `programs/svm/x3_atomic_swap/deploy-devnet.sh`. Configure the returned
//! program ID into [`SvmLiveConfig::program_id`].

use solana_sdk::{
    hash::{hashv, Hash},
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair},
    signer::Signer,
    transaction::Transaction,
};

/// Well-known System Program address (never drifts; avoids the modular
/// `solana_system_interface`/`solana_program` version split).
pub const SYSTEM_PROGRAM_ID: Pubkey = Pubkey::from_str_const("11111111111111111111111111111111");

/// Program seed used for HTLC PDA derivation (must match the BPF program).
pub const HTLC_ACCOUNT_SEED: &[u8] = b"htlc";

/// Live client configuration. All fields are caller-supplied; no key material
/// is hardcoded here or anywhere in this crate.
#[derive(Clone, Debug)]
pub struct SvmLiveConfig {
    /// Solana JSON-RPC endpoint (defaults to public devnet).
    pub rpc_url: String,
    /// Program ID of the deployed `x3_atomic_swap` BPF program.
    pub program_id: Pubkey,
    /// File path to the fee-payer keypair JSON (resolved from a secret-store
    /// reference by the caller — never an inline secret).
    pub keypair_path: String,
    /// Confirmation commitment used when awaiting the transaction result.
    pub commitment: String,
}

impl Default for SvmLiveConfig {
    fn default() -> Self {
        Self {
            rpc_url: "https://api.devnet.solana.com".to_string(),
            program_id: Pubkey::default(),
            keypair_path: String::new(),
            commitment: "confirmed".to_string(),
        }
    }
}

/// A live, signed transaction submission result.
#[derive(Clone, Debug)]
pub struct LiveSubmission {
    /// Payer pubkey that funded and signed the tx.
    pub payer: Pubkey,
    /// The HTLC program address this tx targeted.
    pub program_id: Pubkey,
    /// Genuine 64-byte transaction signature (base58-encoded).
    pub signature: String,
    /// HTLC PDA address targeted (CreateHtlc) or affected (Claim/Refund).
    pub htlc_account: Pubkey,
}

/// Derive the HTLC PDA `(address, bump)` for a swap id, matching the program.
pub fn derive_htlc_pda(program_id: &Pubkey, swap_id: &[u8; 32]) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[HTLC_ACCOUNT_SEED, swap_id], program_id)
}

/// Build a `CreateHtlc` instruction with the on-chain data layout.
///
/// The parameter count mirrors the full on-chain HTLC field set.
#[allow(clippy::too_many_arguments)]
pub fn build_create_htlc_ix(
    program_id: &Pubkey,
    htlc_account: &Pubkey,
    payer: &Pubkey,
    initializer: &Pubkey,
    swap_id: &[u8; 32],
    claimant: &Pubkey,
    refund_authority: &Pubkey,
    hashlock: &[u8; 32],
    token_mint: &Pubkey,
    amount: u64,
    timeout: u64,
) -> Instruction {
    let mut data = Vec::with_capacity(1 + 32 * 5 + 8 + 8);
    data.push(0u8); // tag: CreateHtlc
    data.extend_from_slice(swap_id);
    data.extend_from_slice(claimant.as_ref());
    data.extend_from_slice(refund_authority.as_ref());
    data.extend_from_slice(hashlock);
    data.extend_from_slice(token_mint.as_ref());
    data.extend_from_slice(&amount.to_le_bytes());
    data.extend_from_slice(&timeout.to_le_bytes());
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*htlc_account, false),
            AccountMeta::new(*payer, true),
            AccountMeta::new_readonly(*initializer, true),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

/// Build a `ClaimHtlc` instruction (tag 1 + preimage_len + preimage).
pub fn build_claim_htlc_ix(
    program_id: &Pubkey,
    htlc_account: &Pubkey,
    claimant: &Pubkey,
    preimage: &[u8],
) -> Instruction {
    let preimage_len = preimage.len().min(255) as u8;
    let mut data = Vec::with_capacity(2 + preimage_len as usize);
    data.push(1u8);
    data.push(preimage_len);
    data.extend_from_slice(&preimage[..preimage_len as usize]);
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*htlc_account, false),
            AccountMeta::new_readonly(*claimant, true),
        ],
        data,
    }
}

/// Build a `RefundHtlc` instruction (tag 2).
pub fn build_refund_htlc_ix(
    program_id: &Pubkey,
    htlc_account: &Pubkey,
    refund_authority: &Pubkey,
) -> Instruction {
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*htlc_account, false),
            AccountMeta::new_readonly(*refund_authority, true),
        ],
        data: vec![2u8],
    }
}

/// Convenience: compute the lock hash for a preimage using Solana's hashing.
pub fn lock_hash(preimage: &[u8]) -> Hash {
    hashv(&[preimage])
}

/// Load the fee-payer signing keypair from a file path.
///
/// The path must point at a Solana keypair JSON as produced by
/// `solana-keygen` / a secret-store-backed keyfile. This function never
/// accepts inline key bytes.
pub fn load_payer(cfg: &SvmLiveConfig) -> Result<Keypair, String> {
    if cfg.keypair_path.is_empty() {
        return Err("x3-svm-client: keypair_path is empty (not configured)".into());
    }
    read_keypair_file(&cfg.keypair_path).map_err(|e| {
        format!(
            "x3-svm-client: failed to read keypair '{}': {}",
            cfg.keypair_path, e
        )
    })
}

/// Broadcast `CreateHtlc` and await confirmation, returning the real signature.
///
/// The fee payer signs the transaction. When the on-chain `initializer` (the
/// party locking funds) is a distinct wallet, pass its keypair as
/// `initializer_signer` so the message includes both signatures; for a
/// self-lock (`initializer == payer`) pass `None`.
///
/// # Default-OFF
/// This only performs network I/O when a caller explicitly constructs a
/// [`SvmLiveConfig`] with a real program id + keypair path and calls this
/// method. Nothing in this crate triggers a broadcast on its own.
#[allow(clippy::too_many_arguments)]
pub fn broadcast_create_htlc(
    cfg: &SvmLiveConfig,
    payer: &Keypair,
    initializer_signer: Option<&Keypair>,
    initializer_pk: &Pubkey,
    swap_id: &[u8; 32],
    claimant: &Pubkey,
    refund_authority: &Pubkey,
    hashlock: &[u8; 32],
    amount: u64,
    timeout_slots: u64,
) -> Result<LiveSubmission, String> {
    let program_id = cfg.program_id;
    if program_id == Pubkey::default() {
        return Err("x3-svm-client: program_id not configured".into());
    }
    let (htlc_account, _bump) = derive_htlc_pda(&program_id, swap_id);
    let payer_pk = payer.pubkey();
    let ix = build_create_htlc_ix(
        &program_id,
        &htlc_account,
        &payer_pk,
        initializer_pk,
        swap_id,
        claimant,
        refund_authority,
        hashlock,
        &Pubkey::default(), // native SOL
        amount,
        timeout_slots,
    );
    let mut signers: Vec<&Keypair> = vec![payer];
    if let Some(s) = initializer_signer {
        // Only add the initializer as a distinct signer when it is not the payer.
        if s.pubkey() != payer_pk {
            signers.push(s);
        }
    }
    submit(cfg, &signers, vec![ix], &htlc_account)
}

/// Broadcast `ClaimHtlc` and await confirmation.
pub fn broadcast_claim_htlc(
    cfg: &SvmLiveConfig,
    payer: &Keypair,
    claimant: &Pubkey,
    swap_id: &[u8; 32],
    preimage: &[u8],
) -> Result<LiveSubmission, String> {
    let (htlc_account, _bump) = derive_htlc_pda(&cfg.program_id, swap_id);
    let ix = build_claim_htlc_ix(&cfg.program_id, &htlc_account, claimant, preimage);
    submit(cfg, &[payer], vec![ix], &htlc_account)
}

/// Broadcast `RefundHtlc` and await confirmation.
pub fn broadcast_refund_htlc(
    cfg: &SvmLiveConfig,
    payer: &Keypair,
    refund_authority: &Pubkey,
    swap_id: &[u8; 32],
) -> Result<LiveSubmission, String> {
    let (htlc_account, _bump) = derive_htlc_pda(&cfg.program_id, swap_id);
    let ix = build_refund_htlc_ix(&cfg.program_id, &htlc_account, refund_authority);
    submit(cfg, &[payer], vec![ix], &htlc_account)
}

/// Sign and submit a transaction, returning its genuine signature.
fn submit(
    cfg: &SvmLiveConfig,
    signers: &[&Keypair],
    instructions: Vec<Instruction>,
    htlc_account: &Pubkey,
) -> Result<LiveSubmission, String> {
    let client = RpcClient::new(cfg.rpc_url.clone());

    let recent_blockhash = client
        .get_latest_blockhash()
        .map_err(|e| format!("x3-svm-client: get_latest_blockhash failed: {}", e))?;

    let payer = signers[0];
    let payer_pk = payer.pubkey();
    let tx = Transaction::new_signed_with_payer(
        &instructions,
        Some(&payer_pk),
        signers,
        recent_blockhash,
    );

    let signature = client
        .send_and_confirm_transaction(&tx)
        .map_err(|e| format!("x3-svm-client: send_and_confirm failed: {}", e))?;

    Ok(LiveSubmission {
        payer: payer_pk,
        program_id: cfg.program_id,
        signature: signature.to_string(),
        htlc_account: *htlc_account,
    })
}

// Synchronous JSON-RPC client for signing + submitting real transactions.
use solana_rpc_client::rpc_client::RpcClient;

#[cfg(test)]
mod tests {
    use super::*;

    fn test_cfg() -> SvmLiveConfig {
        SvmLiveConfig::default()
    }

    #[test]
    fn pda_is_stable_and_matches_program_seed() {
        let pid = Pubkey::new_unique();
        let swap_id = [7u8; 32];
        let (pda, bump) = derive_htlc_pda(&pid, &swap_id);
        // Recompute via find_program_address to confirm seeds tag + swap_id.
        let expect = Pubkey::find_program_address(&[HTLC_ACCOUNT_SEED, &swap_id], &pid);
        assert_eq!((pda, bump), expect);
        assert!(!pda.eq(&Pubkey::default()));
    }

    #[test]
    fn create_instruction_matches_on_chain_layout() {
        let pid = Pubkey::new_unique();
        let (htlc, _) = derive_htlc_pda(&pid, &[1u8; 32]);
        let payer = Pubkey::new_unique();
        let initializer = Pubkey::new_unique();
        let claimant = Pubkey::new_unique();
        let refund = Pubkey::new_unique();
        let hashlock = [0xABu8; 32];
        let ix = build_create_htlc_ix(
            &pid,
            &htlc,
            &payer,
            &initializer,
            &[1u8; 32],
            &claimant,
            &refund,
            &hashlock,
            &Pubkey::default(),
            1_000,
            400,
        );
        // tag 0, then 32+32+32+32+32+8+8
        assert_eq!(ix.data.len(), 1 + 32 * 5 + 8 + 8);
        assert_eq!(ix.data[0], 0u8);
        assert_eq!(ix.accounts.len(), 4);
        // payer + initializer are signers
        assert!(ix.accounts.iter().filter(|m| m.is_signer).count() == 2);
    }

    #[test]
    fn claim_preimage_encoding() {
        let pid = Pubkey::new_unique();
        let htlc = Pubkey::new_unique();
        let claimant = Pubkey::new_unique();
        let ix = build_claim_htlc_ix(&pid, &htlc, &claimant, b"secret-preimage");
        assert_eq!(ix.data[0], 1u8);
        assert_eq!(ix.data[1] as usize, b"secret-preimage".len());
        assert_eq!(&ix.data[2..], b"secret-preimage");
        assert_eq!(ix.accounts.len(), 2);
    }

    #[test]
    fn lock_hash_matches_preimage_commitment() {
        let h = lock_hash(b"x3");
        assert_eq!(h.as_ref().len(), 32);
    }

    #[test]
    fn payer_load_rejects_empty_path() {
        let cfg = test_cfg(); // keypair_path empty by default
        assert!(load_payer(&cfg).is_err());
    }
}
