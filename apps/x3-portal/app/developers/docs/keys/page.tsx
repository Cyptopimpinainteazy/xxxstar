'use client';

import React from 'react';
import DocLayout, { CodeBlock, Callout } from '@/components/docs/DocLayout';

export default function KeysPage() {
  return (
    <DocLayout
      title="Key Management"
      description="Secure key generation and management for X3 Atlas Sphere"
    >
      <p className="lead text-xl text-gray-400 mb-8">
        Proper key management is critical for securing your accounts and validator operations.
        This guide covers key types, generation, and security best practices.
      </p>

      <Callout type="warning" title="Security Critical">
        Never share private keys or seed phrases. Store backups in multiple secure, 
        offline locations. Compromised keys can result in loss of funds.
      </Callout>

      <h2>Key Types</h2>
      <ul>
        <li><strong>Sr25519</strong> - Used for Aura block production, account signatures</li>
        <li><strong>Ed25519</strong> - Used for GRANDPA finality voting</li>
        <li><strong>ECDSA</strong> - Compatible with Ethereum-style addresses</li>
      </ul>

      <h2>Generate Keys</h2>

      <h3>Using the Node Binary</h3>
      <CodeBlock language="bash">
{`# Generate Sr25519 key (recommended for most uses)
./atlas-sphere-node key generate --scheme Sr25519

# Output:
# Secret phrase: word1 word2 word3 ... word12
# Secret seed: 0x...
# Public key (hex): 0x...
# Account ID: 0x...
# SS58 Address: 5...

# Generate Ed25519 key (for GRANDPA)
./atlas-sphere-node key generate --scheme Ed25519

# Generate ECDSA key (Ethereum compatible)
./atlas-sphere-node key generate --scheme Ecdsa`}
      </CodeBlock>

      <h3>Using subkey Tool</h3>
      <CodeBlock language="bash">
{`# Install subkey
cargo install --force subkey --git https://github.com/paritytech/substrate

# Generate key
subkey generate --scheme sr25519

# Inspect existing key
subkey inspect "word1 word2 ... word12"

# Generate with custom network ID
subkey generate --scheme sr25519 --network atlas`}
      </CodeBlock>

      <h3>Using Polkadot.js</h3>
      <CodeBlock language="typescript">
{`import { Keyring } from '@polkadot/keyring';
import { mnemonicGenerate, mnemonicToMiniSecret } from '@polkadot/util-crypto';

// Generate mnemonic
const mnemonic = mnemonicGenerate();
console.log('Mnemonic:', mnemonic);

// Create keyring and add account
const keyring = new Keyring({ type: 'sr25519', ss58Format: 42 });
const pair = keyring.addFromMnemonic(mnemonic);

console.log('Address:', pair.address);
console.log('Public key:', pair.publicKey);`}
      </CodeBlock>

      <h2>Validator Session Keys</h2>
      <CodeBlock language="bash">
{`# Generate all session keys at once
./atlas-sphere-node key generate-session-keys --chain testnet

# Or rotate keys via RPC (when node is running)
curl -H "Content-Type: application/json" \\
  -d '{"id":1, "jsonrpc":"2.0", "method": "author_rotateKeys"}' \\
  http://localhost:9933

# Insert individual keys
./atlas-sphere-node key insert \\
  --base-path /var/lib/atlas \\
  --chain testnet \\
  --scheme Sr25519 \\
  --suri "mnemonic phrase here" \\
  --key-type aura

./atlas-sphere-node key insert \\
  --base-path /var/lib/atlas \\
  --chain testnet \\
  --scheme Ed25519 \\
  --suri "different mnemonic here" \\
  --key-type gran`}
      </CodeBlock>

      <h2>Key Storage</h2>

      <h3>Keystore Location</h3>
      <CodeBlock language="bash">
{`# Default keystore location
~/.local/share/atlas-sphere-node/chains/<chain>/keystore/

# Custom base path
/var/lib/atlas/chains/<chain>/keystore/

# List keys in keystore
ls -la /var/lib/atlas/chains/testnet/keystore/`}
      </CodeBlock>

      <h3>Encrypted Backup</h3>
      <CodeBlock language="bash">
{`# Create encrypted backup of keys
tar czf - keys/ | gpg --symmetric --cipher-algo AES256 > keys-backup.tar.gz.gpg

# Restore from backup
gpg --decrypt keys-backup.tar.gz.gpg | tar xzf -

# Use hardware security module (HSM) for production
# Configure via --keystore-uri`}
      </CodeBlock>

      <h2>Account Derivation</h2>
      <CodeBlock language="bash">
{`# Soft derivation (/)
subkey inspect "mnemonic//account1"
subkey inspect "mnemonic//account2"

# Hard derivation (//)
subkey inspect "mnemonic//hard"

# Password protected
subkey inspect "mnemonic///password"

# Combined
subkey inspect "mnemonic//hard/soft///password"`}
      </CodeBlock>

      <h2>Address Formats</h2>
      <CodeBlock language="typescript">
{`import { encodeAddress, decodeAddress } from '@polkadot/util-crypto';

// Convert between formats
const publicKey = '0x...';

// Atlas format (SS58 prefix 42)
const atlasAddress = encodeAddress(publicKey, 42);

// Substrate generic (prefix 42)
const substrateAddress = encodeAddress(publicKey, 42);

// Polkadot format (prefix 0)
const polkadotAddress = encodeAddress(publicKey, 0);

// Decode any SS58 address
const decoded = decodeAddress(atlasAddress);`}
      </CodeBlock>

      <h2>Security Best Practices</h2>
      <ul>
        <li>✅ Generate keys on air-gapped machine</li>
        <li>✅ Store seed phrases in multiple physical locations</li>
        <li>✅ Use hardware wallets for significant holdings</li>
        <li>✅ Never store seeds in plain text digitally</li>
        <li>✅ Use different keys for different purposes</li>
        <li>✅ Rotate validator keys periodically</li>
        <li>✅ Set up key recovery procedures</li>
        <li>✅ Test recovery before storing real value</li>
      </ul>

      <Callout type="info" title="Hardware Wallets">
        For maximum security, use Ledger hardware wallets with the Polkadot app.
        Session keys can be injected from hot wallets while controller keys stay cold.
      </Callout>
    </DocLayout>
  );
}
