'use client';

import DocLayout, { CodeBlock, Callout } from '@/components/docs/DocLayout';
import { Zap, ArrowRight, CheckCircle, XCircle } from 'lucide-react';

export default function ComitsPage() {
  return (
    <DocLayout 
      title="Comit Transactions" 
      description="Atomic cross-VM operations in X3 Atlas Sphere"
      lastUpdated="December 2024"
    >
      <div className="space-y-8">
        {/* Overview */}
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">What is a Comit?</h2>
          <p className="text-gray-400 mb-4">
            A <strong className="text-orange-400">Comit</strong> (Cross-VM Commit) is X3 Atlas Sphere&apos;s core innovation - 
            an atomic transaction that executes operations on both EVM and SVM simultaneously. Either both 
            operations succeed, or both are rolled back, ensuring consistent state across virtual machines.
          </p>
          
          <div className="glass-card p-6 my-6">
            <h3 className="font-semibold text-white mb-4">Comit Guarantees</h3>
            <ul className="space-y-2">
              {[
                'Atomicity: Both payloads execute or neither does',
                'Consistency: State remains valid after execution',
                'Isolation: Comits don\'t interfere with each other',
                'Durability: Finalized state is permanent',
              ].map((item, i) => (
                <li key={i} className="flex items-center text-gray-400">
                  <CheckCircle className="w-5 h-5 text-emerald-500 mr-2 flex-shrink-0" />
                  {item}
                </li>
              ))}
            </ul>
          </div>
        </section>

        {/* Structure */}
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Comit Structure</h2>
          <p className="text-gray-400 mb-4">
            A Comit contains the following fields:
          </p>
          <CodeBlock language="typescript" title="Comit Structure">
{`interface Comit {
  // Unique identifier for this Comit
  comit_id: H256;
  
  // Account submitting the Comit
  origin: AccountId;
  
  // EVM transaction payload
  evm_payload: {
    to: H160;           // Contract address
    data: Bytes;        // Calldata
    value: U256;        // ETH value (optional)
    gas_limit: u64;     // Max gas
  };
  
  // SVM instruction payload
  svm_payload: {
    program_id: Pubkey;     // Program address
    data: Bytes;            // Instruction data
    accounts: AccountMeta[];// Account inputs
  };
  
  // Replay protection
  nonce: u64;
  
  // Fee for execution
  fee: Balance;
  
  // Hash of inputs for verification
  prepare_root: H256;
}`}
          </CodeBlock>
        </section>

        {/* Lifecycle */}
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Comit Lifecycle</h2>
          <div className="space-y-4">
            <div className="flex items-start space-x-4">
              <div className="flex-shrink-0 w-10 h-10 rounded-full bg-orange-500/20 flex items-center justify-center">
                <span className="text-orange-400 font-bold">1</span>
              </div>
              <div className="glass-card p-4 flex-1">
                <h3 className="font-semibold text-white mb-1">Submission</h3>
                <p className="text-sm text-gray-400">
                  User submits Comit via <code className="text-orange-400">atlasKernel::submitComit</code> extrinsic.
                  Origin must be authorized and nonce must match.
                </p>
              </div>
            </div>
            
            <div className="flex items-start space-x-4">
              <div className="flex-shrink-0 w-10 h-10 rounded-full bg-orange-500/20 flex items-center justify-center">
                <span className="text-orange-400 font-bold">2</span>
              </div>
              <div className="glass-card p-4 flex-1">
                <h3 className="font-semibold text-white mb-1">Validation</h3>
                <p className="text-sm text-gray-400">
                  Kernel validates payload sizes (≤16KB each, ≤32KB combined), checks nonce, 
                  verifies authorization, and ensures sufficient fees.
                </p>
              </div>
            </div>
            
            <div className="flex items-start space-x-4">
              <div className="flex-shrink-0 w-10 h-10 rounded-full bg-orange-500/20 flex items-center justify-center">
                <span className="text-orange-400 font-bold">3</span>
              </div>
              <div className="glass-card p-4 flex-1">
                <h3 className="font-semibold text-white mb-1">Execution</h3>
                <p className="text-sm text-gray-400">
                  EVM payload dispatched to Frontier adapter, SVM payload to Sealevel adapter.
                  Both execute in deterministic order, collecting receipts.
                </p>
              </div>
            </div>
            
            <div className="flex items-start space-x-4">
              <div className="flex-shrink-0 w-10 h-10 rounded-full bg-orange-500/20 flex items-center justify-center">
                <span className="text-orange-400 font-bold">4</span>
              </div>
              <div className="glass-card p-4 flex-1">
                <h3 className="font-semibold text-white mb-1">Verification</h3>
                <p className="text-sm text-gray-400">
                  Kernel recomputes <code className="text-orange-400">prepare_root</code> from inputs 
                  and verifies it matches the submitted value.
                </p>
              </div>
            </div>
            
            <div className="flex items-start space-x-4">
              <div className="flex-shrink-0 w-10 h-10 rounded-full bg-emerald-500/20 flex items-center justify-center">
                <span className="text-emerald-400 font-bold">5</span>
              </div>
              <div className="glass-card p-4 flex-1">
                <h3 className="font-semibold text-white mb-1">Finalization</h3>
                <p className="text-sm text-gray-400">
                  Canonical ledger updated with state changes, nonce incremented, 
                  <code className="text-orange-400 ml-1">ComitFinalized</code> event emitted.
                </p>
              </div>
            </div>
          </div>
        </section>

        {/* Creating a Comit */}
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Creating a Comit</h2>
          <CodeBlock language="typescript" title="create-comit.ts">
{`import { AtlasClient, Comit, ComitBuilder } from '@x3/atlas-sdk';
import { ethers } from 'ethers';

async function createComit() {
  const atlas = new AtlasClient({ 
    rpcUrl: 'https://rpc.testnet.atlas-sphere.io' 
  });

  // Use the builder for easier Comit construction
  const comit = new ComitBuilder()
    // Add EVM operation - swap on Uniswap
    .withEvmPayload({
      to: '0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D',
      data: ethers.utils.defaultAbiCoder.encode(
        ['address[]', 'uint256', 'uint256'],
        [
          ['0xTokenA...', '0xTokenB...'],
          ethers.utils.parseEther('1'),
          ethers.utils.parseEther('0.9'),
        ]
      ),
      gasLimit: 300000,
    })
    // Add SVM operation - deposit to protocol
    .withSvmPayload({
      programId: 'YourProgramId...',
      data: Buffer.from([/* instruction data */]),
      accounts: [
        { pubkey: 'Account1...', isSigner: false, isWritable: true },
        { pubkey: 'Account2...', isSigner: false, isWritable: true },
      ],
    })
    // Set fee
    .withFee('1000000000000') // 1 ATLAS
    .build();

  // Submit the Comit
  const receipt = await atlas.submitComit(comit, signer);
  
  console.log('Comit ID:', receipt.comitId);
  console.log('Block:', receipt.blockNumber);
  console.log('EVM Gas Used:', receipt.evmReceipt.gasUsed);
  console.log('SVM Compute Units:', receipt.svmReceipt.computeUnits);
}`}
          </CodeBlock>
        </section>

        {/* Error Handling */}
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Error Handling</h2>
          <p className="text-gray-400 mb-4">
            If a Comit fails, both payloads are rolled back. The failure reason is included in the event:
          </p>
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="border-b border-[#1a1a1a]">
                  <th className="text-left py-3 px-4 text-gray-500 font-medium">Error Code</th>
                  <th className="text-left py-3 px-4 text-gray-500 font-medium">Description</th>
                </tr>
              </thead>
              <tbody>
                <tr className="border-b border-[#111111]">
                  <td className="py-3 px-4 text-red-400 font-mono text-sm">0x01</td>
                  <td className="py-3 px-4 text-gray-400">Not authorized to submit Comits</td>
                </tr>
                <tr className="border-b border-[#111111]">
                  <td className="py-3 px-4 text-red-400 font-mono text-sm">0x02</td>
                  <td className="py-3 px-4 text-gray-400">Invalid nonce</td>
                </tr>
                <tr className="border-b border-[#111111]">
                  <td className="py-3 px-4 text-red-400 font-mono text-sm">0x03</td>
                  <td className="py-3 px-4 text-gray-400">Payload too large</td>
                </tr>
                <tr className="border-b border-[#111111]">
                  <td className="py-3 px-4 text-red-400 font-mono text-sm">0x04</td>
                  <td className="py-3 px-4 text-gray-400">Insufficient fee</td>
                </tr>
                <tr className="border-b border-[#111111]">
                  <td className="py-3 px-4 text-red-400 font-mono text-sm">0x06</td>
                  <td className="py-3 px-4 text-gray-400">Prepare root mismatch</td>
                </tr>
                <tr className="border-b border-[#111111]">
                  <td className="py-3 px-4 text-red-400 font-mono text-sm">0x10-0x1F</td>
                  <td className="py-3 px-4 text-gray-400">EVM execution error</td>
                </tr>
                <tr className="border-b border-[#111111]">
                  <td className="py-3 px-4 text-red-400 font-mono text-sm">0x11-0x1F</td>
                  <td className="py-3 px-4 text-gray-400">SVM execution error</td>
                </tr>
              </tbody>
            </table>
          </div>
          
          <CodeBlock language="typescript" title="Error Handling">
{`try {
  const receipt = await atlas.submitComit(comit, signer);
  console.log('Success!', receipt);
} catch (error) {
  if (error.code === '0x01') {
    console.error('Account not authorized. Request authorization first.');
  } else if (error.code === '0x02') {
    console.error('Nonce mismatch. Fetch latest nonce and retry.');
  } else if (error.code.startsWith('0x10')) {
    console.error('EVM execution failed:', error.evmError);
  } else if (error.code.startsWith('0x11')) {
    console.error('SVM execution failed:', error.svmError);
  }
}`}
          </CodeBlock>
        </section>

        {/* Fee Calculation */}
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Fee Calculation</h2>
          <p className="text-gray-400 mb-4">
            Comit fees are calculated as follows:
          </p>
          <CodeBlock language="text" title="Fee Formula">
{`total_fee = base_fee + (evm_gas_used / 1000) + (svm_compute_units / 1000)

Where:
- base_fee: 0.001 ATLAS (minimum fee)
- evm_gas_used: Actual gas consumed by EVM payload
- svm_compute_units: Actual compute units consumed by SVM payload
- Gas price: 1 micro-ATLAS per unit`}
          </CodeBlock>
          
          <Callout type="info" title="Fee Refunds">
            If execution uses less gas/compute than estimated, the difference is refunded 
            to the origin account. Failed Comits burn a partial fee proportional to work done.
          </Callout>
        </section>

        {/* Best Practices */}
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Best Practices</h2>
          <ul className="space-y-3">
            <li className="flex items-start space-x-3">
              <CheckCircle className="w-5 h-5 text-emerald-500 flex-shrink-0" />
              <span className="text-gray-400"><strong className="text-white">Estimate gas/compute</strong> before submission to avoid out-of-gas failures</span>
            </li>
            <li className="flex items-start space-x-3">
              <CheckCircle className="w-5 h-5 text-emerald-500 flex-shrink-0" />
              <span className="text-gray-400"><strong className="text-white">Verify prepare_root</strong> client-side before submitting to catch errors early</span>
            </li>
            <li className="flex items-start space-x-3">
              <CheckCircle className="w-5 h-5 text-emerald-500 flex-shrink-0" />
              <span className="text-gray-400"><strong className="text-white">Handle all failure cases</strong> with appropriate retry logic</span>
            </li>
            <li className="flex items-start space-x-3">
              <XCircle className="w-5 h-5 text-red-500 flex-shrink-0" />
              <span className="text-gray-400"><strong className="text-white">Don&apos;t assume ordering</strong> between different Comits - use nonces for dependent operations</span>
            </li>
          </ul>
        </section>
      </div>
    </DocLayout>
  );
}
