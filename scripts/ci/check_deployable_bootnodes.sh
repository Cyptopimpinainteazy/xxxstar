#!/usr/bin/env bash
# Are the chain specs we actually ship reachable?
#
# `verify_chain_spec_baseline.sh` pins bootNodes against a recorded baseline, so
# it catches *drift*: change an address and the gate fires. It cannot catch
# *invalidity*: a spec whose bootNodes are `127.0.0.1` matches its own baseline
# perfectly and still cannot be joined by anyone off the machine that produced
# it. That is the gap this gate closes, for the specs that are deployed rather
# than generated locally.
#
# Checked:
#   1. a Live spec with an empty bootNodes list           -> FAIL
#   2. a bootnode on a loopback or unspecified address     -> FAIL
#   3. a bootnode on a private range                       -> WARN (may be a private devnet)
#   4. a Live spec whose genesis.raw.top is empty          -> FAIL (nothing to sync to)
#
# Usage: check_deployable_bootnodes.sh
# Exit 0 = every deployable spec is joinable; 1 = at least one is not.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

python3 - "$REPO_ROOT" <<'PY'
import ipaddress
import json
import os
import re
import sys

repo_root = sys.argv[1]

failures: list[str] = []
warnings: list[str] = []


def multiaddr_ip(addr: str) -> str | None:
    """Pull the IP out of `/ip4/1.2.3.4/tcp/...` or `/ip6/::1/tcp/...`."""
    match = re.match(r"^/ip4/([^/]+)/", addr) or re.match(r"^/ip6/([^/]+)/", addr)
    return match.group(1) if match else None


def check(spec_name: str, spec: dict) -> None:
    chain_type = spec.get("chainType", "?")
    bootnodes = spec.get("bootNodes") or []

    if chain_type != "Live":
        return

    if not bootnodes:
        failures.append(
            f"{spec_name}: Live spec with no bootNodes — a node started from it can never discover a peer"
        )
    for addr in bootnodes:
        raw_ip = multiaddr_ip(addr)
        if raw_ip is None:
            warnings.append(f"{spec_name}: bootNode {addr!r} is not an /ip4 or /ip6 multiaddr")
            continue
        try:
            ip = ipaddress.ip_address(raw_ip)
        except ValueError:
            failures.append(f"{spec_name}: bootNode {addr!r} has an unparseable address")
            continue
        if ip.is_loopback or ip.is_unspecified:
            failures.append(
                f"{spec_name}: bootNode {addr} is on {raw_ip}, which is only reachable from the host that wrote it"
            )
        elif ip.is_private:
            warnings.append(f"{spec_name}: bootNode {addr} is on a private range ({raw_ip})")

    raw = (spec.get("genesis") or {}).get("raw")
    if isinstance(raw, dict):
        top = raw.get("top")
        if isinstance(top, dict) and not top:
            failures.append(
                f"{spec_name}: Live spec with an empty genesis.raw.top — there is no state to sync to"
            )


def load(path: str) -> dict:
    with open(path, encoding="utf-8") as handle:
        return json.load(handle)


# 1. Chain specs referenced by the deployment artifacts.
for relative in (
    "deployment/chain-specs/x3-testnet-raw.json",
    "deployment/chain-specs/fresh/x3-testnet-plain.json",
):
    path = os.path.join(repo_root, relative)
    if not os.path.exists(path):
        continue
    check(relative, load(path))

# 2. The spec embedded in the k8s ConfigMap, which is what a cluster actually
#    mounts — the file on disk is not the artifact that runs.
configmap = os.path.join(repo_root, "k8s/02-configmaps.yaml")
if os.path.exists(configmap):
    with open(configmap, encoding="utf-8") as handle:
        lines = handle.read().splitlines()
    embedded: list[str] = []
    collecting = False
    for line in lines:
        if re.match(r"^\s+x3-testnet-raw\.json:\s*\|", line):
            collecting = True
            continue
        if collecting:
            # The block ends at the next line that is not indented as part of it:
            # the following ConfigMap key (two spaces) or the `---` separator (none).
            if line.strip() and not line.startswith("    "):
                break
            embedded.append(line[4:] if line.startswith("    ") else line)
    if embedded:
        try:
            check("k8s/02-configmaps.yaml:x3-testnet-raw.json", json.loads("\n".join(embedded)))
        except json.JSONDecodeError as err:
            failures.append(f"k8s/02-configmaps.yaml: embedded spec is not valid JSON ({err})")

for warning in warnings:
    print(f"WARN  {warning}")
for failure in failures:
    print(f"FAIL  {failure}")

if failures:
    print(f"\n{len(failures)} deployable chain spec problem(s); {len(warnings)} warning(s)")
    print("These specs are shipped by Dockerfile.validator and k8s/02-configmaps.yaml, so this is")
    print("the list of reasons a stranger cannot join: fix the bootNodes or stop calling the spec")
    print("deployable.")
    sys.exit(1)

print(f"\nOK - every deployable chain spec is reachable ({len(warnings)} warning(s))")
PY
