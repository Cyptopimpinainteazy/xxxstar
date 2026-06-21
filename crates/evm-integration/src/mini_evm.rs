//! Minimal no-std EVM executor using SputnikVM (`evm` crate)
//!
//! Provides real EVM bytecode execution without requiring `std`.
//! Uses `evm::executor::stack::StackExecutor` with an in-memory backend
//! backed by `alloc::collections::BTreeMap` (available in no-std + alloc).
//!
//! # Precompiles
//! Standard Ethereum precompiles at addresses 0x01–0x09:
//! - 0x01: ecrecover (secp256k1 signature recovery)
//! - 0x02: SHA-256
//! - 0x03: RIPEMD-160 (real implementation using `ripemd` crate)
//! - 0x04: identity (data copy)
//! - 0x05: modexp (modular exponentiation using binary exponentiation)
//! - 0x06: bn128Add (BN254 G1 point addition via ark-bls12-381)
//! - 0x07: bn128Mul (BN254 G1 scalar multiplication via ark-bls12-381)
//! - 0x08: bn128Pairing (BN254 pairing check via ark-bls12-381)
//! - 0x09: blake2f (Blake2b compression function F)

use crate::{EvmError, EvmExecutionResult, EvmResult};

use evm::{
    backend::{MemoryAccount, MemoryBackend, MemoryVicinity},
    executor::stack::{
        MemoryStackState, PrecompileFailure, PrecompileFn, PrecompileOutput, StackExecutor,
        StackSubstateMetadata,
    },
    Config, Context, ExitError, ExitReason, ExitSucceed,
};
use primitive_types::{H160 as EvmH160, U256 as EvmU256};
use sp_core::{H160 as SpH160, U256 as SpU256};
use sp_std::collections::btree_map::BTreeMap;

// Arkworks — BN254 (alt_bn128) curve operations for EVM precompiles
use ark_ec::{pairing::Pairing, AffineRepr, CurveGroup};
use ark_ff::{BigInteger, Field, PrimeField, Zero};

fn to_evm_u256(value: SpU256) -> EvmU256 {
    let buf = value.to_big_endian();
    EvmU256::from_big_endian(&buf)
}

fn to_evm_h160(value: SpH160) -> EvmH160 {
    EvmH160::from_slice(value.as_bytes())
}
use sp_std::prelude::Vec;
use sp_std::vec;

/// Default chain ID for X3 Chain (used by both mini_evm and lib.rs default config)
pub const X3_DEFAULT_CHAIN_ID: u64 = 42;

/// Execute EVM bytecode using the SputnikVM interpreter (no-std compatible).
///
/// `payload` is the call-data / init-code.  The adapter seeds a deterministic
/// contract address with `payload` as its code and issues a `CALL` into it.
///
/// `caller` and `value` are forwarded to the executor; `evm_config` supplies
/// chain_id, gas_price, block context, etc.
pub fn execute_evm(
    payload: &[u8],
    caller: SpH160,
    value: SpU256,
    evm_config: &crate::EvmConfig,
) -> EvmResult<EvmExecutionResult> {
    if payload.is_empty() {
        return Err(EvmError::InvalidPayload);
    }

    let gas_limit = evm_config.gas_limit;
    let config = Config::shanghai();

    let gas_price_evm: EvmU256 = EvmU256::from_big_endian(&evm_config.gas_price.to_big_endian());
    let origin = to_evm_h160(caller);
    let block_number_evm: EvmU256 =
        EvmU256::from_big_endian(&SpU256::from(evm_config.block_number).to_big_endian());
    let block_coinbase = to_evm_h160(evm_config.coinbase);
    let block_timestamp_evm: EvmU256 =
        EvmU256::from_big_endian(&SpU256::from(evm_config.block_timestamp).to_big_endian());
    let block_gas_limit_evm: EvmU256 =
        EvmU256::from_big_endian(&SpU256::from(gas_limit).to_big_endian());
    let chain_id_evm: EvmU256 =
        EvmU256::from_big_endian(&SpU256::from(evm_config.chain_id).to_big_endian());
    let block_base_fee_per_gas_evm: EvmU256 =
        EvmU256::from_big_endian(&evm_config.base_fee.to_big_endian());

    let vicinity = MemoryVicinity {
        gas_price: gas_price_evm,
        origin,
        block_hashes: vec![],
        block_number: block_number_evm,
        block_coinbase,
        block_timestamp: block_timestamp_evm,
        block_difficulty: EvmU256::zero(),
        block_gas_limit: block_gas_limit_evm,
        chain_id: chain_id_evm,
        block_base_fee_per_gas: block_base_fee_per_gas_evm,
        block_randomness: None,
    };

    // Derive a deterministic contract address from the payload hash
    let target_addr = derive_target(payload);

    let mut state_map: BTreeMap<EvmH160, MemoryAccount> = BTreeMap::new();
    // Caller account seeded with enough balance for gas + value transfer
    state_map.insert(
        to_evm_h160(caller),
        MemoryAccount {
            nonce: EvmU256::zero(),
            balance: EvmU256::from(u128::MAX),
            storage: BTreeMap::new(),
            code: vec![],
        },
    );
    // Contract: code = payload, already deployed
    state_map.insert(
        target_addr,
        MemoryAccount {
            nonce: EvmU256::one(),
            balance: EvmU256::zero(),
            storage: BTreeMap::new(),
            code: payload.to_vec(),
        },
    );

    let mut backend = MemoryBackend::new(&vicinity, state_map);
    let metadata = StackSubstateMetadata::new(gas_limit, &config);
    let stack_state = MemoryStackState::new(metadata, &mut backend);
    let precompiles = standard_precompiles();
    let mut executor = StackExecutor::new_with_precompiles(stack_state, &config, &precompiles);

    let (exit_reason, return_data) = executor.transact_call(
        to_evm_h160(caller), // caller
        target_addr,         // target
        to_evm_u256(value),  // value
        payload.to_vec(),    // call data
        gas_limit,
        vec![], // access_list
    );

    let gas_used = executor.used_gas();

    let success = matches!(exit_reason, ExitReason::Succeed(_));

    if !success {
        let err = map_exit_reason(&exit_reason, gas_used);
        return Err(err);
    }

    // Compute a deterministic state root from the execution inputs and result.
    // This commits the caller address, target, gas consumption, and output
    // into a single hash so validators can agree on execution outcome.
    let state_root = {
        use sp_io::hashing::blake2_256;
        let mut preimage = Vec::new();
        preimage.extend_from_slice(caller.as_bytes());
        preimage.extend_from_slice(target_addr.as_bytes());
        preimage.extend_from_slice(&gas_used.to_le_bytes());
        preimage.extend_from_slice(&return_data);
        blake2_256(&preimage)
    };

    Ok(EvmExecutionResult {
        success: true,
        output: return_data,
        gas_used,
        logs: Vec::new(), // canonical ledger owns the event log
        state_changes: Vec::new(),
        state_root,
    })
}

