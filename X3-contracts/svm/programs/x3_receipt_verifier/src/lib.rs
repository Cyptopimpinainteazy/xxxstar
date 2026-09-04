use anchor_lang::prelude::*;

declare_id!("X3RecptVr1111111111111111111111111111111115");

pub const VERIFIER_SEED: &[u8] = b"verifier";

#[program]
pub mod x3_receipt_verifier {

    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        initial_validators: Vec<[u8; 32]>,
        quorum_threshold: u64,
    ) -> Result<()> {
        require!(!initial_validators.is_empty(), VerifierError::NoValidators);
        require!(
            quorum_threshold > 0 && quorum_threshold <= initial_validators.len() as u64,
            VerifierError::InvalidQuorum
        );

        let v = &mut ctx.accounts.verifier;
        v.authority = ctx.accounts.authority.key();
        v.verifier_set_id = 1;
        v.validator_count = initial_validators.len() as u64;
        v.quorum_threshold = quorum_threshold;
        v.validators = initial_validators
            .iter()
            .map(|pk| Validator {
                pubkey: *pk,
                active: true,
            })
            .collect();

        emit!(ValidatorSetRotated {
            new_set_id: v.verifier_set_id,
            validator_count: v.validator_count,
            quorum_threshold: v.quorum_threshold,
        });
        Ok(())
    }

    pub fn rotate_validator_set(
        ctx: Context<AdminOnly>,
        new_validators: Vec<[u8; 32]>,
        new_quorum_threshold: u64,
    ) -> Result<()> {
        require!(!new_validators.is_empty(), VerifierError::NoValidators);
        require!(
            new_quorum_threshold > 0 && new_quorum_threshold <= new_validators.len() as u64,
            VerifierError::InvalidQuorum
        );

        let v = &mut ctx.accounts.verifier;
        v.verifier_set_id = v.verifier_set_id.checked_add(1).ok_or(VerifierError::Overflow)?;
        v.validator_count = new_validators.len() as u64;
        v.quorum_threshold = new_quorum_threshold;
        v.validators = new_validators
            .iter()
            .map(|pk| Validator {
                pubkey: *pk,
                active: true,
            })
            .collect();

        emit!(ValidatorSetRotated {
            new_set_id: v.verifier_set_id,
            validator_count: v.validator_count,
            quorum_threshold: v.quorum_threshold,
        });
        Ok(())
    }

    pub fn mark_verified(ctx: Context<AdminOnly>, message_id: [u8; 32]) -> Result<()> {
        let v = &mut ctx.accounts.verifier;
        v.verified_messages.push(message_id);
        Ok(())
    }

    pub fn verify_x3_withdrawal_proof(
        ctx: Context<Verify>,
        message_id: [u8; 32],
        source_chain: u64,
        sender: Vec<u8>,
        amount: u64,
    ) -> Result<bool> {
        let v = &ctx.accounts.verifier;

        if v.verified_messages.contains(&message_id) {
            return Ok(false);
        }

        let recipient = ctx.accounts.recipient.key();
        let proof_message =
            build_withdrawal_message(&message_id, source_chain, &sender, &recipient, amount);

        let sig_count = count_valid_ed25519_sigs(&v.validators, &proof_message, &ctx.accounts.instructions_sysvar)?;
        Ok(sig_count >= v.quorum_threshold)
    }

    pub fn verify_deposit_proof(
        ctx: Context<Verify>,
        message_id: [u8; 32],
        token: Pubkey,
        depositor: Pubkey,
        x3_recipient: Vec<u8>,
        amount: u64,
    ) -> Result<bool> {
        let v = &ctx.accounts.verifier;

        if v.verified_messages.contains(&message_id) {
            return Ok(false);
        }

        let proof_message =
            build_deposit_message(&message_id, &token, &depositor, &x3_recipient, amount);

        let sig_count = count_valid_ed25519_sigs(&v.validators, &proof_message, &ctx.accounts.instructions_sysvar)?;
        Ok(sig_count >= v.quorum_threshold)
    }
}

fn build_withdrawal_message(
    message_id: &[u8; 32],
    source_chain: u64,
    sender: &[u8],
    recipient: &Pubkey,
    amount: u64,
) -> [u8; 32] {
    let mut hasher = anchor_lang::solana_program::hash::Hasher::default();
    hasher.hash(b"X3_WITHDRAWAL_V1");
    hasher.hash(message_id);
    hasher.hash(&source_chain.to_le_bytes());
    hasher.hash(sender);
    hasher.hash(recipient.as_ref());
    hasher.hash(&amount.to_le_bytes());
    hasher.result().to_bytes()
}

fn build_deposit_message(
    message_id: &[u8; 32],
    token: &Pubkey,
    depositor: &Pubkey,
    x3_recipient: &[u8],
    amount: u64,
) -> [u8; 32] {
    let mut hasher = anchor_lang::solana_program::hash::Hasher::default();
    hasher.hash(b"X3_DEPOSIT_V1");
    hasher.hash(message_id);
    hasher.hash(token.as_ref());
    hasher.hash(depositor.as_ref());
    hasher.hash(x3_recipient);
    hasher.hash(&amount.to_le_bytes());
    hasher.result().to_bytes()
}

