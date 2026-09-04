#!/usr/bin/env python3
"""Build a VALID X3 testnet chain spec from fresh (non-forbidden-seed) validator keys.

Why: the runtime rejects the stale dev-seed raw spec (no Aura authorities), and the
supported path (--chain=testnet build-spec) is env-gated + forbids known seeds. This
generates fresh validator authorities and invokes the node's own spec builder with the
X3_TESTNET_* env contract, so the produced spec is guaranteed consistent with the binary.

TESTNET-ONLY key material under $OUT_DIR/validator-keys. Never echoed to logs.
"""
import json, os, re, subprocess, sys, secrets, base64

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(os.path.dirname(__file__))))
NODE = os.path.join(ROOT, "target", "release", "x3-chain-node")
COUNT = int(sys.argv[1]) if len(sys.argv) > 1 else 7
RAW = "--raw" if "--skip-raw" not in sys.argv else ""
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

def b58ok(a, scheme):
    return a

def rand_suri(kind, i):
    # Valid 32-byte secret as 0x-hex; distinct per role/validator
    return "0x" + secrets.token_hex(32)

# track suris (testnet only). save to chmod600 file.
suri_log = os.path.join(keysdir, "suris.txt")
f = open(suri_log, "w")
f.write("# TESTNET-ONLY validator SURIs (generated %s). NEVER use on mainnet.\n" % __import__("datetime").datetime.utcnow())
for i in range(1, COUNT + 1):
    aura_suri = rand_suri("aura", i)
    gran_suri = rand_suri("gran", i)
    acct_suri = rand_suri("acct", i)
    f.write(f"validator-{i} aura={aura_suri}\n")
    f.write(f"validator-{i} grandpa={gran_suri}\n")
    f.write(f"validator-{i} account={acct_suri}\n")
    # stash per-validator suri file (keystore injection needs full URIs)
    with open(os.path.join(keysdir, f"validator-{i}.suri"), "w") as vf:
        vf.write(f"aura={aura_suri}\n")
        vf.write(f"grandpa={gran_suri}\n")
        vf.write(f"account={acct_suri}\n")
    os.chmod(os.path.join(keysdir, f"validator-{i}.suri"), 0o600)
f.close()
os.chmod(suri_log, 0o600)
print(f"[keys] generated {COUNT} fresh validator keypairs -> {keysdir}")

# now derive authorities, endowed/council/treasury from the authority account addresses
aut = []
endowed = []
for i in range(1, COUNT + 1):
    with open(os.path.join(keysdir, f"validator-{i}.suri")) as vf:
        kv = dict(l.split("=", 1) for l in vf.read().splitlines() if l)
    aura = subkey_inspect_ss58(kv["aura"].strip(), "sr25519")
    gran = subkey_inspect_ss58(kv["grandpa"].strip(), "ed25519")
    acct = subkey_inspect_ss58(kv["account"].strip(), "sr25519")
    aut.append({"aura": aura, "grandpa": gran})
    endowed.append(acct)

authf = os.path.join(outdir, "authorities.json")
endf = os.path.join(outdir, "endowed.json")
counf = os.path.join(outdir, "council.json")
treaf = os.path.join(outdir, "treasury.json")
json.dump(aut, open(authf, "w"))
json.dump(endowed, open(endf, "w"))
json.dump(endowed[: max(1, COUNT // 2)], open(counf, "w"))
json.dump(endowed[: max(1, COUNT // 3)], open(treaf, "w"))
print("[keys] authorities/endowed/council/treasury JSON written to", outdir)

# optional simple EVM/SVM escrow addresses
evm = "0x" + "11" * 20          # deterministic non-zero placeholder (testnet)
svm = "0x" + "22" * 32
env = dict(os.environ)
env["X3_TESTNET_AUTHORITIES"] = json.dumps(aut)
env["X3_TESTNET_ENDOWED_ACCOUNTS"] = json.dumps(endowed)
env["X3_TESTNET_COUNCIL_MEMBERS"] = json.dumps(endowed[: max(1, COUNT // 3)])
env["X3_TESTNET_TREASURY_SIGNERS"] = json.dumps(endowed[: max(1, COUNT // 3)])
env["X3_EVM_ESCROW_ADDR"] = evm
env["X3_SVM_ESCROW_ADDR"] = svm
plain = os.path.join(outdir, "x3-testnet-plain.json")
rawp = os.path.join(outdir, "x3-testnet-raw.json")
cmd = [NODE, "build-spec", "--chain=testnet", "--disable-log-color"] + ([RAW] if RAW == "--raw" else [])
print("[spec] running:", " ".join(cmd))
r = subprocess.run(cmd, env=env, capture_output=True, text=True)
if r.returncode != 0:
    sys.stderr.write("build-spec FAILED:\n" + (r.stderr or "")[-4000:] + "\n")
    sys.exit(1)
# extract JSON from banner-ish stdout
text = r.stdout
start = text.find("{")
json.loads(text[start:])  # validate
open(rawp, "w").write(text[start:])
if RAW == "--raw":
    print("[spec] raw spec written:", rawp)
else:
    open(plain, "w").write(text[start:])
    print("[spec] plain spec written:", plain)
print("[OK] SUCCESS")