/// Estimate gas for EVM payload using EIP-2028 calldata pricing
/// plus base transaction cost (21,000 for calls, 53,000 for creates).
/// This is a lower-bound estimate without execution.
pub fn estimate_gas_evm(payload: &[u8]) -> EvmResult<u64> {
    if payload.is_empty() {
        return Err(EvmError::InvalidPayload);
    }
    let calldata_gas: u64 = payload
        .iter()
        .map(|&b| if b == 0 { 4u64 } else { 16u64 })
        .sum();
    // Base cost: 21,000 for CALL. Add 10% buffer for execution overhead.
    let base = 21_000u64.saturating_add(calldata_gas);
    Ok(base.saturating_mul(11) / 10)
}

/// Validate EVM bytecode per EIP-170 (max 24 KiB deployed code) and
/// EIP-3541 (reject 0xEF prefix reserved for EOF).
pub fn validate_evm(payload: &[u8]) -> EvmResult<()> {
    if payload.is_empty() {
        return Err(EvmError::InvalidPayload);
    }
    // EIP-170: max contract code size = 24_576 bytes
    if payload.len() > 24_576 {
        return Err(EvmError::ExecutionFailed(0xEF));
    }
    // EIP-3541: reject bytecode starting with 0xEF (reserved for EOF)
    if payload.first() == Some(&0xEF) {
        return Err(EvmError::InvalidOpcode(0xEF));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Derive a stable H160 target address from payload bytes via keccak-256
/// (consistent with Ethereum's CREATE address derivation).
fn derive_target(payload: &[u8]) -> EvmH160 {
    let hash = sp_io::hashing::keccak_256(payload);
    EvmH160::from_slice(&hash[12..32])
}

fn map_exit_reason(reason: &ExitReason, gas_used: u64) -> EvmError {
    match reason {
        ExitReason::Revert(_) => EvmError::ExecutionReverted,
        ExitReason::Error(evm::ExitError::OutOfGas) => EvmError::OutOfGas,
        ExitReason::Error(evm::ExitError::StackOverflow) => EvmError::StackOverflow,
        ExitReason::Error(evm::ExitError::StackUnderflow) => EvmError::StackUnderflow,
        ExitReason::Error(evm::ExitError::CreateCollision) => EvmError::CreateCollision,
        ExitReason::Error(evm::ExitError::InvalidCode(op)) => EvmError::InvalidOpcode(op.as_u8()),
        _ => EvmError::ExecutionFailed(gas_used as u32),
    }
}

// ---------------------------------------------------------------------------
// Standard Ethereum Precompiles (0x01 – 0x09)
// ---------------------------------------------------------------------------

/// Build a BTreeMap of standard Ethereum precompiled contracts.
fn standard_precompiles() -> BTreeMap<EvmH160, PrecompileFn> {
    let mut map = BTreeMap::new();
    map.insert(addr(1), precompile_ecrecover as PrecompileFn);
    map.insert(addr(2), precompile_sha256 as PrecompileFn);
    map.insert(addr(3), precompile_ripemd160 as PrecompileFn);
    map.insert(addr(4), precompile_identity as PrecompileFn);
    map.insert(addr(5), precompile_modexp as PrecompileFn);
    map.insert(addr(6), precompile_bn128_add as PrecompileFn);
    map.insert(addr(7), precompile_bn128_mul as PrecompileFn);
    map.insert(addr(8), precompile_bn128_pairing as PrecompileFn);
    map.insert(addr(9), precompile_blake2f as PrecompileFn);
    map
}

/// Convert a small integer into an H160 precompile address.
fn addr(n: u8) -> EvmH160 {
    let mut bytes = [0u8; 20];
    bytes[19] = n;
    EvmH160::from_slice(&bytes)
}

/// Gas cost helper: base + per-word cost.
fn word_gas(input_len: usize, base: u64, per_word: u64) -> u64 {
    let words = (input_len as u64).div_ceil(32);
    base + words * per_word
}

// ---------------------------------------------------------------------------
// 0x01 — ecrecover
// ---------------------------------------------------------------------------
/// Recovers the signer address from a secp256k1 signature.
/// Input: 128 bytes — hash(32) | v(32) | r(32) | s(32)
/// Output: 32 bytes — zero-padded 20-byte recovered address
fn precompile_ecrecover(
    input: &[u8],
    target_gas: Option<u64>,
    _context: &Context,
    _is_static: bool,
) -> Result<(PrecompileOutput, u64), PrecompileFailure> {
    const GAS_COST: u64 = 3000;
    if let Some(gas) = target_gas {
        if gas < GAS_COST {
            return Err(PrecompileFailure::Error {
                exit_status: ExitError::OutOfGas,
            });
        }
    }

    // Pad input to 128 bytes
    let mut padded = [0u8; 128];
    let copy_len = input.len().min(128);
    padded[..copy_len].copy_from_slice(&input[..copy_len]);

    let hash: &[u8; 32] = padded[0..32].try_into().expect("32-byte slice");
    // v is the last byte of the 32-byte v field (byte 63)
    let v_raw = padded[63];
    let r = &padded[64..96];
    let s = &padded[96..128];

    // v must be 27 or 28
    if v_raw != 27 && v_raw != 28 {
        // Return empty (no recovery) per EIP — not an error
        return Ok((
            PrecompileOutput {
                exit_status: ExitSucceed::Returned,
                output: vec![0u8; 32],
            },
            GAS_COST,
        ));
    }

    // Build the 65-byte signature: r(32) || s(32) || recovery_id(1)
    let mut sig = [0u8; 65];
    sig[0..32].copy_from_slice(r);
    sig[32..64].copy_from_slice(s);
    sig[64] = v_raw - 27; // recovery id: 0 or 1

    match sp_io::crypto::secp256k1_ecdsa_recover(&sig, hash) {
        Ok(pubkey) => {
            // pubkey is 64 bytes (uncompressed without 0x04 prefix)
            let address_hash = sp_io::hashing::keccak_256(&pubkey);
            let mut output = [0u8; 32];
            // Address is last 20 bytes of keccak hash, left-padded to 32
            output[12..32].copy_from_slice(&address_hash[12..32]);
            Ok((
                PrecompileOutput {
                    exit_status: ExitSucceed::Returned,
                    output: output.to_vec(),
                },
                GAS_COST,
            ))
        }
        Err(_) => {
            // Recovery failed — return empty per spec
            Ok((
                PrecompileOutput {
                    exit_status: ExitSucceed::Returned,
                    output: vec![0u8; 32],
                },
                GAS_COST,
            ))
        }
    }
}

// ---------------------------------------------------------------------------
// 0x02 — SHA-256
// ---------------------------------------------------------------------------
fn precompile_sha256(
    input: &[u8],
    target_gas: Option<u64>,
    _context: &Context,
    _is_static: bool,
) -> Result<(PrecompileOutput, u64), PrecompileFailure> {
    let gas = word_gas(input.len(), 60, 12);
    if let Some(limit) = target_gas {
        if limit < gas {
            return Err(PrecompileFailure::Error {
                exit_status: ExitError::OutOfGas,
            });
        }
    }
    let hash = sp_io::hashing::sha2_256(input);
    Ok((
        PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            output: hash.to_vec(),
        },
        gas,
    ))
}

// ---------------------------------------------------------------------------
// 0x03 — RIPEMD-160
// ---------------------------------------------------------------------------
fn precompile_ripemd160(
    input: &[u8],
    target_gas: Option<u64>,
    _context: &Context,
    _is_static: bool,
) -> Result<(PrecompileOutput, u64), PrecompileFailure> {
    let gas = word_gas(input.len(), 600, 120);
    if let Some(limit) = target_gas {
        if limit < gas {
            return Err(PrecompileFailure::Error {
                exit_status: ExitError::OutOfGas,
            });
        }
    }
    use ripemd::{Digest, Ripemd160};
    let mut hasher = Ripemd160::new();
    hasher.update(input);
    let hash = hasher.finalize();
    // Per EIP: left-pad 20-byte hash to 32 bytes
    let mut output = [0u8; 32];
    output[12..].copy_from_slice(&hash);
    Ok((
        PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            output: output.to_vec(),
        },
        gas,
    ))
}

