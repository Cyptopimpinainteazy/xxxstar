'use client';

import React from 'react';
import DocLayout, { CodeBlock, Callout } from '@/components/docs/DocLayout';

export default function ChainSpecPage() {
  return (
    <DocLayout
      title="Chain Specification"
      description="Understanding and customizing X3 Atlas Sphere chain specs"
    >
      <p className="lead text-xl text-gray-400 mb-8">
        Chain specifications define the genesis state and network parameters for X3 Atlas Sphere.
        Learn how to use built-in specs or create custom ones.
      </p>

      <h2>Built-in Chain Specs</h2>
      <CodeBlock language="bash">
{`# List available chain specs
./atlas-sphere-node build-spec --list

# Available specs:
# - dev       : Single-node development
# - local     : Multi-node local testnet
# - testnet   : Public testnet
# - mainnet   : Production network (future)`}
      </CodeBlock>

      <h2>Using a Chain Spec</h2>
      <CodeBlock language="bash">
{`# Start with built-in spec
./atlas-sphere-node --chain testnet

# Start with custom spec file
./atlas-sphere-node --chain /path/to/custom-spec.json

# Start with raw spec (for production)
./atlas-sphere-node --chain /path/to/custom-spec-raw.json`}
      </CodeBlock>

      <h2>Generating Chain Specs</h2>
      <CodeBlock language="bash">
{`# Generate plain spec (readable JSON)
./atlas-sphere-node build-spec \\
  --chain testnet \\
  --disable-default-bootnode \\
  > custom-spec.json

# Convert to raw spec (production)
./atlas-sphere-node build-spec \\
  --chain custom-spec.json \\
  --raw \\
  > custom-spec-raw.json`}
      </CodeBlock>

      <h2>Chain Spec Structure</h2>
      <CodeBlock language="json" title="chain-spec.json">
{`{
  "name": "X3 Atlas Sphere Testnet",
  "id": "atlas_testnet",
  "chainType": "Live",
  "bootNodes": [
    "/dns/boot1.testnet.atlas-sphere.io/tcp/30333/p2p/12D3KooW..."
  ],
  "telemetryEndpoints": [
    ["wss://telemetry.atlas-sphere.io/submit", 0]
  ],
  "protocolId": "atlas",
  "properties": {
    "tokenSymbol": "ATLAS",
    "tokenDecimals": 18,
    "ss58Format": 42
  },
  "genesis": {
    "runtime": {
      "system": {
        "code": "0x..."
      },
      "balances": {
        "balances": [
          ["5GrwvaEF...", 1000000000000000000000000],
          ["5FHneW46...", 1000000000000000000000000]
        ]
      },
      "aura": {
        "authorities": [
          "5GrwvaEF...",
          "5FHneW46..."
        ]
      },
      "grandpa": {
        "authorities": [
          ["5FA9nQDV...", 1],
          ["5GoNkf6W...", 1]
        ]
      },
      "sudo": {
        "key": "5GrwvaEF..."
      },
      "atlasKernel": {
        "authorizedAccounts": [],
        "registeredAssets": []
      }
    }
  }
}`}
      </CodeBlock>

      <h2>Customizing Genesis</h2>

      <h3>Initial Balances</h3>
      <CodeBlock language="json">
{`"balances": {
  "balances": [
    ["5GrwvaEF...", "1000000000000000000000000"],
    ["5FHneW46...", "500000000000000000000000"],
    ["5DAAnrj7...", "250000000000000000000000"]
  ]
}`}
      </CodeBlock>

      <h3>Initial Validators</h3>
      <CodeBlock language="json">
{`"aura": {
  "authorities": [
    "5GrwvaEF...",  // Validator 1 Aura key
    "5FHneW46...",  // Validator 2 Aura key
    "5DAAnrj7..."   // Validator 3 Aura key
  ]
},
"grandpa": {
  "authorities": [
    ["5FA9nQDV...", 1],  // Validator 1 GRANDPA key, weight 1
    ["5GoNkf6W...", 1],  // Validator 2 GRANDPA key, weight 1
    ["5CiPPseX...", 1]   // Validator 3 GRANDPA key, weight 1
  ]
}`}
      </CodeBlock>

      <h3>Atlas Kernel Configuration</h3>
      <CodeBlock language="json">
{`"atlasKernel": {
  "authorizedAccounts": [
    "5GrwvaEF...",  // Pre-authorized for Comits
    "5FHneW46..."
  ],
  "registeredAssets": [
    {
      "id": "0x01000000...",
      "symbol": "USDC",
      "decimals": 6
    }
  ]
}`}
      </CodeBlock>

      <h2>Bootnodes</h2>
      <CodeBlock language="json">
{`"bootNodes": [
  "/dns/boot1.testnet.atlas-sphere.io/tcp/30333/p2p/12D3KooWEyoppNCUVQCNiY8bDYwLTK3YuEMhrGrXE8m5eDVqS3Rb",
  "/dns/boot2.testnet.atlas-sphere.io/tcp/30333/p2p/12D3KooWHdiAxVd8uMQR1hGWXccidmfCwLqcMpGwR6QcTP6QRMuD",
  "/ip4/192.168.1.100/tcp/30333/p2p/12D3KooWBmAwcd4PJNJvfV89HwE48nwkRmAgo8Vy3uQEyNNHBox2"
]`}
      </CodeBlock>

      <h2>Local Testnet Setup</h2>
      <CodeBlock language="bash">
{`# Generate spec for local 3-node testnet
./atlas-sphere-node build-spec \\
  --chain local \\
  --disable-default-bootnode \\
  > local-testnet.json

# Edit local-testnet.json to add your validators

# Convert to raw
./atlas-sphere-node build-spec \\
  --chain local-testnet.json \\
  --raw \\
  > local-testnet-raw.json

# Start nodes
# Node 1 (Alice)
./atlas-sphere-node \\
  --chain local-testnet-raw.json \\
  --alice \\
  --port 30333 \\
  --node-key 0000000000000000000000000000000000000000000000000000000000000001

# Node 2 (Bob) 
./atlas-sphere-node \\
  --chain local-testnet-raw.json \\
  --bob \\
  --port 30334 \\
  --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooW...`}
      </CodeBlock>

      <Callout type="info" title="Raw vs Plain Specs">
        Use raw specs in production. They contain pre-computed storage values and 
        load faster. Plain specs are for human editing during development.
      </Callout>
    </DocLayout>
  );
}
