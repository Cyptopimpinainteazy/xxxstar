#!/usr/bin/env python3
"""Build a VALID X3 testnet chain spec from fresh (non-forbidden-seed) validator keys.

Why: the runtime requires Live chain specs with Aura+Grandpa authorities, and the stale
dev-seed raw spec is invalid. The supported path (--chain=testnet build-spec) is env-gated
and forbids known seeds. This derives per-validator a single master seed whose sr25519 key
is the Aura authority and ed25519 key is the Grandpa authority (same phrase, scheme-specific
derivation, matching how `X3_DEV_SEED` is used to insert both block-author keys at boot).

TESTNET-ONLY key material under $OUT_DIR/validator-keys. SURIs are NOT committed.
"""
import json, os, re, subprocess, sys, secrets, datetime

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(os.path.dirname(__file__))))
NODE = os.path.join(ROOT, "target", "release", "x3-chain-node")
COUNT = int(sys.argv[1]) if len(sys.argv) > 1 else 7
make_raw = "--skip-raw" not in sys.argv
COUNT = max(1, min(COUNT, 12))
outdir = os.path.join(ROOT, "deployment", "chain-specs", "fresh")
keysdir = os.path.join(outdir, "validator-keys")
os.makedirs(keysdir, exist_ok=True)


def subkey_inspect_ss58(suri, scheme):
    out = subprocess.run(["subkey", "inspect", "--scheme", scheme, suri],
                         capture_output=True, text=True, check=True).stdout
    for line in out.splitlines():
        m = re.match(r"\s*Public key \(SS58\):\s*(\S+)", line)
        if m:
            return m.group(1)
    raise RuntimeError(f"could not parse subkey output for {scheme}")


suri_log = os.path.join(keysdir, "suris.txt")
f = open(suri_log, "w")
f.write("# TESTNET-ONLY master seeds (generated %s). NEVER use on mainnet; not committed.\n"
        % datetime.datetime.utcnow())
aut = []
endowed = []
for i in range(1, COUNT + 1):
    master = "0x" + secrets.token_hex(32)
    rec = os.path.join(keysdir, f"validator-{i}.suri")
    with open(rec, "w") as vf:
        vf.write("seed=" + master + "\n")
        vf.write("aura=" + master + "\n")
        vf.write("grandpa=" + master + "\n")
    os.chmod(rec, 0o600)
    f.write(f"validator-{i} = {master}\n")
    aura = subkey_inspect_ss58(master, "sr25519")     # Aura authority
    gran = subkey_inspect_ss58(master, "ed25519")     # Grandpa authority
    acct = subkey_inspect_ss58(master + "//acct", "sr25519")
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
json.dump(endowed[: max(1, COUNT // 3)], open(counf, "w"))
json.dump(endowed[: max(1, COUNT // 3)], open(treaf, "w"))

env = dict(os.environ)
env["X3_TESTNET_AUTHORITIES"] = json.dumps(aut)
env["X3_TESTNET_ENDOWED_ACCOUNTS"] = json.dumps(endowed)
env["X3_TESTNET_COUNCIL_MEMBERS"] = json.dumps(endowed[: max(1, COUNT // 3)])
env["X3_TESTNET_TREASURY_SIGNERS"] = json.dumps(endowed[: max(1, COUNT // 3)])
env["X3_EVM_ESCROW_ADDR"] = "0x" + "11" * 20
env["X3_SVM_ESCROW_ADDR"] = "0x" + "22" * 32
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
print("[OK] SUCCESS")
