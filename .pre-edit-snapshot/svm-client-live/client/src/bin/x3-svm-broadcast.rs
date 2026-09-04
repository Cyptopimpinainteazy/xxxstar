//! # x3-svm-broadcast — explicit live Solana devnet HTLC broadcaster (default OFF)
//!
//! Thin, explicit CLI wrapper around the `x3_svm_client` library. It NEVER
//! auto-broadcasts: it only runs when a human invokes it with every piece of
//! live configuration supplied (RPC url, program id, fee-payer keypair path,
//! and an explicit `lock`/`claim`/`refund` action). No secret is accepted
//! inline — the keypair is read from a file path (typically resolved by a
//! secret-store reference by the caller).
//!
//! # Usage (self-initiated `lock`, payer == initializer)
//! ```bash
//! cargo run --release --bin x3-svm-broadcast -- \
//!   --rpc https://api.devnet.solana.com \
//!   --program-id <DEPLOYED_PROGRAM_ID> \
//!   --payer-keypair /path/to/payer.json \
//!   lock \
//!   --swap-id <32-byte-hex> \
//!   --claimant <BASE58_PUBKEY> \
//!   --refund-authority <BASE58_PUBKEY> \
//!   --hashlock <32-byte-hex> \
//!   --amount <lamports> --timeout-slots <slots>
//! ```
//!
//! For `claim` / `refund`, see the per-action branches below.

use std::process::ExitCode;

use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signer;
use x3_svm_client::{load_payer, SvmLiveConfig};

fn parse_hex32(name: &str, hex: &str) -> Result<[u8; 32], String> {
    let hex = hex.strip_prefix("0x").unwrap_or(hex);
    let bytes = hex::decode(hex).map_err(|e| format!("{name}: bad hex ({e})"))?;
    if bytes.len() != 32 {
        return Err(format!("{name}: expected 32 bytes, got {}", bytes.len()));
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}

struct LockArgs {
    swap_id: [u8; 32],
    claimant: String,
    refund_authority: String,
    hashlock: [u8; 32],
    amount: u64,
    timeout_slots: u64,
}

fn parse_lock_args(a: &[String]) -> Result<LockArgs, String> {
    let amount = pick("--amount", a)?
        .parse::<u64>()
        .map_err(|e| format!("--amount: {e}"))?;
    let timeout_slots = pick("--timeout-slots", a)?
        .parse::<u64>()
        .map_err(|e| format!("--timeout-slots: {e}"))?;
    Ok(LockArgs {
        swap_id: parse_hex32("--swap-id", &pick("--swap-id", a)?)?,
        claimant: pick("--claimant", a)?,
        refund_authority: pick("--refund-authority", a)?,
        hashlock: parse_hex32("--hashlock", &pick("--hashlock", a)?)?,
        amount,
        timeout_slots,
    })
}

fn pick(key: &str, a: &[String]) -> Result<String, String> {
    a.iter()
        .position(|x| x == key)
        .and_then(|i| a.get(i + 1))
        .cloned()
        .ok_or_else(|| format!("missing {key}"))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let rpc = match pick("--rpc", &args) {
        Ok(v) => v,
        Err(_) => {
            eprintln!("usage: --rpc <url> --program-id <id> --payer-keypair <path> <lock|claim|refund> ...");
            return ExitCode::from(2);
        }
    };
    let program_id_s = match pick("--program-id", &args) {
        Ok(v) => v,
        Err(_) => {
            eprintln!("missing --program-id");
            return ExitCode::from(2);
        }
    };
    let keypair_path = match pick("--payer-keypair", &args) {
        Ok(v) => v,
        Err(_) => {
            eprintln!("missing --payer-keypair <path>");
            return ExitCode::from(2);
        }
    };
    let program_id = match program_id_s.parse::<Pubkey>() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("bad --program-id: {e}");
            return ExitCode::from(2);
        }
    };

    let cfg = SvmLiveConfig {
        rpc_url: rpc,
        program_id,
        keypair_path,
        commitment: "confirmed".into(),
    };
    let payer = match load_payer(&cfg) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(1);
        }
    };

    let action = match args
        .iter()
        .position(|x| x == "lock" || x == "claim" || x == "refund")
    {
        Some(i) => args[i].clone(),
        None => {
            eprintln!("missing action: lock|claim|refund");
            return ExitCode::from(2);
        }
    };

    match action.as_str() {
        "lock" => {
            let rest: Vec<String> = args
                .iter()
                .skip_while(|x| *x != "lock")
                .skip(1)
                .cloned()
                .collect();
            match lock(&cfg, &payer, &rest) {
                Ok(s) => {
                    println!(
                        "LOCK_SUBMITTED sig={} htlc={} payer={}",
                        s.signature, s.htlc_account, s.payer
                    );
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("lock failed: {e}");
                    ExitCode::from(1)
                }
            }
        }
        "claim" => {
            let rest: Vec<String> = args
                .iter()
                .skip_while(|x| *x != "claim")
                .skip(1)
                .cloned()
                .collect();
            match claim(&cfg, &payer, &rest) {
                Ok(s) => {
                    println!(
                        "CLAIM_SUBMITTED sig={} htlc={}",
                        s.signature, s.htlc_account
                    );
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("claim failed: {e}");
                    ExitCode::from(1)
                }
            }
        }
        "refund" => {
            let rest: Vec<String> = args
                .iter()
                .skip_while(|x| *x != "refund")
                .skip(1)
                .cloned()
                .collect();
            match refund(&cfg, &payer, &rest) {
                Ok(s) => {
                    println!(
                        "REFUND_SUBMITTED sig={} htlc={}",
                        s.signature, s.htlc_account
                    );
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("refund failed: {e}");
                    ExitCode::from(1)
                }
            }
        }
        _ => {
            eprintln!("unknown action {action}");
            ExitCode::from(2)
        }
    }
}

