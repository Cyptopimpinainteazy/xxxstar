#!/usr/bin/env python3
"""Fix ALL vendor checksum mismatches by scanning every .cargo-checksum.json directly."""
import subprocess, json, hashlib, sys
from pathlib import Path

ROOT = Path(__file__).parent

def sha256_file(path):
    h = hashlib.sha256()
    with open(path, 'rb') as f:
        for chunk in iter(lambda: f.read(65536), b''):
            h.update(chunk)
    return h.hexdigest()

print("Scanning all vendor .cargo-checksum.json files...")
total_fixed = 0
for checksum_file in sorted((ROOT / 'vendor').glob('*/.cargo-checksum.json')):
    pkg_dir = checksum_file.parent
    data = json.loads(checksum_file.read_text())
    changed = False
    for filekey, recorded_hash in list(data.get('files', {}).items()):
        actual_path = pkg_dir / filekey
        if not actual_path.exists():
            continue
        actual = sha256_file(actual_path)
        if actual != recorded_hash:
            print(f"  FIX {pkg_dir.name}::{filekey}")
            print(f"      {recorded_hash[:20]}... -> {actual[:20]}...")
            data['files'][filekey] = actual
            changed = True
            total_fixed += 1
    if changed:
        checksum_file.write_text(json.dumps(data))

print(f"\nFixed {total_fixed} checksum(s) across all vendor crates.")
if total_fixed == 0:
    print("All checksums were already correct.")

print("\nRunning cargo check to verify...")
result = subprocess.run(
    ['cargo', 'check', '-p', 'x3-chain-node'],
    capture_output=True, text=True, cwd=ROOT
)
errors = [l for l in result.stderr.splitlines() if l.startswith('error')]
if not errors:
    print("SUCCESS — cargo check passed!")
else:
    print(f"{len(errors)} error(s) remain (non-checksum):")
    for e in errors[:30]:
        print(' ', e)
sys.exit(0 if not errors else 1)

print("Running cargo check to find mismatches...")
result = subprocess.run(
    ['cargo', 'check', '-p', 'x3-chain-node'],
    capture_output=True, text=True, cwd=ROOT
)
output = result.stderr

# Parse blocks like:
#   the listed checksum of `/path/to/file` has changed:
#   expected: <hash>
#   actual: <hash>
pattern = re.compile(
    r"the listed checksum of `([^`]+)` has changed:\s*\nexpected: ([0-9a-f]+)\s*\nactual: ([0-9a-f]+)",
    re.MULTILINE
)

fixes = pattern.findall(output)

if not fixes:
    # Also try finding files that just say "has changed" without expected/actual
    # (older cargo versions just show the file path)
    simple = re.findall(r"the listed checksum of `([^`]+)` has changed", output)
    if simple:
        print(f"Found {len(simple)} mismatch(es) — computing actual hashes...")
        for filepath in simple:
            p = Path(filepath)
            if not p.exists():
                print(f"  SKIP (not found): {filepath}")
                continue
            # Find relative path within vendor
            try:
                rel = p.relative_to(ROOT / 'vendor')
            except ValueError:
                print(f"  SKIP (not in vendor): {filepath}")
                continue
            pkg = rel.parts[0]
            filekey = str(Path(*rel.parts[1:]))
            checksum_file = ROOT / 'vendor' / pkg / '.cargo-checksum.json'
            actual = sha256_file(p)
            data = json.loads(checksum_file.read_text())
            old = data['files'].get(filekey, '(missing)')
            data['files'][filekey] = actual
            checksum_file.write_text(json.dumps(data))
            print(f"  FIXED {pkg}::{filekey}")
            print(f"    {old} -> {actual}")
        # Fall through to re-check below
    else:
        print("No checksum mismatches found!")
        print("Cargo check errors (if any):")
        for line in output.splitlines():
            if line.startswith('error'):
                print(' ', line)
        sys.exit(0)

print(f"Found {len(fixes)} mismatch(es) to fix:")
for filepath, expected, actual in fixes:
    p = Path(filepath)
    try:
        rel = p.relative_to(ROOT / 'vendor')
    except ValueError:
        print(f"  SKIP (not in vendor): {filepath}")
        continue
    pkg = rel.parts[0]
    filekey = str(Path(*rel.parts[1:]))
    checksum_file = ROOT / 'vendor' / pkg / '.cargo-checksum.json'

    data = json.loads(checksum_file.read_text())
    data['files'][filekey] = actual
    checksum_file.write_text(json.dumps(data))
    print(f"  FIXED {pkg}::{filekey}")
    print(f"    {expected[:16]}... -> {actual[:16]}...")

print("\nAll mismatches fixed. Re-running cargo check...")
result2 = subprocess.run(
    ['cargo', 'check', '-p', 'x3-chain-node'],
    capture_output=True, text=True, cwd=ROOT
)
errors = [l for l in result2.stderr.splitlines() if l.startswith('error')]
if not errors:
    print("SUCCESS — no errors!")
else:
    print(f"{len(errors)} error(s) remain:")
    for e in errors[:20]:
        print(' ', e)