// ---------------------------------------------------------------------------
// 0x04 — identity
// ---------------------------------------------------------------------------
fn precompile_identity(
    input: &[u8],
    target_gas: Option<u64>,
    _context: &Context,
    _is_static: bool,
) -> Result<(PrecompileOutput, u64), PrecompileFailure> {
    let gas = word_gas(input.len(), 15, 3);
    if let Some(limit) = target_gas {
        if limit < gas {
            return Err(PrecompileFailure::Error {
                exit_status: ExitError::OutOfGas,
            });
        }
    }
    Ok((
        PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            output: input.to_vec(),
        },
        gas,
    ))
}

// ---------------------------------------------------------------------------
// 0x05 — modexp (modular exponentiation)
// ---------------------------------------------------------------------------
/// Big-integer modular exponentiation (EIP-198).
///
/// Input: 96 bytes of header (base_len, exp_len, mod_len as big-endian u32s)
/// followed by base, exponent, and modulus byte strings.
///
/// Uses binary exponentiation with modular reduction via `sp_core::U256`
/// for operands up to 32 bytes. For larger operands, falls back to a
/// byte-level binary exponentiation with long-division-style reduction.
fn precompile_modexp(
    input: &[u8],
    target_gas: Option<u64>,
    _context: &Context,
    _is_static: bool,
) -> Result<(PrecompileOutput, u64), PrecompileFailure> {
    if input.len() < 96 {
        return Err(PrecompileFailure::Error {
            exit_status: ExitError::Other(sp_std::borrow::Cow::Borrowed("modexp: input too short")),
        });
    }

    let base_len = u32::from_be_bytes([input[0], input[1], input[2], input[3]]) as usize;
    let exp_len = u32::from_be_bytes([input[4], input[5], input[6], input[7]]) as usize;
    let mod_len = u32::from_be_bytes([input[8], input[9], input[10], input[11]]) as usize;

    let expected = 96 + base_len + exp_len + mod_len;
    if input.len() < expected {
        return Err(PrecompileFailure::Error {
            exit_status: ExitError::Other(sp_std::borrow::Cow::Borrowed(
                "modexp: input too short for declared lengths",
            )),
        });
    }

    // Gas calculation per EIP-198
    let gas = if mod_len == 0 {
        0
    } else {
        let max_len = core::cmp::max(base_len, mod_len);
        let words = max_len.div_ceil(8);
        let mult_complexity = match words {
            0..=1 => 1,
            2..=3 => 2,
            4..=7 => 4,
            8..=15 => 8,
            16..=31 => 16,
            32..=63 => 32,
            64..=127 => 64,
            128..=255 => 128,
            256..=511 => 256,
            _ => 512,
        };
        let exp_header = if exp_len <= 32 {
            let mut exp_buf = [0u8; 32];
            let start = 96 + base_len;
            let copy_len = core::cmp::min(exp_len, 32);
            exp_buf[32 - copy_len..].copy_from_slice(&input[start..start + copy_len]);
            u256_from_be_slice(&exp_buf)
        } else {
            let start = 96 + base_len + exp_len - 32;
            u256_from_be_slice(&input[start..start + 32])
        };
        let exp_bits = if exp_len <= 32 {
            bit_length(&exp_header, exp_len)
        } else {
            let leading = u256_from_be_slice(&input[96 + base_len..96 + base_len + 32]);
            let leading_bits = bit_length(&leading, 32);
            if leading_bits == 0 {
                // If the high 32 bytes are zero, count bits from the rest
                let start = 96 + base_len + 32;
                let remaining = exp_len - 32;
                let mut buf = [0u8; 32];
                let copy_len = core::cmp::min(remaining, 32);
                buf[32 - copy_len..].copy_from_slice(&input[start..start + copy_len]);
                let rem_val = u256_from_be_slice(&buf);
                bit_length(&rem_val, remaining)
            } else {
                leading_bits + 8 * (exp_len - 32)
            }
        };
        let adjusted = core::cmp::max(mult_complexity, 1);
        adjusted * exp_bits as u64 / 8
    };

    if let Some(limit) = target_gas {
        if limit < gas {
            return Err(PrecompileFailure::Error {
                exit_status: ExitError::OutOfGas,
            });
        }
    }

    if mod_len == 0 {
        return Ok((
            PrecompileOutput {
                exit_status: ExitSucceed::Returned,
                output: vec![],
            },
            gas,
        ));
    }

    let base_start = 96;
    let exp_start = 96 + base_len;
    let mod_start = 96 + base_len + exp_len;

    // Strip leading zeros from base and modulus
    let base_trimmed = trim_leading_zeros(&input[base_start..base_start + base_len]);
    let exp_trimmed = trim_leading_zeros(&input[exp_start..exp_start + exp_len]);
    let modulus_trimmed = trim_leading_zeros(&input[mod_start..mod_start + mod_len]);

    // If modulus is 0 or 1, result is 0
    if modulus_trimmed.is_empty() || (modulus_trimmed.len() == 1 && modulus_trimmed[0] == 1) {
        let mut result = vec![0u8; mod_len];
        if mod_len > 0 {
            result[mod_len - 1] = 0;
        }
        return Ok((
            PrecompileOutput {
                exit_status: ExitSucceed::Returned,
                output: result,
            },
            gas,
        ));
    }

    // For small operands (<= 32 bytes), use U256 arithmetic
    if base_trimmed.len() <= 32 && modulus_trimmed.len() <= 32 {
        let base_val = bigint_from_be_bytes(base_trimmed);
        let mod_val = bigint_from_be_bytes(modulus_trimmed);
        let result = if exp_trimmed.len() <= 32 {
            let exp_val = bigint_from_be_bytes(exp_trimmed);
            mod_pow_u256(base_val, exp_val, mod_val)
        } else {
            // Large exponent — use binary method byte-by-byte
            mod_pow_binary(base_trimmed, exp_trimmed, mod_val)
        };
        let mut output = vec![0u8; mod_len];
        let result_bytes = result.to_big_endian();
        let copy_len = core::cmp::min(mod_len, 32);
        output[mod_len - copy_len..].copy_from_slice(&result_bytes[32 - copy_len..]);
        return Ok((
            PrecompileOutput {
                exit_status: ExitSucceed::Returned,
                output,
            },
            gas,
        ));
    }

    // Large operands — use byte-level binary exponentiation
    let result = mod_pow_binary_large(base_trimmed, exp_trimmed, modulus_trimmed);
    let mut output = vec![0u8; mod_len];
    let copy_len = core::cmp::min(result.len(), mod_len);
    output[mod_len - copy_len..].copy_from_slice(&result[..copy_len]);
    Ok((
        PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            output,
        },
        gas,
    ))
}

