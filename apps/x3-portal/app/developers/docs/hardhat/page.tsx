'use client';

import React from 'react';
import DocLayout, { CodeBlock, Callout } from '@/components/docs/DocLayout';

export default function HardhatGuidePage() {
  return (
    <DocLayout
      title="Hardhat Development Guide"
      description="Complete guide to using Hardhat for X3 Atlas Sphere development"
    >
      <p className="lead text-xl text-gray-400 mb-8">
        Hardhat is the recommended development environment for building EVM smart contracts
        on X3 Atlas Sphere. This guide covers setup, testing, and deployment workflows.
      </p>

      <h2>Project Setup</h2>
      <CodeBlock language="bash">
{`# Create new project
mkdir my-x3-project && cd my-x3-project
npm init -y

# Install Hardhat and dependencies
npm install --save-dev hardhat @nomicfoundation/hardhat-toolbox
npm install --save-dev @openzeppelin/contracts

# Initialize Hardhat
npx hardhat init
# Select "Create a TypeScript project"`}
      </CodeBlock>

      <h2>Hardhat Configuration</h2>
      <CodeBlock language="typescript" title="hardhat.config.ts">
{`import { HardhatUserConfig } from 'hardhat/config';
import '@nomicfoundation/hardhat-toolbox';
import * as dotenv from 'dotenv';

dotenv.config();

const config: HardhatUserConfig = {
  solidity: {
    version: '0.8.20',
    settings: {
      optimizer: {
        enabled: true,
        runs: 200,
      },
    },
  },
  networks: {
    hardhat: {
      chainId: 5330,
    },
    x3Local: {
      url: 'http://127.0.0.1:9933',
      chainId: 5330,
      accounts: [
        // Dev account - DO NOT use in production
        '0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80',
      ],
    },
    x3Testnet: {
      url: 'https://rpc.testnet.atlas-sphere.io',
      chainId: 5330,
      accounts: process.env.PRIVATE_KEY ? [process.env.PRIVATE_KEY] : [],
      gasPrice: 1000000000,
    },
  },
  etherscan: {
    apiKey: {
      x3Testnet: process.env.EXPLORER_API_KEY || '',
    },
    customChains: [
      {
        network: 'x3Testnet',
        chainId: 5330,
        urls: {
          apiURL: 'https://api.explorer.testnet.atlas-sphere.io/api',
          browserURL: 'https://explorer.testnet.atlas-sphere.io',
        },
      },
    ],
  },
};

export default config;`}
      </CodeBlock>

      <h2>Writing Tests</h2>
      <CodeBlock language="typescript" title="test/Token.test.ts">
{`import { expect } from 'chai';
import { ethers } from 'hardhat';
import { loadFixture } from '@nomicfoundation/hardhat-toolbox/network-helpers';

describe('AtlasToken', function () {
  async function deployTokenFixture() {
    const [owner, addr1, addr2] = await ethers.getSigners();
    const initialSupply = ethers.parseEther('1000000');

    const AtlasToken = await ethers.getContractFactory('AtlasToken');
    const token = await AtlasToken.deploy(initialSupply);

    return { token, owner, addr1, addr2, initialSupply };
  }

  describe('Deployment', function () {
    it('Should assign total supply to owner', async function () {
      const { token, owner, initialSupply } = await loadFixture(deployTokenFixture);
      expect(await token.balanceOf(owner.address)).to.equal(initialSupply);
    });

    it('Should have correct name and symbol', async function () {
      const { token } = await loadFixture(deployTokenFixture);
      expect(await token.name()).to.equal('Atlas Token');
      expect(await token.symbol()).to.equal('ATL');
    });
  });

  describe('Transfers', function () {
    it('Should transfer tokens between accounts', async function () {
      const { token, owner, addr1 } = await loadFixture(deployTokenFixture);
      const amount = ethers.parseEther('100');

      await token.transfer(addr1.address, amount);
      expect(await token.balanceOf(addr1.address)).to.equal(amount);
    });

    it('Should emit Transfer event', async function () {
      const { token, owner, addr1 } = await loadFixture(deployTokenFixture);
      const amount = ethers.parseEther('100');

      await expect(token.transfer(addr1.address, amount))
        .to.emit(token, 'Transfer')
        .withArgs(owner.address, addr1.address, amount);
    });

    it('Should fail if sender has insufficient balance', async function () {
      const { token, addr1, addr2 } = await loadFixture(deployTokenFixture);
      const amount = ethers.parseEther('100');

      await expect(
        token.connect(addr1).transfer(addr2.address, amount)
      ).to.be.revertedWithCustomError(token, 'ERC20InsufficientBalance');
    });
  });
});`}
      </CodeBlock>

      <h2>Running Tests</h2>
      <CodeBlock language="bash">
{`# Run all tests
npx hardhat test

# Run with gas reporting
REPORT_GAS=true npx hardhat test

# Run specific test file
npx hardhat test test/Token.test.ts

# Run with coverage
npx hardhat coverage`}
      </CodeBlock>

      <h2>Deployment Scripts</h2>
      <CodeBlock language="typescript" title="scripts/deploy.ts">
{`import { ethers, run, network } from 'hardhat';

async function main() {
  const [deployer] = await ethers.getSigners();
  console.log('Deploying contracts with:', deployer.address);
  console.log('Account balance:', ethers.formatEther(await ethers.provider.getBalance(deployer.address)));

  // Deploy token
  const AtlasToken = await ethers.getContractFactory('AtlasToken');
  const initialSupply = ethers.parseEther('1000000');
  const token = await AtlasToken.deploy(initialSupply);
  await token.waitForDeployment();
  
  const tokenAddress = await token.getAddress();
  console.log('AtlasToken deployed to:', tokenAddress);

  // Verify on explorer (if not local)
  if (network.name !== 'hardhat' && network.name !== 'x3Local') {
    console.log('Waiting for block confirmations...');
    await token.deploymentTransaction()?.wait(5);
    
    console.log('Verifying contract...');
    await run('verify:verify', {
      address: tokenAddress,
      constructorArguments: [initialSupply],
    });
  }
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});`}
      </CodeBlock>

      <Callout type="info" title="Environment Variables">
        Create a <code>.env</code> file with your private key and API keys:
        <pre className="mt-2 text-xs">
{`PRIVATE_KEY=your_private_key_here
EXPLORER_API_KEY=your_api_key`}
        </pre>
      </Callout>

      <h2>Useful Hardhat Tasks</h2>
      <CodeBlock language="bash">
{`# Compile contracts
npx hardhat compile

# Clean artifacts
npx hardhat clean

# Check contract sizes
npx hardhat compile && npx hardhat size-contracts

# Run local node (with X3 chain ID)
npx hardhat node

# Deploy to specific network
npx hardhat run scripts/deploy.ts --network x3Testnet

# Verify existing contract
npx hardhat verify --network x3Testnet CONTRACT_ADDRESS "constructor_arg1"`}
      </CodeBlock>
    </DocLayout>
  );
}