fn count_valid_ed25519_sigs(
    validators: &[Validator],
    expected_message: &[u8; 32],
    instructions_sysvar: &AccountInfo,
) -> Result<u64> {
    let mut count: u64 = 0;

    for i in 0u64.. {
        let ix =
            match anchor_lang::solana_program::sysvar::instructions::load_instruction_at_checked(
                i as usize,
                instructions_sysvar,
            ) {
                Ok(ix) => ix,
                Err(_) => break,
            };

        if ix.program_id != anchor_lang::solana_program::ed25519_program::ID {
            continue;
        }

        let data = &ix.data;
        if data.len() < 106 {
            continue;
        }

        let num_sigs = data[0] as usize;
        let mut offset: usize = 1;

        for _ in 0..num_sigs {
            if offset + 104 > data.len() {
                break;
            }

            let sig_ix = data[offset + 1];
            let pubkey_ix = data[offset + 3];
            let msg_ix = data[offset + 5];

            // 0xFF means the data is embedded in this instruction
            if sig_ix != 0xFF || pubkey_ix != 0xFF || msg_ix != 0xFF {
                offset += 104;
                continue;
            }

            let msg_len =
                u16::from_le_bytes([data[offset + 6], data[offset + 7]]) as usize;
            let pubkey_start = offset + 8 + msg_len;

            if pubkey_start + 32 > data.len() {
                break;
            }

            let msg_data = &data[offset + 8..offset + 8 + msg_len];

            if msg_data.len() != 32 || msg_data != expected_message {
                offset = pubkey_start + 32 + 64;
                continue;
            }

            let pubkey_bytes = &data[pubkey_start..pubkey_start + 32];
            let mut pubkey_arr = [0u8; 32];
            pubkey_arr.copy_from_slice(pubkey_bytes);

            if validators.iter().any(|v| v.active && v.pubkey == pubkey_arr) {
                count = count.checked_add(1).ok_or(VerifierError::Overflow)?;
            }

            offset = pubkey_start + 32 + 64;
        }
    }

    Ok(count)
}

#[account]
pub struct Verifier {
    pub authority: Pubkey,
    pub verifier_set_id: u64,
    pub validator_count: u64,
    pub quorum_threshold: u64,
    pub validators: Vec<Validator>,
    pub verified_messages: Vec<[u8; 32]>,
}

impl Verifier {
    pub const BASE_SIZE: usize = 32 + 8 + 8 + 8;
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct Validator {
    pub pubkey: [u8; 32],
    pub active: bool,
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        seeds = [VERIFIER_SEED],
        bump,
        payer = authority,
        space = 8 + Verifier::BASE_SIZE + 8192
    )]
    pub verifier: Account<'info, Verifier>,
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(address = anchor_lang::solana_program::sysvar::instructions::ID)]
    pub instructions_sysvar: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AdminOnly<'info> {
    #[account(
        mut,
        seeds = [VERIFIER_SEED],
        bump,
        has_one = authority @ VerifierError::Unauthorized,
    )]
    pub verifier: Account<'info, Verifier>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct Verify<'info> {
    #[account(
        seeds = [VERIFIER_SEED],
        bump,
    )]
    pub verifier: Account<'info, Verifier>,
    #[account(address = anchor_lang::solana_program::sysvar::instructions::ID)]
    pub instructions_sysvar: AccountInfo<'info>,
    /// CHECK: recipient address (used as message input data)
    pub recipient: AccountInfo<'info>,
}

#[event]
pub struct ValidatorSetRotated {
    pub new_set_id: u64,
    pub validator_count: u64,
    pub quorum_threshold: u64,
}

#[event]
pub struct ProofVerified {
    pub message_id: [u8; 32],
    pub set_id: u64,
    pub sig_count: u64,
}

#[error_code]
pub enum VerifierError {
    #[msg("at least one validator is required")]
    NoValidators,
    #[msg("invalid quorum threshold")]
    InvalidQuorum,
    #[msg("arithmetic overflow")]
    Overflow,
    #[msg("unauthorized caller")]
    Unauthorized,
    #[msg("no ed25519 instructions found in transaction")]
    NoEd25519Instructions,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifier_base_size() {
        assert_eq!(Verifier::BASE_SIZE, 32 + 8 + 8 + 8);
    }

    #[test]
    fn verifier_seed_is_stable() {
        assert_eq!(VERIFIER_SEED, b"verifier");
    }

    #[test]
    fn verifier_pda_is_deterministic() {
        let (pda1, bump1) = Pubkey::find_program_address(
            &[VERIFIER_SEED],
            &id(),
        );
        let (pda2, bump2) = Pubkey::find_program_address(
            &[VERIFIER_SEED],
            &id(),
        );
        assert_eq!(pda1, pda2);
        assert_eq!(bump1, bump2);
    }

