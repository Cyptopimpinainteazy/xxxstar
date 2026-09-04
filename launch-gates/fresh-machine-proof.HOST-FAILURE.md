# `fresh-machine-proof.sh` — Host Hardware Failure (2026-04-28)

## Bottom line

`launch-gates/fresh-machine-proof.sh` ran end-to-end twice on this development
host with the following script-level fixes already applied:

* dynamic `REPO_URL` (was hardcoded to a dead URL)
* real critical-pallet names
* `WORKSPACE_ROOT` / `PROOF_LOG` absolutized so logging survives `cd /tmp`
* `((var++))` replaced with `var=$((var+1))` to play nice with `set -e`
* `NODE_BIN=""` initialized + guarded with `${NODE_BIN:-}` in steps 8–9
* `CARGO_BUILD_JOBS=4`, `NUM_JOBS=4`, `MAKEFLAGS=-j4` to limit parallel C++ jobs

**Both runs FAILED at step 3 (`cargo check --workspace --release`)** with the
same root cause: native compiler processes segfaulted at random addresses
inside unrelated stages of the build.

**This is not an X3 codebase defect.** The host RAM (or kernel/CPU/cache) is
producing memory corruption. `Fresh machine boot` cannot be flipped to PASS on
this host until the underlying hardware fault is resolved.

## Evidence (kernel ring buffer, 19:19–19:25 local)

`launch-gates/evidence/host-segfaults-20260428-1926.txt` — 23 segfault lines
across at least five distinct binaries:

| Process              | Count | Random address signature                     |
|----------------------|------:|----------------------------------------------|
| `cc1plus` (g++ 11)   |     5 | `at 0x21`, `at 0x76029c755900`, …            |
| `build-script-bu`    |     1 | `at 0x575ca3c9c393` (heap)                   |
| `rustc` / librustc   |     2 | `at 0x7cd1c7e941c2`, `at 0x3` (libgcc_s)     |
| `python3` (apport)   |     2 | `at 0x0`, `at 0x18`                          |
| `chrome` ServiceWorker|    1 | `at 0x7f52a487d9c8`                          |

Random different IPs across unrelated processes is a classic signature of
faulty RAM or a CPU/cache fault, not a software bug.

## What the script said

Run 2 (after all bash fixes), `launch-gates/evidence/proof-fresh-machine.log`:

```
[19:23:02] Step 3: Running cargo check (full workspace)...
error: could not compile `cpp_demangle` (lib)
process didn't exit successfully: `rustc … cpp_demangle …` (signal: 11, SIGSEGV: invalid memory reference)
❌ FAIL: cargo check failed
```

Run 1, same log entry showed an analogous **g++ ICE** in
`stl_tree.h` while compiling `librocksdb-sys`'s `transaction_util.cc`:

```
internal compiler error: Segmentation fault
```

Different process, different file, same class of fault.

## What this means for `mainnet-gate`

`Fresh machine boot` will continue to report `UNKNOWN` until either:

1. The host hardware is repaired (run `memtest86+` overnight; if errors,
   replace DIMMs), **OR**
2. The proof is run on a different physical/virtual machine (clean Ubuntu
   22.04 + `rustup-init` + the same script).

The script itself is now believed correct. The five blockers found and fixed in
this session were:

1. Hardcoded `REPO_URL` → unreachable repo.
2. Critical-pallet names referenced packages that don't exist.
3. `PROOF_LOG` was relative; `cd /tmp/...` broke logging.
4. `((var++))` exits 1 when `var=0`, tripping `set -e`.
5. `NODE_BIN` set only on build success; unbound under `set -u` masked
   downstream failures.

Plus the parallelism cap added defensively against the host segfaults — that
did not help because the faults are not load-related; they reproduced at
`-j4` as well as `-j14`.

## Recommended next step (do NOT mark Fresh-machine PASS)

Run on a different host with at least 16 GB of confirmed-good RAM:

```bash
git clone https://github.com/Cyptopimpinainteazy/x3-atomic-star.git
cd x3-atomic-star
bash launch-gates/fresh-machine-proof.sh .
```

When that produces `RESULT: ✅ PASS`, copy the `proof-fresh-machine.log` back
into this repo as `launch-gates/evidence/proof-fresh-machine.log` and re-run
`./target/release/x3-proof mainnet-gate -v`.

Until then, the gate's honest status remains `BLOCKED — host hardware`.
