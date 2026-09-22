#!/usr/bin/env python3
"""Build a LOADABLE X3 testnet chain spec from fresh (non-forbidden-seed) validator keys.

Two things this used to get wrong, both of which produced a spec the node refuses:

1. It shelled out to `subkey` for key derivation. `subkey` is not part of this
   repository's toolchain and is frequently absent, so the documented testnet
   genesis path simply failed. The node now derives keys itself
   (`keys generate --key-type aura|grandpa --seed <suri>`), which is what this uses.
2. It wrote a **raw** spec by default. The node's own loader rejects a *Live* spec
   in raw form — `validate_live_json_chain_spec_value` needs
   `genesis.runtimeGenesis.config.<pallet>`, which a raw spec does not have:

       $ x3-chain-node build-spec --chain deployment/chain-specs/x3-testnet-raw.json
       Error: Input("Live chain spec requires at least one Aura authority")

   Plain is the loadable form for a Live chain and is now the default; `--raw`
   emits the raw form as well, for distribution tooling that wants it.

The script asserts the artifact it produced actually loads, by feeding it back
through `build-spec --chain <file>`.

One master seed per validator: its sr25519 key is the Aura authority, its ed25519
key is the GRANDPA authority — the same scheme-specific derivation `X3_DEV_SEED`
uses to insert both block-author keys at boot, so a validator started with
`X3_DEV_SEED=<master>` matches the genesis.

TESTNET-ONLY key material under $OUT_DIR/validator-keys. SURIs are NOT committed.
"""
import json, os, re, subprocess, sys, secrets, datetime

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(os.path.dirname(__file__))))
# The node binary: honour X3_NODE_BIN, else the usual target paths.
NODE = os.environ.get("X3_NODE_BIN") or next(
    (
        p
        for p in (
            os.path.join(ROOT, "target", "release", "x3-chain-node"),
            os.path.join(ROOT, "target", "debug", "x3-chain-node"),
        )
        if os.path.exists(p)
    ),
    os.path.join(ROOT, "target", "release", "x3-chain-node"),
)
COUNT = int(sys.argv[1]) if len(sys.argv) > 1 else 7
# Plain is the default because that is the form the node loads for a Live chain.
# `--raw` additionally emits the raw form (distribution tooling); `--skip-raw` is
# still accepted so existing callers keep working.
make_raw = "--raw" in sys.argv
COUNT = max(1, min(COUNT, 12))
# Its own directory, ignored by git. `deployment/chain-specs/fresh/*.json` is a
# tracked fixture shared by the `run-fresh-*` / mesh tooling, and this script used to
# overwrite it along with `authorities.json` and friends — leaving a dirty tree and,
# worse, a spec whose authorities matched the *new* seeds while the committed fixture
# still looked authoritative. `OUT_DIR` overrides.
outdir = os.environ.get("OUT_DIR") or os.path.join(
    ROOT, "deployment", "chain-specs", "fresh", "generated"
)
keysdir = os.path.join(outdir, "validator-keys")
os.makedirs(keysdir, exist_ok=True)


def public_ss58(suri, key_type):
    """Derive an SS58 public key with the node's own CLI.

    `keys generate` writes the key to stdout and everything else (the banner) to
    stderr, so stdout is pipeable. A placeholder or stale binary prints advice
    instead of a key; the regex check turns that into a clear error here rather
    than a confusing failure from the node later.
    """
    out = subprocess.run([NODE, "keys", "generate", "--key-type", key_type,
                          "--seed", suri, "--output", "ss58"],
                         capture_output=True, text=True, check=True).stdout.strip()
    if not re.fullmatch(r"[1-9A-HJ-NP-Za-km-z]{47,48}", out):
        raise RuntimeError(
            f"{NODE} keys generate returned {out[:60]!r} for {key_type}, not an SS58 "
            "address — rebuild the node (cargo build --release -p x3-chain-node)"
        )
    return out


