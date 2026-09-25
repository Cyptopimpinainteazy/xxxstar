#!/usr/bin/env bash
# X3 economic model gate — the Alpha economic policy, checked against the code.
#
# `docs/economics/X3_ALPHA_ECONOMIC_POLICY.md` is the authoritative statement. This gate fails when
# the code, the deployment manifest, or an active launch document drifts from it. Every check below
# reads a real artifact; none of them asserts a constant against itself.
#
# Usage: scripts/mainnet/x3_economic_model_gate.sh [--quiet]
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

python3 - "$@" <<'PYEOF'
import json, pathlib, re, sys

root = pathlib.Path.cwd()
quiet = "--quiet" in sys.argv
failures, checks = [], []

def check(name, ok, detail=""):
    checks.append((name, ok, detail))
    if not ok:
        failures.append(name)

def read(path):
    return (root / path).read_text()

# ── artifacts ────────────────────────────────────────────────────────────────────────────────
manifest_path = "deployment/genesis/x3-testnet-allocations.json"
manifest = json.loads(read(manifest_path))
pallet = read("pallets/x3-coin/src/lib.rs")
runtime = read("runtime/src/lib.rs")
runtime_manifest = read("runtime/Cargo.toml")
policy = read("docs/economics/X3_ALPHA_ECONOMIC_POLICY.md")

def rust_const(name, src=pallet):
    """Read `pub const NAME: u128 = <literal> [* X3_BASE_UNITS_PER_X3];` as whole X3 or base units."""
    m = re.search(rf"pub const {name}: u128 = ([0-9_]+)( \* X3_BASE_UNITS_PER_X3)?;", src)
    if not m:
        return None
    value = int(m.group(1).replace("_", ""))
    return value * 10 ** 12 if m.group(2) else value

# ── 1. the manifest is internally consistent ─────────────────────────────────────────────────
treasury_base = int(manifest["treasury"]["allocation"]["amount_base"])
bucket_base = sum(int(a["amount_base"]) for a in manifest["allocations"])
total_base = int(manifest["token"]["total_supply_base"])
check("genesis accounting identity (treasury + buckets == total)",
      treasury_base + bucket_base == total_base,
      f"{treasury_base} + {bucket_base} != {total_base}" if treasury_base + bucket_base != total_base else "")

# ── 2. code supply matches the manifest ──────────────────────────────────────────────────────
manifest_total = int(manifest["token"]["total_supply"])
code_total = rust_const("X3_TOTAL_SUPPLY_X3")
check("total supply matches between pallet and manifest",
      code_total == manifest_total, f"pallet={code_total} manifest={manifest_total}")

# ── 3. decimals match everywhere ─────────────────────────────────────────────────────────────
manifest_decimals = int(manifest["token"]["decimals"])
m = re.search(r"pub const X3_DECIMALS: u8 = ([0-9]+);", pallet)
code_decimals = int(m.group(1)) if m else None
check("pallet decimals match the manifest", code_decimals == manifest_decimals,
      f"pallet={code_decimals} manifest={manifest_decimals}")
cs = re.search(r"const X3: u128 = 1_000_000_000_000;", read("node/src/chain_spec.rs"))
check("genesis endowment unit is 1 X3 = 10^12 (12 decimals)", bool(cs) and manifest_decimals == 12)
check("manifest base units match its decimals",
      total_base == manifest_total * 10 ** manifest_decimals)

# ── 4. every manifest bucket maps to a pallet constant ───────────────────────────────────────
bucket_const = {
    "validators_staking": "X3_VALIDATOR_SECURITY_RESERVE",
    "ecosystem_grants": "X3_ECOSYSTEM_ALLOCATION",
    "presale_early_investors": "X3_PRESALE_ALLOCATION",
    "bonus_pool": "X3_BONUS_POOL_ALLOCATION",
    "team_core_contributors": "X3_TEAM_ALLOCATION",
}
unmapped, mismatched = [], []
for entry in manifest["allocations"]:
    name = bucket_const.get(entry["bucket"])
    if not name:
        unmapped.append(entry["bucket"]); continue
    value = rust_const(name)
    if value != int(entry["amount_base"]):
        mismatched.append(f"{entry['bucket']}: pallet={value} manifest={entry['amount_base']}")
