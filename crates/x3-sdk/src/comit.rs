//! Comit transaction builder for X3 Chain.
//!
//! Provides a fluent API for constructing Comit transactions
//! that execute atomically across EVM and SVM.
//!
//! ## Payload convention (settled 2026-09-26, X3-LANG-004)
//!
//! A payload handed to `submit_comit` **is the artifact its adapter executes** —
//! EVM init/call bytecode, an eBPF program, or X3BC — and the adapter that runs it
//! is the component that validates it. A SCALE-encoded [`Packet`] is therefore *not*
//! a valid `submit_comit` payload: it is a semantic operation with no executor, and
//! every adapter refuses one by name (`payload_is_packet`). `evm_call`,
//! `evm_batch`, `svm_invoke` and `svm_deploy` still build packets and are deprecated
//! for that reason; `evm_deploy` builds init code and is the supported deploy path.

use crate::error::{AtlasError, Result};
use crate::types::{Balance, BlockNumber, ComitPayload, Gas, Nonce};
use crate::utils::compute_prepare_root;
use crate::{MAX_COMBINED_PAYLOAD_SIZE, MAX_EVM_PAYLOAD_SIZE, MAX_SVM_PAYLOAD_SIZE};
use codec::Encode;
use x3_packet_schema::{
    EvmCall, EvmPacket, Packet, SvmAccount, SvmDeployMetadata, SvmPacket, U256,
};

/// Default EVM gas limit
pub const DEFAULT_EVM_GAS_LIMIT: Gas = 500_000;

/// Default SVM compute unit limit
pub const DEFAULT_SVM_COMPUTE_LIMIT: Gas = 200_000;

/// Base fee per payload byte
pub const FEE_PER_BYTE: Balance = 1000;

/// Builder for constructing Comit transactions.
///
/// # Example
///
/// ```rust,no_run
/// use x3_sdk::ComitBuilder;
///
/// let comit = ComitBuilder::new()
///     .with_evm_payload(&[0x60, 0x80, 0x60, 0x40])
///     .with_svm_payload(&[0x01, 0x02, 0x03])
///     .with_nonce(42)
///     .with_fee(1_000_000)
///     .with_deadline(100)
///     .build()
///     .unwrap();
/// ```
#[derive(Clone, Debug, Default)]
pub struct ComitBuilder {
    evm_payload: Option<Vec<u8>>,
    svm_payload: Option<Vec<u8>>,
    nonce: Option<Nonce>,
    evm_gas_limit: Option<Gas>,
    svm_compute_limit: Option<Gas>,
    fee: Option<Balance>,
    deadline: Option<BlockNumber>,
    auto_fee: bool,
}