def public_hex(suri, key_type):
    """The public key for `suri` in hex, with the node's own CLI (`--output hex`)."""
    out = subprocess.run([NODE, "keys", "generate", "--key-type", key_type,
                          "--seed", suri, "--output", "hex"],
                         capture_output=True, text=True, check=True).stdout.strip()
    if not re.fullmatch(r"0x[0-9a-f]{64}", out):
        raise RuntimeError(
            f"{NODE} keys generate ({key_type}, hex) returned {out[:60]!r}, not a "
            "32-byte hex public key"
        )
    return out


def peer_id_for(pubkey_hex):
    """libp2p peer id for an ed25519 public key (one implementation, in scripts/mainnet)."""
    helper = os.path.join(ROOT, "scripts", "mainnet", "peer-id-from-ed25519-pubkey.py")
    out = subprocess.run(["python3", helper, pubkey_hex],
                         capture_output=True, text=True, check=True).stdout.strip()
    # ed25519 identity multihash → base58btc: `12D3Koo` plus 44–46 characters.
    if not re.fullmatch(r"12D3Koo[1-9A-HJ-NP-Za-km-z]{44,46}", out):
        raise RuntimeError(f"peer id helper returned {out[:60]!r}, not a peer id")
    return out


suri_log = os.path.join(keysdir, "suris.txt")
f = open(suri_log, "w")
f.write("# TESTNET-ONLY master seeds (generated %s). NEVER use on mainnet; not committed.\n"
        % datetime.datetime.utcnow())
aut = []
endowed = []
bootnodes = []
# The bootnode addresses have to be the p2p ports the launcher will actually listen on
# (`P2P_BASE`); the *RPC* base is orthogonal and need not match.
p2p_base = int(os.environ.get("P2P_BASE", "30333"))
for i in range(1, COUNT + 1):
    master = "0x" + secrets.token_hex(32)
    rec = os.path.join(keysdir, f"validator-{i}.suri")
    with open(rec, "w") as vf:
        vf.write("seed=" + master + "\n")
        vf.write("aura=" + master + "\n")
        vf.write("grandpa=" + master + "\n")
    os.chmod(rec, 0o600)
    f.write(f"validator-{i} = {master}\n")
    aura = public_ss58(master, "aura")     # sr25519 — Aura authority
    gran = public_ss58(master, "grandpa")  # ed25519 — GRANDPA authority
    # Network identity: one 32-byte secret per validator, written beside the seed so
    # the launcher can start each node with the `--node-key` its spec's bootNodes
    # entry was derived from. The node needs *some* identity — without one it exits
    # with `NetworkKeyNotFound` — and a Live spec needs at least one bootnode whose
    # peer id is therefore known before the node starts.
    nodekey_path = os.path.join(keysdir, f"validator-{i}.nodekey")
    nodekey = "0x" + secrets.token_hex(32)
    with open(nodekey_path, "w") as nf:
        nf.write(nodekey + "\n")
    os.chmod(nodekey_path, 0o600)
    node_pub = public_hex(nodekey, "grandpa")
    peer = peer_id_for(node_pub)
    bootnodes.append(f"/ip4/127.0.0.1/tcp/{p2p_base + i - 1}/p2p/{peer}")
    # The authority's own Aura account is endowed; there is no separate "//acct"
    # derivation to get wrong (the old subkey call derived one, and it is not
    # needed: a validator that holds a bond and an authority key may be one account).
    acct = aura
    aut.append({"aura": aura, "grandpa": gran})
    endowed.append(acct)
f.close()
os.chmod(suri_log, 0o600)
print(f"[keys] {COUNT} master seeds -> {keysdir} (SURIs NOT committed)")

