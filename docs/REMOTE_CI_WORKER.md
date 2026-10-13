# SSH CI worker

`x3star3` (`lojak@192.168.0.55`) is a secondary local CI worker. It has
16 hardware threads, 80 GiB RAM and approximately 850 GiB free disk at setup.
Jobs use four Cargo workers and one CI gate at a time, with an exclusive lock
and reduced CPU priority. There is no additional listener or paid service.

The controller requires an existing trusted SSH host key and key-based login.
It refuses unknown host keys. Only trusted repository commits should run here:
CI commands execute repository code with the worker user's privileges.
Do not use this account for untrusted pull requests or production secrets.

## Provisioning

The user-owned installation is `~/.local/share/x3-ci`; no sudo is required.
The installed Rust toolchain is 1.90.0 for x86_64 Linux, including rustfmt,
Clippy, `wasm32-unknown-unknown` and `wasm32v1-none`. Toolchain files were copied
with checksum-enabled rsync from the existing controller installation. The
worker uses its own Cargo download cache; no controller Cargo credentials,
GitHub token or SSH private keys were copied.

Required host programs are Bash, Git, Python 3, SSH, flock, nice, a C/C++
compiler, make, cmake, pkg-config and OpenSSL development headers.
The installation layout is:

```text
~/.local/share/x3-ci/
  bin/run-checks.sh       # scripts/ci/remote-worker.sh
  bin/rustup
  bin/protoc             # protoc 3.12.4
  lib/                   # libprotobuf.so.23 and libprotoc.so.23
  include/google/        # protoc headers
  rustup/toolchains/1.90.0-x86_64-unknown-linux-gnu/
  source/                # public repository clone
  cargo/                 # worker's persistent download cache
  targets/<full-sha>/     # build outputs isolated by source commit
  jobs/<time>-<sha>/      # detached source worktree, logs and receipt
```

Clone `https://github.com/Cyptopimpinainteazy/xxxstar.git` into `source`.
Install the repository's worker script as `bin/run-checks.sh` after provisioning
the tools. Verify `rustc --version`, `cargo --version` and `protoc --version`
using the environment exported by the script. Deploy script updates only when
the worker is idle. Preserve job logs and receipts before reclaiming disk.
Node, Foundry, Solana, Docker, cargo-audit, cargo-deny and srtool are not installed
by this setup; gates requiring them need separate provisioning.

## Run and inspect

From a clean controller checkout:

```bash
scripts/ci/run-remote-checks.sh lojak@192.168.0.55 FULL_40_CHARACTER_SHA workspace-check
```

Use comma-separated gate slugs accepted by `scripts/local-ci.sh --only`.
The process returns the CI command's exit code and prints its evidence directory.
Each job fetches and verifies the full commit, creates a detached worktree and
retains `output.log`, the local CI gate logs under `source/.ai/runlogs`, and
`receipt.json`. The receipt records host, source commit, selected gates, exit
code, source cleanliness and the output log's SHA-256. A hash detects log changes;
it is not a cryptographic attestation by an independent verifier.

A scoped worker PASS applies only to the selected gates. It does **not** publish
or satisfy GitHub's required full `x3/local-ci` status. Required release gates
must still run on the exact change being merged. Busy workers return exit 75;
argument errors return exit 2.