/// Convert a big-endian byte slice to U256.
fn u256_from_be_slice(slice: &[u8]) -> sp_core::U256 {
    let mut buf = [0u8; 32];
    let len = core::cmp::min(slice.len(), 32);
    buf[32 - len..].copy_from_slice(&slice[..len]);
    sp_core::U256::from_big_endian(&buf)
}

/// Count the number of bits in a U256 value, clamped to max_bytes.
fn bit_length(val: &sp_core::U256, max_bytes: usize) -> usize {
    let be = val.to_big_endian();
    let start = 32 - core::cmp::min(max_bytes, 32);
    for (i, &byte) in be[start..32].iter().enumerate() {
        if byte != 0 {
            return (32 - (start + i)) * 8 - byte.leading_zeros() as usize;
        }
    }
    0
}

/// Trim leading zero bytes from a slice.
fn trim_leading_zeros(bytes: &[u8]) -> &[u8] {
    let mut i = 0;
    while i < bytes.len() && bytes[i] == 0 {
        i += 1;
    }
    &bytes[i..]
}

/// Convert big-endian bytes to U256 (padded to 32 bytes).
fn bigint_from_be_bytes(bytes: &[u8]) -> sp_core::U256 {
    let mut buf = [0u8; 32];
    let len = core::cmp::min(bytes.len(), 32);
    buf[32 - len..].copy_from_slice(bytes);
    sp_core::U256::from_big_endian(&buf)
}