authf = os.path.join(outdir, "authorities.json")
endf = os.path.join(outdir, "endowed.json")
counf = os.path.join(outdir, "council.json")
treaf = os.path.join(outdir, "treasury.json")
json.dump(aut, open(authf, "w"))
json.dump(endowed, open(endf, "w"))
json.dump(endowed[: max(2, COUNT // 3)], open(counf, "w"))
json.dump(endowed[: max(2, COUNT // 3)], open(treaf, "w"))

env = dict(os.environ)
env["X3_TESTNET_AUTHORITIES"] = json.dumps(aut)
env["X3_TESTNET_ENDOWED_ACCOUNTS"] = json.dumps(endowed)
env["X3_TESTNET_COUNCIL_MEMBERS"] = json.dumps(endowed[: max(2, COUNT // 3)])
env["X3_TESTNET_TREASURY_SIGNERS"] = json.dumps(endowed[: max(2, COUNT // 3)])
env["X3_EVM_ESCROW_ADDR"] = "0x" + "11" * 20
env["X3_SVM_ESCROW_ADDR"] = "0x" + "22" * 32
# A Live spec with no bootNodes cannot start a node at all:
#   Error: Input("Live chain spec requires at least one bootnode")
# The entries have to be the peer ids of the identities the launcher will start,
# so they are derived here from each validator's node key before the spec exists.
env["TESTNET_BOOTNODES"] = ",".join(bootnodes)
cmd = [NODE, "build-spec", "--chain=testnet", "--disable-log-color"]
if make_raw:
    cmd.append("--raw")
print("[spec] running:", " ".join(cmd))
r = subprocess.run(cmd, env=env, capture_output=True, text=True)
if r.returncode != 0:
    sys.stderr.write("build-spec FAILED:\n" + (r.stderr or "")[-4000:] + "\n")
    sys.exit(1)
text = r.stdout
start = text.find("{")
json.loads(text[start:])
name = "x3-testnet-raw.json" if make_raw else "x3-testnet-plain.json"
outfile = os.path.join(outdir, name)
open(outfile, "w").write(text[start:])
print(f"[spec] {name} written -> {outdir} ({os.path.getsize(outfile)} bytes)")

# The artifact has to survive the node's own loader, which is the check that
# would have caught the raw-Live spec this repository used to ship:
#   Error: Input("Live chain spec requires at least one Aura authority")
check = subprocess.run(
    [NODE, "build-spec", "--chain", outfile, "--disable-log-color"],
    capture_output=True, text=True,
)
if check.returncode != 0:
    sys.stderr.write(
        f"[spec] the node cannot load the spec this script just wrote: {outfile}\n"
        + (check.stderr or "")[-2000:]
        + "\n"
    )
    sys.exit(1)
print(f"[spec] {name} loads in the node (build-spec --chain {name} -> ok)")

# The loader above accepts a spec with no bootNodes; the node's *validator* startup
# does not. Assert the form the launcher actually needs, and that every entry is one
# of the peer ids this run derived — a bootnode list naming some other identity is a
# network that never connects.
written = json.loads(open(outfile).read())
spec_boot = written.get("bootNodes") or []
missing = [b for b in bootnodes if b not in spec_boot]
if not spec_boot:
    sys.stderr.write(
        f"[spec] {name} has no bootNodes: a validator started from it exits with "
        'Error: Input("Live chain spec requires at least one bootnode")\n'
    )
    sys.exit(1)
if missing:
    sys.stderr.write(
        f"[spec] {name} is missing {len(missing)} of the {len(bootnodes)} derived "
        f"bootnode entries, e.g. {missing[0]}\n"
    )
    sys.exit(1)
print(f"[spec] {name} carries all {len(bootnodes)} derived bootnodes "
      f"(peer ids match the node keys beside the seeds)")
print(f"[spec] launch it with:")
print(
    "  COUNT=%d CHAIN_SPEC=%s NODE_BIN=%s "
    "bash scripts/testnet/run-7-validators-local.sh"
    % (COUNT, outfile, NODE)
)

if make_raw:
    print(
        "[spec] note: the raw form is for distribution tooling. A *Live* spec in "
        "raw form is rejected by the node's loader — validators must be given the "
        "plain file."
    )
print("[OK] SUCCESS")
