//! X3-native trading against authenticated capabilities and the canonical ledger.
//!
//! The initial venue profile is a zero-fee constant-product pool. Lenders explicitly
//! opt into zero-fee, same-dispatch loans. No remote RPC, external venue, oracle,
//! bridge, or protected ordering capability is claimed by this host.
//! A real storage transaction previews the route and is rolled back. Its affected
//! ledger commitment is checked again before committing the real execution.
use crate::{
    CanonicalLedger, Config, ExecutionReceipt, NativeTradingAssets, NativeTradingFeeAsset,
    NativeTradingProviders, NativeTradingVenues,
};
use alloc::{
    collections::{BTreeMap, BTreeSet},
    string::String,
    vec::Vec,
};
use frame_support::{
    storage::{with_transaction, TransactionOutcome},
    traits::Get,
};
use parity_scale_codec::Encode;
use sp_core::H256;
use sp_runtime::{traits::CheckedConversion, DispatchError};
use x3_lang_execution::{
    artifact::{decode_artifact, TradingArtifact},
    ir::*,
    trading::*,
};

// Keep the concrete route tuple named so the host's type contract is readable.
type NativeRoute<T> = (<T as frame_system::Config>::AccountId, <T as Config>::AssetId, <T as Config>::AssetId);

fn error(message: &str) -> HostError {
    HostError {
        code: "X3_NATIVE_TRADING_REJECTED".into(),
        message: message.into(),
    }
}

pub(crate) fn asset_hash(asset: &AssetKey) -> Result<H256, DispatchError> {
    let bytes =
        serde_json::to_vec(asset).map_err(|_| DispatchError::Other("invalid trading identity"))?;
    Ok(H256::from(sp_io::hashing::blake2_256(&bytes)))
}

pub(crate) fn decode_asset(bytes: &[u8]) -> Result<AssetKey, DispatchError> {
    // Identity registration is governance-only and bounded before JSON allocation.
    if bytes.len() > 512 {
        return Err(DispatchError::Other("trading identity too large"));
    }
    let asset: AssetKey = serde_json::from_slice(bytes)
        .map_err(|_| DispatchError::Other("invalid trading identity"))?;
    if asset.vm_family != "x3vm"
        || asset.chain != "x3"
        || asset.canonical_id.is_empty()
        || asset.symbol.is_empty()
    {
        return Err(DispatchError::Other(
            "only X3-native trading identities are supported",
        ));
    }
    Ok(asset)
}

pub(crate) fn capability_name(
    name: Vec<u8>,
) -> Result<frame_support::BoundedVec<u8, frame_support::traits::ConstU32<64>>, DispatchError> {
    if name.is_empty()
        || !name
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || *b == b'_' || *b == b'-')
    {
        return Err(DispatchError::Other(
            "invalid native trading capability name",
        ));
    }
    name.try_into()
        .map_err(|_| DispatchError::Other("native trading name too long"))
}

/// Native execution is atomic even when invoked directly by another kernel flow.
pub(crate) fn execute<T: Config>(
    origin: &T::AccountId,
    bytes: &[u8],
    gas_limit: u64,
    charged_fee: u128,
) -> Result<ExecutionReceipt, DispatchError> {
    with_transaction(
        || match execute_inner::<T>(origin, bytes, gas_limit, charged_fee) {
            Ok(receipt) => TransactionOutcome::Commit(Ok(receipt)),
            Err(e) => TransactionOutcome::Rollback(Err(e)),
        },
    )
}