/// Modular exponentiation using U256 arithmetic (small operands).
fn mod_pow_u256(base: sp_core::U256, exp: sp_core::U256, modulus: sp_core::U256) -> sp_core::U256 {
    if modulus.is_zero() {
        return sp_core::U256::zero();
    }
    let one = sp_core::U256::one();
    if modulus == one {
        return sp_core::U256::zero();
    }
    let mut result = one;
    let mut b = base % modulus;
    let mut e = exp;
    while !e.is_zero() {
        if e.bit(0) {
            result = (result * b) % modulus;
        }
        e >>= 1;
        b = (b * b) % modulus;
    }
    result
}

/// Modular exponentiation with byte-level exponent (for exp > 32 bytes).
fn mod_pow_binary(base: &[u8], exp: &[u8], modulus: sp_core::U256) -> sp_core::U256 {
    let one = sp_core::U256::one();
    let mut result = one;
    let b = bigint_from_be_bytes(base) % modulus;
    let mut base_val = b;
    for &byte in exp {
        for bit in (0..8).rev() {
            if byte & (1 << bit) != 0 {
                result = (result * base_val) % modulus;
            }
            base_val = (base_val * base_val) % modulus;
        }
    }
    result
}

/// Modular exponentiation for large operands using byte-level arithmetic.
fn mod_pow_binary_large(base: &[u8], exp: &[u8], modulus: &[u8]) -> Vec<u8> {
    let one = vec![1u8];
    let mut result = one.clone();
    let mut b = base.to_vec();
    // Reduce base modulo modulus first
    b = long_division_remainder(&b, modulus);
    for &byte in exp {
        for bit in (0..8).rev() {
            if byte & (1 << bit) != 0 {
                result = long_multiplication_mod(&result, &b, modulus);
            }
            b = long_multiplication_mod(&b, &b, modulus);
        }
    }
    result
}

/// Long multiplication followed by modular reduction.
fn long_multiplication_mod(a: &[u8], b: &[u8], modulus: &[u8]) -> Vec<u8> {
    let product = long_multiplication(a, b);
    long_division_remainder(&product, modulus)
}

/// Long multiplication of two big-endian byte arrays.
fn long_multiplication(a: &[u8], b: &[u8]) -> Vec<u8> {
    let len = a.len() + b.len();
    let mut result = vec![0u8; len];
    for (i, &ai) in a.iter().enumerate().rev() {
        let mut carry = 0u16;
        for (j, &bj) in b.iter().enumerate().rev() {
            let idx = i + j + 1;
            let sum = result[idx] as u16 + (ai as u16) * (bj as u16) + carry;
            result[idx] = (sum & 0xFF) as u8;
            carry = sum >> 8;
        }
        result[i] += carry as u8;
    }
    trim_leading_zeros(&result).to_vec()
}

/// Long division: return remainder of dividing `a` by `modulus`.
fn long_division_remainder(a: &[u8], modulus: &[u8]) -> Vec<u8> {
    if modulus.is_empty() || (modulus.len() == 1 && modulus[0] == 1) {
        return vec![0u8];
    }
    if a.len() < modulus.len() {
        return a.to_vec();
    }
    if a.len() == modulus.len() && a <= modulus {
        return if a < modulus { a.to_vec() } else { vec![0u8] };
    }
    // Binary long division
    let mut remainder = a.to_vec();
    let mod_len = modulus.len();
    while remainder.len() >= mod_len {
        if remainder.len() > mod_len || &remainder[..mod_len] >= modulus {
            // Subtract modulus aligned to the MSB
            let shift = remainder.len() - mod_len;
            let mut borrow = 0i16;
            for i in (0..mod_len).rev() {
                let diff = remainder[shift + i] as i16 - modulus[i] as i16 - borrow;
                if diff < 0 {
                    remainder[shift + i] = (diff + 256) as u8;
                    borrow = 1;
                } else {
                    remainder[shift + i] = diff as u8;
                    borrow = 0;
                }
            }
            // Trim leading zeros
            let trimmed = trim_leading_zeros(&remainder);
            remainder = trimmed.to_vec();
        } else {
            break;
        }
    }
    if remainder.is_empty() {
        vec![0u8]
    } else {
        remainder
    }
}

