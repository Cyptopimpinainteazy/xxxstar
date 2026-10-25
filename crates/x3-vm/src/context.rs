//! The block context a program can read: who called it, on which block, when, with what value, on
//! which chain.
//!
//! Every field is optional, and the opcode that reads a field the host did not supply refuses
//! (`VMErrorKind::ContextUnavailable`) instead of answering a default. A zero sender or a zero
//! block height is a plausible-looking value a program would act on, so "the host did not say" must
//! not be spelled the same way as "the answer is zero". A host that has a field supplies it; a host
//! that does not (a cross-VM call has no signing account, an off-chain tool has no block) leaves it
//! out, and the program learns that rather than a fabrication.

/// Values the host knows about the execution, each `None` when it does not.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BlockContext {
    /// The account that submitted the call, as 32 bytes.
    pub sender: Option<[u8; 32]>,
    /// The block the call executes in.
    pub block_height: Option<u64>,
    /// The block's timestamp, in milliseconds since the Unix epoch.
    pub timestamp_ms: Option<u64>,
    /// Native value transferred with the call (`Some(0)` when the call carries none).
    pub value: Option<u128>,
    /// The chain the call executes on.
    pub chain_id: Option<u64>,
}

impl BlockContext {
    /// A context with no fields: every context opcode refuses.
    pub const fn unknown() -> Self {
        Self {
            sender: None,
            block_height: None,
            timestamp_ms: None,
            value: None,
            chain_id: None,
        }
    }
}
