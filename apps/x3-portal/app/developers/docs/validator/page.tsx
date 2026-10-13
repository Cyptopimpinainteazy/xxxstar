'use client';

import React from 'react';
import DocLayout, { CodeBlock, Callout } from '@/components/docs/DocLayout';

export default function ValidatorPage() {
  return (
    <DocLayout
      title="Run a Validator"
      description="Become a validator on X3 Atlas Sphere network"
    >
      <p className="lead text-xl text-gray-400 mb-8">
        Validators secure the X3 Atlas Sphere network by producing blocks and participating 
        in consensus. This guide covers validator setup and operation.
      </p>

      <Callout type="warning" title="Testnet Only">
        Currently, validator slots are limited to authorized participants on testnet.
        Mainnet will feature permissionless validation.
      </Callout>

      <h2>System Requirements</h2>
      <ul>
        <li><strong>CPU</strong>: 8+ cores, 3.0 GHz</li>
        <li><strong>RAM</strong>: 32 GB minimum, 64 GB recommended</li>
        <li><strong>Storage</strong>: 1 TB NVMe SSD</li>
        <li><strong>Network</strong>: 100 Mbps symmetric, low latency</li>
        <li><strong>OS</strong>: Ubuntu 22.04 LTS</li>
      </ul>

      <h2>Installation</h2>
      <CodeBlock language="bash">
{`# Install dependencies
sudo apt update && sudo apt install -y build-essential git clang curl libssl-dev

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
rustup target add wasm32-unknown-unknown

# Clone and build
git clone https://github.com/Cyptopimpinainteazy/atlas-sphere.git
cd atlas-sphere
cargo build --release

# Verify binary
./target/release/atlas-sphere-node --version`}
      </CodeBlock>

      <h2>Generate Validator Keys</h2>
      <CodeBlock language="bash">
{`# Generate session keys
./target/release/atlas-sphere-node key generate --scheme Sr25519 \\
  --output-type json > aura-key.json

./target/release/atlas-sphere-node key generate --scheme Ed25519 \\
  --output-type json > grandpa-key.json

# Secure the keys
chmod 600 aura-key.json grandpa-key.json
cp *.json /secure/backup/location/

# Insert keys into keystore
./target/release/atlas-sphere-node key insert \\
  --base-path /var/lib/atlas-validator \\
  --chain testnet \\
  --scheme Sr25519 \\
  --suri "$(cat aura-key.json | jq -r '.secretPhrase')" \\
  --key-type aura

./target/release/atlas-sphere-node key insert \\
  --base-path /var/lib/atlas-validator \\
  --chain testnet \\
  --scheme Ed25519 \\
  --suri "$(cat grandpa-key.json | jq -r '.secretPhrase')" \\
  --key-type gran`}
      </CodeBlock>

      <h2>Validator Configuration</h2>
      <CodeBlock language="bash" title="/etc/atlas-validator/config.yaml">
{`# Validator configuration
base_path: /var/lib/atlas-validator
chain: testnet
name: my-validator-01
validator: true

# Network
port: 30333
rpc_port: 9933
ws_port: 9944
rpc_external: false
ws_external: false

# Performance
execution: wasm
wasm_execution: compiled
state_pruning: archive

# Telemetry
telemetry_url: "wss://telemetry.atlas-sphere.io/submit 0"

# Prometheus metrics
prometheus_external: true
prometheus_port: 9615`}
      </CodeBlock>

      <h2>Systemd Service</h2>
      <CodeBlock language="ini" title="/etc/systemd/system/atlas-validator.service">
{`[Unit]
Description=X3 Atlas Sphere Validator
After=network.target
Wants=network.target

[Service]
Type=simple
User=atlas
Group=atlas
ExecStart=/usr/local/bin/atlas-sphere-node \\
  --base-path /var/lib/atlas-validator \\
  --chain testnet \\
  --validator \\
  --name "my-validator-01" \\
  --port 30333 \\
  --rpc-port 9933 \\
  --ws-port 9944 \\
  --prometheus-port 9615 \\
  --prometheus-external \\
  --telemetry-url "wss://telemetry.atlas-sphere.io/submit 0"

Restart=always
RestartSec=10

# Hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/atlas-validator

[Install]
WantedBy=multi-user.target`}
      </CodeBlock>

      <CodeBlock language="bash">
{`# Enable and start service
sudo systemctl daemon-reload
sudo systemctl enable atlas-validator
sudo systemctl start atlas-validator

# Check status
sudo systemctl status atlas-validator
sudo journalctl -u atlas-validator -f`}
      </CodeBlock>

      <h2>Register as Validator</h2>
      <p>
        Once your node is synced, register your validator with the network:
      </p>
      <CodeBlock language="typescript">
{`import { ApiPromise, WsProvider, Keyring } from '@polkadot/api';

const api = await ApiPromise.create({ 
  provider: new WsProvider('ws://localhost:9944') 
});

const keyring = new Keyring({ type: 'sr25519' });
const controller = keyring.addFromUri('//Controller');

// Set session keys (from rotateKeys RPC call)
const sessionKeys = await api.rpc.author.rotateKeys();

await api.tx.session
  .setKeys(sessionKeys, '0x')
  .signAndSend(controller);

console.log('Session keys set:', sessionKeys.toHex());`}
      </CodeBlock>

      <h2>Monitoring</h2>
      <CodeBlock language="yaml" title="prometheus.yml">
{`scrape_configs:
  - job_name: 'atlas-validator'
    static_configs:
      - targets: ['localhost:9615']
    
    relabel_configs:
      - source_labels: [__address__]
        target_label: instance
        regex: '(.+):9615'
        replacement: '\${1}'`}
      </CodeBlock>

      <p>Key metrics to monitor:</p>
      <ul>
        <li><code>substrate_block_height</code> - Current block height</li>
        <li><code>substrate_finality_grandpa_round</code> - GRANDPA rounds</li>
        <li><code>substrate_sub_libp2p_peers_count</code> - Connected peers</li>
        <li><code>substrate_tasks_spawned_total</code> - Task health</li>
      </ul>

      <h2>Security Checklist</h2>
      <ul>
        <li>✅ Keys stored in encrypted backup</li>
        <li>✅ Firewall configured (only 30333 public)</li>
        <li>✅ RPC/WS not exposed publicly</li>
        <li>✅ Automatic security updates enabled</li>
        <li>✅ Monitoring and alerting configured</li>
        <li>✅ DDoS protection in place</li>
      </ul>
    </DocLayout>
  );
}