fn execute_inner<T: Config>(
    origin: &T::AccountId,
    bytes: &[u8],
    gas_limit: u64,
    charged_fee: u128,
) -> Result<ExecutionReceipt, DispatchError> {
    let artifact = decode_artifact(bytes)
        .map_err(|_| DispatchError::Other("invalid canonical trading artifact"))?;
    if artifact.gas_used > gas_limit {
        return Err(DispatchError::Other("canonical trading gas exhausted"));
    }
    let mut host = NativeHost::<T>::new(origin.clone(), &artifact, charged_fee)?;
    let context = TradeExecutionContext {
        mode: ExecutionMode::Production,
        current_block: host.block,
    };
    // Preview uses the same interpreter and real ledger in a rollback-only storage layer.
    let mut vm = TradingVm::new();
    vm.simulate_atomic(&artifact.operations, &mut host, context)
        .map_err(|_| DispatchError::Other("native trading preview rejected"))?;
    host.manifest.state_commitment = host
        .projected
        .ok_or(DispatchError::Other("missing native state projection"))?;
    let execution = vm
        .execute_atomic(&artifact.operations, &mut host, context)
        .map_err(|_| DispatchError::Other("native trading execution rejected"))?;
    if host.touched.len() > crate::Pallet::<T>::MAX_STATE_CHANGES {
        return Err(DispatchError::Other("too many native state changes"));
    }
    let mut changes = Vec::new();
    for (account, asset) in &host.touched {
        let balance = CanonicalLedger::<T>::get(account, asset);
        if asset.encode().len() > 32 || balance.encode().len() > 32 {
            return Err(DispatchError::Other(
                "native state change encoding too wide",
            ));
        }
        changes.push(crate::Pallet::<T>::build_balance_state_change(
            &account.encode(),
            *asset,
            balance,
        ));
    }
    let deltas: Vec<_> = execution.committed_state.net_deltas.iter().collect();
    // This is execution evidence; block inclusion/finality remains the chain's responsibility.
    let evidence = serde_json::to_vec(&(
        "X3:NATIVE_TRADING_EXECUTION:V1",
        sp_io::hashing::sha2_256(bytes),
        host.manifest.state_commitment,
        &artifact.operations,
        deltas,
        &execution.committed_state.cost_ledger,
    ))
    .map_err(|_| DispatchError::Other("native trading receipt encoding failed"))?;
    Ok(ExecutionReceipt {
        version: crate::EXECUTION_RECEIPT_VERSION,
        success: true,
        gas_used: artifact.gas_used,
        return_data: evidence,
        protocol_version: 2,
        logs: Vec::new(),
        state_changes: changes,
        storage_writes: Vec::new(),
        migration_history: Vec::new(),
        compatibility_flags: 0,
        from: Vec::new(),
        to: Vec::new(),
        value: 0,
    })
}

struct NativeHost<T: Config> {
    origin: T::AccountId,
    manifest: CapabilityManifest,
    block: u64,
    active: bool,
    debts: BTreeMap<String, (T::AccountId, AssetKey, u128)>,
    touched: BTreeSet<(T::AccountId, T::AssetId)>,
    projected: Option<[u8; 32]>,
    costs: Vec<CommittedCost>,
    last_impact: Option<(QuoteRequest, MeasuredRisk)>,
}