check("every manifest bucket has a pallet constant", not unmapped, ", ".join(unmapped))
check("every bucket amount matches between pallet and manifest", not mismatched, "; ".join(mismatched))
check("treasury bucket matches between pallet and manifest",
      rust_const("X3_TREASURY_ALLOCATION") == treasury_base)
check("allocation buckets sum to the total supply (compile-time assertion present)",
      "X3_ALLOCATION_SUM == X3_TOTAL_SUPPLY" in pallet)

# ── 5. the fee policy the runtime actually implements ────────────────────────────────────────
impl = re.search(r"impl frame_support::traits::OnUnbalanced<NegativeImbalance> for DealWithFees \{(.*?)\n\}", runtime, re.S)
drops = bool(impl) and "drop(amount)" in impl.group(1)
check("collected fees are burned (DealWithFees drops the imbalance)", drops)
check("policy documents the burn", "100% burned" in policy or "100% burn" in policy.lower())
mult = re.search(r"type FeeMultiplierUpdate = \(\);", runtime)
check("fee multiplier is static (FeeMultiplierUpdate = ())", bool(mult))
check("policy documents static deterministic pricing", "static and deterministic" in policy)
check("policy does not claim EIP-1559 runtime pricing or a fee market",
      "EIP-1559" not in policy.replace("EIP-1559-style runtime pricing", "").replace("EIP-1559 simulation", ""))
check("policy states tips are not implemented in Alpha",
      "Priority fee / tip" in policy and "not implemented" in policy)

# ── 6. staking claims vs the runtime ─────────────────────────────────────────────────────────
staking_wired = re.search(r"^\s*pallet-staking\s*=", runtime_manifest, re.M) is not None
check("pallet_staking is absent from the runtime", not staking_wired)
check("policy states permissionless staking is disabled", "disabled" in policy and "pallet_staking" in policy)

# ── 7. no inflation path ─────────────────────────────────────────────────────────────────────
check("the pallet pins total supply to the constant (no issuance path)",
      "expected_total: T::Balance = X3_TOTAL_SUPPLY.saturated_into()" in pallet)
check("the mint path is treasury-funded (cannot issue new supply)",
      "InsufficientTreasuryBalance" in pallet and "verify_supply_invariant" in pallet)
check("policy states inflation is zero and the supply is fixed", "Inflation | **0**" in policy)

# ── 8. no conflicting launch economics in active documents ───────────────────────────────────
active_docs = [
    "LAUNCH_SCOPE.md", "CURRENT_MAINNET_STATUS.md", "TESTNET_GAP_LEDGER.md", "README.md",
    "docs/economics/X3_ALPHA_ECONOMIC_POLICY.md",
]
conflicts = []
for doc in active_docs:
    p = root / doc
    if not p.exists():
        continue
    text = p.read_text()
    if re.search(r"2,?000,?000,?000\s*(X3)?\b", text):
        conflicts.append(doc)
for p in sorted((root / "pallets").glob("*/README.md")):
    if re.search(r"2,?000,?000,?000\s*(X3)?\b", p.read_text()):
        conflicts.append(str(p.relative_to(root)))
check("no active launch document claims a 2,000,000,000 supply", not conflicts, ", ".join(conflicts))

# ── 9. the open items are declared, not hidden ───────────────────────────────────────────────
check("policy declares the unimplemented validator payout path",
      "Validator payout path from the security reserve" in policy and "not implemented" in policy)

for name, ok, detail in checks:
    if quiet and ok:
        continue
    print(f"[{'PASS' if ok else 'FAIL'}] {name}" + (f" — {detail}" if detail and not ok else ""))

print()
if failures:
    print(f"X3_ECONOMIC_MODEL_GATE: FAIL ({len(failures)}/{len(checks)} checks failed)")
    for f in failures:
        print(f"  - {f}")
    sys.exit(1)
print(f"X3_ECONOMIC_MODEL_GATE: PASS ({len(checks)} checks)")
PYEOF
