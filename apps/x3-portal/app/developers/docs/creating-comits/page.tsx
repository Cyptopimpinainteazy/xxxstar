'use client';

import React from 'react';
import DocLayout, { CodeBlock, Callout } from '@/components/docs/DocLayout';

export default function CreatingComitsPage() {
  return (
    <DocLayout
      title="Creating Comit Transactions"
      description="Build atomic cross-VM transactions that execute on both EVM and SVM"
    >
      <p className="lead text-xl text-gray-400 mb-8">
        Comit transactions are the core innovation of X3 Atlas Sphere—atomic operations 
        that execute across both EVM and SVM in a single transaction with guaranteed consistency.
      </p>

      <Callout type="info" title="What is a Comit?">
        A Comit (Cross-VM Commit) bundles an EVM payload and SVM payload that execute 
        atomically. Either both succeed, or both fail—no partial states.
      </Callout>

      <h2>Comit Structure</h2>
      <CodeBlock language="rust">
{`pub struct Comit {
    /// Unique identifier
    pub comit_id: H256,
    /// Substrate account submitting the Comit
    pub origin: AccountId,
    /// EVM transaction payload
    pub evm_payload: Vec<u8>,
    /// SVM instruction payload  
    pub svm_payload: Vec<u8>,
    /// Replay protection
    pub nonce: u64,
    /// Fee amount in ATLAS
    pub fee: Balance,
    /// Hash of inputs for verification
    pub prepare_root: H256,
}`}
      </CodeBlock>

      <h2>Creating a Comit via SDK</h2>
      <CodeBlock language="typescript" title="create-comit.ts">
{`import { ApiPromise, WsProvider } from '@polkadot/api';
import { Keyring } from '@polkadot/keyring';
import { ethers } from 'ethers';

// Connect to X3
const wsProvider = new WsProvider('wss://rpc.testnet.atlas-sphere.io');
const api = await ApiPromise.create({ provider: wsProvider });

// Setup accounts
const keyring = new Keyring({ type: 'sr25519' });
const account = keyring.addFromUri('//Alice');

// Build EVM payload (e.g., ERC20 transfer)
const erc20Interface = new ethers.Interface([
  'function transfer(address to, uint256 amount)'
]);
const evmPayload = erc20Interface.encodeFunctionData('transfer', [
  recipientAddress,
  ethers.parseEther('100')
]);

// Build SVM payload (e.g., SPL token mint)
const svmPayload = buildSplMintInstruction(
  mintAddress,
  destinationAta,
  mintAuthority,
  1_000_000_000n
);

// Create and submit Comit
const comit = api.tx.atlasKernel.submitComit(
  evmPayload,
  svmPayload,
  fee
);

const hash = await comit.signAndSend(account, ({ status, events }) => {
  if (status.isInBlock) {
    console.log('Comit included in block:', status.asInBlock.toHex());
    
    // Check for success
    const comitFinalized = events.find(({ event }) => 
      event.section === 'atlasKernel' && 
      event.method === 'ComitFinalized'
    );
    
    if (comitFinalized) {
      console.log('Comit executed successfully!');
    }
  }
});`}
      </CodeBlock>

      <h2>Comit Lifecycle</h2>
      <p>
        Comits go through these stages:
      </p>
      <ol className="list-decimal list-inside space-y-2 text-gray-400">
        <li><strong>Submission</strong> - Comit submitted to mempool, validated</li>
        <li><strong>Execution Start</strong> - Block producer begins processing</li>
        <li><strong>EVM Execution</strong> - EVM payload executed</li>
        <li><strong>SVM Execution</strong> - SVM payload executed</li>
        <li><strong>Verification</strong> - prepare_root verified against inputs</li>
        <li><strong>Finalization</strong> - State committed, nonce incremented</li>
      </ol>

      <h2>Prepare Root Calculation</h2>
      <CodeBlock language="typescript">
{`import { keccak256 } from 'ethers';

// prepare_root = hash(origin || nonce || evm_payload || svm_payload)
function calculatePrepareRoot(
  origin: string,
  nonce: bigint,
  evmPayload: Uint8Array,
  svmPayload: Uint8Array
): string {
  const encoded = ethers.solidityPacked(
    ['bytes32', 'uint64', 'bytes', 'bytes'],
    [origin, nonce, evmPayload, svmPayload]
  );
  return keccak256(encoded);
}`}
      </CodeBlock>

      <h2>Error Handling</h2>
      <CodeBlock language="typescript">
{`// Listen for Comit events
api.query.system.events((events) => {
  events.forEach(({ event }) => {
    if (event.section === 'atlasKernel') {
      switch (event.method) {
        case 'ComitSubmitted':
          console.log('Comit submitted:', event.data);
          break;
        case 'ComitFinalized':
          console.log('Success:', event.data);
          break;
        case 'ComitFailed':
          const [comitId, reason] = event.data;
          console.error('Failed:', decodeFailureReason(reason));
          break;
      }
    }
  });
});

function decodeFailureReason(code: number): string {
  const reasons: Record<number, string> = {
    0x01: 'Invalid nonce',
    0x02: 'Insufficient fee',
    0x03: 'Not authorized',
    0x04: 'EVM execution failed',
    0x05: 'SVM execution failed',
    0x06: 'Prepare root mismatch',
    0x10: 'EVM error (check gas)',
    0x11: 'SVM error (check compute)',
  };
  return reasons[code] || 'Unknown error';
}`}
      </CodeBlock>

      <h2>Gas and Compute Units</h2>
      <Callout type="warning" title="Fee Calculation">
        Comit fees combine EVM gas and SVM compute units:
        <code className="block mt-2">fee = base_fee + (evm_gas / 1000) + (svm_compute / 1000)</code>
      </Callout>

      <h2>Best Practices</h2>
      <ul>
        <li><strong>Validate locally first</strong> - Test EVM and SVM payloads independently</li>
        <li><strong>Check authorization</strong> - Ensure account is authorized before submission</li>
        <li><strong>Handle failures gracefully</strong> - Both VMs rollback on failure</li>
        <li><strong>Monitor events</strong> - Subscribe to Comit events for status</li>
        <li><strong>Estimate fees</strong> - Use RPC to estimate before submission</li>
      </ul>

      <h2>RPC Methods</h2>
      <CodeBlock language="typescript">
{`// Check if account is authorized
const isAuthorized = await api.rpc.atlasKernel.isAuthorized(accountId);

// Get current nonce
const nonce = await api.query.atlasKernel.comitNonces(accountId);

// Estimate Comit fee
const estimate = await api.rpc.atlasKernel.estimateComitFee(
  evmPayload,
  svmPayload
);`}
      </CodeBlock>
    </DocLayout>
  );
}
