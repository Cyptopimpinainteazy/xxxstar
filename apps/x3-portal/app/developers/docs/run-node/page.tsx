'use client';

import DocLayout, { CodeBlock, Callout } from '@/components/docs/DocLayout';
import { Server, Terminal, Database, Shield } from 'lucide-react';

export default function RunNodePage() {
  return (
    <DocLayout 
      title="Running a Node" 
      description="Start your own X3 Atlas Sphere node"
      lastUpdated="December 2024"
    >
      <div className="space-y-8">
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Prerequisites</h2>
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="border-b border-[#1a1a1a]">
                  <th className="text-left py-3 px-4 text-gray-500 font-medium">Component</th>
                  <th className="text-left py-3 px-4 text-gray-500 font-medium">Requirement</th>
                </tr>
              </thead>
              <tbody>
                <tr className="border-b border-[#111111]">
                  <td className="py-3 px-4 text-white">OS</td>
                  <td className="py-3 px-4 text-gray-400">Ubuntu 22.04+ / Debian 12+ / macOS 13+</td>
                </tr>
                <tr className="border-b border-[#111111]">
                  <td className="py-3 px-4 text-white">CPU</td>
                  <td className="py-3 px-4 text-gray-400">4+ cores (8+ recommended)</td>
                </tr>
                <tr className="border-b border-[#111111]">
                  <td className="py-3 px-4 text-white">RAM</td>
                  <td className="py-3 px-4 text-gray-400">8 GB minimum (16 GB recommended)</td>
                </tr>
                <tr className="border-b border-[#111111]">
                  <td className="py-3 px-4 text-white">Storage</td>
                  <td className="py-3 px-4 text-gray-400">100 GB SSD (NVMe recommended)</td>
                </tr>
                <tr className="border-b border-[#111111]">
                  <td className="py-3 px-4 text-white">Network</td>
                  <td className="py-3 px-4 text-gray-400">100 Mbps+ stable connection</td>
                </tr>
              </tbody>
            </table>
          </div>
        </section>

        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Quick Start (Docker)</h2>
          <CodeBlock language="bash" title="Docker Setup">
{`# Pull the latest image
docker pull atlassphere/node:latest

# Run a full node
docker run -d \\
  --name atlas-node \\
  -p 9933:9933 \\
  -p 9944:9944 \\
  -p 30333:30333 \\
  -v atlas-data:/data \\
  atlassphere/node:latest \\
  --chain testnet \\
  --name "My Atlas Node" \\
  --rpc-cors all \\
  --rpc-external \\
  --ws-external

# Check logs
docker logs -f atlas-node`}
          </CodeBlock>
        </section>

        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Building from Source</h2>
          <CodeBlock language="bash" title="Build Instructions">
{`# Clone the repository
git clone https://github.com/atlas-sphere/atlas-sphere.git
cd atlas-sphere

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Add WASM target
rustup target add wasm32-unknown-unknown

# Build the node
cargo build --release

# Verify build
./target/release/atlas-sphere-node --version`}
          </CodeBlock>
        </section>

        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Running the Node</h2>
          <h3 className="text-lg font-semibold text-white mt-6 mb-3">Development Node</h3>
          <CodeBlock language="bash" title="Dev Node">
{`# Run a development node with temporary storage
./target/release/atlas-sphere-node --dev

# Or use the convenience script
./run-dev-node.sh`}
          </CodeBlock>

          <h3 className="text-lg font-semibold text-white mt-6 mb-3">Testnet Node</h3>
          <CodeBlock language="bash" title="Testnet Node">
{`./target/release/atlas-sphere-node \\
  --chain testnet \\
  --name "My Testnet Node" \\
  --base-path /var/lib/atlas \\
  --rpc-cors all \\
  --rpc-methods Safe \\
  --rpc-port 9933 \\
  --ws-port 9944 \\
  --port 30333 \\
  --telemetry-url "wss://telemetry.atlas-sphere.io/submit/ 0"`}
          </CodeBlock>
        </section>

        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Configuration Options</h2>
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-[#1a1a1a]">
                  <th className="text-left py-3 px-4 text-gray-500 font-medium">Flag</th>
                  <th className="text-left py-3 px-4 text-gray-500 font-medium">Description</th>
                </tr>
              </thead>
              <tbody>
                <tr className="border-b border-[#111111]">
                  <td className="py-3 px-4 text-orange-400 font-mono">--chain</td>
                  <td className="py-3 px-4 text-gray-400">Chain spec: dev, local, testnet, mainnet</td>
                </tr>
                <tr className="border-b border-[#111111]">
                  <td className="py-3 px-4 text-orange-400 font-mono">--base-path</td>
                  <td className="py-3 px-4 text-gray-400">Directory for chain data</td>
                </tr>
                <tr className="border-b border-[#111111]">
                  <td className="py-3 px-4 text-orange-400 font-mono">--validator</td>
                  <td className="py-3 px-4 text-gray-400">Run as a validator node</td>
                </tr>
                <tr className="border-b border-[#111111]">
                  <td className="py-3 px-4 text-orange-400 font-mono">--rpc-external</td>
                  <td className="py-3 px-4 text-gray-400">Accept external RPC connections</td>
                </tr>
                <tr className="border-b border-[#111111]">
                  <td className="py-3 px-4 text-orange-400 font-mono">--ws-external</td>
                  <td className="py-3 px-4 text-gray-400">Accept external WebSocket connections</td>
                </tr>
                <tr className="border-b border-[#111111]">
                  <td className="py-3 px-4 text-orange-400 font-mono">--bootnodes</td>
                  <td className="py-3 px-4 text-gray-400">Addresses of bootstrap nodes</td>
                </tr>
              </tbody>
            </table>
          </div>
        </section>

        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Systemd Service</h2>
          <CodeBlock language="ini" title="/etc/systemd/system/atlas-node.service">
{`[Unit]
Description=Atlas Sphere Node
After=network.target

[Service]
Type=simple
User=atlas
ExecStart=/usr/local/bin/atlas-sphere-node \\
  --chain testnet \\
  --name "My Node" \\
  --base-path /var/lib/atlas \\
  --rpc-cors all \\
  --rpc-methods Safe
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target`}
          </CodeBlock>

          <CodeBlock language="bash" title="Enable Service">
{`sudo systemctl daemon-reload
sudo systemctl enable atlas-node
sudo systemctl start atlas-node
sudo systemctl status atlas-node`}
          </CodeBlock>
        </section>

        <Callout type="info" title="Need Help?">
          Join our Discord for node operator support and discussions.
        </Callout>
      </div>
    </DocLayout>
  );
}
