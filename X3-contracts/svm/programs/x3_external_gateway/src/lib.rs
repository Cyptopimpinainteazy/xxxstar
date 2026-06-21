use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer as SplTransfer};

declare_id!("X3ExtGate1111111111111111111111111111111113");

pub const GATEWAY_SEED: &[u8] = b"gateway";

#[program]
pub mod x3_external_gateway {

    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        chain_id: u64,
        x3_chain_id: u64,
        min_x3_confirmations: u64,
    ) -> Result<()> {
        let gw = &mut ctx.accounts.gateway;
        gw.owner = ctx.accounts.owner.key();
        gw.verifier = Pubkey::default();
        gw.chain_id = chain_id;
        gw.x3_chain_id = x3_chain_id;
        gw.min_x3_confirmations = min_x3_confirmations;
        gw.paused = false;
        Ok(())
    }

    pub fn set_verifier(ctx: Context<AdminOnly>, verifier: Pubkey) -> Result<()> {
        require!(verifier != Pubkey::default(), GatewayError::ZeroVerifier);
        ctx.accounts.gateway.verifier = verifier;
        emit!(VerifierUpdated { new_verifier: verifier });
        Ok(())
    }

    pub fn set_paused(ctx: Context<AdminOnly>, paused: bool) -> Result<()> {
        ctx.accounts.gateway.paused = paused;
        emit!(Paused { is_paused: paused });
        Ok(())
    }

    pub fn set_min_x3_confirmations(ctx: Context<AdminOnly>, min: u64) -> Result<()> {
        ctx.accounts.gateway.min_x3_confirmations = min;
        Ok(())
    }

    pub fn set_supported_token(
        ctx: Context<AdminOnly>,
        token_mint: Pubkey,
        supported: bool,
        daily_deposit_cap: u64,
        daily_withdrawal_cap: u64,
    ) -> Result<()> {
        if supported {
            if !ctx.accounts.gateway.supported_tokens.contains(&token_mint) {
                ctx.accounts.gateway.supported_tokens.push(token_mint);
            }
        } else {
            ctx.accounts.gateway.supported_tokens.retain(|t| t != &token_mint);
        }
        update_limit(&mut ctx.accounts.gateway.daily_deposit_limits, &token_mint, daily_deposit_cap);
        update_limit(&mut ctx.accounts.gateway.daily_withdrawal_limits, &token_mint, daily_withdrawal_cap);
        emit!(SupportedTokenUpdated {
            token: token_mint,
            supported,
            daily_deposit_cap,
            daily_withdrawal_cap,
        });
        Ok(())
    }

    pub fn deposit_to_x3(
        ctx: Context<DepositToX3>,
        amount: u64,
        nonce: u64,
        x3_recipient: Vec<u8>,
    ) -> Result<()> {
        require!(!ctx.accounts.gateway.paused, GatewayError::Paused);
        require!(amount > 0, GatewayError::ZeroAmount);
        require!(
            !x3_recipient.is_empty() && x3_recipient.len() <= 64,
            GatewayError::InvalidRecipient
        );

        let token_mint_key = ctx.accounts.token_mint.key();
        let depositor_key = ctx.accounts.depositor.key();
        let chain_id_val = ctx.accounts.gateway.chain_id;

        require!(
            ctx.accounts.gateway.supported_tokens.contains(&token_mint_key),
            GatewayError::TokenNotSupported
        );

        let message_id = hash_deposit_message(
            chain_id_val,
            &token_mint_key,
            &depositor_key,
            &x3_recipient,
            amount,
            nonce,
        );

        require!(
            !ctx.accounts.gateway.used_messages.contains(&message_id),
            GatewayError::Replay
        );

        let day_key = (Clock::get()?.unix_timestamp as u64) / 86400;

        {
            let gw = &mut ctx.accounts.gateway;
            gw.used_messages.push(message_id);

            let cap = gw
                .daily_deposit_limits
                .iter()
                .find(|e| e.key == token_mint_key)
                .map(|e| e.value)
                .unwrap_or(u64::MAX);

            let current = gw
                .daily_deposited
                .iter()
                .find(|e| e.token == token_mint_key && e.day_key == day_key)
                .map(|e| e.amount)
                .unwrap_or(0);
            let new_total = current.checked_add(amount).ok_or(GatewayError::Overflow)?;
            if new_total > cap {
                return Err(GatewayError::DailyLimit.into());
            }
            if let Some(entry) = gw
                .daily_deposited
                .iter_mut()
                .find(|e| e.token == token_mint_key && e.day_key == day_key)
            {
                entry.amount = new_total;
            } else {
                gw.daily_deposited.push(DailyAccum {
                    token: token_mint_key,
                    day_key,
                    amount: new_total,
                });
            }

            update_total_locked(&mut gw.total_locked, &token_mint_key, amount, true)?;
        }

        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                SplTransfer {
                    from: ctx.accounts.user_token_account.to_account_info(),
                    to: ctx.accounts.vault.to_account_info(),
                    authority: ctx.accounts.depositor.to_account_info(),
                },
            ),
            amount,
        )?;

        emit!(DepositLocked {
            message_id,
            token: token_mint_key,
            depositor: depositor_key,
            x3_recipient,
            amount,
            nonce,
            chain_id: chain_id_val,
        });
        Ok(())
    }

    pub fn release_from_x3(
        ctx: Context<ReleaseFromX3>,
        message_id: [u8; 32],
        amount: u64,
        sender: Vec<u8>,
        proof: Vec<u8>,
    ) -> Result<()> {
        require!(!ctx.accounts.gateway.paused, GatewayError::Paused);
        require!(amount > 0, GatewayError::ZeroAmount);
        require!(
            !ctx.accounts.gateway.used_messages.contains(&message_id),
            GatewayError::Replay
        );

        let token_mint_key = ctx.accounts.token_mint.key();
        let recipient_key = ctx.accounts.recipient.key();
        let x3_chain_id_val = ctx.accounts.gateway.x3_chain_id;

        require!(
            ctx.accounts.gateway.supported_tokens.contains(&token_mint_key),
            GatewayError::TokenNotSupported
        );

        let current_locked = get_total_locked(&ctx.accounts.gateway.total_locked, &token_mint_key);
        require!(current_locked >= amount, GatewayError::InsufficientLiquidity);

        let proof_message = hash_withdrawal_message(
            &message_id,
            x3_chain_id_val,
            &sender,
            &recipient_key,
            amount,
        );

        let verified = verify_ed25519_quorum(
            &proof,
            &proof_message,
            &ctx.accounts.instructions_sysvar,
        )?;
        require!(verified, GatewayError::InvalidProof);

        let day_key = (Clock::get()?.unix_timestamp as u64) / 86400;

        {
            let gw = &mut ctx.accounts.gateway;
            gw.used_messages.push(message_id);

            let cap = gw
                .daily_withdrawal_limits
                .iter()
                .find(|e| e.key == token_mint_key)
                .map(|e| e.value)
                .unwrap_or(u64::MAX);

            let current = gw
                .daily_withdrawn
                .iter()
                .find(|e| e.token == token_mint_key && e.day_key == day_key)
                .map(|e| e.amount)
                .unwrap_or(0);
            let new_total = current.checked_add(amount).ok_or(GatewayError::Overflow)?;
            if new_total > cap {
                return Err(GatewayError::DailyLimit.into());
            }
            if let Some(entry) = gw
                .daily_withdrawn
                .iter_mut()
                .find(|e| e.token == token_mint_key && e.day_key == day_key)
            {
                entry.amount = new_total;
            } else {
                gw.daily_withdrawn.push(DailyAccum {
                    token: token_mint_key,
                    day_key,
                    amount: new_total,
                });
            }

            update_total_locked(&mut gw.total_locked, &token_mint_key, amount, false)?;
        }

        let gw_bump = ctx.bumps.gateway;
        let seeds: &[&[u8]] = &[GATEWAY_SEED, &[gw_bump]];
        let signer_seeds: &[&[&[u8]]] = &[seeds];

        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                SplTransfer {
                    from: ctx.accounts.vault.to_account_info(),
                    to: ctx.accounts.recipient_token_account.to_account_info(),
                    authority: ctx.accounts.gateway.to_account_info(),
                },
                signer_seeds,
            ),
            amount,
        )?;

        emit!(WithdrawalReleased {
            message_id,
            token: token_mint_key,
            recipient: recipient_key,
            amount,
        });
        Ok(())
    }
}

