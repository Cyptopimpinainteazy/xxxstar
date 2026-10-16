'use client';

import React from 'react';
import DocLayout, { CodeBlock, Callout } from '@/components/docs/DocLayout';

export default function EvmDeployPage() {
  return (
    <DocLayout
      title="Deploying EVM Contracts"
      description="Deploy smart contracts to X3 Atlas Sphere's EVM environment"
    >
      <p className="lead text-xl text-gray-400 mb-8">
        Deploy your Solidity smart contracts to X3 Atlas Sphere using familiar EVM tools.
        Our EVM environment is fully compatible with existing Ethereum development workflows.
      </p>

      <h2>Deployment Options</h2>
      <p>
        You can deploy contracts to X3 using several methods:
      </p>
      <ul>
        <li><strong>Hardhat</strong> - Recommended for most projects</li>
        <li><strong>Foundry</strong> - For Rust-preferring developers</li>
        <li><strong>Remix IDE</strong> - Quick deployments via browser</li>
        <li><strong>Direct RPC</strong> - Using ethers.js or web3.js</li>
      </ul>

      <h2>Network Configuration</h2>
      <CodeBlock language="typescript" title="hardhat.config.ts">
{`import { HardhatUserConfig } from 'hardhat/config';

const config: HardhatUserConfig = {
  solidity: '0.8.20',
  networks: {
    x3Testnet: {
      url: 'https://rpc.testnet.atlas-sphere.io',
      chainId: 5330,
      accounts: [process.env.PRIVATE_KEY!],
      gasPrice: 1000000000, // 1 gwei
    },
    x3Local: {
      url: 'http://127.0.0.1:9933',
      chainId: 5330,
      accounts: ['0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80'],
    },
  },
};

export default config;`}
      </CodeBlock>

      <h2>Simple Contract Deployment</h2>
      <p>
        Here's a simple ERC20 token deployment script:
      </p>
      <CodeBlock language="solidity" title="contracts/AtlasToken.sol">
{`// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";

contract AtlasToken is ERC20 {
    constructor(uint256 initialSupply) ERC20("Atlas Token", "ATL") {
        _mint(msg.sender, initialSupply);
    }
}`}
      </CodeBlock>

      <CodeBlock language="typescript" title="scripts/deploy.ts">
{`import { ethers } from 'hardhat';

async function main() {
  const [deployer] = await ethers.getSigners();
  console.log('Deploying with account:', deployer.address);

  const AtlasToken = await ethers.getContractFactory('AtlasToken');
  const initialSupply = ethers.parseEther('1000000');
  
  const token = await AtlasToken.deploy(initialSupply);
  await token.waitForDeployment();

  console.log('AtlasToken deployed to:', await token.getAddress());
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});`}
      </CodeBlock>

      <h2>Deploy Command</h2>
      <CodeBlock language="bash">
{`# Deploy to testnet
npx hardhat run scripts/deploy.ts --network x3Testnet

# Deploy to local node
npx hardhat run scripts/deploy.ts --network x3Local`}
      </CodeBlock>

      <Callout type="info" title="Gas Prices">
        X3 uses a fixed gas price of 1 gwei on testnet. Gas is paid in ATLAS tokens.
        Use the faucet to get test tokens.
      </Callout>

      <h2>Verifying Deployment</h2>
      <p>
        After deployment, verify your contract in the X3 Explorer:
      </p>
      <CodeBlock language="typescript" title="scripts/verify.ts">
{`import { run } from 'hardhat';

async function verify(address: string, constructorArgs: any[]) {
  await run('verify:verify', {
    address,
    constructorArguments: constructorArgs,
  });
}

// Example: verify('0x...', [ethers.parseEther('1000000')])`}
      </CodeBlock>

      <h2>Contract Size Limits</h2>
      <p>
        X3 EVM supports the standard Ethereum contract size limits:
      </p>
      <ul>
        <li>Maximum contract size: 24KB (EIP-170)</li>
        <li>Maximum initcode: 48KB (EIP-3860)</li>
        <li>Block gas limit: 30 million</li>
      </ul>

      <Callout type="info" title="Cross-VM Contracts">
        For contracts that interact with SVM programs via Comits, see the 
        <a href="/developers/docs/creating-comits" className="text-orange-400 hover:text-orange-300 ml-1">Creating Comits</a> guide.
      </Callout>
    </DocLayout>
  );
}