// ---------------------------------------------------------------------------
// 0x06 — bn128Add (BN254 G1 point addition)
// ---------------------------------------------------------------------------
/// BN254 G1 point addition (EIP-196).
///
/// Input: 128 bytes — two G1 points (x: 32, y: 32 each) in affine coordinates.
/// Output: 64 bytes — the resulting G1 point (x: 32, y: 32).
///
/// Points are encoded as big-endian field elements. The point at infinity
/// is represented as (0, 0).
fn precompile_bn128_add(
    input: &[u8],
    target_gas: Option<u64>,
    _context: &Context,
    _is_static: bool,
) -> Result<(PrecompileOutput, u64), PrecompileFailure> {
    if input.len() < 128 {
        return Err(PrecompileFailure::Error {
            exit_status: ExitError::Other(sp_std::borrow::Cow::Borrowed(
                "bn128add: input must be exactly 128 bytes",
            )),
        });
    }

    // Gas: 150 (EIP-196)
    const GAS_COST: u64 = 150;
    if let Some(limit) = target_gas {
        if limit < GAS_COST {
            return Err(PrecompileFailure::Error {
                exit_status: ExitError::OutOfGas,
            });
        }
    }

    let p1 = decode_g1(&input[..64]);
    let p2 = decode_g1(&input[64..128]);

    // If either point is invalid (not on curve), return error
    let p1_affine = match p1 {
        Some(p) => p,
        None => {
            return Err(PrecompileFailure::Error {
                exit_status: ExitError::Other(sp_std::borrow::Cow::Borrowed(
                    "bn128add: invalid point 1",
                )),
            });
        }
    };
    let p2_affine = match p2 {
        Some(p) => p,
        None => {
            return Err(PrecompileFailure::Error {
                exit_status: ExitError::Other(sp_std::borrow::Cow::Borrowed(
                    "bn128add: invalid point 2",
                )),
            });
        }
    };

    // Convert to projective, add, convert back
    let p1_proj = ark_bn254::G1Projective::from(p1_affine);
    let p2_proj = ark_bn254::G1Projective::from(p2_affine);
    let result_proj = p1_proj + p2_proj;
    let result_affine = result_proj.into_affine();

    let output = encode_g1(&result_affine);
    Ok((
        PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            output,
        },
        GAS_COST,
    ))
}

// ---------------------------------------------------------------------------
// 0x07 — bn128Mul (BN254 G1 scalar multiplication)
// ---------------------------------------------------------------------------
/// BN254 G1 scalar multiplication (EIP-196).
///
/// Input: 96 bytes — G1 point (x: 32, y: 32) + scalar (32 bytes big-endian).
/// Output: 64 bytes — the resulting G1 point (x: 32, y: 32).
fn precompile_bn128_mul(
    input: &[u8],
    target_gas: Option<u64>,
    _context: &Context,
    _is_static: bool,
) -> Result<(PrecompileOutput, u64), PrecompileFailure> {
    if input.len() < 96 {
        return Err(PrecompileFailure::Error {
            exit_status: ExitError::Other(sp_std::borrow::Cow::Borrowed(
                "bn128mul: input must be exactly 96 bytes",
            )),
        });
    }

    // Gas: 6,000 (EIP-196)
    const GAS_COST: u64 = 6_000;
    if let Some(limit) = target_gas {
        if limit < GAS_COST {
            return Err(PrecompileFailure::Error {
                exit_status: ExitError::OutOfGas,
            });
        }
    }

    let point = decode_g1(&input[..64]);
    let point_affine = match point {
        Some(p) => p,
        None => {
            return Err(PrecompileFailure::Error {
                exit_status: ExitError::Other(sp_std::borrow::Cow::Borrowed(
                    "bn128mul: invalid point",
                )),
            });
        }
    };

    // Decode scalar (big-endian 32 bytes)
    let scalar = ark_bn254::Fr::from_be_bytes_mod_order(&input[64..96]);

    // Multiply
    let result_proj = ark_bn254::G1Projective::from(point_affine) * scalar;
    let result_affine = result_proj.into_affine();

    let output = encode_g1(&result_affine);
    Ok((
        PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            output,
        },
        GAS_COST,
    ))
}