impl<T: Config> NativeHost<T> {
    fn new(
        origin: T::AccountId,
        artifact: &TradingArtifact,
        fee: u128,
    ) -> Result<Self, DispatchError> {
        let Some(TradingOperation::BeginAtomicTrade { policy, .. }) = artifact.operations.first()
        else {
            return Err(DispatchError::Other("missing native trading policy"));
        };
        // A per-dispatch VM cannot honestly enforce a strategy's persistent loss ceiling.
        // Exact/host state binding is enforced against the affected canonical ledger.
        if policy.chain != "x3"
            || policy.max_cumulative_loss.is_some()
            || policy.submission_profile == SubmissionProfile::Protected
        {
            return Err(DispatchError::Other("unsupported native trading policy"));
        }
        let fee_identity = asset_hash(&policy.max_gas_asset)?;
        if NativeTradingFeeAsset::<T>::get() != Some(fee_identity) {
            return Err(DispatchError::Other("native trading fee asset mismatch"));
        }
        let block: u64 = frame_system::Pallet::<T>::block_number()
            .checked_into()
            .ok_or(DispatchError::Other("block number overflow"))?;
        let mut host = Self {
            origin,
            manifest: CapabilityManifest {
                mode: CapabilityMode::Production,
                version: "trading-policy-v1".into(),
                chain: "x3".into(),
                state_commitment: [0; 32],
                private_submission: T::PrivateSubmissionChannel::get(),
                providers: BTreeSet::new(),
                venues: BTreeSet::new(),
                bridges: BTreeSet::new(),
            },
            block,
            active: false,
            debts: BTreeMap::new(),
            touched: BTreeSet::new(),
            projected: None,
            costs: if fee == 0 {
                Vec::new()
            } else {
                alloc::vec![CommittedCost {
                    asset: policy.max_gas_asset.clone(),
                    amount: fee,
                    kind: "gas".into()
                }]
            },
            last_impact: None,
        };
        host.asset_id(&policy.max_gas_asset)
            .map_err(|_| DispatchError::Other("unknown native gas asset"))?;
        for op in &artifact.operations {
            match op {
                TradingOperation::OpenDebt {
                    provider, asset, ..
                } => {
                    let name = capability_name(provider.as_bytes().to_vec())?;
                    let account = NativeTradingProviders::<T>::get(&name)
                        .ok_or(DispatchError::Other("unknown native lender"))?;
                    if account == host.origin {
                        return Err(DispatchError::Other("self lending is not supported"));
                    }
                    host.asset_id(asset)
                        .map_err(|_| DispatchError::Other("unknown native borrow asset"))?;
                    host.manifest.providers.insert(provider.clone());
                }
                TradingOperation::ExecuteSwap {
                    venue, from, to, ..
                } => {
                    host.route(venue, from, to)
                        .map_err(|_| DispatchError::Other("invalid native pool route"))?;
                    host.manifest.venues.insert(venue.clone());
                }
                TradingOperation::Bridge { .. } => {
                    return Err(DispatchError::Other("native host has no bridge capability"))
                }
                TradingOperation::AssertMinNetProfit {
                    settlement_asset, ..
                } => {
                    host.asset_id(settlement_asset)
                        .map_err(|_| DispatchError::Other("unknown native settlement asset"))?;
                }
                _ => {}
            }
        }
        Ok(host)
    }
    fn require_transaction(&self) -> Result<(), HostError> {
        if self.active {
            Ok(())
        } else {
            Err(error("native host transaction is not active"))
        }
    }
    fn asset_id(&self, asset: &AssetKey) -> Result<T::AssetId, HostError> {
        if asset.vm_family != "x3vm" || asset.chain != "x3" {
            return Err(error("foreign asset refused"));
        }
        let hash = asset_hash(asset).map_err(|_| error("invalid asset identity"))?;
        NativeTradingAssets::<T>::get(hash).ok_or_else(|| error("unregistered asset identity"))
    }
    fn route(
        &self,
        venue: &str,
        from: &AssetKey,
        to: &AssetKey,
    ) -> Result<NativeRoute<T>, HostError> {
        let name =
            capability_name(venue.as_bytes().to_vec()).map_err(|_| error("invalid venue name"))?;
        let (account, a, b) =
            NativeTradingVenues::<T>::get(name).ok_or_else(|| error("unknown venue"))?;
        let from = self.asset_id(from)?;
        let to = self.asset_id(to)?;
        if account == self.origin || !((from == a && to == b) || (from == b && to == a)) {
            return Err(error("venue account or asset pair mismatch"));
        }
        Ok((account, from, to))
    }
    fn balance(account: &T::AccountId, asset: T::AssetId) -> Result<u128, HostError> {
        Ok(CanonicalLedger::<T>::get(account, asset).into())
    }
    fn transfer(
        &mut self,
        from: &T::AccountId,
        to: &T::AccountId,
        asset: T::AssetId,
        amount: u128,
    ) -> Result<(), HostError> {
        self.require_transaction()?;
        if from == to || amount == 0 {
            return Err(error("invalid native asset transfer"));
        }
        let old_from = Self::balance(from, asset)?;
        let old_to = Self::balance(to, asset)?;
        let new_from = old_from
            .checked_sub(amount)
            .ok_or_else(|| error("insufficient native balance"))?;
        let new_to = old_to
            .checked_add(amount)
            .ok_or_else(|| error("native balance overflow"))?;
        let new_from = T::Balance::try_from(new_from)
            .map_err(|_| error("native balance conversion overflow"))?;
        let new_to = T::Balance::try_from(new_to)
            .map_err(|_| error("native balance conversion overflow"))?;
        CanonicalLedger::<T>::insert(from, asset, new_from);
        CanonicalLedger::<T>::insert(to, asset, new_to);
        self.touched.insert((from.clone(), asset));
        self.touched.insert((to.clone(), asset));
        Ok(())
    }
    fn ledger_commitment(&self) -> [u8; 32] {
        let ledger: Vec<_> = self
            .touched
            .iter()
            .map(|(account, asset)| (account, asset, CanonicalLedger::<T>::get(account, asset)))
            .collect();
        sp_io::hashing::blake2_256(&("X3:NATIVE_LEDGER:V1", ledger).encode())
    }
    fn quote_output(&self, request: &QuoteRequest) -> Result<(u128, u128, u128), HostError> {
        self.require_transaction()?;
        let (account, from, to) = self.route(&request.venue, &request.from, &request.to)?;
        let reserve_in = Self::balance(&account, from)?;
        let reserve_out = Self::balance(&account, to)?;
        let output = x3_dex::amm_pools::AMMPool::constant_product_output(
            request.input,
            reserve_in,
            reserve_out,
            0,
        )
        .map_err(error)?;
        if request.input == 0 || reserve_in == 0 || reserve_out == 0 || output == 0 {
            return Err(error("insufficient native liquidity"));
        }
        Ok((output, reserve_in, reserve_out))
    }
}