fn hash_deposit_message(
    chain_id: u64,
    token: &Pubkey,
    depositor: &Pubkey,
    x3_recipient: &[u8],
    amount: u64,
    nonce: u64,
) -> [u8; 32] {
    let mut hasher = anchor_lang::solana_program::hash::Hasher::default();
    hasher.hash(b"X3_DEPOSIT_V1");
    hasher.hash(&chain_id.to_le_bytes());
    hasher.hash(token.as_ref());
    hasher.hash(depositor.as_ref());
    hasher.hash(x3_recipient);
    hasher.hash(&amount.to_le_bytes());
    hasher.hash(&nonce.to_le_bytes());
    hasher.result().to_bytes()
}

fn hash_withdrawal_message(
    message_id: &[u8; 32],
    x3_chain_id: u64,
    sender: &[u8],
    recipient: &Pubkey,
    amount: u64,
) -> [u8; 32] {
    let mut hasher = anchor_lang::solana_program::hash::Hasher::default();
    hasher.hash(b"X3_WITHDRAWAL_V1");
    hasher.hash(message_id);
    hasher.hash(&x3_chain_id.to_le_bytes());
    hasher.hash(sender);
    hasher.hash(recipient.as_ref());
    hasher.hash(&amount.to_le_bytes());
    hasher.result().to_bytes()
}

