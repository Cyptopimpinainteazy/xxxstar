#!/usr/bin/env bash
# Every crate that declares a `no_std` posture must compile without its default features.
#
# ## Why this exists
#
# TICKET-082 measured `cargo check -p x3-crosschain-intent --no-default-features` at **244
# errors**, and behind them TICKET-092: a `#[cfg(any(test, feature = "std"))]` around an
# Ed25519 check **with no `else`**, so in the configuration the runtime's wasm build uses,
# the verification was removed and a validator's stake counted toward a quorum anyway.
#
# Nothing built that configuration. `cargo check --workspace` builds every crate *with* its
# default features, so a dependency feature the code cannot do without — gated behind `std`
# for no reason — is invisible until something turns the feature off. The runtime sets
# `default-features = false` on 64 local crates, so "off" is not hypothetical: it is the
# configuration the chain ships.
#
# ## Why one crate at a time
#
# The first version of this gate passed `-p` for every crate in **one** cargo invocation,
# and it **passed with TICKET-082 deliberately reintroduced**: cargo unifies features across
# a graph, so `hex/alloc` enabled by any other crate in the invocation satisfied the intent
# crate too. A single invocation checks the graph's union, not the crate. Measured both
# ways: isolated, the intent crate shows 14 errors with the bug in; its pallet shows 0,
# because the pallet's subtree unifies `std` from elsewhere again. The crate has to be
# checked **alone** for its own `--no-default-features` to mean anything.
#
# ## Scope, derived
#
# The list is derived by grepping for the posture itself — `#![no_std]` or
# `#![cfg_attr(not(feature = "std"), no_std)]` — and mapping each file to its package. A
# crate joins this gate by declaring that it can be `no_std`, which is exactly the claim
# being checked. Nothing here is hardcoded.
#
# Slow by construction (one cargo invocation per crate), so it belongs to the opt-in
# `variants` group rather than the fast set.
set -uo pipefail

cd "$(dirname "$0")/.."

# A workspace member's manifest is one that `cargo metadata` knows about; filter with it once
# rather than letting 87 `-p` failures stand in for the answer.
mapfile -t crates < <(
  python3 - <<'PY'
import os
import re
import subprocess

posture = re.compile(r'#!\[no_std\]|cfg_attr\(not\(feature = "std"\), no_std\)')
manifests = {}
for root, dirs, files in os.walk('.'):
    dirs[:] = [d for d in dirs if d not in {'.git', 'target', 'node_modules'}]
    if 'Cargo.toml' in files:
        manifests[os.path.normpath(root)] = os.path.join(root, 'Cargo.toml')

found = set()
for root, manifest in manifests.items():
    src = os.path.join(root, 'src')
    if not os.path.isdir(src):
        continue
    for walk_root, _dirs, files in os.walk(src):
        for name in files:
            if not name.endswith('.rs'):
                continue
            try:
                text = open(os.path.join(walk_root, name), encoding='utf-8', errors='ignore').read()
            except OSError:
                continue
            if posture.search(text):
                head = open(manifest).read()
                m = re.search(r'^name\s*=\s*"([^"]+)"', head, re.M)
                if m:
                    found.add(m.group(1))
                break

# `alloc` is a supported no_std configuration of its own: a crate with an allocator is not
# claiming to work without one. `x3-common` declares `alloc = ["serde/alloc"]` and fails
# `--no-default-features` with two `String: serde::Serialize` errors — and passes with
# `--features alloc`, so that is the configuration to check. Passing the feature it declares
# rather than failing it is the difference between a gate and a gate nobody runs.
def declares_alloc(manifest_path):
    try:
        head = open(manifest_path).read()
    except OSError:
        return False
    return re.search(r'^alloc\s*=', head, re.M) is not None

# Only workspace members can be selected with `-p`.
meta = subprocess.run(
    ['cargo', 'metadata', '--no-deps', '--format-version', '1'],
    capture_output=True, text=True, check=True,
)
import json

entries = {p['name']: p['manifest_path'] for p in json.loads(meta.stdout)['packages']}
for name in sorted(found & set(entries)):
    print(f"{name}|{'alloc' if declares_alloc(entries[name]) else 'bare'}")
PY
)

if [ "${#crates[@]}" -eq 0 ]; then
  echo "no-default-features: derived no crates — the scan is wrong, and a gate that checks" >&2
  echo "no-default-features: nothing must fail rather than pass" >&2
  exit 1
fi

# Crates whose `no_std` declaration is false **today**. Each is measured and ticketed
# (TICKET-093); this list must shrink to empty, because a crate here is a crate whose own
# declaration lies. The gate fails on anything *not* on it, so it cannot grow silently —
# a new entry has to be written here, with a reason, which is the point.
#
# Measured with `cargo check -p <crate> --no-default-features`:
KNOWN_UNBUILDABLE=(
  "x3-chain-runtime"        #   1 error  (E0599)
  "x3-external-chains"      # 184 errors (E0433 ×87, E0412 ×55)
  "x3-gateway-risk-engine"  #   7 errors (E0412, E0599)
  "x3-liquidity-core"       #   3 errors (E0599)
  "x3-sdk"                  # 216 errors (E0412 ×147, E0433 ×31)
)

is_known() {
  local crate="$1"
  for known in "${KNOWN_UNBUILDABLE[@]}"; do
    [ "$known" = "$crate" ] && return 0
  done
  return 1
}

echo "no-default-features: ${#crates[@]} workspace crates declare a no_std posture"

failed=()
for entry in "${crates[@]}"; do
  crate="${entry%%|*}"
  mode="${entry##*|}"
  extra=()
  [ "$mode" = alloc ] && extra=(--features alloc)
  if SKIP_WASM_BUILD=1 cargo check -p "$crate" --no-default-features "${extra[@]}" >/dev/null 2>&1; then
    printf '  ok    %s (%s)\n' "$crate" "$mode"
  elif is_known "$crate"; then
    printf '  known %s (%s) — on the ticketed list\n' "$crate" "$mode"
  else
    printf '  FAIL  %s (%s)\n' "$crate" "$mode"
    failed+=("$crate")
  fi
done

if [ "${#failed[@]}" -gt 0 ]; then
  echo
  echo "no-default-features: ${#failed[@]} of ${#crates[@]} cannot build without default"
  echo "no-default-features: features and is **not** on the known list — new drift:"
  printf '  %s\n' "${failed[@]}"
  echo
  echo "Each one is a crate whose own declaration says it can be no_std, and cannot be."
  exit 1
fi

if [ "${#KNOWN_UNBUILDABLE[@]}" -gt 0 ]; then
  echo
  echo "no-default-features: all ${#crates[@]} crates accounted for; ${#KNOWN_UNBUILDABLE[@]} are on the"
  echo "no-default-features: known list (TICKET-093) and must come off it as they are fixed."
fi
