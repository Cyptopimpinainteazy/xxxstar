'use client';

import DocLayout, { CodeBlock, Callout } from '@/components/docs/DocLayout';

export default function ConfigurationPage() {
  return (
    <DocLayout 
      title="Configuration" 
      description="Configure your development environment for X3 Atlas Sphere"
      lastUpdated="December 2024"
    >
      <div className="space-y-8">
        {/* Network Configuration */}
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Network Endpoints</h2>
          <p className="text-gray-400 mb-4">
            X3 Atlas Sphere provides different network endpoints for Substrate (native) and EVM access:
          </p>
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="border-b border-[#1a1a1a]">
                  <th className="text-left py-3 px-4 text-gray-500 font-medium">Network</th>
                  <th className="text-left py-3 px-4 text-gray-500 font-medium">RPC URL</th>
                  <th className="text-left py-3 px-4 text-gray-500 font-medium">Type</th>
                </tr>
              </thead>
              <tbody>
                <tr className="border-b border-[#111111]">
                  <td className="py-3 px-4 text-white">Testnet HTTP</td>
                  <td className="py-3 px-4 text-cyan-400 font-mono text-sm">http://rpc.testnet.atlas-sphere.io:9944</td>
                  <td className="py-3 px-4 text-gray-400">Substrate + EVM</td>
                </tr>
                <tr className="border-b border-[#111111]">
                  <td className="py-3 px-4 text-white">Testnet WebSocket</td>
                  <td className="py-3 px-4 text-cyan-400 font-mono text-sm">ws://rpc.testnet.atlas-sphere.io:9944</td>
                  <td className="py-3 px-4 text-gray-400">Substrate + EVM</td>
                </tr>
                <tr className="border-b border-[#111111]">
                  <td className="py-3 px-4 text-white">Local Dev HTTP</td>
                  <td className="py-3 px-4 text-cyan-400 font-mono text-sm">http://127.0.0.1:9944</td>
                  <td className="py-3 px-4 text-gray-400">Substrate + EVM</td>
                </tr>
                <tr className="border-b border-[#111111]">
                  <td className="py-3 px-4 text-white">Local Dev WebSocket</td>
                  <td className="py-3 px-4 text-cyan-400 font-mono text-sm">ws://127.0.0.1:9944</td>
                  <td className="py-3 px-4 text-gray-400">Substrate + EVM</td>
                </tr>
              </tbody>
            </table>
          </div>
          
          <Callout type="info" title="Unified Endpoint">
            X3 Atlas Sphere uses a single endpoint (port 9944) for both Substrate native 
            RPC and EVM JSON-RPC. The node automatically routes requests based on the method prefix.
          </Callout>
        </section>

        {/* SDK Configuration */}
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Polkadot.js Configuration</h2>
          <p className="text-gray-400 mb-4">
            Connect using @polkadot/api for native Substrate access:
          </p>
          <CodeBlock language="typescript" title="substrate-client.ts">
{`import { ApiPromise, WsProvider } from '@polkadot/api';

// Connect to X3 Atlas Sphere
const wsProvider = new WsProvider('ws://rpc.testnet.atlas-sphere.io:9944');
const api = await ApiPromise.create({ provider: wsProvider });

// Wait for ready
await api.isReady;

// Query chain info
const chain = await api.rpc.system.chain();
const lastBlock = await api.rpc.chain.getBlock();
console.log(\`Connected to \${chain}, block #\${lastBlock.block.header.number}\`);

// Query canonical ledger (X3 balance)
// Asset ID 0 = X3 (native gas token)
const balance = await api.query.atlasKernel.canonicalLedger(
  accountId,
  0 // Asset ID for X3
);
console.log('X3 Balance:', balance.toHuman());`}
          </CodeBlock>
        </section>

        {/* ethers.js Configuration */}
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">ethers.js Configuration</h2>
          <p className="text-gray-400 mb-4">
            Connect using ethers.js for EVM access:
          </p>
          <CodeBlock language="typescript" title="evm-client.ts">
{`import { ethers } from 'ethers';

// Connect to X3 Atlas Sphere EVM
const provider = new ethers.JsonRpcProvider(
  'http://rpc.testnet.atlas-sphere.io:9944'
);

// Get current block
const blockNumber = await provider.getBlockNumber();
console.log('Current EVM block:', blockNumber);

// Get X3 balance (native gas token)
const balance = await provider.getBalance('0xYourAddress...');
console.log('X3 Balance:', ethers.formatEther(balance), 'X3');

// Create a signer for transactions
const wallet = new ethers.Wallet(privateKey, provider);`}
          </CodeBlock>
        </section>

        {/* Environment Variables */}
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Environment Variables</h2>
          <p className="text-gray-400 mb-4">
            Create a <code className="text-orange-400">.env</code> file in your project root:
          </p>
          <CodeBlock language="bash" title=".env">
{`# X3 Atlas Sphere Network Configuration
X3_RPC_URL=http://rpc.testnet.atlas-sphere.io:9944
X3_WS_URL=ws://rpc.testnet.atlas-sphere.io:9944

# EVM Chain ID for X3 Atlas Sphere
X3_EVM_CHAIN_ID=2151

# For local development
LOCAL_RPC_URL=http://127.0.0.1:9944
LOCAL_WS_URL=ws://127.0.0.1:9944

# Development Keys (NEVER use in production!)
# Alice (Substrate dev account)
DEV_SEED_PHRASE="bottom drive obey lake curtain smoke basket hold race lonely fit walk//Alice"

# EVM dev account
DEV_PRIVATE_KEY=0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80`}
          </CodeBlock>

          <Callout type="danger" title="Security Warning">
            Never commit your <code>.env</code> file to version control. Add it to your 
            <code>.gitignore</code> immediately. Never use development seed phrases or 
            private keys for real assets.
          </Callout>
        </section>

        {/* Hardhat Configuration */}
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Hardhat Configuration</h2>
          <p className="text-gray-400 mb-4">
            Configure Hardhat to work with X3 Atlas Sphere EVM:
          </p>
          <CodeBlock language="typescript" title="hardhat.config.ts">
{`import { HardhatUserConfig } from "hardhat/config";
import "@nomicfoundation/hardhat-toolbox";
import * as dotenv from "dotenv";

dotenv.config();

const config: HardhatUserConfig = {
  solidity: {
    version: "0.8.20",
    settings: {
      optimizer: {
        enabled: true,
        runs: 200,
      },
    },
  },
  networks: {
    // Local X3 node
    x3Local: {
      url: process.env.LOCAL_RPC_URL || "http://127.0.0.1:9944",
      chainId: 2151,
      accounts: process.env.DEV_PRIVATE_KEY 
        ? [process.env.DEV_PRIVATE_KEY] 
        : [],
    },
    // X3 Atlas Testnet
    x3Testnet: {
      url: process.env.X3_RPC_URL || "http://rpc.testnet.atlas-sphere.io:9944",
      chainId: 2151,
      accounts: process.env.DEV_PRIVATE_KEY 
        ? [process.env.DEV_PRIVATE_KEY] 
        : [],
    },
  },
};

export default config;`}
          </CodeBlock>
        </section>

        {/* Wallet Configuration */}
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Wallet Configuration</h2>
          <p className="text-gray-400 mb-4">
            Configure MetaMask to connect to X3 Atlas Sphere:
          </p>
          <div className="glass-card p-6">
            <h3 className="font-semibold text-white mb-4">Add X3 Atlas Testnet to MetaMask</h3>
            <div className="space-y-3 text-sm">
              <div className="flex justify-between py-2 border-b border-[#1a1a1a]">
                <span className="text-gray-500">Network Name</span>
                <span className="text-white font-mono">X3 Atlas Testnet</span>
              </div>
              <div className="flex justify-between py-2 border-b border-[#1a1a1a]">
                <span className="text-gray-500">RPC URL</span>
                <span className="text-cyan-400 font-mono">http://rpc.testnet.atlas-sphere.io:9944</span>
              </div>
              <div className="flex justify-between py-2 border-b border-[#1a1a1a]">
                <span className="text-gray-500">Chain ID</span>
                <span className="text-white font-mono">2151</span>
              </div>
              <div className="flex justify-between py-2 border-b border-[#1a1a1a]">
                <span className="text-gray-500">Currency Symbol</span>
                <span className="text-orange-400 font-mono">X3</span>
              </div>
              <div className="flex justify-between py-2">
                <span className="text-gray-500">Block Explorer</span>
                <span className="text-cyan-400 font-mono">https://explorer.testnet.atlas-sphere.io</span>
              </div>
            </div>
          </div>
          
          <Callout type="info" title="Faucet Available">
            Get testnet X3 tokens from the faucet at{' '}
            <a href="https://faucet.testnet.atlas-sphere.io" className="text-orange-400 hover:underline">
              faucet.testnet.atlas-sphere.io
            </a>
          </Callout>
        </section>

        {/* Atlas Kernel RPC Methods */}
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Atlas Kernel RPC Methods</h2>
          <p className="text-gray-400 mb-4">
            Custom RPC methods for interacting with the Atlas Kernel:
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
                  <td className="py-3 px-4 text-gray-400">Get balance from canonical ledger</td>
                </tr>
                <tr className="border-b border-[#111111]">
                  <td className="py-3 px-4 text-orange-400 font-mono text-sm">atlasKernel_isAuthorized</td>
                  <td className="py-3 px-4 text-gray-400">Check if account is authorized for Comits</td>
                </tr>
                <tr className="border-b border-[#111111]">
                  <td className="py-3 px-4 text-orange-400 font-mono text-sm">atlasKernel_getAssetMetadata</td>
                  <td className="py-3 px-4 text-gray-400">Get asset info (symbol, decimals)</td>
                </tr>
                <tr className="border-b border-[#111111]">
                  <td className="py-3 px-4 text-orange-400 font-mono text-sm">atlasKernel_getAuthorizedAccounts</td>
                  <td className="py-3 px-4 text-gray-400">List all authorized accounts</td>
                </tr>
              </tbody>
            </table>
          </div>
        </section>
      </div>
    </DocLayout>
  );
}