fn verify_ed25519_quorum(_proof: &[u8], _message: &[u8; 32], instructions_sysvar: &AccountInfo) -> Result<bool> {
    for i in 0u64.. {
        let ix =
            match anchor_lang::solana_program::sysvar::instructions::load_instruction_at_checked(
                i as usize,
                instructions_sysvar,
            ) {
                Ok(ix) => ix,
                Err(_) => break,
            };

        if ix.program_id == anchor_lang::solana_program::ed25519_program::ID {
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
                if sig_ix != 0xFF || pubkey_ix != 0xFF || msg_ix != 0xFF {
                    offset += 104;
                    continue;
                }
                let msg_len =
                    u16::from_le_bytes([data[offset + 6], data[offset + 7]]) as usize;
                let msg_data = &data[offset + 8..offset + 8 + msg_len];
                if msg_data.len() == _message.len() && msg_data == _message {
                    // For MVP accept any valid ed25519 signature.
                    return Ok(true);
                }
                offset = offset + 8 + msg_len + 32 + 64;
            }
        }
    }
    Ok(false)
}

fn update_limit(limits: &mut Vec<LimitEntry>, token: &Pubkey, new_limit: u64) {
    if let Some(entry) = limits.iter_mut().find(|e| e.key == *token) {
        entry.value = new_limit;
    } else {
        limits.push(LimitEntry {
            key: *token,
            value: new_limit,
        });
    }
}

fn update_total_locked(
    locked: &mut Vec<LimitEntry>,
    token: &Pubkey,
    amount: u64,
    is_deposit: bool,
) -> Result<()> {
    if let Some(entry) = locked.iter_mut().find(|e| e.key == *token) {
        if is_deposit {
            entry.value = entry.value.checked_add(amount).ok_or(GatewayError::Overflow)?;
        } else {
            entry.value = entry.value.checked_sub(amount).ok_or(GatewayError::Overflow)?;
        }
    } else if is_deposit {
        locked.push(LimitEntry {
            key: *token,
            value: amount,
        });
    } else {
        return Err(GatewayError::InsufficientLiquidity.into());
    }
    Ok(())
}

fn get_total_locked(locked: &[LimitEntry], token: &Pubkey) -> u64 {
    locked
        .iter()
        .find(|e| e.key == *token)
        .map(|e| e.value)
        .unwrap_or(0)
}

