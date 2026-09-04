//! X3 Atomic Swap HTLC — Client Example
//!
//! This example demonstrates how to construct and send HTLC instructions
//! using `solana-sdk`. It is intended as documentation-grade reference code.
//! To run, you need a running `solana-test-validator` with the HTLC program
//! deployed.
//!
//! # Usage (requires solana-test-validator)
//!
//! ```bash
//! # Terminal 1: Start test validator with the program deployed
//! solana-test-validator --bpf-program <HTLC_PROGRAM_ID> target/deploy/x3_atomic_swap.so
//!
//! # Terminal 2: Run this example
//! cargo run --example client --features no-entrypoint
//! ```
//!
//! # Overview
//!
//! 1. Generate keypairs for payer, initializer, and claimant.
//! 2. Derive the HTLC PDA from a random swap ID.
//! 3. Build and send a `CreateHtlc` instruction.
//! 4. Build and send a `ClaimHtlc` instruction with the correct preimage.
//!
//! # Instructions Assembled
//!
//! - **CreateHtlc**: Tag `0`, followed by `swap_id (32) + claimant (32) +
//!   refund_authority (32) + hashlock (32) + token_mint (32) + amount (8) +
//!   timeout (8)` = 177 bytes total.
//! - **ClaimHtlc**: Tag `1`, followed by `preimage_len (1) + preimage (N)`.
//! - **RefundHtlc**: Tag `2`, no additional data.

#![allow(dead_code)]

use solana_sdk::{
    hash::hashv,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    system_program,
    transaction::Transaction,
};

/// Program ID of the deployed X3 Atomic Swap HTLC BPF program.
///
/// Replace with the actual deployed program ID.
const HTLC_PROGRAM_ID: &str = "Htlc1111111111111111111111111111111111111111";

/// Seed used for PDA derivation (must match `HTLC_ACCOUNT_SEED` in the program).
const HTLC_ACCOUNT_SEED: &[u8] = b"htlc";

/// Build a `CreateHtlc` instruction.
///
/// Assembles the instruction data manually using the on-chain format:
/// tag(1) + swap_id(32) + claimant(32) + refund_authority(32) + hashlock(32)
/// + token_mint(32) + amount(8) + timeout(8).
fn build_create_htlc_ix(
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
    let mut data = Vec::with_capacity(1 + 176);
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
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data,
    }
}

/// Build a `ClaimHtlc` instruction.
///
/// Data format: tag(1) + preimage_len(1) + preimage(N).
fn build_claim_htlc_ix(
    program_id: &Pubkey,
    htlc_account: &Pubkey,
    claimant: &Pubkey,
    preimage: &[u8],
) -> Instruction {
    let preimage_len = preimage.len().min(255) as u8;
    let mut data = Vec::with_capacity(2 + preimage_len as usize);
    data.push(1u8); // tag: ClaimHtlc
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

/// Build a `RefundHtlc` instruction.
///
/// Data format: tag(2).
fn build_refund_htlc_ix(
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
        data: vec![2u8], // tag: RefundHtlc
    }
}

/// Derive the HTLC PDA for a given swap ID.
fn derive_htlc_pda(program_id: &Pubkey, swap_id: &[u8; 32]) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[HTLC_ACCOUNT_SEED, swap_id], program_id)
}