// ---------------------------------------------------------------------------
// 0x08 — bn128Pairing (BN254 pairing check)
// ---------------------------------------------------------------------------
/// BN254 pairing check (EIP-197).
///
/// Input: pairs of (128 bytes G2 point + 64 bytes G1 point), repeated.
/// Output: 32 bytes — 0x00...01 if the pairing product equals 1, else 0x00...00.
///
/// G2 points are encoded as 128 bytes: (x_imaginary, x_real, y_imaginary, y_real),
/// each as 32-byte big-endian field elements in Fp2.
/// G1 points are encoded as 64 bytes: (x, y), each as 32-byte big-endian.
fn precompile_bn128_pairing(
    input: &[u8],
    target_gas: Option<u64>,
    _context: &Context,
    _is_static: bool,
) -> Result<(PrecompileOutput, u64), PrecompileFailure> {
    // Each pair is 192 bytes (128 G2 + 64 G1)
    if !input.len().is_multiple_of(192) {
        return Err(PrecompileFailure::Error {
            exit_status: ExitError::Other(sp_std::borrow::Cow::Borrowed(
                "bn128pairing: input length must be multiple of 192",
            )),
        });
    }

    let num_pairs = input.len() / 192;
    if num_pairs == 0 {
        // Empty input — pairing of no elements is 1 by convention
        let mut output = vec![0u8; 32];
        output[31] = 1;
        return Ok((
            PrecompileOutput {
                exit_status: ExitSucceed::Returned,
                output,
            },
            0,
        ));
    }

    // Gas: base 45,000 + 34,000 per pair (EIP-197)
    let gas = 45_000 + 34_000 * num_pairs as u64;
    if let Some(limit) = target_gas {
        if limit < gas {
            return Err(PrecompileFailure::Error {
                exit_status: ExitError::OutOfGas,
            });
        }
    }

    // Use ark-bn254's multi-pairing check
    let mut pairs = Vec::with_capacity(num_pairs);
    for i in 0..num_pairs {
        let offset = i * 192;
        let g2_bytes = &input[offset..offset + 128];
        let g1_bytes = &input[offset + 128..offset + 192];

        let g1 = match decode_g1(g1_bytes) {
            Some(p) => p,
            None => {
                return Err(PrecompileFailure::Error {
                    exit_status: ExitError::Other(sp_std::borrow::Cow::Borrowed(
                        "bn128pairing: invalid G1 point",
                    )),
                });
            }
        };

        let g2 = match decode_g2(g2_bytes) {
            Some(p) => p,
            None => {
                return Err(PrecompileFailure::Error {
                    exit_status: ExitError::Other(sp_std::borrow::Cow::Borrowed(
                        "bn128pairing: invalid G2 point",
                    )),
                });
            }
        };

        pairs.push((g1, g2));
    }

    // Perform the multi-pairing check
    let result = ark_bn254::Bn254::multi_pairing(
        pairs.iter().map(|(g1, _)| g1),
        pairs.iter().map(|(_, g2)| g2),
    );

    let is_one = result.0 == ark_bn254::Fq12::ONE;

    let mut output = vec![0u8; 32];
    if is_one {
        output[31] = 1;
    }
    Ok((
        PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            output,
        },
        gas,
    ))
}

// ---------------------------------------------------------------------------
// BN254 helper functions
// ---------------------------------------------------------------------------

/// Decode a G1 point from 64 bytes (x: 32, y: 32 big-endian).
/// Returns None if the point is invalid (not on curve or both coordinates zero).
fn decode_g1(bytes: &[u8]) -> Option<ark_bn254::G1Affine> {
    debug_assert!(bytes.len() >= 64);
    let x = ark_bn254::Fq::from_be_bytes_mod_order(&bytes[..32]);
    let y = ark_bn254::Fq::from_be_bytes_mod_order(&bytes[32..64]);

    // Point at infinity: (0, 0)
    if x.is_zero() && y.is_zero() {
        return Some(ark_bn254::G1Affine::identity());
    }

    let point = ark_bn254::G1Affine::new_unchecked(x, y);
    // Verify the point is on the curve
    if !point.is_on_curve() {
        return None;
    }
    Some(point)
}

/// Encode a G1 point to 64 bytes (x: 32, y: 32 big-endian).
fn encode_g1(point: &ark_bn254::G1Affine) -> Vec<u8> {
    if point.is_zero() {
        return vec![0u8; 64];
    }
    let mut output = vec![0u8; 64];
    // x() and y() return Option<&Fq> — safe to unwrap since we checked !is_zero()
    if let (Some(x), Some(y)) = (point.x(), point.y()) {
        let x_bigint = x.into_bigint();
        let y_bigint = y.into_bigint();
        let x_bytes = x_bigint.to_bytes_be();
        let y_bytes = y_bigint.to_bytes_be();
        output[..32].copy_from_slice(&x_bytes);
        output[32..64].copy_from_slice(&y_bytes);
    }
    output
}
/// Decode a G2 point from 128 bytes
/// (x_imaginary: 32, x_real: 32, y_imaginary: 32, y_real: 32 big-endian).
/// Returns None if the point is invalid.
fn decode_g2(bytes: &[u8]) -> Option<ark_bn254::G2Affine> {
    debug_assert!(bytes.len() >= 128);
    // Fp2 element: c0 + c1 * u, where u^2 = -1
    // EVM encoding: (c1, c0) = (imaginary, real)
    let x_c1 = ark_bn254::Fq::from_be_bytes_mod_order(&bytes[..32]); // imaginary
    let x_c0 = ark_bn254::Fq::from_be_bytes_mod_order(&bytes[32..64]); // real
    let y_c1 = ark_bn254::Fq::from_be_bytes_mod_order(&bytes[64..96]); // imaginary
    let y_c0 = ark_bn254::Fq::from_be_bytes_mod_order(&bytes[96..128]); // real

    let x = ark_bn254::Fq2::new(x_c0, x_c1);
    let y = ark_bn254::Fq2::new(y_c0, y_c1);

    // Point at infinity
    if x.is_zero() && y.is_zero() {
        return Some(ark_bn254::G2Affine::identity());
    }

    let point = ark_bn254::G2Affine::new_unchecked(x, y);
    if !point.is_on_curve() {
        return None;
    }
    Some(point)
}