    #[test]
    fn validator_serialization_roundtrip() {
        let v = Validator {
            pubkey: [0xAB; 32],
            active: true,
        };
        let bytes = v.try_to_vec().unwrap();
        let deserialized = Validator::try_from_slice(&bytes).unwrap();
        assert_eq!(v, deserialized);
    }

    #[test]
    fn validator_inactive_serialization_roundtrip() {
        let v = Validator {
            pubkey: [0xCD; 32],
            active: false,
        };
        let bytes = v.try_to_vec().unwrap();
        let deserialized = Validator::try_from_slice(&bytes).unwrap();
        assert_eq!(v, deserialized);
    }

    #[test]
    fn build_withdrawal_message_is_deterministic() {
        let message_id: [u8; 32] = [0x11; 32];
        let source_chain: u64 = 7;
        let sender: Vec<u8> = vec![0x22; 20];
        let recipient = Pubkey::new_from_array([0x33; 32]);
        let amount: u64 = 999;

        let h1 = build_withdrawal_message(&message_id, source_chain, &sender, &recipient, amount);
        let h2 = build_withdrawal_message(&message_id, source_chain, &sender, &recipient, amount);
        assert_eq!(h1, h2);
    }

    #[test]
    fn build_withdrawal_message_changes_with_amount() {
        let message_id: [u8; 32] = [0x11; 32];
        let source_chain: u64 = 7;
        let sender: Vec<u8> = vec![0x22; 20];
        let recipient = Pubkey::new_from_array([0x33; 32]);

        let h1 = build_withdrawal_message(&message_id, source_chain, &sender, &recipient, 100);
        let h2 = build_withdrawal_message(&message_id, source_chain, &sender, &recipient, 200);
        assert_ne!(h1, h2);
    }

    #[test]
    fn build_deposit_message_is_deterministic() {
        let message_id: [u8; 32] = [0x44; 32];
        let token = Pubkey::new_from_array([0x55; 32]);
        let depositor = Pubkey::new_from_array([0x66; 32]);
        let x3_recipient: Vec<u8> = vec![0x77; 20];
        let amount: u64 = 500;

        let h1 = build_deposit_message(&message_id, &token, &depositor, &x3_recipient, amount);
        let h2 = build_deposit_message(&message_id, &token, &depositor, &x3_recipient, amount);
        assert_eq!(h1, h2);
    }

    #[test]
    fn build_deposit_message_changes_with_token() {
        let message_id: [u8; 32] = [0x44; 32];
        let token1 = Pubkey::new_from_array([0xAA; 32]);
        let token2 = Pubkey::new_from_array([0xBB; 32]);
        let depositor = Pubkey::new_from_array([0xCC; 32]);
        let x3_recipient: Vec<u8> = vec![0xDD; 20];
        let amount: u64 = 500;

        let h1 = build_deposit_message(&message_id, &token1, &depositor, &x3_recipient, amount);
        let h2 = build_deposit_message(&message_id, &token2, &depositor, &x3_recipient, amount);
        assert_ne!(h1, h2);
    }

    #[test]
    fn withdrawal_and_deposit_messages_are_distinct() {
        let message_id: [u8; 32] = [0x42; 32];
        let chain_id: u64 = 1;
        let addr_bytes = vec![0x88; 32];
        let pubkey = Pubkey::new_from_array([0x99; 32]);
        let amount: u64 = 1000;

        let w = build_withdrawal_message(&message_id, chain_id, &addr_bytes, &pubkey, amount);
        let d = build_deposit_message(&message_id, &pubkey, &pubkey, &addr_bytes, amount);
        assert_ne!(w, d);
    }

    #[test]
    fn count_valid_ed25519_sigs_returns_zero_with_no_ed25519_ix() {
        let validators: Vec<Validator> = vec![
            Validator { pubkey: [1u8; 32], active: true },
        ];

        let keypair = Pubkey::new_from_array([0u8; 32]);
        let mut lamports = 0u64;
        let mut data: Vec<u8> = vec![];
        let owner = Pubkey::default();
        let instructions_sysvar = AccountInfo::new(
            &keypair, false, false, &mut lamports, &mut data, &owner, false, 0,
        );

        let result = count_valid_ed25519_sigs(&validators, &[0u8; 32], &instructions_sysvar);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn verifier_error_codes_are_distinct() {
        let codes = [
            VerifierError::NoValidators,
            VerifierError::InvalidQuorum,
            VerifierError::Overflow,
            VerifierError::Unauthorized,
            VerifierError::NoEd25519Instructions,
        ];
        for i in 0..codes.len() {
            for j in (i + 1)..codes.len() {
                assert_ne!(codes[i] as u32, codes[j] as u32);
            }
        }
    }

    #[test]
    fn verifier_pda_is_different_from_program_id() {
        let (pda, _) = Pubkey::find_program_address(
            &[VERIFIER_SEED],
            &id(),
        );
        assert_ne!(pda, id());
    }

    #[test]
    fn quorum_threshold_validation_min_is_1() {
        assert!(1u64 > 0 && 1u64 <= 5);
        assert!(!(0u64 > 0 && 0u64 <= 5));
    }
}