impl ComitBuilder {
    /// Create a new ComitBuilder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new EVM-only Comit builder with raw payload.
    ///
    /// **Deprecated for `submit_comit`**: The kernel's `submit_comit` extrinsic
    /// now requires SCALE-encoded [`Packet`] objects with proper domain
    /// validation.  Use [`evm_deploy`](Self::evm_deploy),
    /// [`evm_call`](Self::evm_call), or [`evm_batch`](Self::evm_batch)
    /// for the packet-validated path.  This method remains for the legacy
    /// `submit_comit_raw` path that accepts arbitrary byte payloads.
    #[deprecated(
        since = "0.2.0",
        note = "Use evm_deploy(), evm_call(), or evm_batch() for the packet-validated submit_comit path"
    )]
    pub fn evm(payload: impl AsRef<[u8]>) -> Self {
        Self::new().with_evm_payload(payload)
    }

    /// Create an EVM Deploy packet from contract bytecode.
    ///
    /// The payload is the **init code itself**, which is what the chain's EVM adapter
    /// executes. It used to SCALE-encode `Packet::Evm(EvmPacket::Deploy { .. })`, and
    /// that packet could never run: its first byte is the enum discriminant `0x00`,
    /// which is EVM `STOP`, so the interpreter halted on byte 0 — and, before
    /// X3-LANG-004, the kernel reported that halt as `success: true` with a receipt.
    /// The packet shape is now refused by every adapter by name.
    pub fn evm_deploy(bytecode: Vec<u8>) -> Self {
        Self::new().with_evm_payload(bytecode)
    }

    /// Create an EVM Call packet.
    ///
    /// Produces `Packet::Evm(EvmPacket::Call { contract, function_selector, args, value })`
    /// and SCALE-encodes it.
    ///
    /// **Not submittable via `submit_comit`**: a SCALE-encoded `Packet` is refused by every
    /// adapter, because the adapter interface is `(payload, gas_limit) -> receipt` and has
    /// nowhere to put the target contract, the selector, or the value this packet names.
    /// A call must be expressed as the bytecode that performs it.
    #[deprecated(
        since = "0.3.0",
        note = "packets are refused by the chain's adapters; a call must be built as the EVM bytecode that performs it"
    )]
    pub fn evm_call(
        contract: [u8; 20],
        function_selector: [u8; 4],
        args: Vec<u8>,
        value: U256,
    ) -> Self {
        let packet = Packet::Evm(EvmPacket::Call {
            contract,
            function_selector,
            args,
            value,
        });
        let encoded = packet.encode();
        Self::new().with_evm_payload(encoded)
    }

    /// Create an EVM Batch packet from a list of (call, value) pairs.
    ///
    /// **Not submittable via `submit_comit`** — see [`evm_call`](Self::evm_call).
    #[deprecated(
        since = "0.3.0",
        note = "packets are refused by the chain's adapters; a batch must be built as bytecode"
    )]
    pub fn evm_batch(calls: Vec<(EvmCall, Option<U256>)>, continue_on_revert: bool) -> Self {
        let packet = Packet::Evm(EvmPacket::Batch {
            calls,
            continue_on_revert,
        });
        let encoded = packet.encode();
        Self::new().with_evm_payload(encoded)
    }

    /// Create an SVM Invoke packet.
    ///
    /// Produces `Packet::Svm(SvmPacket::Invoke { program_id, accounts, data })`
    /// and SCALE-encodes it.
    ///
    /// **Not submittable via `submit_comit`**: the SVM adapter executes its payload as an
    /// eBPF program and has no account list or program-id context to route this packet to.
    #[deprecated(
        since = "0.3.0",
        note = "packets are refused by the chain's adapters; an invocation must be expressed as an eBPF program"
    )]
    pub fn svm_invoke(program_id: [u8; 32], accounts: Vec<SvmAccount>, data: Vec<u8>) -> Self {
        let packet = Packet::Svm(SvmPacket::Invoke {
            program_id,
            accounts,
            data,
        });
        let encoded = packet.encode();
        Self::new().with_svm_payload(encoded)
    }

    /// Create an SVM Deploy packet.
    ///
    /// Produces `Packet::Svm(SvmPacket::Deploy { bytecode, metadata })`
    /// and SCALE-encodes it.
    ///
    /// **Not submittable via `submit_comit`** — see [`svm_invoke`](Self::svm_invoke). Use
    /// [`svm`](Self::svm) / [`with_svm_payload`](Self::with_svm_payload) with the eBPF
    /// program itself; the `name`/`version` metadata has no home in the kernel's payload.
    #[deprecated(
        since = "0.3.0",
        note = "packets are refused by the chain's adapters; deploy the eBPF program itself"
    )]
    pub fn svm_deploy(bytecode: Vec<u8>, name: String, version: String) -> Self {
        let packet = Packet::Svm(SvmPacket::Deploy {
            bytecode,
            metadata: SvmDeployMetadata {
                name,
                version,
                upgrade_authority: None,
            },
        });
        let encoded = packet.encode();
        Self::new().with_svm_payload(encoded)
    }

    /// Create a new SVM-only Comit builder.
    pub fn svm(payload: impl AsRef<[u8]>) -> Self {
        Self::new().with_svm_payload(payload)
    }

    /// Create a new dual-VM Comit builder.
    pub fn dual(evm_payload: impl AsRef<[u8]>, svm_payload: impl AsRef<[u8]>) -> Self {
        Self::new()
            .with_evm_payload(evm_payload)
            .with_svm_payload(svm_payload)
    }

    /// Set the EVM payload.
    pub fn with_evm_payload(mut self, payload: impl AsRef<[u8]>) -> Self {
        self.evm_payload = Some(payload.as_ref().to_vec());
        self
    }

    /// Set the SVM payload.
    pub fn with_svm_payload(mut self, payload: impl AsRef<[u8]>) -> Self {
        self.svm_payload = Some(payload.as_ref().to_vec());
        self
    }

    /// Set the nonce.
    pub fn with_nonce(mut self, nonce: Nonce) -> Self {
        self.nonce = Some(nonce);
        self
    }

    /// Set the EVM gas limit.
    pub fn with_evm_gas_limit(mut self, limit: Gas) -> Self {
        self.evm_gas_limit = Some(limit);
        self
    }

    /// Set the SVM compute unit limit.
    pub fn with_svm_compute_limit(mut self, limit: Gas) -> Self {
        self.svm_compute_limit = Some(limit);
        self
    }

    /// Set the fee explicitly.
    pub fn with_fee(mut self, fee: Balance) -> Self {
        self.fee = Some(fee);
        self.auto_fee = false;
        self
    }

    /// Enable automatic fee calculation.
    pub fn with_auto_fee(mut self) -> Self {
        self.auto_fee = true;
        self.fee = None;
        self
    }

    /// Set the deadline block number.
    pub fn with_deadline(mut self, deadline: BlockNumber) -> Self {
        self.deadline = Some(deadline);
        self
    }

    /// Build the Comit payload.
    pub fn build(self) -> Result<ComitPayload> {
        // Validate payloads
        self.validate_payloads()?;

        // Calculate prepare root (before moving payloads)
        let prepare_root =
            compute_prepare_root(self.evm_payload.as_deref(), self.svm_payload.as_deref());

        // Determine gas limits
        let evm_gas_limit = self.evm_gas_limit.unwrap_or_else(|| {
            if self.evm_payload.is_some() {
                DEFAULT_EVM_GAS_LIMIT
            } else {
                0
            }
        });

        let svm_compute_limit = self.svm_compute_limit.unwrap_or_else(|| {
            if self.svm_payload.is_some() {
                DEFAULT_SVM_COMPUTE_LIMIT
            } else {
                0
            }
        });

        // Calculate fee (before moving payloads)
        let fee = if self.auto_fee {
            self.calculate_auto_fee(
                &self.evm_payload,
                &self.svm_payload,
                evm_gas_limit,
                svm_compute_limit,
            )
        } else {
            self.fee.ok_or_else(|| {
                AtlasError::InvalidPayload("Fee must be set or auto_fee enabled".to_string())
            })?
        };

        // Get nonce (default to 0 if not set - should be queried from chain)
        let nonce = self.nonce.unwrap_or(0);

        Ok(ComitPayload {
            evm_payload: self.evm_payload,
            svm_payload: self.svm_payload,
            nonce,
            prepare_root,
            evm_gas_limit,
            svm_compute_limit,
            fee,
            deadline: self.deadline.unwrap_or(0),
        })
    }

    /// Validate payload sizes.
    fn validate_payloads(&self) -> Result<()> {
        // Check EVM payload size
        if let Some(ref payload) = self.evm_payload {
            if payload.len() > MAX_EVM_PAYLOAD_SIZE {
                return Err(AtlasError::PayloadTooLarge(payload.len()));
            }
        }

        // Check SVM payload size
        if let Some(ref payload) = self.svm_payload {
            if payload.len() > MAX_SVM_PAYLOAD_SIZE {
                return Err(AtlasError::PayloadTooLarge(payload.len()));
            }
        }

        // Check combined size
        let total_size = self.evm_payload.as_ref().map(|p| p.len()).unwrap_or(0)
            + self.svm_payload.as_ref().map(|p| p.len()).unwrap_or(0);

        if total_size > MAX_COMBINED_PAYLOAD_SIZE {
            return Err(AtlasError::PayloadTooLarge(total_size));
        }

        // Must have at least one payload
        if self.evm_payload.is_none() && self.svm_payload.is_none() {
            return Err(AtlasError::InvalidPayload(
                "At least one payload (EVM or SVM) is required".to_string(),
            ));
        }

        Ok(())
    }

    /// Calculate automatic fee based on payload sizes and gas limits.
    fn calculate_auto_fee(
        &self,
        evm_payload: &Option<Vec<u8>>,
        svm_payload: &Option<Vec<u8>>,
        evm_gas_limit: Gas,
        svm_compute_limit: Gas,
    ) -> Balance {
        let evm_size = evm_payload.as_ref().map(|p| p.len()).unwrap_or(0);
        let svm_size = svm_payload.as_ref().map(|p| p.len()).unwrap_or(0);

        // Base fee calculation:
        // - Per-byte cost for payload storage
        // - Gas cost for execution
        // - Base transaction fee
        let base_fee: Balance = 10_000; // Minimum transaction fee
        let byte_fee: Balance = (evm_size + svm_size) as Balance * FEE_PER_BYTE;
        let gas_fee: Balance =
            (evm_gas_limit as Balance * 100) + (svm_compute_limit as Balance * 50);

        base_fee + byte_fee + gas_fee
    }
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Create an EVM-only Comit transaction.
pub fn evm_comit(payload: impl AsRef<[u8]>) -> ComitBuilder {
    ComitBuilder::evm(payload)
}

