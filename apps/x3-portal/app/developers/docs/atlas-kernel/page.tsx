'use client';

import DocLayout, { CodeBlock, Callout } from '@/components/docs/DocLayout';
import { Cpu, Shield, Database, Zap, ArrowRight } from 'lucide-react';

export default function AtlasKernelPage() {
  return (
    <DocLayout 
      title="Atlas Kernel" 
      description="The core orchestration layer for dual VM execution"
      lastUpdated="December 2024"
    >
      <div className="space-y-8">
        {/* Overview */}
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Overview</h2>
          <p className="text-gray-400 mb-4">
            The Atlas Kernel is a Substrate pallet that serves as the orchestration layer for X3 Atlas Sphere. 
            It manages cross-VM operations, maintains the canonical ledger, handles account authorization, 
            and ensures atomic execution of Comit transactions.
          </p>
          
          <div className="glass-card p-6 my-6">
            <h3 className="font-semibold text-white mb-4">Key Responsibilities</h3>
            <div className="grid md:grid-cols-2 gap-4">
              <div className="flex items-start space-x-3">
                <Zap className="w-5 h-5 text-orange-400 flex-shrink-0" />
                <div>
                  <span className="text-white font-medium">Comit Processing</span>
                  <p className="text-xs text-gray-500">Validate and execute cross-VM transactions</p>
                </div>
              </div>
              <div className="flex items-start space-x-3">
                <Database className="w-5 h-5 text-orange-400 flex-shrink-0" />
                <div>
                  <span className="text-white font-medium">Canonical Ledger</span>
                  <p className="text-xs text-gray-500">Unified asset storage and management</p>
                </div>
              </div>
              <div className="flex items-start space-x-3">
                <Shield className="w-5 h-5 text-orange-400 flex-shrink-0" />
                <div>
                  <span className="text-white font-medium">Authorization</span>
                  <p className="text-xs text-gray-500">Account permission management</p>
                </div>
              </div>
              <div className="flex items-start space-x-3">
                <Cpu className="w-5 h-5 text-orange-400 flex-shrink-0" />
                <div>
                  <span className="text-white font-medium">VM Orchestration</span>
                  <p className="text-xs text-gray-500">Coordinate EVM and SVM execution</p>
                </div>
              </div>
            </div>
          </div>
        </section>

        {/* Storage Items */}
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Storage</h2>
          <p className="text-gray-400 mb-4">
            The Atlas Kernel maintains several key storage items:
          </p>
          <CodeBlock language="rust" title="Storage Items">
{`// Canonical Ledger - maps (AccountId, AssetId) to Balance
#[pallet::storage]
pub type CanonicalLedger<T: Config> = StorageDoubleMap<
    _,
    Blake2_128Concat, T::AccountId,
    Blake2_128Concat, AssetId,
    Balance,
    ValueQuery,
>;

// Authorized Accounts - accounts permitted to submit Comits
#[pallet::storage]
pub type AuthorizedAccounts<T: Config> = StorageMap<
    _,
    Blake2_128Concat, T::AccountId,
    (),
    OptionQuery,
>;

// Account Nonces - for replay protection
#[pallet::storage]
pub type AccountNonces<T: Config> = StorageMap<
    _,
    Blake2_128Concat, T::AccountId,
    u64,
    ValueQuery,
>;

// Registered Assets - asset metadata
#[pallet::storage]
pub type RegisteredAssets<T: Config> = StorageMap<
    _,
    Blake2_128Concat, AssetId,
    AssetMetadata,
    OptionQuery,
>;`}
          </CodeBlock>
        </section>

        {/* Dispatchable Functions */}
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Dispatchable Functions</h2>
          
          <h3 className="text-lg font-semibold text-white mt-6 mb-3">submit_comit</h3>
          <p className="text-gray-400 mb-4">
            Submit a cross-VM transaction for atomic execution.
          </p>
          <CodeBlock language="rust" title="submit_comit">
{`#[pallet::call_index(0)]
#[pallet::weight(T::WeightInfo::submit_comit())]
pub fn submit_comit(
    origin: OriginFor<T>,
    comit: Comit<T::AccountId>,
) -> DispatchResultWithPostInfo {
    let who = ensure_signed(origin)?;
    
    // Check authorization
    ensure!(
        AuthorizedAccounts::<T>::contains_key(&who),
        Error::<T>::NotAuthorized
    );
    
    // Validate nonce
    let nonce = AccountNonces::<T>::get(&who);
    ensure!(comit.nonce == nonce, Error::<T>::InvalidNonce);
    
    // Execute on both VMs
    let evm_result = Self::execute_evm(&comit.evm_payload)?;
    let svm_result = Self::execute_svm(&comit.svm_payload)?;
    
    // Verify prepare_root
    Self::verify_prepare_root(&comit)?;
    
    // Update state
    AccountNonces::<T>::insert(&who, nonce + 1);
    Self::deposit_event(Event::ComitFinalized { 
        comit_id: comit.comit_id,
        evm_result,
        svm_result,
    });
    
    Ok(().into())
}`}
          </CodeBlock>

          <h3 className="text-lg font-semibold text-white mt-6 mb-3">authorize_account</h3>
          <p className="text-gray-400 mb-4">
            Authorize an account to submit Comit transactions (governance only).
          </p>
          <CodeBlock language="rust" title="authorize_account">
{`#[pallet::call_index(1)]
#[pallet::weight(T::WeightInfo::authorize_account())]
pub fn authorize_account(
    origin: OriginFor<T>,
    account: T::AccountId,
) -> DispatchResult {
    T::GovernanceOrigin::ensure_origin(origin)?;
    
    AuthorizedAccounts::<T>::insert(&account, ());
    Self::deposit_event(Event::AccountAuthorized { account });
    
    Ok(())
}`}
          </CodeBlock>

          <h3 className="text-lg font-semibold text-white mt-6 mb-3">update_canonical_balance</h3>
          <p className="text-gray-400 mb-4">
            Update an account&apos;s balance in the canonical ledger (governance only).
          </p>
          <CodeBlock language="rust" title="update_canonical_balance">
{`#[pallet::call_index(2)]
#[pallet::weight(T::WeightInfo::update_canonical_balance())]
pub fn update_canonical_balance(
    origin: OriginFor<T>,
    account: T::AccountId,
    asset_id: AssetId,
    balance: Balance,
) -> DispatchResult {
    T::GovernanceOrigin::ensure_origin(origin)?;
    
    CanonicalLedger::<T>::insert(&account, &asset_id, balance);
    Self::deposit_event(Event::BalanceUpdated { account, asset_id, balance });
    
    Ok(())
}`}
          </CodeBlock>
        </section>

        {/* RPC Methods */}
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">RPC Methods</h2>
          <p className="text-gray-400 mb-4">
            The Atlas Kernel exposes custom RPC methods for querying state:
          </p>
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="border-b border-[#1a1a1a]">
                  <th className="text-left py-3 px-4 text-gray-500 font-medium">Method</th>
                  <th className="text-left py-3 px-4 text-gray-500 font-medium">Description</th>
                </tr>
              </thead>
              <tbody>
                <tr className="border-b border-[#111111]">
                  <td className="py-3 px-4 text-orange-400 font-mono text-sm">atlasKernel_getCanonicalBalance</td>
                  <td className="py-3 px-4 text-gray-400">Get balance for (account, asset) pair</td>
                </tr>
                <tr className="border-b border-[#111111]">
                  <td className="py-3 px-4 text-orange-400 font-mono text-sm">atlasKernel_getAssetMetadata</td>
                  <td className="py-3 px-4 text-gray-400">Get asset metadata (symbol, decimals)</td>
                </tr>
                <tr className="border-b border-[#111111]">
                  <td className="py-3 px-4 text-orange-400 font-mono text-sm">atlasKernel_isAuthorized</td>
                  <td className="py-3 px-4 text-gray-400">Check if account is authorized</td>
                </tr>
                <tr className="border-b border-[#111111]">
                  <td className="py-3 px-4 text-orange-400 font-mono text-sm">atlasKernel_getAccountNonce</td>
                  <td className="py-3 px-4 text-gray-400">Get current nonce for account</td>
                </tr>
              </tbody>
            </table>
          </div>
          
          <CodeBlock language="typescript" title="RPC Example">
{`// Query canonical balance
const balance = await api.rpc.atlasKernel.getCanonicalBalance(
  accountId,
  'ATLAS' // Asset ID
);

// Check authorization
const isAuthorized = await api.rpc.atlasKernel.isAuthorized(accountId);

// Get account nonce
const nonce = await api.rpc.atlasKernel.getAccountNonce(accountId);`}
          </CodeBlock>
        </section>

        {/* Events */}
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Events</h2>
          <CodeBlock language="rust" title="Events">
{`#[pallet::event]
#[pallet::generate_deposit(pub(super) fn deposit_event)]
pub enum Event<T: Config> {
    /// A Comit was submitted
    ComitSubmitted { comit_id: H256, origin: T::AccountId },
    
    /// Comit execution started
    ComitExecutionStarted { comit_id: H256 },
    
    /// Comit execution completed
    ComitExecutionCompleted { comit_id: H256, success: bool },
    
    /// Comit finalized
    ComitFinalized { 
        comit_id: H256, 
        evm_result: ExecutionReceipt,
        svm_result: ExecutionReceipt,
    },
    
    /// Comit failed
    ComitFailed { comit_id: H256, reason: ComitFailureReason },
    
    /// Account authorized
    AccountAuthorized { account: T::AccountId },
    
    /// Account deauthorized  
    AccountDeauthorized { account: T::AccountId },
    
    /// Canonical balance updated
    BalanceUpdated { 
        account: T::AccountId, 
        asset_id: AssetId, 
        balance: Balance 
    },
}`}
          </CodeBlock>
        </section>

        <Callout type="info" title="Deep Dive">
          For implementation details, see the source code at 
          <code className="text-orange-400 ml-1">pallets/atlas-kernel/src/lib.rs</code>
        </Callout>
      </div>
    </DocLayout>
  );
}
