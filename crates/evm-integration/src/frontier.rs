// Frontier EVM Executor for X3 Chain
// Provides real EVM execution using Frontier's pallet-evm

use crate::{
    EvmConfig, EvmError, EvmExecutionResult, EvmExecutor, EvmLog, EvmResult, EvmStateChange,
};
use sp_core::{H160, U256};
use sp_runtime::traits::UniqueSaturatedInto;
use sp_std::vec::Vec;

use fp_evm::{ExitReason, Log};
use pallet_evm::{Config as EvmPalletConfig, Runner};

/// Frontier-based EVM executor
/// Uses pallet-evm's Runner trait for actual bytecode execution
pub struct FrontierEvmExecutor<T: EvmPalletConfig> {
    _phantom: core::marker::PhantomData<T>,
}

impl<T: EvmPalletConfig> FrontierEvmExecutor<T> {
    /// Create a new Frontier EVM executor
    pub fn new() -> Self {
        Self {
            _phantom: core::marker::PhantomData,
        }
    }
}

impl<T: EvmPalletConfig> Default for FrontierEvmExecutor<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert Frontier log to X3 EVM log
fn convert_log(log: Log) -> EvmLog {
    EvmLog {
        address: log.address,
        topics: log.topics,
        data: log.data,
    }
}

/// Convert exit reason to EvmError
fn exit_reason_to_error(reason: &ExitReason, gas_used: u64) -> EvmError {
    match reason {
        ExitReason::Succeed(_) => {
            // Should not be called for success, but never panic in production.
            EvmError::ExecutionFailed(0)
        }
        ExitReason::Error(e) => match e {
            fp_evm::ExitError::StackOverflow => EvmError::StackOverflow,
            fp_evm::ExitError::StackUnderflow => EvmError::StackUnderflow,
            fp_evm::ExitError::OutOfGas => EvmError::OutOfGas,
            fp_evm::ExitError::InvalidCode(op) => EvmError::InvalidOpcode(op.as_u8()),
            fp_evm::ExitError::CreateCollision => EvmError::CreateCollision,
            _ => EvmError::ExecutionFailed(gas_used as u32),
        },
        ExitReason::Revert(_) => EvmError::ExecutionReverted,
        ExitReason::Fatal(_) => EvmError::ExecutionFailed(0x10 | (gas_used as u32 & 0x0FFFFFFF)),
    }
}