impl<T: Config> TradingHost for NativeHost<T> {
    fn capabilities(&self) -> &CapabilityManifest {
        &self.manifest
    }
    fn begin_transaction(&mut self) -> Result<(), HostError> {
        if self.active {
            return Err(error("duplicate native transaction"));
        }
        self.debts.clear();
        self.touched.clear();
        self.last_impact = None;
        sp_io::storage::start_transaction();
        self.active = true;
        Ok(())
    }
    fn commit_transaction(&mut self) -> Result<(), HostError> {
        self.require_transaction()?;
        if !self.debts.is_empty() {
            return Err(error("native debt remains open"));
        }
        if self.projected != Some(self.ledger_commitment()) {
            return Err(error("native projected state mismatch"));
        }
        sp_io::storage::commit_transaction();
        self.active = false;
        Ok(())
    }
    fn rollback_transaction(&mut self) -> Result<(), HostError> {
        self.require_transaction()?;
        self.projected = Some(self.ledger_commitment());
        sp_io::storage::rollback_transaction();
        self.active = false;
        self.debts.clear();
        Ok(())
    }
    fn open_debt(&mut self, request: BorrowRequest) -> Result<BorrowResult, HostError> {
        self.require_transaction()?;
        if self.debts.contains_key(&request.debt_id) {
            return Err(error("duplicate native debt"));
        }
        let name = capability_name(request.provider.as_bytes().to_vec())
            .map_err(|_| error("invalid lender name"))?;
        let lender =
            NativeTradingProviders::<T>::get(name).ok_or_else(|| error("unknown native lender"))?;
        let asset = self.asset_id(&request.asset)?;
        self.transfer(&lender, &self.origin.clone(), asset, request.principal)?;
        self.debts.insert(
            request.debt_id,
            (lender, request.asset.clone(), request.principal),
        );
        Ok(BorrowResult {
            asset: request.asset,
            principal: request.principal,
            fee: 0,
            state_commitment: self.manifest.state_commitment,
        })
    }
    fn quote(&self, request: QuoteRequest) -> Result<QuoteResult, HostError> {
        let (output, _, _) = self.quote_output(&request)?;
        Ok(QuoteResult {
            expected_output: output,
            quote_block: self.block,
            sources: Vec::new(),
        })
    }
    fn swap(&mut self, request: SwapRequest) -> Result<SwapResult, HostError> {
        let quote = QuoteRequest {
            venue: request.venue.clone(),
            from: request.from.clone(),
            to: request.to.clone(),
            input: request.input,
        };
        let (output, reserve_in, _) = self.quote_output(&quote)?;
        if output < request.min_output {
            return Err(error("native swap below min output"));
        }
        let (pool, from, to) = self.route(&request.venue, &request.from, &request.to)?;
        let denominator = reserve_in
            .checked_add(request.input)
            .ok_or_else(|| error("reserve overflow"))?;
        let impact = sp_core::U256::from(request.input) * sp_core::U256::from(10_000u128)
            / sp_core::U256::from(denominator);
        self.last_impact = Some((
            quote,
            MeasuredRisk {
                source: request.venue.clone(),
                bps: impact.low_u128(),
            },
        ));
        self.transfer(&self.origin.clone(), &pool, from, request.input)?;
        self.transfer(&pool, &self.origin.clone(), to, output)?;
        Ok(SwapResult {
            from: request.from,
            to: request.to.clone(),
            input: request.input,
            output,
            fee: 0,
            fee_asset: request.to,
            state_commitment: self.manifest.state_commitment,
        })
    }
    fn price_impact(&self, request: &QuoteRequest) -> Result<Option<MeasuredRisk>, HostError> {
        Ok(self
            .last_impact
            .as_ref()
            .filter(|(quoted, _)| quoted == request)
            .map(|(_, risk)| risk.clone()))
    }
    fn close_debt(&mut self, request: RepayRequest) -> Result<RepayResult, HostError> {
        let (lender, asset, principal) = self
            .debts
            .get(&request.debt_id)
            .cloned()
            .ok_or_else(|| error("unknown native debt"))?;
        if request.asset != asset || request.amount != principal {
            return Err(error("native repayment binding mismatch"));
        }
        let id = self.asset_id(&asset)?;
        self.transfer(&self.origin.clone(), &lender, id, principal)?;
        self.debts.remove(&request.debt_id);
        Ok(RepayResult {
            debt_id: request.debt_id,
            asset,
            amount_paid: principal,
            fee: 0,
            state_commitment: self.manifest.state_commitment,
        })
    }
    fn execution_costs(&self) -> Result<Vec<CommittedCost>, HostError> {
        self.require_transaction()?;
        Ok(self.costs.clone())
    }
}
impl<T: Config> Drop for NativeHost<T> {
    fn drop(&mut self) {
        if self.active {
            sp_io::storage::rollback_transaction();
        }
    }
}
