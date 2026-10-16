'use client';

import DocLayout, { CodeBlock, Callout } from '@/components/docs/DocLayout';
import { Layers, Cpu, Database, Zap } from 'lucide-react';

export default function DualVMPage() {
  return (
    <DocLayout 
      title="Dual VM Architecture" 
      description="Understanding how EVM and SVM coexist in X3 Atlas Sphere"
      lastUpdated="December 2024"
    >
      <div className="space-y-8">
        {/* Overview */}
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Overview</h2>
          <p className="text-gray-400 mb-4">
            X3 Atlas Sphere is the first blockchain to natively support both the Ethereum Virtual Machine (EVM) 
            and the Solana Virtual Machine (SVM) within a single runtime. This enables unprecedented 
            interoperability between the two largest smart contract ecosystems.
          </p>
          <div className="grid md:grid-cols-2 gap-4 my-6">
            <div className="glass-card p-4 border-l-4 border-l-blue-500">
              <h3 className="font-semibold text-blue-400 mb-2">EVM (Frontier-based)</h3>
              <p className="text-sm text-gray-500">
                Full Ethereum compatibility for Solidity, Vyper contracts. Supports all ERC standards, 
                existing tooling like Hardhat, Foundry, and web3.js/ethers.js libraries.
              </p>
            </div>
            <div className="glass-card p-4 border-l-4 border-l-emerald-500">
              <h3 className="font-semibold text-emerald-400 mb-2">SVM (Sealevel-based)</h3>
              <p className="text-sm text-gray-500">
                Solana program execution with parallel transaction processing. Supports Anchor framework, 
                Rust programs, and SPL token standard.
              </p>
            </div>
          </div>
        </section>

        {/* Architecture Diagram */}
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Architecture</h2>
          <div className="glass-card p-6">
            <div className="flex flex-col items-center space-y-4">
              {/* Top Layer */}
              <div className="text-center">
                <div className="inline-flex items-center px-4 py-2 rounded-lg bg-orange-500/20 border border-orange-500/30">
                  <Cpu className="w-5 h-5 text-orange-400 mr-2" />
                  <span className="text-white font-semibold">Atlas Kernel (Orchestration Layer)</span>
                </div>
              </div>
              
              {/* Arrow */}
              <div className="h-8 w-0.5 bg-gradient-to-b from-orange-500 to-[#1a1a1a]" />
              
              {/* Middle Layer - VMs */}
              <div className="grid grid-cols-2 gap-8 w-full max-w-lg">
                <div className="text-center">
                  <div className="p-4 rounded-lg bg-blue-500/10 border border-blue-500/30">
                    <Layers className="w-8 h-8 text-blue-400 mx-auto mb-2" />
                    <span className="text-blue-400 font-semibold">EVM Adapter</span>
                    <p className="text-xs text-gray-500 mt-1">Frontier-based execution</p>
                  </div>
                </div>
                <div className="text-center">
                  <div className="p-4 rounded-lg bg-emerald-500/10 border border-emerald-500/30">
                    <Layers className="w-8 h-8 text-emerald-400 mx-auto mb-2" />
                    <span className="text-emerald-400 font-semibold">SVM Adapter</span>
                    <p className="text-xs text-gray-500 mt-1">Sealevel execution</p>
                  </div>
                </div>
              </div>
              
              {/* Arrow */}
              <div className="h-8 w-0.5 bg-gradient-to-b from-[#1a1a1a] to-violet-500" />
              
              {/* Bottom Layer */}
              <div className="text-center">
                <div className="inline-flex items-center px-4 py-2 rounded-lg bg-violet-500/20 border border-violet-500/30">
                  <Database className="w-5 h-5 text-violet-400 mr-2" />
                  <span className="text-white font-semibold">Canonical Ledger (Unified State)</span>
                </div>
              </div>
            </div>
          </div>
        </section>

        {/* How It Works */}
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">How It Works</h2>
          <p className="text-gray-400 mb-4">
            The dual VM architecture works through a layered approach:
          </p>
          <ol className="space-y-4">
            <li className="flex items-start space-x-3">
              <span className="flex-shrink-0 w-8 h-8 rounded-full bg-orange-500/20 flex items-center justify-center text-orange-400 font-bold">1</span>
              <div>
                <h3 className="font-semibold text-white">Transaction Submission</h3>
                <p className="text-gray-400">Users submit transactions (regular or Comits) through the standard extrinsic system</p>
              </div>
            </li>
            <li className="flex items-start space-x-3">
              <span className="flex-shrink-0 w-8 h-8 rounded-full bg-orange-500/20 flex items-center justify-center text-orange-400 font-bold">2</span>
              <div>
                <h3 className="font-semibold text-white">Atlas Kernel Processing</h3>
                <p className="text-gray-400">The kernel validates the transaction, checks authorization, and routes to appropriate VM(s)</p>
              </div>
            </li>
            <li className="flex items-start space-x-3">
              <span className="flex-shrink-0 w-8 h-8 rounded-full bg-orange-500/20 flex items-center justify-center text-orange-400 font-bold">3</span>
              <div>
                <h3 className="font-semibold text-white">Parallel Execution</h3>
                <p className="text-gray-400">EVM and SVM adapters execute their respective payloads in deterministic order</p>
              </div>
            </li>
            <li className="flex items-start space-x-3">
              <span className="flex-shrink-0 w-8 h-8 rounded-full bg-orange-500/20 flex items-center justify-center text-orange-400 font-bold">4</span>
              <div>
                <h3 className="font-semibold text-white">State Aggregation</h3>
                <p className="text-gray-400">Results are verified, and state changes are committed to the canonical ledger atomically</p>
              </div>
            </li>
          </ol>
        </section>

        {/* Code Example */}
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Interacting with Both VMs</h2>
          <p className="text-gray-400 mb-4">
            Here&apos;s an example of interacting with both VMs in a single transaction:
          </p>
          <CodeBlock language="typescript" title="dual-vm-example.ts">
{`import { AtlasClient, Comit } from '@x3/atlas-sdk';
import { ethers } from 'ethers';
import { Connection, PublicKey } from '@solana/frontend/web3.js';

async function dualVMOperation() {
  const atlas = new AtlasClient({ 
    rpcUrl: 'https://rpc.testnet.atlas-sphere.io' 
  });

  // Create a Comit (cross-VM transaction)
  const comit = new Comit({
    // EVM payload: Call a Uniswap-style swap
    evmPayload: {
      to: '0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D', // Router
      data: ethers.utils.defaultAbiCoder.encode(
        ['address', 'uint256'],
        ['0xTokenAddress...', 1000000]
      ),
      gasLimit: 200000,
    },
    
    // SVM payload: Interact with Jupiter aggregator
    svmPayload: {
      programId: new PublicKey('JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB'),
      data: Buffer.from([/* instruction data */]),
      accounts: [
        { pubkey: new PublicKey('...'), isSigner: false, isWritable: true },
      ],
    },
  });

  // Submit atomically - both succeed or both fail
  const receipt = await atlas.submitComit(comit);
  
  console.log('EVM result:', receipt.evmReceipt);
  console.log('SVM result:', receipt.svmReceipt);
  console.log('Comit finalized at block:', receipt.blockNumber);
}

dualVMOperation();`}
          </CodeBlock>
        </section>

        {/* Benefits */}
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Benefits</h2>
          <div className="grid md:grid-cols-2 gap-4">
            <div className="glass-card p-4">
              <Zap className="w-6 h-6 text-orange-400 mb-2" />
              <h3 className="font-semibold text-white mb-1">Atomic Execution</h3>
              <p className="text-sm text-gray-500">Both VM operations succeed or fail together, preventing inconsistent states</p>
            </div>
            <div className="glass-card p-4">
              <Layers className="w-6 h-6 text-orange-400 mb-2" />
              <h3 className="font-semibold text-white mb-1">Unified Liquidity</h3>
              <p className="text-sm text-gray-500">Assets exist once in the canonical ledger, accessible from both VMs</p>
            </div>
            <div className="glass-card p-4">
              <Database className="w-6 h-6 text-orange-400 mb-2" />
              <h3 className="font-semibold text-white mb-1">No Bridges Required</h3>
              <p className="text-sm text-gray-500">Native interoperability without wrapped tokens or trusted bridges</p>
            </div>
            <div className="glass-card p-4">
              <Cpu className="w-6 h-6 text-orange-400 mb-2" />
              <h3 className="font-semibold text-white mb-1">Ecosystem Compatibility</h3>
              <p className="text-sm text-gray-500">Use existing tools from both Ethereum and Solana ecosystems</p>
            </div>
          </div>
        </section>

        <Callout type="info" title="Learn More">
          Continue to the Atlas Kernel documentation to understand how the orchestration layer 
          manages cross-VM operations and ensures deterministic execution.
        </Callout>
      </div>
    </DocLayout>
  );
}