fn lock(
    cfg: &SvmLiveConfig,
    payer: &solana_sdk::signature::Keypair,
    a: &[String],
) -> Result<x3_svm_client::LiveSubmission, String> {
    let m = parse_lock_args(a)?;
    let claimant = m
        .claimant
        .parse::<Pubkey>()
        .map_err(|e| format!("--claimant: {e}"))?;
    let refund_authority = m
        .refund_authority
        .parse::<Pubkey>()
        .map_err(|e| format!("--refund-authority: {e}"))?;
    let payer_pk = payer.pubkey();
    x3_svm_client::broadcast_create_htlc(
        cfg,
        payer,
        None, // payer == initializer (self-lock)
        &payer_pk,
        &m.swap_id,
        &claimant,
        &refund_authority,
        &m.hashlock,
        m.amount,
        m.timeout_slots,
    )
}

fn claim(
    cfg: &SvmLiveConfig,
    payer: &solana_sdk::signature::Keypair,
    a: &[String],
) -> Result<x3_svm_client::LiveSubmission, String> {
    let swap_id = parse_hex32("--swap-id", &pick("--swap-id", a)?)?;
    let preimage = a
        .iter()
        .position(|x| x == "--preimage")
        .and_then(|i| a.get(i + 1))
        .ok_or_else(|| "missing --preimage".to_string())?
        .clone();
    let payer_pk = payer.pubkey();
    x3_svm_client::broadcast_claim_htlc(cfg, payer, &payer_pk, &swap_id, preimage.as_bytes())
}

fn refund(
    cfg: &SvmLiveConfig,
    payer: &solana_sdk::signature::Keypair,
    a: &[String],
) -> Result<x3_svm_client::LiveSubmission, String> {
    let swap_id = parse_hex32("--swap-id", &pick("--swap-id", a)?)?;
    let payer_pk = payer.pubkey();
    x3_svm_client::broadcast_refund_htlc(cfg, payer, &payer_pk, &swap_id)
}