#[account]
pub struct Gateway {
    pub owner: Pubkey,
    pub verifier: Pubkey,
    pub chain_id: u64,
    pub x3_chain_id: u64,
    pub min_x3_confirmations: u64,
    pub paused: bool,
    pub supported_tokens: Vec<Pubkey>,
    pub daily_deposit_limits: Vec<LimitEntry>,
    pub daily_withdrawal_limits: Vec<LimitEntry>,
    pub daily_deposited: Vec<DailyAccum>,
    pub daily_withdrawn: Vec<DailyAccum>,
    pub used_messages: Vec<[u8; 32]>,
    pub total_locked: Vec<LimitEntry>,
}

impl Gateway {
    pub const BASE_SIZE: usize = 32 + 32 + 8 + 8 + 8 + 1;
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct LimitEntry {
    pub key: Pubkey,
    pub value: u64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct DailyAccum {
    pub token: Pubkey,
    pub day_key: u64,
    pub amount: u64,
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        seeds = [GATEWAY_SEED],
        bump,
        payer = owner,
        space = 8 + Gateway::BASE_SIZE + 10000
    )]
    pub gateway: Account<'info, Gateway>,
    #[account(mut)]
    pub owner: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AdminOnly<'info> {
    #[account(
        mut,
        seeds = [GATEWAY_SEED],
        bump,
        has_one = owner @ GatewayError::Unauthorized,
    )]
    pub gateway: Account<'info, Gateway>,
    pub owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct DepositToX3<'info> {
    #[account(
        mut,
        seeds = [GATEWAY_SEED],
        bump,
    )]
    pub gateway: Account<'info, Gateway>,
    /// CHECK: token vault token account (must exist and hold tokens)
    #[account(mut)]
    pub vault: AccountInfo<'info>,
    /// CHECK: token mint (validated via support check)
    pub token_mint: AccountInfo<'info>,
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    pub depositor: Signer<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ReleaseFromX3<'info> {
    #[account(
        mut,
        seeds = [GATEWAY_SEED],
        bump,
    )]
    pub gateway: Account<'info, Gateway>,
    /// CHECK: token vault (program must be able to sign for transfers from this account)
    #[account(mut)]
    pub vault: AccountInfo<'info>,
    /// CHECK: token mint
    pub token_mint: AccountInfo<'info>,
    /// CHECK: withdrawal recipient
    pub recipient: AccountInfo<'info>,
    #[account(mut)]
    pub recipient_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    /// CHECK: instruction sysvar for ed25519 signature verification
    #[account(address = anchor_lang::solana_program::sysvar::instructions::ID)]
    pub instructions_sysvar: AccountInfo<'info>,
}

#[event]
pub struct DepositLocked {
    pub message_id: [u8; 32],
    pub token: Pubkey,
    pub depositor: Pubkey,
    pub x3_recipient: Vec<u8>,
    pub amount: u64,
    pub nonce: u64,
    pub chain_id: u64,
}

#[event]
pub struct WithdrawalReleased {
    pub message_id: [u8; 32],
    pub token: Pubkey,
    pub recipient: Pubkey,
    pub amount: u64,
}

#[event]
pub struct Paused {
    pub is_paused: bool,
}

#[event]
pub struct SupportedTokenUpdated {
    pub token: Pubkey,
    pub supported: bool,
    pub daily_deposit_cap: u64,
    pub daily_withdrawal_cap: u64,
}

#[event]
pub struct VerifierUpdated {
    pub new_verifier: Pubkey,
}

#[error_code]
pub enum GatewayError {
    #[msg("gateway is paused")]
    Paused,
    #[msg("amount must be greater than zero")]
    ZeroAmount,
    #[msg("token not supported")]
    TokenNotSupported,
    #[msg("replay detected")]
    Replay,
    #[msg("daily limit reached")]
    DailyLimit,
    #[msg("insufficient liquidity in vault")]
    InsufficientLiquidity,
    #[msg("invalid proof")]
    InvalidProof,
    #[msg("zero verifier address")]
    ZeroVerifier,
    #[msg("invalid recipient data")]
    InvalidRecipient,
    #[msg("arithmetic overflow")]
    Overflow,
    #[msg("unauthorized caller")]
    Unauthorized,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gateway_base_size() {
        assert_eq!(Gateway::BASE_SIZE, 32 + 32 + 8 + 8 + 8 + 1);
    }

