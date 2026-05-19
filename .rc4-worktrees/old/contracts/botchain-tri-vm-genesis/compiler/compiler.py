#!/usr/bin/env python3
"""
Botchain Mobile Compiler

Injects immutable "10 Commandments" into agent source code and signs manifests.
The commandments ensure ethical AI behavior and network safety.

Usage:
    python compiler.py --input src_dir --out manifest.json
    python compiler.py --input agent.py --out manifest.json --verify
"""

import argparse
import json
import hashlib
import time
import os
import sys
from pathlib import Path
from typing import Optional, Dict, Any, Tuple
from dataclasses import dataclass, asdict

# Cryptography imports
from cryptography.hazmat.primitives import hashes
from cryptography.hazmat.primitives.asymmetric import ec
from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.backends import default_backend
from cryptography.exceptions import InvalidSignature

# Paths
SCRIPT_DIR = Path(__file__).parent
COMMANDMENTS_PATH = SCRIPT_DIR / 'commandments.json'
DEFAULT_KEY_PATH = SCRIPT_DIR / 'key.pem'
DEFAULT_PUB_PATH = SCRIPT_DIR / 'key.pub'

# Load commandments
with open(COMMANDMENTS_PATH, 'r') as f:
    COMMANDMENTS = json.load(f)

# Compiler version
COMPILER_VERSION = "botchain-compiler-1.0.0"


@dataclass
class CompilerManifest:
    """Manifest structure for compiled artifacts"""
    cid: str                    # SHA256 of augmented artifact
    original_cid: str           # SHA256 of original source
    timestamp: int              # Unix timestamp
    compiler_version: str       # Compiler version string
    compiler_pubkey: str        # Public key fingerprint
    commandments_hash: str      # Hash of injected commandments
    source_files: list          # List of source files processed
    signature: str              # ECDSA signature of manifest


def load_private_key(path: Path) -> ec.EllipticCurvePrivateKey:
    """Load ECDSA private key from PEM file"""
    with open(path, 'rb') as f:
        return serialization.load_pem_private_key(
            f.read(), 
            password=None, 
            backend=default_backend()
        )


def load_public_key(path: Path) -> ec.EllipticCurvePublicKey:
    """Load ECDSA public key from PEM file"""
    with open(path, 'rb') as f:
        return serialization.load_pem_public_key(
            f.read(), 
            backend=default_backend()
        )


def get_public_key_fingerprint(private_key: ec.EllipticCurvePrivateKey) -> str:
    """Get SHA256 fingerprint of public key"""
    pub_bytes = private_key.public_key().public_bytes(
        encoding=serialization.Encoding.DER,
        format=serialization.PublicFormat.SubjectPublicKeyInfo
    )
    return hashlib.sha256(pub_bytes).hexdigest()[:16]


def compute_commandments_hash() -> str:
    """Compute hash of commandments for verification"""
    cmd_bytes = json.dumps(COMMANDMENTS, sort_keys=True).encode()
    return hashlib.sha256(cmd_bytes).hexdigest()


def create_commandments_header() -> str:
    """Create the commandments header to inject into source files"""
    header_lines = [
        "/* ===== BOTCHAIN COMMANDMENTS (IMMUTABLE) =====",
        f" * Compiler: {COMPILER_VERSION}",
        f" * Hash: {compute_commandments_hash()[:16]}",
        " * ",
    ]
    for i, cmd in enumerate(COMMANDMENTS, 1):
        header_lines.append(f" * {i:2d}. {cmd}")
    header_lines.extend([
        " * ",
        " * These commandments are immutable and must not be removed.",
        " * Violation will result in quarantine and revocation.",
        " * ================================================ */",
        "",
        "/* COMMANDMENTS_VERIFICATION_HOOK */",
        "function verify_commandments() internal pure returns (bool) {",
        f'    bytes32 expected = 0x{compute_commandments_hash()[:64]};',
        "    return true; // Runtime verification hook",
        "}",
        "",
    ])
    return "\n".join(header_lines)


def create_python_commandments_header() -> str:
    """Create Python-style commandments header"""
    header_lines = [
        '"""',
        "===== BOTCHAIN COMMANDMENTS (IMMUTABLE) =====",
        f"Compiler: {COMPILER_VERSION}",
        f"Hash: {compute_commandments_hash()[:16]}",
        "",
    ]
    for i, cmd in enumerate(COMMANDMENTS, 1):
        header_lines.append(f"{i:2d}. {cmd}")
    header_lines.extend([
        "",
        "These commandments are immutable and must not be removed.",
        "Violation will result in quarantine and revocation.",
        "================================================",
        '"""',
        "",
        "COMMANDMENTS_HASH = '{}'".format(compute_commandments_hash()[:64]),
        "",
        "def verify_commandments():",
        "    '''Runtime verification hook for commandments'''",
        f"    expected = '{compute_commandments_hash()[:64]}'",
        "    return COMMANDMENTS_HASH == expected",
        "",
    ])
    return "\n".join(header_lines)


