'use client';

import DocLayout, { CodeBlock, Callout } from '@/components/docs/DocLayout';
import { Shield, Key, CheckCircle, XCircle } from 'lucide-react';

export default function AuthorizationPage() {
  return (
    <DocLayout 
      title="Account Authorization" 
      description="Permission system for Comit submissions"
      lastUpdated="December 2024"
    >
      <div className="space-y-8">
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Overview</h2>
          <p className="text-gray-400 mb-4">
            To submit Comit transactions in X3 Atlas Sphere, accounts must be explicitly authorized. 
            This authorization system provides an additional layer of security and allows for controlled 
            access to cross-VM operations during the early network phases.
          </p>
        </section>

        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Authorization Flow</h2>
          <div className="space-y-4">
            <div className="flex items-start space-x-4">
              <div className="flex-shrink-0 w-10 h-10 rounded-full bg-orange-500/20 flex items-center justify-center">
                <span className="text-orange-400 font-bold">1</span>
              </div>
              <div className="glass-card p-4 flex-1">
                <h3 className="font-semibold text-white mb-1">Request Authorization</h3>
                <p className="text-sm text-gray-400">
                  Submit a governance proposal or request authorization through official channels.
                </p>
              </div>
            </div>
            
            <div className="flex items-start space-x-4">
              <div className="flex-shrink-0 w-10 h-10 rounded-full bg-orange-500/20 flex items-center justify-center">
                <span className="text-orange-400 font-bold">2</span>
              </div>
              <div className="glass-card p-4 flex-1">
                <h3 className="font-semibold text-white mb-1">Governance Approval</h3>
                <p className="text-sm text-gray-400">
                  Token holders vote on authorization requests. Approved accounts are added to the whitelist.
                </p>
              </div>
            </div>
            
            <div className="flex items-start space-x-4">
              <div className="flex-shrink-0 w-10 h-10 rounded-full bg-emerald-500/20 flex items-center justify-center">
                <CheckCircle className="w-5 h-5 text-emerald-400" />
              </div>
              <div className="glass-card p-4 flex-1">
                <h3 className="font-semibold text-white mb-1">Submit Comits</h3>
                <p className="text-sm text-gray-400">
                  Authorized accounts can now submit Comit transactions.
                </p>
              </div>
            </div>
          </div>
        </section>

        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Checking Authorization</h2>
          <CodeBlock language="typescript" title="Check Authorization Status">
{`import { AtlasClient } from '@x3/atlas-sdk';

const atlas = new AtlasClient({ rpcUrl: '...' });

// Check if an account is authorized
const isAuthorized = await atlas.isAuthorized(
  '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY'
);

if (isAuthorized) {
  console.log('Account is authorized to submit Comits');
} else {
  console.log('Account needs authorization');
}

// Get all authorized accounts
const authorizedAccounts = await atlas.getAuthorizedAccounts();
console.log('Authorized accounts:', authorizedAccounts);`}
          </CodeBlock>
        </section>

        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Development Mode</h2>
          <p className="text-gray-400 mb-4">
            When running a local development node with the <code className="text-orange-400">dev-bypass</code> 
            feature enabled, authorization checks are skipped. This allows for easier testing.
          </p>
          <CodeBlock language="bash" title="Enable Dev Bypass">
{`# Build with dev-bypass feature
cargo build --release --features dev-bypass

# Or use the dev node which has it enabled by default
./run-dev-node.sh`}
          </CodeBlock>
          
          <Callout type="warning" title="Production Warning">
            Never use <code>dev-bypass</code> in production. Authorization checks are 
            essential for network security.
          </Callout>
        </section>

        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Governance Functions</h2>
          <CodeBlock language="rust" title="Authorization Extrinsics">
{`// Authorize an account (governance only)
#[pallet::call_index(1)]
pub fn authorize_account(
    origin: OriginFor<T>,
    account: T::AccountId,
) -> DispatchResult {
    T::GovernanceOrigin::ensure_origin(origin)?;
    AuthorizedAccounts::<T>::insert(&account, ());
    Self::deposit_event(Event::AccountAuthorized { account });
    Ok(())
}

// Deauthorize an account (governance only)
#[pallet::call_index(2)]
pub fn deauthorize_account(
    origin: OriginFor<T>,
    account: T::AccountId,
) -> DispatchResult {
    T::GovernanceOrigin::ensure_origin(origin)?;
    AuthorizedAccounts::<T>::remove(&account);
    Self::deposit_event(Event::AccountDeauthorized { account });
    Ok(())
}`}
          </CodeBlock>
        </section>
      </div>
    </DocLayout>
  );
}
