use crate::routing::SwapRoute;
use crate::SwapRouterError;
use crate::{SwapParams, VmType};
use sp_core::U256;

pub struct FeeCalculator;
pub struct FeeStructure;

impl FeeStructure {
    pub const BPS_DENOMINATOR: u64 = 10_000;
    pub const CROSS_VM_TRADE_BPS: u64 = 400; // 4%
    pub const SAME_VM_CROSS_CHAIN_BPS: u64 = 200; // 2%
    pub const SAME_CHAIN_BPS: u64 = 0;
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub struct ProtocolFees {
    pub protocol_fee: U256,
    pub gas_fee: U256,
    pub total_fee: U256,
}

impl FeeCalculator {
    pub fn new() -> Result<Self, SwapRouterError> {
        Ok(Self)
    }

    pub async fn calculate_swap_fees(
        &self,
        route: &SwapRoute,
        params: &SwapParams,
    ) -> Result<ProtocolFees, SwapRouterError> {
        let source_vm = if params.source_vm == VmType::Unknown {
            Self::infer_vm_from_chain(params.chain_in)
        } else {
            params.source_vm
        };
        let destination_vm = if params.destination_vm == VmType::Unknown {
            Self::infer_vm_from_chain(params.chain_out)
        } else {
            params.destination_vm
        };

        let protocol_bps = if source_vm != VmType::Unknown
            && destination_vm != VmType::Unknown
            && source_vm != destination_vm
        {
            FeeStructure::CROSS_VM_TRADE_BPS
        } else if params.chain_in != params.chain_out {
            FeeStructure::SAME_VM_CROSS_CHAIN_BPS
        } else {
            FeeStructure::SAME_CHAIN_BPS
        };

        let protocol_fee = route.estimated_output * U256::from(protocol_bps)
            / U256::from(FeeStructure::BPS_DENOMINATOR);
        let gas_fee = route.gas_estimate;
        let total_fee = protocol_fee + gas_fee;
        Ok(ProtocolFees {
            protocol_fee,
            gas_fee,
            total_fee,
        })
    }

    fn infer_vm_from_chain(chain_id: u64) -> VmType {
        match chain_id {
            42 => VmType::X3Vm,
            1 | 10 | 56 | 137 | 324 | 8453 | 42161 | 43114 => VmType::Evm,
            101 | 102 | 103 | 1399811149 => VmType::Svm,
            _ => VmType::Unknown,
        }
    }
}