/// Create an SVM-only Comit transaction.
///
/// **Deprecated for `submit_comit`**: Use [`ComitBuilder::svm_invoke`]
/// or [`ComitBuilder::svm_deploy`] for the packet-validated path.
/// This function remains for the legacy `submit_comit_raw` path.
#[deprecated(
    since = "0.2.0",
    note = "Use svm_invoke() or svm_deploy() for the packet-validated submit_comit path"
)]
pub fn svm_comit(payload: impl AsRef<[u8]>) -> ComitBuilder {
    ComitBuilder::svm(payload)
}

/// Create a dual-VM Comit transaction.
pub fn dual_comit(evm_payload: impl AsRef<[u8]>, svm_payload: impl AsRef<[u8]>) -> ComitBuilder {
    ComitBuilder::dual(evm_payload, svm_payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use codec::Decode;

    #[test]
    fn test_evm_only_comit() {
        let comit = ComitBuilder::evm([0x60, 0x80, 0x60, 0x40])
            .with_nonce(1)
            .with_fee(1_000_000)
            .build()
            .unwrap();

        assert!(comit.is_evm_only());
        assert!(!comit.is_svm_only());
        assert!(!comit.is_dual_vm());
        assert_eq!(comit.nonce, 1);
        assert_eq!(comit.fee, 1_000_000);
    }

    #[test]
    fn test_svm_only_comit() {
        let comit = ComitBuilder::svm([0x01, 0x02, 0x03])
            .with_nonce(2)
            .with_fee(500_000)
            .build()
            .unwrap();

        assert!(!comit.is_evm_only());
        assert!(comit.is_svm_only());
        assert!(!comit.is_dual_vm());
    }

    #[test]
    fn test_dual_vm_comit() {
        let comit = ComitBuilder::dual([0xaa, 0xbb], [0xcc, 0xdd])
            .with_nonce(3)
            .with_fee(2_000_000)
            .build()
            .unwrap();

        assert!(!comit.is_evm_only());
        assert!(!comit.is_svm_only());
        assert!(comit.is_dual_vm());
    }

    #[test]
    fn test_auto_fee() {
        let comit = ComitBuilder::evm([0u8; 100])
            .with_nonce(1)
            .with_auto_fee()
            .build()
            .unwrap();

        // Base (10_000) + bytes (100 * 1000) + gas (500_000 * 100) = ~50_110_000
        assert!(comit.fee > 0);
    }

    #[test]
    fn test_payload_size_validation() {
        // EVM payload too large
        let result = ComitBuilder::evm([0u8; MAX_EVM_PAYLOAD_SIZE + 1])
            .with_fee(1000)
            .build();
        assert!(result.is_err());

        // Combined payload too large (add 1 byte to make it over the limit)
        let result =
            ComitBuilder::dual([0u8; MAX_EVM_PAYLOAD_SIZE], [0u8; MAX_SVM_PAYLOAD_SIZE + 1])
                .with_fee(1000)
                .build();
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_payload_rejected() {
        let result = ComitBuilder::new().with_fee(1000).build();
        assert!(result.is_err());
    }

    /// The deploy path has to hand the chain the artifact, not a packet describing it.
    ///
    /// `evm_deploy` used to return `Packet::Evm(EvmPacket::Deploy { .. }).encode()`, whose
    /// first byte is the enum discriminant `0x00` — EVM `STOP`. The interpreter halted on
    /// byte 0 and, before X3-LANG-004, the kernel receipted that halt as `success: true`.
    #[test]
    fn evm_deploy_payload_is_init_code_not_a_packet() {
        // PUSH1 0x00; PUSH1 0x00; RETURN — the smallest contract that deploys.
        let init_code = vec![0x60u8, 0x00, 0x60, 0x00, 0xf3];

        let comit = ComitBuilder::evm_deploy(init_code.clone())
            .with_fee(1_000_000)
            .build()
            .unwrap();
        let payload = comit.evm_payload.expect("evm_deploy sets the evm payload");

        assert_eq!(
            payload, init_code,
            "the payload has to be the init code itself"
        );
        assert_eq!(
            payload[0], 0x60,
            "a packet would start with the enum discriminant 0x00 (EVM STOP)"
        );
        assert!(
            Packet::decode(&mut &payload[..]).is_err(),
            "the payload must not be a valid SCALE-encoded Packet: every adapter refuses one by name"
        );
    }

    #[test]
    fn test_convenience_functions() {
        let _evm = evm_comit([0x00]).with_fee(1000).build().unwrap();
        let _svm = svm_comit([0x00]).with_fee(1000).build().unwrap();
        let _dual = dual_comit([0x00], [0x01]).with_fee(1000).build().unwrap();
    }
}
