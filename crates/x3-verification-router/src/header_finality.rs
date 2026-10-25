//! Independently verifiable external-header finality certificates.

extern crate alloc;

use alloc::vec::Vec;

pub const HEADER_FINALITY_FORMAT_V1: u8 = 1;
pub const HEADER_FINALITY_ETHEREUM: u8 = 1;
pub const HEADER_FINALITY_SOLANA: u8 = 2;
pub const MAX_ATTESTERS: usize = 64;
pub const MAX_HEADER_CERTIFICATE_BYTES: usize = 110 + MAX_ATTESTERS * 96;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderFinalityError {
    UnsupportedFormat,
    MalformedProof,
    UnsupportedChain,
    WrongChain,
    HeightMismatch,
    BlockHashMismatch,
    StateRootMismatch,
    AuxiliaryRootMismatch,
    NoAuthorizedAttesters,
    InsufficientValidAttestations { got: u32, need: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FinalizedHeader {
    pub chain: u8,
    pub height: u64,
    pub block_hash: [u8; 32],
    pub state_root: [u8; 32],
    pub auxiliary_root: [u8; 32],
}

pub fn header_finality_message(header: &FinalizedHeader) -> [u8; 32] {
    use blake2::digest::consts::U32;
    use blake2::{Blake2b, Digest};

    let mut data = Vec::with_capacity(9 + 32 * 3 + 8);
    data.extend_from_slice(b"X3-HDR1");
    data.push(header.chain);
    data.extend_from_slice(&header.height.to_le_bytes());
    data.extend_from_slice(&header.block_hash);
    data.extend_from_slice(&header.state_root);
    data.extend_from_slice(&header.auxiliary_root);
    let mut hasher = Blake2b::<U32>::new();
    hasher.update(&data);
    hasher.finalize().into()
}

pub fn encode_header_certificate(
    header: FinalizedHeader,
    attestations: &[([u8; 32], [u8; 64])],
) -> Vec<u8> {
    let mut out = Vec::with_capacity(110 + attestations.len() * 96);
    out.push(HEADER_FINALITY_FORMAT_V1);
    out.push(header.chain);
    out.extend_from_slice(&header.height.to_le_bytes());
    out.extend_from_slice(&header.block_hash);
    out.extend_from_slice(&header.state_root);
    out.extend_from_slice(&header.auxiliary_root);
    out.extend_from_slice(&(attestations.len() as u32).to_le_bytes());
    for (key, signature) in attestations {
        out.extend_from_slice(key);
        out.extend_from_slice(signature);
    }
    out
}

pub fn verify_header_certificate(
    expected_chain: u8,
    expected: FinalizedHeader,
    authorized: &[[u8; 32]],
    threshold: u32,
    payload: &[u8],
) -> Result<FinalizedHeader, HeaderFinalityError>
where
{
    use alloc::collections::btree_set::BTreeSet;
    use ed25519_dalek::{Signature, VerifyingKey};

    if payload.is_empty() {
        return Err(HeaderFinalityError::MalformedProof);
    }
    if payload[0] != HEADER_FINALITY_FORMAT_V1 {
        return Err(HeaderFinalityError::UnsupportedFormat);
    }
    if !matches!(
        expected_chain,
        HEADER_FINALITY_ETHEREUM | HEADER_FINALITY_SOLANA
    ) {
        return Err(HeaderFinalityError::UnsupportedChain);
    }
    if expected.chain != expected_chain {
        return Err(HeaderFinalityError::WrongChain);
    }
    if payload.len() < 110 {
        return Err(HeaderFinalityError::MalformedProof);
    }
    if payload[1] != expected_chain {
        return Err(HeaderFinalityError::WrongChain);
    }

    let mut at = 2usize;
    let mut take = |n: usize| -> Result<&[u8], HeaderFinalityError> {
        let end = at
            .checked_add(n)
            .ok_or(HeaderFinalityError::MalformedProof)?;
        let slice = payload
            .get(at..end)
            .ok_or(HeaderFinalityError::MalformedProof)?;
        at = end;
        Ok(slice)
    };

    let mut height_bytes = [0u8; 8];
    height_bytes.copy_from_slice(take(8)?);
    let mut block_hash = [0u8; 32];
    block_hash.copy_from_slice(take(32)?);
    let mut state_root = [0u8; 32];
    state_root.copy_from_slice(take(32)?);
    let mut auxiliary_root = [0u8; 32];
    auxiliary_root.copy_from_slice(take(32)?);
    let mut count_bytes = [0u8; 4];
    count_bytes.copy_from_slice(take(4)?);
    let count = u32::from_le_bytes(count_bytes) as usize;
    if count > MAX_ATTESTERS {
        return Err(HeaderFinalityError::MalformedProof);
    }
    let expected_len = 110usize
        .checked_add(
            count
                .checked_mul(96)
                .ok_or(HeaderFinalityError::MalformedProof)?,
        )
        .ok_or(HeaderFinalityError::MalformedProof)?;
    if payload.len() != expected_len {
        return Err(HeaderFinalityError::MalformedProof);
    }

    let decoded = FinalizedHeader {
        chain: expected_chain,
        height: u64::from_le_bytes(height_bytes),
        block_hash,
        state_root,
        auxiliary_root,
    };
    if decoded.height != expected.height {
        return Err(HeaderFinalityError::HeightMismatch);
    }
    if decoded.block_hash != expected.block_hash {
        return Err(HeaderFinalityError::BlockHashMismatch);
    }
    if decoded.state_root != expected.state_root {
        return Err(HeaderFinalityError::StateRootMismatch);
    }
    if decoded.auxiliary_root != expected.auxiliary_root {
        return Err(HeaderFinalityError::AuxiliaryRootMismatch);
    }

    if authorized.is_empty() || threshold == 0 {
        return Err(HeaderFinalityError::NoAuthorizedAttesters);
    }

    let message = header_finality_message(&decoded);
    let mut counted = BTreeSet::new();
    for _ in 0..count {
        let mut public_key = [0u8; 32];
        public_key.copy_from_slice(take(32)?);
        let signature_bytes = take(64)?;
        if !authorized.contains(&public_key) || counted.contains(&public_key) {
            continue;
        }
        let Ok(verifying_key) = VerifyingKey::from_bytes(&public_key) else {
            continue;
        };
        let Ok(signature) = Signature::from_slice(signature_bytes) else {
            continue;
        };
        if verifying_key.verify_strict(&message, &signature).is_ok() {
            counted.insert(public_key);
        }
    }

    if counted.len() < threshold as usize {
        return Err(HeaderFinalityError::InsufficientValidAttestations {
            got: counted.len() as u32,
            need: threshold,
        });
    }
    Ok(decoded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn key(seed: u8) -> SigningKey {
        SigningKey::from_bytes(&[seed; 32])
    }

    fn header() -> FinalizedHeader {
        FinalizedHeader {
            chain: HEADER_FINALITY_ETHEREUM,
            height: 42,
            block_hash: [1; 32],
            state_root: [2; 32],
            auxiliary_root: [3; 32],
        }
    }

    fn signed(header: FinalizedHeader, keys: &[SigningKey]) -> Vec<([u8; 32], [u8; 64])> {
        let message = header_finality_message(&header);
        keys.iter()
            .map(|key| {
                (
                    key.verifying_key().to_bytes(),
                    key.sign(&message).to_bytes(),
                )
            })
            .collect()
    }

    #[test]
    fn threshold_of_distinct_authorized_attesters_is_accepted() {
        let keys = [key(1), key(2), key(3)];
        let authorized = keys
            .iter()
            .map(|key| key.verifying_key().to_bytes())
            .collect::<Vec<_>>();
        let payload = encode_header_certificate(header(), &signed(header(), &keys[..2]));

        assert_eq!(
            verify_header_certificate(HEADER_FINALITY_ETHEREUM, header(), &authorized, 2, &payload,),
            Ok(header())
        );
    }

    #[test]
    fn unknown_duplicate_and_bad_signatures_do_not_count() {
        let keys = [key(1), key(2), key(9)];
        let authorized = vec![
            keys[0].verifying_key().to_bytes(),
            keys[1].verifying_key().to_bytes(),
        ];
        let mut attestations = signed(header(), &keys);
        attestations[1] = attestations[0];
        attestations[2].1[0] ^= 1;
        let payload = encode_header_certificate(header(), &attestations);

        assert_eq!(
            verify_header_certificate(HEADER_FINALITY_ETHEREUM, header(), &authorized, 2, &payload,),
            Err(HeaderFinalityError::InsufficientValidAttestations { got: 1, need: 2 })
        );
    }

    #[test]
    fn every_header_field_and_chain_is_bound() {
        let keys = [key(1), key(2)];
        let authorized = keys
            .iter()
            .map(|key| key.verifying_key().to_bytes())
            .collect::<Vec<_>>();
        let payload = encode_header_certificate(header(), &signed(header(), &keys));

        let mut wrong = header();
        wrong.height += 1;
        assert_eq!(
            verify_header_certificate(HEADER_FINALITY_ETHEREUM, wrong, &authorized, 2, &payload,),
            Err(HeaderFinalityError::HeightMismatch)
        );

        let mut wrong = header();
        wrong.block_hash[0] ^= 1;
        assert_eq!(
            verify_header_certificate(HEADER_FINALITY_ETHEREUM, wrong, &authorized, 2, &payload,),
            Err(HeaderFinalityError::BlockHashMismatch)
        );

        let mut wrong = header();
        wrong.state_root[0] ^= 1;
        assert_eq!(
            verify_header_certificate(HEADER_FINALITY_ETHEREUM, wrong, &authorized, 2, &payload,),
            Err(HeaderFinalityError::StateRootMismatch)
        );

        let mut wrong = header();
        wrong.auxiliary_root[0] ^= 1;
        assert_eq!(
            verify_header_certificate(HEADER_FINALITY_ETHEREUM, wrong, &authorized, 2, &payload,),
            Err(HeaderFinalityError::AuxiliaryRootMismatch)
        );

        assert_eq!(
            verify_header_certificate(HEADER_FINALITY_SOLANA, header(), &authorized, 2, &payload,),
            Err(HeaderFinalityError::WrongChain)
        );

        let mut wrong_expected = header();
        wrong_expected.chain = HEADER_FINALITY_SOLANA;
        assert_eq!(
            verify_header_certificate(
                HEADER_FINALITY_ETHEREUM,
                wrong_expected,
                &authorized,
                2,
                &payload,
            ),
            Err(HeaderFinalityError::WrongChain)
        );
    }

    #[test]
    fn malformed_unsupported_and_unconfigured_inputs_fail_closed() {
        assert_eq!(
            verify_header_certificate(HEADER_FINALITY_ETHEREUM, header(), &[], 1, &[],),
            Err(HeaderFinalityError::MalformedProof)
        );

        let payload = encode_header_certificate(header(), &[]);
        assert_eq!(
            verify_header_certificate(HEADER_FINALITY_ETHEREUM, header(), &[], 1, &payload,),
            Err(HeaderFinalityError::NoAuthorizedAttesters)
        );

        let mut unsupported = payload;
        unsupported[0] = 9;
        let key = key(1);
        assert_eq!(
            verify_header_certificate(
                HEADER_FINALITY_ETHEREUM,
                header(),
                &[key.verifying_key().to_bytes()],
                1,
                &unsupported,
            ),
            Err(HeaderFinalityError::UnsupportedFormat)
        );
    }
}
