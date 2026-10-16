//! Reading a transaction's outputs from its legacy (non-witness) serialization — the form whose
//! SHA-256d is the txid, and so the only form a Merkle proof can bind.

use alloc::vec::Vec;

use crate::SpvError;

/// One transaction output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TxOut {
    /// Satoshis.
    pub value: u64,
    pub script_pubkey: Vec<u8>,
}

/// Why a transaction could not be read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TxError {
    /// The bytes ended before the transaction did.
    Truncated,
    /// Bytes remained after the lock time.
    TrailingBytes,
    /// The witness serialization (`00 01` after the version); its hash is the wtxid, not the txid.
    WitnessSerialization,
    /// A count or length that cannot be real (larger than the bytes that remain).
    ImplausibleLength,
    /// No inputs or no outputs.
    Empty,
}

impl From<TxError> for SpvError {
    fn from(error: TxError) -> Self {
        SpvError::MalformedTransaction(error)
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], TxError> {
        let end = self.at.checked_add(n).ok_or(TxError::Truncated)?;
        let slice = self.bytes.get(self.at..end).ok_or(TxError::Truncated)?;
        self.at = end;
        Ok(slice)
    }

    fn u64(&mut self) -> Result<u64, TxError> {
        let mut word = [0u8; 8];
        word.copy_from_slice(self.take(8)?);
        Ok(u64::from_le_bytes(word))
    }

    /// Bitcoin's CompactSize, bounded by what remains so a forged count cannot drive allocation.
    fn count(&mut self) -> Result<usize, TxError> {
        let first = self.take(1)?[0];
        let value = match first {
            0xfd => u64::from(u16::from_le_bytes([self.take(1)?[0], self.take(1)?[0]])),
            0xfe => {
                let b = self.take(4)?;
                u64::from(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            }
            0xff => self.u64()?,
            n => u64::from(n),
        };
        let remaining = (self.bytes.len() - self.at) as u64;
        if value > remaining {
            return Err(TxError::ImplausibleLength);
        }
        Ok(value as usize)
    }
}

/// The outputs of a legacy-serialized transaction. The whole transaction is read, so trailing or
/// missing bytes are refused rather than ignored.
pub fn transaction_outputs(raw: &[u8]) -> Result<Vec<TxOut>, TxError> {
    let mut r = Reader { bytes: raw, at: 0 };
    r.take(4)?; // version
    let inputs = r.count()?;
    if inputs == 0 {
        // `00` here is the witness marker; a legacy transaction always has an input.
        return Err(if r.bytes.get(r.at) == Some(&1) {
            TxError::WitnessSerialization
        } else {
            TxError::Empty
        });
    }
    for _ in 0..inputs {
        r.take(36)?; // previous outpoint
        let script = r.count()?;
        r.take(script)?;
        r.take(4)?; // sequence
    }
    let count = r.count()?;
    if count == 0 {
        return Err(TxError::Empty);
    }
    let mut outputs = Vec::with_capacity(count.min(raw.len() / 9));
    for _ in 0..count {
        let value = r.u64()?;
        let script = r.count()?;
        outputs.push(TxOut {
            value,
            script_pubkey: r.take(script)?.to_vec(),
        });
    }
    r.take(4)?; // lock time
    if r.at != raw.len() {
        return Err(TxError::TrailingBytes);
    }
    Ok(outputs)
}
