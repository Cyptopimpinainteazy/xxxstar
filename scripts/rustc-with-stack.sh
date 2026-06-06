#!/usr/bin/env bash
set -euo pipefail

export RUST_MIN_STACK="${RUSTC_MIN_STACK_OVERRIDE:-68719476736}"
exec "$@"