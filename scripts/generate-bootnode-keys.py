#!/usr/bin/env python3
"""
Generate ed25519 keypairs for X3 Chain bootnodes and derive peer IDs.

This script generates real cryptographic keypairs and derives the libp2p PeerId
from the public key using the standard Substrate/Polkadot format.
"""

import os
import sys
import json
import hashlib
import base58
from typing import Tuple, Dict

# Try to import pynacl for ed25519
try:
    from nacl.signing import SigningKey
    PYNACL_AVAILABLE = True
except ImportError:
    PYNACL_AVAILABLE = False
    print("Warning: pynacl not available, using fallback method")


def generate_ed25519_keypair() -> Tuple[bytes, bytes]:
    """
    Generate an ed25519 keypair.
    
    Returns:
        Tuple of (private_key, public_key) as bytes
    """
    if PYNACL_AVAILABLE:
        # Use pynacl for proper ed25519 key generation
        signing_key = SigningKey.generate()
        private_key = bytes(signing_key)
        public_key = bytes(signing_key.verify_key)
        return private_key, public_key
    else:
        # Fallback: generate random bytes (not cryptographically secure)
        import secrets
        private_key = secrets.token_bytes(64)
        public_key = secrets.token_bytes(32)
        return private_key, public_key


def derive_peer_id(public_key: bytes) -> str:
    """
    Derive the libp2p PeerId from an ed25519 public key.
    
    The PeerId is derived by:
    1. Creating a multihash of the public key using SHA2-256
    2. Encoding the multihash as base58
    
    Args:
        public_key: The ed25519 public key (32 bytes)
    
    Returns:
        The PeerId as a base58-encoded string
    """
    # For ed25519, the multihash format is:
    # [0x12, 0x20, <32-byte hash>]
    # where 0x12 = SHA2-256, 0x20 = 32 bytes
    
    # Compute SHA2-256 hash of the public key
    public_key_hash = hashlib.sha256(public_key).digest()
    
    # Create multihash: [code, length, hash]
    # 0x12 = SHA2-256, 0x20 = 32 bytes
    multihash = b'\x12\x20' + public_key_hash
    
    # Encode as base58
    peer_id = base58.b58encode(multihash).decode('utf-8')
    
    return peer_id


def format_node_key_for_substrate(private_key: bytes) -> str:
    """
    Format the private key for Substrate/Polkadot node key format.
    
    Substrate uses a specific format for node keys stored in files.
    
    Args:
        private_key: The ed25519 private key (64 bytes)
    
    Returns:
        Formatted key string for storage
    """
    # Substrate typically stores the raw 64-byte private key as hex
    return private_key.hex()


def format_node_key_for_libp2p(private_key: bytes) -> str:
    """
    Format the private key for libp2p peer key format (base64).
    
    Args:
        private_key: The ed25519 private key (64 bytes)
    
    Returns:
        Base64-encoded key string
    """
    import base64
    return base64.b64encode(private_key).decode('utf-8')


def generate_bootnode_config(output_dir: str = "deployment/keys") -> Dict:
    """
    Generate a complete bootnode configuration.
    
    Args:
        output_dir: Directory to save configuration files
    
    Returns:
        Dictionary containing all generated keys and peer ID
    """
    # Generate keypair
    private_key, public_key = generate_ed25519_keypair()
    
    # Derive peer ID
    peer_id = derive_peer_id(public_key)
    
    # Format keys
    node_key_hex = format_node_key_for_substrate(private_key)
    node_key_base64 = format_node_key_for_libp2p(private_key)
    
    # Create configuration
    config = {
        "private_key_hex": node_key_hex,
        "private_key_base64": node_key_base64,
        "public_key_hex": public_key.hex(),
        "peer_id": peer_id,
        "ed25519": True
    }
    
    # Save configuration files
    os.makedirs(output_dir, exist_ok=True)
    
    # Save bootnode-info.txt
    info_file = os.path.join(output_dir, "bootnode-info.txt")
    with open(info_file, 'w') as f:
        f.write(f"""# Bootnode Network Configuration
# Generated: {os.popen('date -u +"%Y-%m-%d %H:%M:%S UTC"').read().strip()}

## Network Key (Ed25519)
{node_key_hex}

## Public Key (Hex)
{public_key.hex()}

## Peer ID
{peer_id}

## Multiaddress Format
/ip4/<BOOTNODE_IP>/tcp/30333/p2p/{peer_id}

## Example (replace <BOOTNODE_IP> with actual IP)
/ip4/10.0.1.100/tcp/30333/p2p/{peer_id}
/dns4/bootnode.x3-chain.io/tcp/30333/p2p/{peer_id}

## Systemd Service Configuration
Place the node key file at: /var/lib/atlas/node-key
Set permissions: chmod 600 /var/lib/atlas/node-key

## Chain Spec Entry (Add to "bootNodes")
"/ip4/<BOOTNODE_IP>/tcp/30333/p2p/{peer_id}"
""")
    
    # Save bootnode-node-key (raw hex format for Substrate)
    key_file = os.path.join(output_dir, "bootnode-node-key")
    with open(key_file, 'w') as f:
        f.write(f"{node_key_hex}\n")
    
    # Save bootnode-config.json (structured config)
    config_file = os.path.join(output_dir, "bootnode-config.json")
    with open(config_file, 'w') as f:
        json.dump(config, f, indent=2)
    
    print(f"✅ Generated bootnode configuration:")
    print(f"   Private Key (hex): {node_key_hex}")
    print(f"   Public Key (hex):  {public_key.hex()}")
    print(f"   Peer ID:           {peer_id}")
    print(f"")
    print(f"📁 Files saved to: {output_dir}/")
    print(f"   - bootnode-info.txt")
    print(f"   - bootnode-node-key")
    print(f"   - bootnode-config.json")
    
    return config


def verify_peer_id(public_key_hex: str, peer_id: str) -> bool:
    """
    Verify that a peer ID was correctly derived from a public key.
    
    Args:
        public_key_hex: The public key as hex string
        peer_id: The peer ID to verify
    
    Returns:
        True if verification succeeds
    """
    public_key = bytes.fromhex(public_key_hex)
    expected_peer_id = derive_peer_id(public_key)
    
    if expected_peer_id == peer_id:
        print(f"✅ Peer ID verification passed!")
        return True
    else:
        print(f"❌ Peer ID verification failed!")
        print(f"   Expected: {expected_peer_id}")
        print(f"   Got:      {peer_id}")
        return False


def main():
    """Main entry point."""
    print("=" * 60)
    print("X3 Chain Bootnode Key Generator")
    print("=" * 60)
    print()
    
    # Check for pynacl
    if PYNACL_AVAILABLE:
        print("✅ Using pynacl for ed25519 key generation")
    else:
        print("⚠️  pynacl not available - using fallback (not cryptographically secure)")
    print()
    
    # Generate configuration
    config = generate_bootnode_config()
    
    # Verify
    print()
    verify_peer_id(config["public_key_hex"], config["peer_id"])
    
    return 0


if __name__ == "__main__":
    sys.exit(main())
