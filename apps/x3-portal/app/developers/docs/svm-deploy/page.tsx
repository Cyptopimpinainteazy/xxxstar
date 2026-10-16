'use client';

import React from 'react';
import DocLayout, { CodeBlock, Callout } from '@/components/docs/DocLayout';

export default function SvmDeployPage() {
  return (
    <DocLayout
      title="Deploy SVM Programs"
      description="Deploy Solana-compatible programs to X3 Atlas Sphere"
    >
      <p className="lead text-xl text-gray-400 mb-8">
        Deploy your SVM programs to X3 Atlas Sphere using the Solana CLI or Anchor.
        Programs can be deployed to testnet or your local development node.
      </p>

      <h2>Prerequisites</h2>
      <CodeBlock language="bash">
{`# Install Solana CLI
sh -c "$(curl -sSfL https://release.solana.com/stable/install)"

# Configure for X3 testnet
solana config set --url https://rpc.testnet.atlas-sphere.io

# Generate keypair (if needed)
solana-keygen new --outfile ~/.config/solana/x3-keypair.json

# Check balance
solana balance`}
      </CodeBlock>

      <h2>Deploy with Anchor</h2>
      <CodeBlock language="toml" title="Anchor.toml">
{`[features]
seeds = false
skip-lint = false

[programs.testnet]
my_program = "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS"

[registry]
url = "https://api.apr.dev"

[provider]
cluster = "https://rpc.testnet.atlas-sphere.io"
wallet = "~/.config/solana/x3-keypair.json"

[scripts]
test = "yarn run ts-mocha -p ./tsconfig.json -t 1000000 tests/**/*.ts"`}
      </CodeBlock>

      <CodeBlock language="bash">
{`# Build program
anchor build

# Deploy
anchor deploy

# Or deploy specific program
anchor deploy --program-name my_program

# Upgrade existing program
anchor upgrade target/deploy/my_program.so --program-id PROGRAM_ADDRESS`}
      </CodeBlock>

      <h2>Deploy with Solana CLI</h2>
      <CodeBlock language="bash">
{`# Build your program
cargo build-bpf

# Deploy program
solana program deploy target/deploy/my_program.so

# Deploy with specific keypair
solana program deploy target/deploy/my_program.so \\
  --program-id ./keypairs/program-keypair.json

# Check program info
solana program show PROGRAM_ADDRESS`}
      </CodeBlock>

      <h2>Program Upgrades</h2>
      <p>
        SVM programs on X3 can be upgradeable, allowing you to fix bugs and add features:
      </p>
      <CodeBlock language="bash">
{`# Set upgrade authority
solana program set-upgrade-authority PROGRAM_ID --new-upgrade-authority NEW_AUTHORITY

# Upgrade program
solana program deploy target/deploy/my_program.so --program-id PROGRAM_ID

# Make immutable (WARNING: cannot be undone)
solana program set-upgrade-authority PROGRAM_ID --final`}
      </CodeBlock>

      <Callout type="warning" title="Upgrade Authority">
        Store your upgrade authority keypair securely. Losing it means you cannot upgrade 
        the program. Consider using a multisig for production deployments.
      </Callout>

      <h2>Deployment Verification</h2>
      <CodeBlock language="bash">
{`# Verify deployment
solana program show PROGRAM_ADDRESS

# Expected output:
# Program Id: Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS
# Owner: BPFLoaderUpgradeab1e11111111111111111111111
# ProgramData Address: ...
# Authority: YOUR_AUTHORITY_ADDRESS
# Last Deployed In Slot: 12345
# Data Length: 123456 bytes

# Get program account data
solana account PROGRAM_ADDRESS`}
      </CodeBlock>

      <h2>IDL Management</h2>
      <p>
        For Anchor programs, deploy the IDL for client discoverability:
      </p>
      <CodeBlock language="bash">
{`# Initialize IDL account
anchor idl init --filepath target/idl/my_program.json PROGRAM_ID

# Upgrade IDL
anchor idl upgrade --filepath target/idl/my_program.json PROGRAM_ID

# Fetch IDL
anchor idl fetch PROGRAM_ID

# Close IDL account
anchor idl close PROGRAM_ID`}
      </CodeBlock>

      <h2>Common Issues</h2>

      <h3>Insufficient Balance</h3>
      <CodeBlock language="bash">
{`# Get testnet tokens from faucet
# Visit: https://faucet.testnet.atlas-sphere.io

# Or use CLI
curl -X POST https://faucet.testnet.atlas-sphere.io/api/request \\
  -H "Content-Type: application/json" \\
  -d '{"address": "YOUR_ADDRESS"}'`}
      </CodeBlock>

      <h3>Program Size</h3>
      <p>
        Programs have size limits. Optimize by:
      </p>
      <ul>
        <li>Using <code>cargo build-bpf</code> with release mode</li>
        <li>Removing unused dependencies</li>
        <li>Splitting into multiple programs</li>
      </ul>

      <h2>Production Checklist</h2>
      <ul>
        <li>✅ Complete security audit</li>
        <li>✅ Test thoroughly on testnet</li>
        <li>✅ Set up multisig upgrade authority</li>
        <li>✅ Document program ID and IDL</li>
        <li>✅ Verify program on explorer</li>
        <li>✅ Monitor program usage and errors</li>
      </ul>
    </DocLayout>
  );
}
