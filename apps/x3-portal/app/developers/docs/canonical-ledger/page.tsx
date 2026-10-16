'use client';

import DocLayout, { CodeBlock, Callout } from '@/components/docs/DocLayout';
import { Database, Shield, Zap, ArrowRight } from 'lucide-react';

export default function CanonicalLedgerPage() {
  return (
    <DocLayout 
      title="Canonical Ledger" 
      description="Unified asset storage accessible from both VMs"
      lastUpdated="December 2024"
    >
      <div className="space-y-8">
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Overview</h2>
          <p className="text-gray-400 mb-4">
            The Canonical Ledger is X3 Atlas Sphere&apos;s unified asset storage system. Unlike traditional 
            multi-chain setups where assets are wrapped or bridged, assets in X3 exist once in the canonical 
            ledger and are natively accessible from both EVM and SVM.
          </p>
          
          <div className="glass-card p-6 my-6">
            <h3 className="font-semibold text-white mb-4">Key Benefits</h3>
            <div className="grid md:grid-cols-2 gap-4">
              <div className="flex items-start space-x-3">
                <Database className="w-5 h-5 text-orange-400 flex-shrink-0" />
                <div>
                  <span className="text-white font-medium">Single Source of Truth</span>
                  <p className="text-xs text-gray-500">No wrapped tokens or fragmented liquidity</p>
                </div>
              </div>
              <div className="flex items-start space-x-3">
                <Shield className="w-5 h-5 text-orange-400 flex-shrink-0" />
                <div>
                  <span className="text-white font-medium">No Bridge Risk</span>
                  <p className="text-xs text-gray-500">Assets never leave the canonical ledger</p>
                </div>
              </div>
              <div className="flex items-start space-x-3">
                <Zap className="w-5 h-5 text-orange-400 flex-shrink-0" />
                <div>
                  <span className="text-white font-medium">Instant Transfers</span>
                  <p className="text-xs text-gray-500">Cross-VM transfers in a single block</p>
                </div>
              </div>
              <div className="flex items-start space-x-3">
                <ArrowRight className="w-5 h-5 text-orange-400 flex-shrink-0" />
                <div>
                  <span className="text-white font-medium">Unified Accounts</span>
                  <p className="text-xs text-gray-500">Same balance across both VMs</p>
                </div>
              </div>
            </div>
          </div>
        </section>

        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Data Structure</h2>
          <CodeBlock language="rust" title="Canonical Ledger Storage">
{`// The canonical ledger maps (AccountId, AssetId) to Balance
pub type CanonicalLedger<T> = StorageDoubleMap<
    _,
    Blake2_128Concat, T::AccountId,  // Account identifier
    Blake2_128Concat, AssetId,        // Asset identifier
    Balance,                          // Amount held
    ValueQuery,
>;

// Asset metadata storage
pub struct AssetMetadata {
    pub symbol: Vec<u8>,      // e.g., "ATLAS", "USDC"
    pub name: Vec<u8>,        // e.g., "Atlas Token"
    pub decimals: u8,         // e.g., 12 for ATLAS, 6 for USDC
    pub total_supply: Balance,
    pub is_native: bool,      // true for ATLAS
}`}
          </CodeBlock>
        </section>

        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Querying Balances</h2>
          <CodeBlock language="typescript" title="Query Examples">
{`import { AtlasClient } from '@x3/atlas-sdk';

const atlas = new AtlasClient({ rpcUrl: '...' });

// Get native ATLAS balance
const atlasBalance = await atlas.getCanonicalBalance(
  '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY',
  'ATLAS'
);
console.log('ATLAS Balance:', atlasBalance.toString());

// Get USDC balance
const usdcBalance = await atlas.getCanonicalBalance(
  '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY',
  'USDC'
);
console.log('USDC Balance:', usdcBalance.toString());

// Get all balances for an account
const allBalances = await atlas.getAccountBalances(
  '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY'
);
allBalances.forEach(b => {
  console.log(\`\${b.symbol}: \${b.balance}\`);
});`}
          </CodeBlock>
        </section>

        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Registered Assets</h2>
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="border-b border-[#1a1a1a]">
                  <th className="text-left py-3 px-4 text-gray-500 font-medium">Asset ID</th>
                  <th className="text-left py-3 px-4 text-gray-500 font-medium">Symbol</th>
                  <th className="text-left py-3 px-4 text-gray-500 font-medium">Decimals</th>
                  <th className="text-left py-3 px-4 text-gray-500 font-medium">Type</th>
                </tr>
              </thead>
              <tbody>
                <tr className="border-b border-[#111111]">
                  <td className="py-3 px-4 text-orange-400 font-mono text-sm">ATLAS</td>
                  <td className="py-3 px-4 text-white">ATLAS</td>
                  <td className="py-3 px-4 text-gray-400">12</td>
                  <td className="py-3 px-4"><span className="badge badge-fire">Native</span></td>
                </tr>
                <tr className="border-b border-[#111111]">
                  <td className="py-3 px-4 text-orange-400 font-mono text-sm">USDC</td>
                  <td className="py-3 px-4 text-white">USDC</td>
                  <td className="py-3 px-4 text-gray-400">6</td>
                  <td className="py-3 px-4"><span className="badge badge-info">Bridged</span></td>
                </tr>
                <tr className="border-b border-[#111111]">
                  <td className="py-3 px-4 text-orange-400 font-mono text-sm">WETH</td>
                  <td className="py-3 px-4 text-white">WETH</td>
                  <td className="py-3 px-4 text-gray-400">18</td>
                  <td className="py-3 px-4"><span className="badge badge-info">Bridged</span></td>
                </tr>
                <tr className="border-b border-[#111111]">
                  <td className="py-3 px-4 text-orange-400 font-mono text-sm">SOL</td>
                  <td className="py-3 px-4 text-white">SOL</td>
                  <td className="py-3 px-4 text-gray-400">9</td>
                  <td className="py-3 px-4"><span className="badge badge-success">Bridged</span></td>
                </tr>
              </tbody>
            </table>
          </div>
        </section>

        <Callout type="info" title="Asset Registration">
          New assets can be registered through governance proposals. Community members can 
          propose adding new assets, which are voted on by token holders.
        </Callout>
      </div>
    </DocLayout>
  );
}
