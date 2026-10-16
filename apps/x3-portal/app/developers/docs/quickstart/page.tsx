'use client';

import DocLayout, { CodeBlock, Callout } from '@/components/docs/DocLayout';
import Link from 'next/link';
import { Terminal, Code, CheckCircle, Coins } from 'lucide-react';

export default function QuickstartPage() {
  return (
    <DocLayout 
      title="Quick Start" 
      description="Get up and running with X3 Atlas Sphere in under 5 minutes"
      lastUpdated="December 2024"
    >
      <div className="space-y-8">
        {/* Prerequisites */}
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Prerequisites</h2>
          <p className="text-gray-400 mb-4">Before you begin, make sure you have:</p>
          <ul className="space-y-2">
            {[
              'Node.js 18+ installed',
              'A code editor (VS Code recommended)',
              'Basic knowledge of JavaScript/TypeScript',
              'MetaMask or Phantom wallet (optional)',
            ].map((item, i) => (
              <li key={i} className="flex items-center text-gray-400">
                <CheckCircle className="w-5 h-5 text-emerald-500 mr-2 flex-shrink-0" />
                {item}
              </li>
            ))}
          </ul>
        </section>

        {/* Step 1: Install SDK */}
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Step 1: Install Dependencies</h2>
          <p className="text-gray-400 mb-4">
            Install the Polkadot API for Substrate access and ethers.js for EVM access:
          </p>
          <CodeBlock language="bash" title="Terminal">
{`# For Substrate (native) access
npm install @polkadot/api @polkadot/keyring

# For EVM access
npm install ethers

# For both (recommended)
npm install @polkadot/api @polkadot/keyring ethers`}
          </CodeBlock>
        </section>

        {/* Step 2: Connect to Network */}
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Step 2: Connect to the Network</h2>
          <p className="text-gray-400 mb-4">
            Connect to X3 Atlas Sphere testnet:
          </p>
          <CodeBlock language="typescript" title="connect.ts">
{`import { ApiPromise, WsProvider } from '@polkadot/api';

// Connect to testnet via WebSocket
const wsProvider = new WsProvider('ws://rpc.testnet.atlas-sphere.io:9944');
const api = await ApiPromise.create({ provider: wsProvider });

// Wait for connection
await api.isReady;

// Get chain info
const chain = await api.rpc.system.chain();
const lastHeader = await api.rpc.chain.getHeader();
console.log(\`Connected to \${chain}\`);
console.log(\`Latest block: #\${lastHeader.number}\`);`}
          </CodeBlock>
          
          <p className="text-gray-400 mt-6 mb-4">
            Or connect using ethers.js for EVM access:
          </p>
          <CodeBlock language="typescript" title="connect-evm.ts">
{`import { ethers } from 'ethers';

// Connect to X3 Atlas EVM
const provider = new ethers.JsonRpcProvider(
  'http://rpc.testnet.atlas-sphere.io:9944'
);

const blockNumber = await provider.getBlockNumber();
console.log('EVM block:', blockNumber);`}
          </CodeBlock>
        </section>

        {/* Step 3: Get Test Tokens */}
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Step 3: Get Test Tokens</h2>
          <p className="text-gray-400 mb-4">
            Visit our faucet to get testnet X3 tokens:
          </p>
          <div className="glass-card p-6 flex items-center justify-between">
            <div>
              <h3 className="font-semibold text-white mb-1">Testnet Faucet</h3>
              <p className="text-sm text-gray-500">Get X3 tokens for testing</p>
            </div>
            <Link 
              href="https://faucet.testnet.atlas-sphere.io" 
              className="btn-primary"
              target="_blank"
            >
              Get X3 Tokens
            </Link>
          </div>
          
          <Callout type="info" title="X3 is the Native Gas Token">
            X3 is used for all transaction fees on Atlas Sphere - whether you're using 
            the EVM, SVM, or cross-VM Comits. It exists once in the Canonical Ledger 
            and is accessible from both VMs.
          </Callout>
        </section>

        {/* Step 4: Query the Chain */}
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Step 4: Query the Chain</h2>
          <p className="text-gray-400 mb-4">
            Query blockchain data using the Polkadot API:
          </p>
          <CodeBlock language="typescript" title="query.ts">
{`// Query system info
const [chain, name, version] = await Promise.all([
  api.rpc.system.chain(),
  api.rpc.system.name(),
  api.rpc.system.version(),
]);
console.log(\`Chain: \${chain}, Node: \${name} v\${version}\`);

// Query canonical ledger balance (Asset ID 0 = X3)
const accountId = '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY'; // Alice
const balance = await api.query.atlasKernel.canonicalLedger(accountId, 0);
console.log('X3 Balance:', balance.toHuman());

// Subscribe to new blocks
const unsubscribe = await api.rpc.chain.subscribeNewHeads((header) => {
  console.log(\`New block #\${header.number}\`);
});`}
          </CodeBlock>
        </section>

        {/* Step 5: Send Transaction */}
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Step 5: Send a Transaction</h2>
          <p className="text-gray-400 mb-4">
            Send your first transaction on X3 Atlas Sphere:
          </p>
          <CodeBlock language="typescript" title="transfer.ts">
{`import { Keyring } from '@polkadot/keyring';

// Create a keyring for signing (Substrate native)
const keyring = new Keyring({ type: 'sr25519' });
const alice = keyring.addFromUri('//Alice'); // Dev account

// Send a balance transfer
const transfer = api.tx.balances.transferKeepAlive(
  '5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty', // Bob
  10_000_000_000_000n // 10 X3 (18 decimals)
);

// Sign and submit
const hash = await transfer.signAndSend(alice);
console.log('Transaction hash:', hash.toHex());`}
          </CodeBlock>
          
          <Callout type="warning" title="Security Note">
            Never use development accounts (//Alice, //Bob) in production. Always use properly 
            generated and secured private keys for real transactions.
          </Callout>
        </section>

        {/* EVM Transactions */}
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Bonus: EVM Transactions</h2>
          <p className="text-gray-400 mb-4">
            Send transactions using ethers.js on the EVM side:
          </p>
          <CodeBlock language="typescript" title="evm-transfer.ts">
{`import { ethers } from 'ethers';

const provider = new ethers.JsonRpcProvider(
  'http://rpc.testnet.atlas-sphere.io:9944'
);

// Create wallet from private key
const wallet = new ethers.Wallet(privateKey, provider);

// Send X3 (native gas token)
const tx = await wallet.sendTransaction({
  to: '0xRecipientAddress...',
  value: ethers.parseEther('1.0'), // 1 X3
});

console.log('TX hash:', tx.hash);
const receipt = await tx.wait();
console.log('Confirmed in block:', receipt?.blockNumber);`}
          </CodeBlock>
        </section>

        {/* What's Next */}
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">What&apos;s Next?</h2>
          <div className="grid md:grid-cols-2 lg:grid-cols-4 gap-4">
            <Link href="/learn/tokenomics" className="glass-card-hover p-4 block card-lift">
              <Coins className="w-6 h-6 text-orange-400 mb-2" />
              <h3 className="font-semibold text-white mb-1">Tokenomics</h3>
              <p className="text-xs text-gray-500">Learn about X3Coin</p>
            </Link>
            <Link href="/developers/docs/dual-vm" className="glass-card-hover p-4 block card-lift">
              <Code className="w-6 h-6 text-orange-400 mb-2" />
              <h3 className="font-semibold text-white mb-1">Dual VM Guide</h3>
              <p className="text-xs text-gray-500">Deploy on EVM and SVM</p>
            </Link>
            <Link href="/developers/docs/comits" className="glass-card-hover p-4 block card-lift">
              <Terminal className="w-6 h-6 text-orange-400 mb-2" />
              <h3 className="font-semibold text-white mb-1">Comit Transactions</h3>
              <p className="text-xs text-gray-500">Cross-VM atomic operations</p>
            </Link>
            <Link href="/network/rpc-providers" className="glass-card-hover p-4 block card-lift">
              <Code className="w-6 h-6 text-orange-400 mb-2" />
              <h3 className="font-semibold text-white mb-1">RPC Providers</h3>
              <p className="text-xs text-gray-500">Network endpoints</p>
            </Link>
          </div>
        </section>
      </div>
    </DocLayout>
  );
}
