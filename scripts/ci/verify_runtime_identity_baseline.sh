#!/usr/bin/env bash
# Hard-invariant gate for the runtime's on-chain identity.
#
# Three consensus surfaces live in a single Rust file. This gate pins all
# three so a silent edit cannot reach mainnet:
#
#   1. RuntimeVersion:
#        spec_name, impl_name, authoring_version,
#        spec_version, impl_version,
#        transaction_version, state_version
#      -- spec_version/transaction_version bumps are consensus-forking
#         events; wallets and nodes switch codepaths on them.
#
#   2. construct_runtime! pallet order (dev and non-dev variants):
#        the ORDER determines pallet indices. Reordering silently
#        renumbers every extrinsic in every downstream pallet and breaks
#        all queued signed txs.
#
#   3. SignedExtra tuple order:
#        the extension order is part of the signed payload. Reorder =
#        every existing signature becomes invalid.
#
# Usage: verify_runtime_identity_baseline.sh <runtime_lib.rs> <baseline.json>
# UPDATE_BASELINE=1 regenerates baseline, preserving accepted_reason per field.
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "::error::usage: $0 <runtime_lib.rs> <baseline.json>"
  exit 1
fi

SRC="$1"
BASELINE="$2"

if [[ ! -f "$SRC" ]]; then
  echo "::error::$SRC missing"
  exit 1
fi

if [[ "${UPDATE_BASELINE:-0}" != "1" && ! -f "$BASELINE" ]]; then
  echo "::error::$BASELINE missing (run with UPDATE_BASELINE=1 to seed)"
  exit 1
fi

python3 - "$SRC" "$BASELINE" <<'PY'
import json, os, re, sys

src_path, baseline_path = sys.argv[1], sys.argv[2]
update = os.environ.get("UPDATE_BASELINE", "0") == "1"

with open(src_path) as f:
    src = f.read()

errors = []

# ---- 1. RuntimeVersion ----
#
# #[sp_version::runtime_version]
# pub const VERSION: sp_version::RuntimeVersion = sp_version::RuntimeVersion {
#     spec_name: create_runtime_str!("x3-chain"),
#     ...
# };
m = re.search(
    r"RuntimeVersion\s*=\s*sp_version::RuntimeVersion\s*\{(.*?)\};",
    src,
    re.DOTALL,
)
if not m:
    print("::error::could not locate RuntimeVersion block in source")
    sys.exit(1)

block = m.group(1)
version = {}
for field in ("spec_name", "impl_name"):
    mm = re.search(rf"{field}\s*:\s*create_runtime_str!\(\s*\"([^\"]+)\"\s*\)", block)
    if not mm:
        errors.append(f"RuntimeVersion.{field} missing")
        continue
    version[field] = mm.group(1)

for field in (
    "authoring_version",
    "spec_version",
    "impl_version",
    "transaction_version",
    "state_version",
):
    mm = re.search(rf"{field}\s*:\s*(\d+)", block)
    if not mm:
        errors.append(f"RuntimeVersion.{field} missing")
        continue
    version[field] = int(mm.group(1))

# ---- 2. construct_runtime! variants ----
#
# Each variant: an optional cfg attribute + construct_runtime!(pub enum Runtime {...});
# We capture each pallet line as a "NAME: path" pair in declared order.
def parse_construct_runtime_blocks(text):
    """Return list of (cfg, [(pallet_name, pallet_path), ...]) in file order."""
    variants = []
    # Walk the file, find each construct_runtime! macro and its enclosing cfg (if any).
    for cr in re.finditer(r"construct_runtime!\s*\(\s*pub\s+enum\s+Runtime\s*\{(.*?)\}\s*\)\s*;", text, re.DOTALL):
        start = cr.start()
        # Look back up to 200 chars for a #[cfg(...)] or #[cfg_attr(...)]. None = universal.
        window = text[max(0, start - 400):start]
        cfg = None
        cfg_matches = list(re.finditer(r"#\[cfg[^\]]*\]", window))
        if cfg_matches:
            cfg = cfg_matches[-1].group(0)
        body = cr.group(1)
        # Each pallet line: NAME: path,   (may carry ::<Instance>)
        # Skip attributes like #[something] and comments.
        pallets = []
        for line in body.splitlines():
            s = line.strip().rstrip(",")
            if not s or s.startswith("//") or s.startswith("#["):
                continue
            mm = re.match(r"([A-Za-z_][A-Za-z0-9_]*)\s*:\s*(.+)$", s)
            if mm:
                name = mm.group(1)
                path = mm.group(2).strip().rstrip(",").strip()
                pallets.append([name, path])
        variants.append({"cfg": cfg, "pallets": pallets})
    return variants

variants = parse_construct_runtime_blocks(src)
if not variants:
    print("::error::no construct_runtime! blocks found")
    sys.exit(1)

# ---- 3. SignedExtra tuple ----
#
# pub type SignedExtra = (
#     frame_system::CheckNonZeroSender<Runtime>,
#     ...
# );
m = re.search(r"pub\s+type\s+SignedExtra\s*=\s*\((.*?)\)\s*;", src, re.DOTALL)
if not m:
    print("::error::SignedExtra tuple not found")
    sys.exit(1)