fn main() {
    // --- Setup ---
    let program_id: Pubkey = HTLC_PROGRAM_ID.parse().expect("valid program ID");

    let payer = Keypair::new();
    let initializer = Keypair::new();
    let claimant = Keypair::new();
    let refund_authority = Keypair::new();
    let token_mint = Pubkey::default(); // native SOL

    // Generate a random swap ID
    let swap_id: [u8; 32] = {
        let mut id = [0u8; 32];
        // In a real scenario, use a cryptographically secure RNG.
        // Here we derive from a known value for reproducibility.
        let hash = hashv(&[b"example-swap-42"]);
        id.copy_from_slice(hash.as_ref());
        id
    };

    // Preimage and hashlock
    let preimage = b"x3-atomic-swap-secret-123";
    let hashlock = hashv(&[preimage]).to_bytes();

    let timelock: u64 = 1_234_567_890; // slot number
    let amount: u64 = 1_000_000_000; // 1 SOL in lamports

    // Derive PDA
    let (htlc_pda, bump) = derive_htlc_pda(&program_id, &swap_id);
    println!("HTLC Program ID:    {}", program_id);
    println!("Payer:              {}", payer.pubkey());
    println!("Initializer:        {}", initializer.pubkey());
    println!("Claimant:           {}", claimant.pubkey());
    println!("Refund Authority:   {}", refund_authority.pubkey());
    println!("HTLC PDA:           {} (bump={})", htlc_pda, bump);
    println!("Swap ID:            {:?}", swap_id);
    println!("Hashlock:           {:?}", hashlock);
    println!("Amount:             {} lamports", amount);
    println!("Timelock (slot):    {}", timelock);

    // --- Step 1: Build CreateHtlc instruction ---
    let create_ix = build_create_htlc_ix(
        &program_id,
        &htlc_pda,
        &payer.pubkey(),
        &initializer.pubkey(),
        &swap_id,
        &claimant.pubkey(),
        &refund_authority.pubkey(),
        &hashlock,
        &token_mint,
        amount,
        timelock,
    );

    println!("\n--- CreateHtlc Instruction ---");
    println!("Program ID: {}", create_ix.program_id);
    println!("Accounts:   {}", create_ix.accounts.len());
    println!("Data len:   {} bytes", create_ix.data.len());

    // --- Step 2: Build ClaimHtlc instruction ---
    let claim_ix = build_claim_htlc_ix(
        &program_id,
        &htlc_pda,
        &claimant.pubkey(),
        preimage,
    );

    println!("\n--- ClaimHtlc Instruction ---");
    println!("Program ID: {}", claim_ix.program_id);
    println!("Accounts:   {}", claim_ix.accounts.len());
    println!("Data len:   {} bytes", claim_ix.data.len());

    // --- Step 3: Build RefundHtlc instruction (alternative to claim) ---
    let refund_ix = build_refund_htlc_ix(
        &program_id,
        &htlc_pda,
        &refund_authority.pubkey(),
    );

    println!("\n--- RefundHtlc Instruction ---");
    println!("Program ID: {}", refund_ix.program_id);
    println!("Accounts:   {}", refund_ix.accounts.len());
    println!("Data len:   {} bytes", refund_ix.data.len());

    // --- Assembling a Transaction (pseudocode) ---
    // In a real scenario with a running solana-test-validator:
    //
    // let client = RpcClient::new("http://localhost:8899");
    //
    // // Airdrop SOL to payer
    // let signature = client.request_airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    // client.confirm_transaction(&signature).unwrap();
    //
    // // Create transaction with CreateHtlc
    // let recent_blockhash = client.get_latest_blockhash().unwrap();
    // let tx = Transaction::new_signed_with_payer(
    //     &[create_ix],
    //     Some(&payer.pubkey()),
    //     &[&payer, &initializer],
    //     recent_blockhash,
    // );
    // let signature = client.send_and_confirm_transaction(&tx).unwrap();
    // println!("\nCreateHtlc tx: {}", signature);
    //
    // // Then claim with the preimage:
    // let recent_blockhash = client.get_latest_blockhash().unwrap();
    // let tx = Transaction::new_signed_with_payer(
    //     &[claim_ix],
    //     Some(&claimant.pubkey()),
    //     &[&claimant],
    //     recent_blockhash,
    // );
    // let signature = client.send_and_confirm_transaction(&tx).unwrap();
    // println!("ClaimHtlc tx: {}", signature);

    println!("\n--- Instructions assembled successfully ---");
    println!("To execute against solana-test-validator:");
    println!("  1. Deploy the BPF program");
    println!("  2. Update HTLC_PROGRAM_ID constant");
    println!("  3. Uncomment the RpcClient code in main()");
    println!("  4. Run with: cargo run --example client --features no-entrypoint");
}