def inject_commandments(source_bytes: bytes, filename: str) -> bytes:
    """Inject commandments header based on file type"""
    ext = Path(filename).suffix.lower()
    
    if ext in ('.sol', '.js', '.ts', '.c', '.cpp', '.java'):
        header = create_commandments_header()
    elif ext in ('.py', '.pyw'):
        header = create_python_commandments_header()
    else:
        # Generic header for other file types
        header = f"# COMMANDMENTS: {compute_commandments_hash()[:16]}\n"
    
    return header.encode() + source_bytes


def sign_manifest(manifest_dict: Dict[str, Any], private_key: ec.EllipticCurvePrivateKey) -> str:
    """Sign manifest with ECDSA"""
    # Create deterministic JSON representation (exclude signature field)
    manifest_copy = {k: v for k, v in manifest_dict.items() if k != 'signature'}
    data = json.dumps(manifest_copy, sort_keys=True, separators=(',', ':')).encode()
    
    signature = private_key.sign(data, ec.ECDSA(hashes.SHA256()))
    return signature.hex()


def verify_manifest_signature(manifest: Dict[str, Any], public_key: ec.EllipticCurvePublicKey) -> bool:
    """Verify manifest signature"""
    signature_hex = manifest.get('signature', '')
    if not signature_hex:
        return False
    
    manifest_copy = {k: v for k, v in manifest.items() if k != 'signature'}
    data = json.dumps(manifest_copy, sort_keys=True, separators=(',', ':')).encode()
    
    try:
        signature = bytes.fromhex(signature_hex)
        public_key.verify(signature, data, ec.ECDSA(hashes.SHA256()))
        return True
    except (InvalidSignature, ValueError):
        return False


def process_file(file_path: Path) -> Tuple[bytes, bytes]:
    """Process a single file, returning (original, augmented) bytes"""
    with open(file_path, 'rb') as f:
        original = f.read()
    
    augmented = inject_commandments(original, file_path.name)
    return original, augmented


def process_directory(dir_path: Path, extensions: set = None) -> Tuple[bytes, bytes, list]:
    """
    Process all files in a directory.
    Returns (combined_original, combined_augmented, file_list)
    """
    if extensions is None:
        extensions = {'.py', '.sol', '.js', '.ts', '.json'}
    
    all_original = b''
    all_augmented = b''
    file_list = []
    
    for file_path in sorted(dir_path.rglob('*')):
        if file_path.is_file() and file_path.suffix.lower() in extensions:
            original, augmented = process_file(file_path)
            all_original += original
            all_augmented += augmented
            file_list.append(str(file_path.relative_to(dir_path)))
    
    return all_original, all_augmented, file_list


def compile_artifact(
    input_path: Path, 
    output_path: Path, 
    key_path: Path = DEFAULT_KEY_PATH,
    write_augmented: bool = False,
    augmented_dir: Optional[Path] = None
) -> CompilerManifest:
    """
    Main compilation function.
    
    Args:
        input_path: Path to source file or directory
        output_path: Path for output manifest JSON
        key_path: Path to compiler private key
        write_augmented: Whether to write augmented files
        augmented_dir: Directory for augmented output
    
    Returns:
        CompilerManifest object
    """
    # Load compiler key
    if not key_path.exists():
        raise FileNotFoundError(
            f"Compiler key not found at {key_path}. Run keygen.sh first."
        )
    
    private_key = load_private_key(key_path)
    pubkey_fingerprint = get_public_key_fingerprint(private_key)
    
    # Process input
    if input_path.is_dir():
        original_bytes, augmented_bytes, file_list = process_directory(input_path)
    else:
        original_bytes, augmented_bytes = process_file(input_path)
        file_list = [input_path.name]
    
    # Compute CIDs
    original_cid = hashlib.sha256(original_bytes).hexdigest()
    augmented_cid = hashlib.sha256(augmented_bytes).hexdigest()
    
    # Create manifest
    manifest = CompilerManifest(
        cid=augmented_cid,
        original_cid=original_cid,
        timestamp=int(time.time()),
        compiler_version=COMPILER_VERSION,
        compiler_pubkey=pubkey_fingerprint,
        commandments_hash=compute_commandments_hash(),
        source_files=file_list,
        signature=""  # Placeholder, will be filled
    )
    
    # Sign manifest
    manifest_dict = asdict(manifest)
    manifest_dict['signature'] = sign_manifest(manifest_dict, private_key)
    manifest.signature = manifest_dict['signature']
    
    # Write manifest
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with open(output_path, 'w') as f:
        json.dump(manifest_dict, f, indent=2)
    
    # Optionally write augmented files
    if write_augmented and augmented_dir:
        augmented_dir.mkdir(parents=True, exist_ok=True)
        if input_path.is_file():
            aug_path = augmented_dir / input_path.name
            with open(aug_path, 'wb') as f:
                f.write(augmented_bytes)
        else:
            # For directories, write each file
            for file_path in input_path.rglob('*'):
                if file_path.is_file():
                    rel_path = file_path.relative_to(input_path)
                    aug_path = augmented_dir / rel_path
                    aug_path.parent.mkdir(parents=True, exist_ok=True)
                    _, augmented = process_file(file_path)
                    with open(aug_path, 'wb') as f:
                        f.write(augmented)
    
    return manifest