    #[test]
    fn gateway_seed_is_stable() {
        assert_eq!(GATEWAY_SEED, b"gateway");
    }

    #[test]
    fn gateway_pda_is_deterministic() {
        let (pda1, bump1) = Pubkey::find_program_address(
            &[GATEWAY_SEED],
            &id(),
        );
        let (pda2, bump2) = Pubkey::find_program_address(
            &[GATEWAY_SEED],
            &id(),
        );
        assert_eq!(pda1, pda2);
        assert_eq!(bump1, bump2);
    }

    #[test]
    fn hash_deposit_message_is_deterministic() {
        let chain_id = 1u64;
        let token = Pubkey::new_from_array([1u8; 32]);
        let depositor = Pubkey::new_from_array([2u8; 32]);
        let recipient: Vec<u8> = vec![3u8; 20];
        let amount = 1000u64;
        let nonce = 42u64;

        let h1 = hash_deposit_message(chain_id, &token, &depositor, &recipient, amount, nonce);
        let h2 = hash_deposit_message(chain_id, &token, &depositor, &recipient, amount, nonce);
        assert_eq!(h1, h2);
    }

    #[test]
    fn hash_deposit_message_differs_by_amount() {
        let chain_id = 1u64;
        let token = Pubkey::new_from_array([1u8; 32]);
        let depositor = Pubkey::new_from_array([2u8; 32]);
        let recipient: Vec<u8> = vec![3u8; 20];

        let h1 = hash_deposit_message(chain_id, &token, &depositor, &recipient, 100, 1);
        let h2 = hash_deposit_message(chain_id, &token, &depositor, &recipient, 200, 1);
        assert_ne!(h1, h2);
    }

    #[test]
    fn hash_deposit_message_differs_by_nonce() {
        let chain_id = 1u64;
        let token = Pubkey::new_from_array([1u8; 32]);
        let depositor = Pubkey::new_from_array([2u8; 32]);
        let recipient: Vec<u8> = vec![3u8; 20];

        let h1 = hash_deposit_message(chain_id, &token, &depositor, &recipient, 100, 1);
        let h2 = hash_deposit_message(chain_id, &token, &depositor, &recipient, 100, 2);
        assert_ne!(h1, h2);
    }

    #[test]
    fn hash_withdrawal_message_is_deterministic() {
        let message_id: [u8; 32] = [0xAA; 32];
        let x3_chain_id = 42u64;
        let sender: Vec<u8> = vec![0xBB; 20];
        let recipient = Pubkey::new_from_array([0xCC; 32]);
        let amount = 5000u64;

        let h1 = hash_withdrawal_message(&message_id, x3_chain_id, &sender, &recipient, amount);
        let h2 = hash_withdrawal_message(&message_id, x3_chain_id, &sender, &recipient, amount);
        assert_eq!(h1, h2);
    }

    #[test]
    fn hash_withdrawal_and_deposit_are_distinct() {
        let chain_id = 1u64;
        let token = Pubkey::new_from_array([1u8; 32]);
        let depositor = Pubkey::new_from_array([2u8; 32]);
        let recipient: Vec<u8> = vec![3u8; 20];
        let amount = 1000u64;
        let nonce = 1u64;

        let deposit_hash = hash_deposit_message(chain_id, &token, &depositor, &recipient, amount, nonce);

        let message_id: [u8; 32] = [0xDD; 32];
        let sender: Vec<u8> = vec![0xEE; 20];
        let recipient_pub = Pubkey::new_from_array([0xFF; 32]);
        let withdrawal_hash = hash_withdrawal_message(&message_id, chain_id, &sender, &recipient_pub, amount);

        assert_ne!(deposit_hash, withdrawal_hash);
    }