signed_extra = []
for line in m.group(1).splitlines():
    s = line.strip().rstrip(",")
    if not s or s.startswith("//"):
        continue
    signed_extra.append(s)

if errors:
    for e in errors:
        print(f"::error::{e}")
    sys.exit(1)

current = {
    "source": os.path.basename(src_path),
    "runtime_version": version,
    "construct_runtime_variants": variants,
    "signed_extra": signed_extra,
}

# ---- Baseline diff / write ----

if update:
    accepted = {}
    if os.path.exists(baseline_path):
        try:
            prev = json.load(open(baseline_path))
            accepted = prev.get("accepted_reasons", {})
        except Exception:
            accepted = {}
    out = dict(current)
    out["accepted_reasons"] = accepted
    with open(baseline_path, "w") as f:
        json.dump(out, f, indent=2, sort_keys=True)
        f.write("\n")
    print(
        f"baseline rewritten: spec_version={version['spec_version']}, "
        f"variants={len(variants)}, "
        f"pallets=[{','.join(str(len(v['pallets'])) for v in variants)}], "
        f"signed_extra={len(signed_extra)}"
    )
    sys.exit(0)

baseline = json.load(open(baseline_path))
errs = []

# RuntimeVersion
bv = baseline.get("runtime_version", {})
for k, v in current["runtime_version"].items():
    if bv.get(k) != v:
        errs.append(
            f"runtime_version.{k} drifted: baseline={bv.get(k)!r} current={v!r}"
        )
for k in bv.keys():
    if k not in current["runtime_version"]:
        errs.append(f"runtime_version.{k} disappeared (baseline={bv[k]!r})")

# construct_runtime variants
bvars = baseline.get("construct_runtime_variants", [])
if len(bvars) != len(current["construct_runtime_variants"]):
    errs.append(
        f"construct_runtime! variant count drifted: "
        f"baseline={len(bvars)} current={len(current['construct_runtime_variants'])}"
    )
else:
    for i, (bvar, cvar) in enumerate(zip(bvars, current["construct_runtime_variants"])):
        if bvar.get("cfg") != cvar.get("cfg"):
            errs.append(
                f"construct_runtime[{i}].cfg drifted: "
                f"baseline={bvar.get('cfg')!r} current={cvar.get('cfg')!r}"
            )
        bp = [tuple(p) for p in bvar.get("pallets", [])]
        cp = [tuple(p) for p in cvar.get("pallets", [])]
        if len(bp) != len(cp):
            errs.append(
                f"construct_runtime[{i}] pallet count drifted: "
                f"baseline={len(bp)} current={len(cp)}"
            )
        # Order-sensitive comparison -- index N is pallet N
        max_n = max(len(bp), len(cp))
        reported = 0
        truncated = False
        for n in range(max_n):
            if reported >= 20:
                truncated = True
                break
            if n >= len(bp):
                errs.append(
                    f"construct_runtime[{i}] pallet index {n} added: {cp[n][0]} ({cp[n][1]})"
                )
                reported += 1
            elif n >= len(cp):
                errs.append(
                    f"construct_runtime[{i}] pallet index {n} removed: {bp[n][0]} ({bp[n][1]})"
                )
                reported += 1
            elif bp[n] != cp[n]:
                errs.append(
                    f"construct_runtime[{i}] pallet index {n} drifted: "
                    f"baseline={bp[n][0]}:{bp[n][1]} current={cp[n][0]}:{cp[n][1]}"
                )
                reported += 1
        if truncated:
            errs.append(
                f"construct_runtime[{i}] ... (further drift truncated)"
            )

# SignedExtra
bs = baseline.get("signed_extra", [])
cs = current["signed_extra"]
if len(bs) != len(cs):
    errs.append(
        f"signed_extra length drifted: baseline={len(bs)} current={len(cs)}"
    )
for n, (a, b) in enumerate(zip(bs, cs)):
    if a != b:
        errs.append(
            f"signed_extra[{n}] drifted: baseline={a!r} current={b!r}"
        )
if len(cs) > len(bs):
    for n in range(len(bs), len(cs)):
        errs.append(f"signed_extra[{n}] added: {cs[n]!r}")
if len(bs) > len(cs):
    for n in range(len(cs), len(bs)):
        errs.append(f"signed_extra[{n}] removed: {bs[n]!r}")

if errs:
    for e in errs[:50]:
        print(f"::error::{e}")
    if len(errs) > 50:
        print(f"::error::... and {len(errs)-50} more")
    print(
        "hint: this is a consensus-surface change. "
        "If intended, regenerate baseline with UPDATE_BASELINE=1 and record "
        "the reason under accepted_reasons in the baseline JSON."
    )
    sys.exit(1)

variants_summary = ",".join(
    str(len(v["pallets"])) for v in current["construct_runtime_variants"]
)
print(
    f"runtime identity matches baseline "
    f"(spec_version={current['runtime_version']['spec_version']}, "
    f"variants=[{variants_summary}], "
    f"signed_extra={len(current['signed_extra'])})"
)
PY
