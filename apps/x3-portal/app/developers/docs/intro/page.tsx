'use client';

import DocLayout, { CodeBlock, Callout } from '@/components/docs/DocLayout';
import Link from 'next/link';
import { ArrowRight, Zap, Layers, Shield, Globe } from 'lucide-react';

export default function IntroPage() {
  return (
    <DocLayout 
      title="Introduction" 
      description="Welcome to X3 Atlas Sphere - the first dual VM Layer-1 blockchain"
      lastUpdated="December 2024"
    >
      <div className="space-y-8">
        {/* Overview */}
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">What is X3 Atlas Sphere?</h2>
          <p className="text-gray-400 mb-4">
            X3 Atlas Sphere is a revolutionary Layer-1 blockchain that enables native interoperability 
            between the <strong className="text-white">Ethereum Virtual Machine (EVM)</strong> and 
            the <strong className="text-white">Solana Virtual Machine (SVM)</strong>. For the first time, 
            developers can build applications that leverage both ecosystems simultaneously with atomic execution guarantees.
          </p>
          <p className="text-gray-400">
            The core innovation is the <span className="text-orange-400 font-semibold">Atlas Kernel</span> - a 
            Substrate pallet that orchestrates cross-domain transactions called <span className="text-orange-400 font-semibold">&quot;Comits&quot;</span> 
            that execute simultaneously across both VMs with deterministic ordering.
          </p>
        </section>

        {/* Key Features */}
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Key Features</h2>
          <div className="grid md:grid-cols-2 gap-4">
            <div className="glass-card p-4">
              <Layers className="w-6 h-6 text-orange-400 mb-2" />
              <h3 className="font-semibold text-white mb-1">Dual VM Execution</h3>
              <p className="text-sm text-gray-500">Run EVM and SVM smart contracts side-by-side with full compatibility</p>
            </div>
            <div className="glass-card p-4">
              <Zap className="w-6 h-6 text-orange-400 mb-2" />
              <h3 className="font-semibold text-white mb-1">Atomic Cross-VM Operations</h3>
              <p className="text-sm text-gray-500">Execute transactions that span both VMs atomically</p>
            </div>
            <div className="glass-card p-4">
              <Shield className="w-6 h-6 text-orange-400 mb-2" />
              <h3 className="font-semibold text-white mb-1">Canonical Ledger</h3>
              <p className="text-sm text-gray-500">Unified asset storage accessible from both VMs</p>
            </div>
            <div className="glass-card p-4">
              <Globe className="w-6 h-6 text-orange-400 mb-2" />
              <h3 className="font-semibold text-white mb-1">Cross-Chain Composability</h3>
              <p className="text-sm text-gray-500">Built-in message lanes for multi-chain operations</p>
            </div>
          </div>
        </section>

        {/* Architecture Overview */}
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Architecture Overview</h2>
          <p className="text-gray-400 mb-4">
            X3 Atlas Sphere is built on Substrate, leveraging its modular runtime architecture. 
            The blockchain consists of several key components:
          </p>
          <ul className="list-disc list-inside text-gray-400 space-y-2 ml-4">
            <li><strong className="text-white">Runtime</strong>: FRAME-based with pallets for balances, transaction payment, and Atlas Kernel</li>
            <li><strong className="text-white">Atlas Kernel Pallet</strong>: Manages Comit submission, nonce tracking, authorization, and VM orchestration</li>
            <li><strong className="text-white">EVM Adapter</strong>: Frontier-based adapter for Ethereum contract execution</li>
            <li><strong className="text-white">SVM Adapter</strong>: Adapter for Solana program execution</li>
            <li><strong className="text-white">Consensus</strong>: Aura block authoring (6-second slots) + GRANDPA finality</li>
          </ul>
        </section>

        <Callout type="info" title="Development Status">
          X3 Atlas Sphere is currently in testnet phase. The mainnet launch is planned for Q2 2025.
          Join our Discord to stay updated on the latest developments.
        </Callout>

        {/* Quick Example */}
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Quick Example</h2>
          <p className="text-gray-400 mb-4">
            Here&apos;s a simple example of querying the network using JavaScript:
          </p>
          <CodeBlock language="typescript" title="query-network.ts">
{`import { ApiPromise, WsProvider } from '@polkadot/api';

async function main() {
  // Connect to X3 Atlas Sphere
  const provider = new WsProvider('wss://rpc.testnet.atlas-sphere.io');
  const api = await ApiPromise.create({ provider });

  // Get the latest block
  const block = await api.rpc.chain.getBlock();
  console.log('Block:', block.block.header.number.toNumber());

  // Query canonical ledger balance
  const balance = await api.query.atlasKernel.canonicalLedger(
    'YOUR_ACCOUNT_ID',
    'ATLAS' // Native token asset ID
  );
  console.log('Balance:', balance.toString());
}

main();`}
          </CodeBlock>
        </section>

        {/* Next Steps */}
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Next Steps</h2>
          <div className="grid md:grid-cols-2 gap-4">
            <Link href="/developers/docs/quickstart" className="glass-card-hover p-4 block card-lift">
              <h3 className="font-semibold text-white mb-1 flex items-center">
                Quick Start Guide <ArrowRight className="w-4 h-4 ml-2" />
              </h3>
              <p className="text-sm text-gray-500">Get up and running in under 5 minutes</p>
            </Link>
            <Link href="/developers/docs/dual-vm" className="glass-card-hover p-4 block card-lift">
              <h3 className="font-semibold text-white mb-1 flex items-center">
                Dual VM Architecture <ArrowRight className="w-4 h-4 ml-2" />
              </h3>
              <p className="text-sm text-gray-500">Understand how EVM and SVM coexist</p>
            </Link>
          </div>
        </section>
      </div>
    </DocLayout>
  );
}