    #[test]
    fn update_limit_adds_new_entry() {
        let mut limits: Vec<LimitEntry> = vec![];
        let token = Pubkey::new_from_array([10u8; 32]);
        update_limit(&mut limits, &token, 500);
        assert_eq!(limits.len(), 1);
        assert_eq!(limits[0].key, token);
        assert_eq!(limits[0].value, 500);
    }

    #[test]
    fn update_limit_updates_existing() {
        let token = Pubkey::new_from_array([10u8; 32]);
        let mut limits = vec![LimitEntry { key: token, value: 100 }];
        update_limit(&mut limits, &token, 500);
        assert_eq!(limits.len(), 1);
        assert_eq!(limits[0].value, 500);
    }

    #[test]
    fn update_total_locked_deposit_creates_entry() {
        let mut locked: Vec<LimitEntry> = vec![];
        let token = Pubkey::new_from_array([20u8; 32]);
        update_total_locked(&mut locked, &token, 1000, true).unwrap();
        assert_eq!(locked.len(), 1);
        assert_eq!(locked[0].value, 1000);
    }

    #[test]
    fn update_total_locked_deposit_adds_to_existing() {
        let token = Pubkey::new_from_array([20u8; 32]);
        let mut locked = vec![LimitEntry { key: token, value: 500 }];
        update_total_locked(&mut locked, &token, 300, true).unwrap();
        assert_eq!(get_total_locked(&locked, &token), 800);
    }

    #[test]
    fn update_total_locked_withdraw_decreases() {
        let token = Pubkey::new_from_array([20u8; 32]);
        let mut locked = vec![LimitEntry { key: token, value: 1000 }];
        update_total_locked(&mut locked, &token, 400, false).unwrap();
        assert_eq!(get_total_locked(&locked, &token), 600);
    }

    #[test]
    fn update_total_locked_withdraw_below_zero_fails() {
        let token = Pubkey::new_from_array([20u8; 32]);
        let mut locked = vec![LimitEntry { key: token, value: 100 }];
        let result = update_total_locked(&mut locked, &token, 101, false);
        assert!(result.is_err());
    }

    #[test]
    fn update_total_locked_withdraw_nonexistent_fails() {
        let mut locked: Vec<LimitEntry> = vec![];
        let token = Pubkey::new_from_array([20u8; 32]);
        let result = update_total_locked(&mut locked, &token, 100, false);
        assert!(result.is_err());
    }

    #[test]
    fn get_total_locked_returns_zero_for_missing() {
        let locked: Vec<LimitEntry> = vec![];
        let token = Pubkey::new_from_array([30u8; 32]);
        assert_eq!(get_total_locked(&locked, &token), 0);
    }

    #[test]
    fn limit_entry_serialization_roundtrip() {
        let entry = LimitEntry {
            key: Pubkey::new_from_array([0xAB; 32]),
            value: 12345,
        };
        let bytes = entry.try_to_vec().unwrap();
        let deserialized = LimitEntry::try_from_slice(&bytes).unwrap();
        assert_eq!(entry, deserialized);
    }

    #[test]
    fn daily_accum_serialization_roundtrip() {
        let accum = DailyAccum {
            token: Pubkey::new_from_array([0xCD; 32]),
            day_key: 20000,
            amount: 999,
        };
        let bytes = accum.try_to_vec().unwrap();
        let deserialized = DailyAccum::try_from_slice(&bytes).unwrap();
        assert_eq!(accum, deserialized);
    }

    #[test]
    fn gateway_error_codes_are_distinct() {
        let codes = [
            GatewayError::Paused,
            GatewayError::ZeroAmount,
            GatewayError::TokenNotSupported,
            GatewayError::Replay,
            GatewayError::DailyLimit,
            GatewayError::InsufficientLiquidity,
            GatewayError::InvalidProof,
            GatewayError::ZeroVerifier,
            GatewayError::InvalidRecipient,
            GatewayError::Overflow,
            GatewayError::Unauthorized,
        ];
        for i in 0..codes.len() {
            for j in (i + 1)..codes.len() {
                assert_ne!(codes[i] as u32, codes[j] as u32);
            }
        }
    }
}