impl<T: EvmPalletConfig> EvmExecutor for FrontierEvmExecutor<T>
where
    T::Runner: Runner<T>,
{
    fn execute(
        &self,
        payload: &[u8],
        caller: H160,
        target: Option<H160>,
        value: U256,
        config: &EvmConfig,
    ) -> EvmResult<EvmExecutionResult> {
        if payload.is_empty() && target.is_none() {
            return Err(EvmError::InvalidPayload);
        }

        let gas_limit = config.gas_limit;

        // Execute via Frontier runner - use real Runner call/create
        let evm_config = config.into_evm_config::<T>();

        let (exit_reason, return_value, gas_used, logs) = match target {
            Some(to) => {
                // Contract call
                let call_info = T::Runner::call(
                    caller,
                    to,
                    payload.to_vec(),
                    value,
                    gas_limit,
                    Some(config.gas_price),
                    None,       // max_priority_fee_per_gas
                    None,       // nonce
                    Vec::new(), // access_list
                    Vec::new(), // authorization_list
                    false,      // is_transactional
                    false,      // validate
                    None,       // weight_limit
                    None,       // proof_size_base_cost
                    &evm_config,
                )
                .map_err(|_| EvmError::ExecutionFailed(0))?;

                (
                    call_info.exit_reason,
                    call_info.value,
                    call_info.used_gas.standard.unique_saturated_into(),
                    call_info.logs,
                )
            }
            None => {
                // Contract creation
                let create_info = T::Runner::create(
                    caller,
                    payload.to_vec(),
                    value,
                    gas_limit,
                    Some(config.gas_price),
                    None,
                    None,
                    Vec::new(),
                    Vec::new(), // authorization_list
                    false,
                    false,
                    None,
                    None,
                    &evm_config,
                )
                .map_err(|_| EvmError::ExecutionFailed(0))?;

                // create returns the deployed contract address, convert to bytes
                (
                    create_info.exit_reason,
                    create_info.value.as_bytes().to_vec(),
                    create_info.used_gas.standard.unique_saturated_into(),
                    create_info.logs,
                )
            }
        };

        // Convert result
        let success = matches!(exit_reason, ExitReason::Succeed(_));

        if !success {
            return Err(exit_reason_to_error(&exit_reason, gas_used));
        }

        // Track a canonical fee debit on the caller account so upper layers can
        // persist balance-oriented state changes in the CanonicalLedger.
        let fee_paid = U256::from(gas_used).saturating_mul(config.gas_price);
        let fee_paid_u128: u128 = fee_paid.unique_saturated_into();

        // A fee above i128::MAX (~170 quintillion base units) indicates a wildly
        // misconfigured gas_price. Rather than silently clamping the debit (which
        // would corrupt the caller's balance record), we abort the execution so
        // the block producer can reject the transaction.
        if fee_paid_u128 > i128::MAX as u128 {
            return Err(EvmError::ExecutionFailed(0xFE));
        }
        // Safe: fee_paid_u128 <= i128::MAX after the guard above.
        let fee_delta = -(fee_paid_u128 as i128);

        let state_changes = vec![EvmStateChange {
            address: caller,
            balance_delta: fee_delta,
            nonce_delta: 1,
            storage_changes: Vec::new(),
            code: None,
        }];

        Ok(EvmExecutionResult {
            success: true,
            output: return_value,
            gas_used,
            logs: logs.into_iter().map(convert_log).collect(),
            state_changes: state_changes.clone(),
            // Use the full substrate storage root so that all EVM contract
            // storage mutations (applied directly by the Frontier runner into
            // pallet-evm's storage) are captured in the commitment.  The
            // compute_state_root() helper only covers the caller's balance/nonce
            // delta and would produce the same root for executions with
            // different contract storage outcomes but identical fees.
            state_root: {
                let root_vec = sp_io::storage::root(sp_runtime::StateVersion::V1);
                let mut root = [0u8; 32];
                let copy_len = root_vec.len().min(32);
                root[..copy_len].copy_from_slice(&root_vec[..copy_len]);
                root
            },
        })
    }

    fn call(
        &self,
        payload: &[u8],
        caller: H160,
        target: H160,
        value: U256,
        config: &EvmConfig,
    ) -> EvmResult<EvmExecutionResult> {
        // For calls, we just execute without state mutation tracking
        self.execute(payload, caller, Some(target), value, config)
    }

    fn validate_bytecode(&self, payload: &[u8]) -> EvmResult<()> {
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

    fn estimate_gas(
        &self,
        payload: &[u8],
        caller: H160,
        target: Option<H160>,
        value: U256,
        config: &EvmConfig,
    ) -> EvmResult<u64> {
        // Compute intrinsic gas floor per EIP-2028
        let calldata_gas: u64 = payload
            .iter()
            .map(|&b| if b == 0 { 4u64 } else { 16u64 })
            .sum();
        let is_create = target.is_none();
        let base_intrinsic = if is_create { 53_000u64 } else { 21_000u64 };
        let intrinsic_floor = base_intrinsic.saturating_add(calldata_gas);

        // Run execution with generous gas ceiling to measure actual consumption.
        // Use the configured gas_limit, but cap at 30M (Ethereum mainnet block
        // gas limit) to prevent excessive execution during estimation.
        let estimation_limit = config.gas_limit.min(30_000_000);
        let estimation_config = EvmConfig {
            gas_limit: estimation_limit,
            ..config.clone()
        };

        match self.execute(payload, caller, target, value, &estimation_config) {
            Ok(result) => {
                // Take the larger of the actual gas used and the intrinsic gas floor,
                // then add a 10% safety buffer for state-dependent opcode variance.
                let measured = result.gas_used.max(intrinsic_floor);
                Ok(measured.saturating_mul(11) / 10)
            }
            Err(EvmError::OutOfGas) => {
                // If even estimation_limit was insufficient, return the limit
                // so the caller knows the tx is likely too expensive.
                Ok(estimation_limit)
            }
            Err(e) => Err(e),
        }
    }
}

/// Compute state root from state changes.
///
/// Commits every address, balance delta, nonce delta, and storage mutation
/// into a single blake2-256 hash so validators reach the same root for
/// identical execution outcomes.
#[allow(dead_code)]
fn compute_state_root(changes: &[EvmStateChange]) -> [u8; 32] {
    use sp_io::hashing::blake2_256;

    if changes.is_empty() {
        return [0u8; 32];
    }

    // Domain separator prevents collisions with other hash uses.
    let mut data = Vec::new();
    data.extend_from_slice(b"x3-evm-frontier-state-root-v1");
    for change in changes {
        data.extend_from_slice(change.address.as_bytes());
        data.extend_from_slice(&change.balance_delta.to_le_bytes());
        data.extend_from_slice(&change.nonce_delta.to_le_bytes());
        for (key, val) in &change.storage_changes {
            data.extend_from_slice(key.as_bytes());
            data.extend_from_slice(val.as_bytes());
        }
        if let Some(ref code) = change.code {
            data.extend_from_slice(code);
        }
    }

    blake2_256(&data)
}

/// Extension trait to convert EvmConfig to Frontier config
impl EvmConfig {
    /// Convert to Frontier's fp_evm::Config type.
    ///
    /// # M-8 Security Audit Note (Config Conversion)
    ///
    /// `fp_evm::Config` is a struct with predetermined EVM opcode costs and
    /// behavioral flags for different Ethereum hard forks. The following
    /// fields from `EvmConfig` are handled as follows:
    ///
    /// ## Fields Used Directly by Runner (not in fp_evm::Config)
    /// - `gas_limit`: Passed to `Runner::call()` / `Runner::create()` directly
    /// - `gas_price`: Passed to Runner for fee calculation
    /// - `chain_id`: Used via `ChainId` pallet config, not fp_evm::Config
    /// - `block_number`: From `frame_system::Pallet::<T>::block_number()`
    /// - `block_timestamp`: From `pallet_timestamp`
    /// - `base_fee`: From `pallet_base_fee` or runtime config
    /// - `coinbase`: From runtime's `FindAuthor` implementation
    ///
    /// ## Why Shanghai Preset
    /// Shanghai enables EIP-3855 (PUSH0), EIP-3860 (initcode limit), and
    /// other modern EVM features. The preset defines opcode gas costs.
    ///
    /// ## Full Customization
    /// For complete control, configure `pallet-evm` in runtime with:
    /// - `type ChainId = ChainIdConstant;`
    /// - `type BlockGasLimit = BlockGasLimit;`
    /// - `type FeeCalculator = FeeCalculator;`
    pub fn into_evm_config<T: EvmPalletConfig>(&self) -> fp_evm::Config {
        // Use Shanghai preset which includes all modern EVM features.
        // Gas limits and chain-specific params are passed separately to Runner.
        fp_evm::Config::shanghai()
    }

    /// Get chain_id from config (used for transaction signing)
    pub fn chain_id(&self) -> u64 {
        self.chain_id
    }

    /// Get gas_limit (passed directly to Runner methods)
    pub fn gas_limit(&self) -> u64 {
        self.gas_limit
    }

    /// Get gas_price (used for fee calculation)
    pub fn gas_price(&self) -> U256 {
        self.gas_price
    }

    /// Get base_fee for EIP-1559 transactions
    pub fn base_fee(&self) -> U256 {
        self.base_fee
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exit_reason_conversion() {
        let err = exit_reason_to_error(&ExitReason::Revert(fp_evm::ExitRevert::Reverted), 1000);
        assert_eq!(err, EvmError::ExecutionReverted);
    }

    #[test]
    fn test_state_root_computation() {
        let empty_root = compute_state_root(&[]);
        assert_eq!(empty_root, [0u8; 32]);

        let change = EvmStateChange {
            address: H160::zero(),
            balance_delta: 100,
            nonce_delta: 1,
            storage_changes: vec![],
            code: None,
        };
        let root = compute_state_root(&[change]);
        assert_ne!(root, [0u8; 32]);
    }
}
