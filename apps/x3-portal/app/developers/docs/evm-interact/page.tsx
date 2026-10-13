'use client';

import React from 'react';
import DocLayout, { CodeBlock, Callout } from '@/components/docs/DocLayout';

export default function EvmInteractPage() {
  return (
    <DocLayout
      title="Interact with EVM Contracts"
      description="Call and interact with deployed smart contracts on X3 Atlas Sphere"
    >
      <p className="lead text-xl text-gray-400 mb-8">
        Interact with your deployed smart contracts using ethers.js, wagmi, or any
        Ethereum-compatible library. X3's EVM is fully JSON-RPC compliant.
      </p>

      <h2>Using ethers.js</h2>
      <CodeBlock language="typescript" title="interact.ts">
{`import { ethers } from 'ethers';

// Connect to X3 testnet
const provider = new ethers.JsonRpcProvider('https://rpc.testnet.atlas-sphere.io');

// With wallet for transactions
const wallet = new ethers.Wallet(privateKey, provider);

// Contract interaction
const abi = ['function balanceOf(address) view returns (uint256)', 'function transfer(address to, uint256 amount) returns (bool)'];
const contract = new ethers.Contract(contractAddress, abi, wallet);

// Read (no gas)
const balance = await contract.balanceOf(wallet.address);
console.log('Balance:', ethers.formatEther(balance));

// Write (requires gas)
const tx = await contract.transfer(recipient, ethers.parseEther('100'));
await tx.wait();
console.log('Transfer complete:', tx.hash);`}
      </CodeBlock>

      <h2>Using wagmi + React</h2>
      <CodeBlock language="typescript" title="WagmiConfig.tsx">
{`import { createConfig, http } from 'wagmi';
import { defineChain } from 'viem';

// Define X3 chain
const x3Testnet = defineChain({
  id: 5330,
  name: 'X3 Atlas Sphere Testnet',
  nativeCurrency: { name: 'ATLAS', symbol: 'ATLAS', decimals: 18 },
  rpcUrls: {
    default: { http: ['https://rpc.testnet.atlas-sphere.io'] },
  },
  blockExplorers: {
    default: { name: 'X3 Explorer', url: 'https://explorer.testnet.atlas-sphere.io' },
  },
});

export const config = createConfig({
  chains: [x3Testnet],
  transports: {
    [x3Testnet.id]: http(),
  },
});`}
      </CodeBlock>

      <CodeBlock language="typescript" title="ContractInteraction.tsx">
{`'use client';

import { useReadContract, useWriteContract } from 'wagmi';
import { parseEther } from 'viem';

const CONTRACT_ADDRESS = '0x...';
const ABI = [...] as const;

export function TokenBalance({ address }: { address: string }) {
  const { data: balance, isLoading } = useReadContract({
    address: CONTRACT_ADDRESS,
    abi: ABI,
    functionName: 'balanceOf',
    args: [address],
  });

  if (isLoading) return <div>Loading...</div>;
  return <div>Balance: {balance?.toString()}</div>;
}

export function TransferButton() {
  const { writeContract, isPending } = useWriteContract();

  const handleTransfer = () => {
    writeContract({
      address: CONTRACT_ADDRESS,
      abi: ABI,
      functionName: 'transfer',
      args: ['0x...recipient', parseEther('10')],
    });
  };

  return (
    <button onClick={handleTransfer} disabled={isPending}>
      {isPending ? 'Transferring...' : 'Transfer 10 Tokens'}
    </button>
  );
}`}
      </CodeBlock>

      <h2>Event Listening</h2>
      <CodeBlock language="typescript" title="events.ts">
{`import { ethers } from 'ethers';

const provider = new ethers.JsonRpcProvider('https://rpc.testnet.atlas-sphere.io');
const contract = new ethers.Contract(address, abi, provider);

// Listen to Transfer events
contract.on('Transfer', (from, to, amount, event) => {
  console.log(\`Transfer: \${from} -> \${to}: \${ethers.formatEther(amount)}\`);
});

// Query past events
const filter = contract.filters.Transfer(null, userAddress);
const events = await contract.queryFilter(filter, -1000); // Last 1000 blocks`}
      </CodeBlock>

      <h2>Multicall for Batch Reads</h2>
      <CodeBlock language="typescript">
{`import { ethers } from 'ethers';

const multicall = new ethers.Contract(MULTICALL_ADDRESS, multicallAbi, provider);

// Batch multiple calls
const calls = [
  { target: tokenA, callData: tokenA.interface.encodeFunctionData('balanceOf', [user]) },
  { target: tokenB, callData: tokenB.interface.encodeFunctionData('balanceOf', [user]) },
];

const results = await multicall.aggregate.staticCall(calls);`}
      </CodeBlock>

      <Callout type="info" title="Gas Estimation">
        Always use <code>estimateGas()</code> before sending transactions to avoid failed txs.
        X3 provides accurate gas estimates via standard EVM methods.
      </Callout>

      <h2>Error Handling</h2>
      <CodeBlock language="typescript">
{`try {
  const tx = await contract.transfer(to, amount);
  await tx.wait();
} catch (error: any) {
  if (error.code === 'INSUFFICIENT_FUNDS') {
    console.error('Not enough ATLAS for gas');
  } else if (error.code === 'CALL_EXCEPTION') {
    // Decode custom error
    const decoded = contract.interface.parseError(error.data);
    console.error('Contract error:', decoded);
  }
}`}
      </CodeBlock>
    </DocLayout>
  );
}