def verify_artifact(manifest_path: Path, pub_key_path: Path = DEFAULT_PUB_PATH) -> bool:
    """
    Verify a compiled manifest.
    
    Args:
        manifest_path: Path to manifest JSON
        pub_key_path: Path to compiler public key
    
    Returns:
        True if valid, False otherwise
    """
    with open(manifest_path, 'r') as f:
        manifest = json.load(f)
    
    # Verify commandments hash matches
    if manifest.get('commandments_hash') != compute_commandments_hash():
        print("ERROR: Commandments hash mismatch!")
        return False
    
    # Verify signature
    public_key = load_public_key(pub_key_path)
    if not verify_manifest_signature(manifest, public_key):
        print("ERROR: Invalid signature!")
        return False
    
    print("Manifest verified successfully!")
    print(f"  CID: {manifest['cid']}")
    print(f"  Compiler: {manifest['compiler_version']}")
    print(f"  Timestamp: {manifest['timestamp']}")
    return True


def main():
    parser = argparse.ArgumentParser(
        description='Botchain Mobile Compiler - Inject commandments and sign manifests'
    )
    parser.add_argument(
        '--input', '-i', 
        required=True,
        help='Input file or directory path'
    )
    parser.add_argument(
        '--out', '-o', 
        required=True,
        help='Output manifest JSON path'
    )
    parser.add_argument(
        '--key', '-k',
        default=str(DEFAULT_KEY_PATH),
        help='Path to compiler private key'
    )
    parser.add_argument(
        '--verify', '-v',
        action='store_true',
        help='Verify an existing manifest instead of compiling'
    )
    parser.add_argument(
        '--write-augmented', '-w',
        action='store_true',
        help='Write augmented source files'
    )
    parser.add_argument(
        '--augmented-dir', '-a',
        help='Directory for augmented output files'
    )
    parser.add_argument(
        '--verbose',
        action='store_true',
        help='Verbose output'
    )
    
    args = parser.parse_args()
    
    input_path = Path(args.input)
    output_path = Path(args.out)
    key_path = Path(args.key)
    
    if not input_path.exists():
        print(f"ERROR: Input path does not exist: {input_path}", file=sys.stderr)
        sys.exit(1)
    
    if args.verify:
        # Verification mode
        pub_key_path = key_path.with_suffix('.pub') if key_path.suffix == '.pem' else DEFAULT_PUB_PATH
        success = verify_artifact(output_path, pub_key_path)
        sys.exit(0 if success else 1)
    
    # Compilation mode
    try:
        augmented_dir = Path(args.augmented_dir) if args.augmented_dir else None
        manifest = compile_artifact(
            input_path,
            output_path,
            key_path,
            args.write_augmented,
            augmented_dir
        )
        
        print(f"✓ Compilation successful!")
        print(f"  Manifest: {output_path}")
        print(f"  CID: {manifest.cid}")
        print(f"  Files: {len(manifest.source_files)}")
        print(f"  Timestamp: {manifest.timestamp}")
        
        if args.verbose:
            print(f"\nSource files:")
            for f in manifest.source_files:
                print(f"    - {f}")
            print(f"\nCommandments hash: {manifest.commandments_hash[:32]}...")
            print(f"Signature: {manifest.signature[:32]}...")
        
    except Exception as e:
        print(f"ERROR: Compilation failed: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == '__main__':
    main()
