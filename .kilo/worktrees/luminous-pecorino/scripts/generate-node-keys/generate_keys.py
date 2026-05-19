#!/usr/bin/env python3
"""
Generate Substrate node keypairs for testnet bootnodes.

This script generates ed25519 keypairs and formats them as Substrate-compatible
node keys that can be used in chain specifications.
"""

import os
import sys
import json
import base64
import hashlib
from typing import Tuple

# Try to import pynacl for ed25519 support
try:
    from nacl.signing import SigningKey
    HAS_PYNACL = True
except ImportError:
    HAS_PYNACL = False
    print("Warning: pynacl not installed, using fallback method")


def generate_ed25519_keypair() -> Tuple[bytes, bytes]:
    """Generate an ed25519 keypair (secret, public)."""
    if HAS_PYNACL:
        signing_key = SigningKey.generate()
        return signing_key.encode(), signing_key.verify_key.encode()
    else:
        # Fallback: generate random bytes
        # Note: This is not a proper ed25519 keypair, but for testing purposes
        import secrets
        secret = secrets.token_bytes(32)
        # For a real ed25519 public key, we'd need to derive it from the secret
        # This is a placeholder for demonstration
        public = hashlib.sha256(secret).digest()[:32]
        return secret, public


def encode_ss58(pubkey: bytes, prefix: int = 42) -> str:
    """Encode a public key as SS58 address."""
    # Simple SS58 encoding (not full implementation)
    data = bytes([prefix]) + pubkey
    checksum = hashlib.sha256(hashlib.sha256(data).digest()).digest()[:4]
    encoded = base64.b64encode(data + checksum).decode()
    return encoded


def format_node_key(secret: bytes, public: bytes, ip: str = "127.0.0.1", port: int = 30333) -> str:
    """
    Format a node keypair as a multiaddr with peer ID.
    
    The peer ID is the base58 encoding of the SHA256 hash of the public key.
    """
    import base58
    
    # Compute peer ID: base58(sha256(public_key))
    peer_id_hash = hashlib.sha256(public).digest()
    peer_id = base58.b58encode(peer_id_hash).decode()
    
    # Format multiaddr
    multiaddr = f"/ip4/{ip}/tcp/{port}/p2p/{peer_id}"
    
    return multiaddr, peer_id


def main():
    """Generate node keypairs and output them in the required format."""
    num_nodes = 3  # Minimum for testnet
    
    print(f"Generating {num_nodes} node keypairs for testnet bootnodes...")
    print()
    
    # Create output directory if it doesn't exist
    output_dir = os.path.dirname(os.path.abspath(__file__))
    keys_dir = os.path.join(output_dir, "..", "..", "deployment", "keys")
    os.makedirs(keys_dir, exist_ok=True)
    
    bootnode_file = os.path.join(keys_dir, "bootnode-info.txt")
    json_file = os.path.join(keys_dir, "bootnode-keys.json")
    
    node_keys = []
    
    for i in range(num_nodes):
        print(f"Generating keypair for node {i + 1}...")
        
        # Generate keypair
        secret, public = generate_ed25519_keypair()
        
        # Format multiaddr
        multiaddr, peer_id = format_node_key(secret, public)
        
        node_keys.append({
            "index": i,
            "multiaddr": multiaddr,
            "peer_id": peer_id,
            "public_key": base64.b64encode(public).decode(),
            "secret_key": base64.b64encode(secret).decode(),
        })
        
        print(f"  Multiaddr: {multiaddr}")
        print(f"  Peer ID: {peer_id}")
        print()
    
    # Write bootnode-info.txt
    with open(bootnode_file, "w") as f:
        for node in node_keys:
            f.write(node["multiaddr"] + "\n")
    
    print(f"Written bootnode addresses to: {bootnode_file}")
    
    # Write JSON file with full key data (for reference, not for production)
    with open(json_file, "w") as f:
        json.dump(node_keys, f, indent=2)
    
    print(f"Written key data to: {json_file}")
    print()
    print("IMPORTANT: The secret keys are stored in the JSON file for reference.")
    print("In production, store these securely and never commit them to version control.")
    print()
    print("To update the chain spec, replace the placeholder peer ID in")
    print("deployment/chain-specs/x3-testnet-raw-temp.json with the peer IDs above.")


if __name__ == "__main__":
    main()
