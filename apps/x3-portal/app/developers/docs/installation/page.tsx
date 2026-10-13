'use client';

import DocLayout, { CodeBlock, Callout } from '@/components/docs/DocLayout';
import { CheckCircle } from 'lucide-react';

export default function InstallationPage() {
  return (
    <DocLayout 
      title="Installation" 
      description="Install all dependencies and tools for X3 Atlas Sphere development"
      lastUpdated="December 2024"
    >
      <div className="space-y-8">
        {/* System Requirements */}
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">System Requirements</h2>
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="border-b border-[#1a1a1a]">
                  <th className="text-left py-3 px-4 text-gray-500 font-medium">Component</th>
                  <th className="text-left py-3 px-4 text-gray-500 font-medium">Minimum</th>
                  <th className="text-left py-3 px-4 text-gray-500 font-medium">Recommended</th>
                </tr>
              </thead>
              <tbody>
                <tr className="border-b border-[#111111]">
                  <td className="py-3 px-4 text-white">Node.js</td>
                  <td className="py-3 px-4 text-gray-400">18.x</td>
                  <td className="py-3 px-4 text-gray-400">20.x LTS</td>
                </tr>
                <tr className="border-b border-[#111111]">
                  <td className="py-3 px-4 text-white">Rust</td>
                  <td className="py-3 px-4 text-gray-400">1.74+</td>
                  <td className="py-3 px-4 text-gray-400">1.75+ (nightly)</td>
                </tr>
                <tr className="border-b border-[#111111]">
                  <td className="py-3 px-4 text-white">Memory</td>
                  <td className="py-3 px-4 text-gray-400">4 GB</td>
                  <td className="py-3 px-4 text-gray-400">8 GB+</td>
                </tr>
                <tr className="border-b border-[#111111]">
                  <td className="py-3 px-4 text-white">Storage</td>
                  <td className="py-3 px-4 text-gray-400">20 GB</td>
                  <td className="py-3 px-4 text-gray-400">50 GB+ SSD</td>
                </tr>
              </tbody>
            </table>
          </div>
        </section>

        {/* Install Node.js */}
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Install Node.js</h2>
          <p className="text-gray-400 mb-4">
            We recommend using nvm (Node Version Manager) to install Node.js:
          </p>
          <CodeBlock language="bash" title="Terminal">
{`# Install nvm (Linux/macOS)
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash

# Restart terminal, then install Node.js 20
nvm install 20
nvm use 20

# Verify installation
node --version  # Should output v20.x.x
npm --version   # Should output 10.x.x`}
          </CodeBlock>
        </section>

        {/* Install Rust */}
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Install Rust (Optional)</h2>
          <p className="text-gray-400 mb-4">
            Required only if you plan to run a node or build Substrate pallets:
          </p>
          <CodeBlock language="bash" title="Terminal">
{`# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add to PATH (restart terminal or run)
source $HOME/.cargo/env

# Install nightly toolchain and WASM target
rustup default stable
rustup update nightly
rustup target add wasm32-unknown-unknown --toolchain nightly

# Verify installation
rustc --version
cargo --version`}
          </CodeBlock>
        </section>

        {/* Install SDK */}
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Install X3 Atlas SDK</h2>
          <p className="text-gray-400 mb-4">
            Install the JavaScript/TypeScript SDK for interacting with X3 Atlas Sphere:
          </p>
          <CodeBlock language="bash" title="Terminal">
{`# Create a new project
mkdir my-x3-app && cd my-x3-app
npm init -y

# Install dependencies
npm install @x3/atlas-sdk @polkadot/api @polkadot/keyring

# For TypeScript projects
npm install -D typescript @types/node ts-node
npx tsc --init`}
          </CodeBlock>
        </section>

        {/* EVM Development */}
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">EVM Development Tools</h2>
          <p className="text-gray-400 mb-4">
            For EVM smart contract development, install Hardhat or Foundry:
          </p>
          
          <h3 className="text-lg font-semibold text-white mb-2">Hardhat</h3>
          <CodeBlock language="bash" title="Terminal">
{`# Install Hardhat
npm install -D hardhat @nomicfoundation/hardhat-toolbox

# Initialize Hardhat project
npx hardhat init

# Install ethers.js
npm install ethers@6`}
          </CodeBlock>

          <h3 className="text-lg font-semibold text-white mt-6 mb-2">Foundry</h3>
          <CodeBlock language="bash" title="Terminal">
{`# Install Foundry
curl -L https://foundry.paradigm.xyz | bash
foundryup

# Verify installation
forge --version
cast --version
anvil --version`}
          </CodeBlock>
        </section>

        {/* SVM Development */}
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">SVM Development Tools</h2>
          <p className="text-gray-400 mb-4">
            For Solana program development, install Anchor:
          </p>
          <CodeBlock language="bash" title="Terminal">
{`# Install Solana CLI
sh -c "$(curl -sSfL https://release.solana.com/v1.18.0/install)"

# Add to PATH
export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"

# Install Anchor
cargo install --git https://github.com/coral-xyz/anchor avm --locked
avm install latest
avm use latest

# Verify installation
solana --version
anchor --version`}
          </CodeBlock>
        </section>

        {/* Verify Installation */}
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Verify Installation</h2>
          <p className="text-gray-400 mb-4">
            Run this script to verify all tools are installed correctly:
          </p>
          <CodeBlock language="typescript" title="verify.ts">
{`import { AtlasClient } from '@x3/atlas-sdk';

async function verify() {
  console.log('Verifying X3 Atlas Sphere setup...\\n');

  // Test SDK connection
  const client = new AtlasClient({
    rpcUrl: 'https://rpc.testnet.atlas-sphere.io',
  });

  const info = await client.getNetworkInfo();
  console.log('✅ Connected to X3 Atlas Sphere');
  console.log('   Chain:', info.chain);
  console.log('   Block:', info.blockNumber);
  console.log('\\nSetup complete! You are ready to build.');
}

verify().catch(console.error);`}
          </CodeBlock>

          <p className="text-gray-400 mt-4">Run with:</p>
          <CodeBlock language="bash" title="Terminal">
{`npx ts-node verify.ts`}
          </CodeBlock>
        </section>

        <Callout type="success" title="You're All Set!">
          With all tools installed, you&apos;re ready to start building on X3 Atlas Sphere. 
          Head to the Quick Start guide to create your first application.
        </Callout>
      </div>
    </DocLayout>
  );
}