// ---------------------------------------------------------------------------
// 0x09 — blake2f
// ---------------------------------------------------------------------------
/// Blake2b compression function F (EIP-152).
/// Input: 213 bytes — rounds(4) | h(64) | m(128) | t(16) | f(1)
/// Output: 64 bytes — the resulting state vector h
fn precompile_blake2f(
    input: &[u8],
    target_gas: Option<u64>,
    _context: &Context,
    _is_static: bool,
) -> Result<(PrecompileOutput, u64), PrecompileFailure> {
    if input.len() != 213 {
        return Err(PrecompileFailure::Error {
            exit_status: ExitError::Other(sp_std::borrow::Cow::Borrowed(
                "blake2f: input must be exactly 213 bytes",
            )),
        });
    }

    let rounds = u32::from_be_bytes([input[0], input[1], input[2], input[3]]) as u64;
    let gas = rounds;
    if let Some(limit) = target_gas {
        if limit < gas {
            return Err(PrecompileFailure::Error {
                exit_status: ExitError::OutOfGas,
            });
        }
    }

    let f_flag = input[212];
    if f_flag != 0 && f_flag != 1 {
        return Err(PrecompileFailure::Error {
            exit_status: ExitError::Other(sp_std::borrow::Cow::Borrowed(
                "blake2f: f flag must be 0 or 1",
            )),
        });
    }

    // Parse state vector h (8 x u64 LE)
    let mut h = [0u64; 8];
    for (i, value) in h.iter_mut().enumerate() {
        let offset = 4 + i * 8;
        *value = u64::from_le_bytes([
            input[offset],
            input[offset + 1],
            input[offset + 2],
            input[offset + 3],
            input[offset + 4],
            input[offset + 5],
            input[offset + 6],
            input[offset + 7],
        ]);
    }

    // Parse message block m (16 x u64 LE)
    let mut m = [0u64; 16];
    for (i, value) in m.iter_mut().enumerate() {
        let offset = 68 + i * 8;
        *value = u64::from_le_bytes([
            input[offset],
            input[offset + 1],
            input[offset + 2],
            input[offset + 3],
            input[offset + 4],
            input[offset + 5],
            input[offset + 6],
            input[offset + 7],
        ]);
    }

    // Parse counter t (2 x u64 LE)
    let t0 = u64::from_le_bytes([
        input[196], input[197], input[198], input[199], input[200], input[201], input[202],
        input[203],
    ]);
    let t1 = u64::from_le_bytes([
        input[204], input[205], input[206], input[207], input[208], input[209], input[210],
        input[211],
    ]);

    let f = f_flag != 0;
    blake2f_compress(&mut h, &m, [t0, t1], f, rounds as u32);

    // Output: h as 64 bytes LE
    let mut output = vec![0u8; 64];
    for i in 0..8 {
        let bytes = h[i].to_le_bytes();
        output[i * 8..(i + 1) * 8].copy_from_slice(&bytes);
    }

    Ok((
        PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            output,
        },
        gas,
    ))
}

/// Blake2b compression function F per RFC 7693.
fn blake2f_compress(h: &mut [u64; 8], m: &[u64; 16], t: [u64; 2], f: bool, rounds: u32) {
    // Blake2b IV
    const IV: [u64; 8] = [
        0x6a09e667f3bcc908,
        0xbb67ae8584caa73b,
        0x3c6ef372fe94f82b,
        0xa54ff53a5f1d36f1,
        0x510e527fade682d1,
        0x9b05688c2b3e6c1f,
        0x1f83d9abfb41bd6b,
        0x5be0cd19137e2179,
    ];

    // Sigma permutation table
    const SIGMA: [[usize; 16]; 10] = [
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
        [11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4],
        [7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8],
        [9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13],
        [2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9],
        [12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11],
        [13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10],
        [6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5],
        [10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0],
    ];

    let mut v = [0u64; 16];
    v[..8].copy_from_slice(h);
    v[8..16].copy_from_slice(&IV);

    v[12] ^= t[0];
    v[13] ^= t[1];

    if f {
        v[14] = !v[14];
    }

    for i in 0..rounds as usize {
        let s = &SIGMA[i % 10];

        // Column step
        g(&mut v, 0, 4, 8, 12, m[s[0]], m[s[1]]);
        g(&mut v, 1, 5, 9, 13, m[s[2]], m[s[3]]);
        g(&mut v, 2, 6, 10, 14, m[s[4]], m[s[5]]);
        g(&mut v, 3, 7, 11, 15, m[s[6]], m[s[7]]);

        // Diagonal step
        g(&mut v, 0, 5, 10, 15, m[s[8]], m[s[9]]);
        g(&mut v, 1, 6, 11, 12, m[s[10]], m[s[11]]);
        g(&mut v, 2, 7, 8, 13, m[s[12]], m[s[13]]);
        g(&mut v, 3, 4, 9, 14, m[s[14]], m[s[15]]);
    }

    for i in 0..8 {
        h[i] ^= v[i] ^ v[i + 8];
    }
}

/// Blake2b G mixing function.
#[inline]
fn g(v: &mut [u64; 16], a: usize, b: usize, c: usize, d: usize, x: u64, y: u64) {
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(x);
    v[d] = (v[d] ^ v[a]).rotate_right(32);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(24);
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(y);
    v[d] = (v[d] ^ v[a]).rotate_right(16);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(63);
}
