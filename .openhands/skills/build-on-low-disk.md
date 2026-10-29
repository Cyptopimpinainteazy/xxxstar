---
name: build-on-low-disk
description: How to build and test Rust in this repo without filling the disk.
triggers:
  - cargo
  - build
  - compile
  - clippy
  - disk
---

# Building on this machine

The disk is nearly full. A fresh `target/` for this workspace grows to 15–35 GB, and
filling the disk breaks every other service on the host. Follow these rules for every
cargo command:

- Keep `SKIP_WASM_BUILD=1` set. The wasm runtime build is the largest single cost and
  is only needed when you are explicitly asked to produce a runtime blob.
- Use the shared build directory in `CARGO_TARGET_DIR` if it is set (the
  `openhands-x3` launcher sets it). Never unset it, and never pass `--target-dir`
  pointing somewhere new.
- Build only what you are changing: `cargo test -p <crate>`, `cargo clippy -p <crate>`.
  Do not run `cargo build --workspace` or `cargo test --workspace` unless asked.
- Check space before a long build: `df -h /home`. If less than 10 GB is free, stop and
  report instead of building.
- If cargo prints `Blocking waiting for file lock on build directory`, another agent is
  using the shared directory. Wait; do not switch to a new target directory.
- Never delete a `target/` directory you did not create in this task.

Find a crate's package name and path in `docs/REPO_MAP.md` before building it.
