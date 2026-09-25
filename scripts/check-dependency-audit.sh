#!/usr/bin/env bash
# Run the dependency-advisory gate this repository's configuration was written for.
#
# `.cargo/audit.toml` carries a long ignore list with reasons for each entry, `deny.toml`
# mirrors it, and `.cargo/README.md` explains the vendoring setup -- but no gate ever
# invoked cargo-audit or cargo-deny. An advisory could appear in the lockfile and nothing
# in this repository would say so.
#
# That is not hypothetical. The first run of this gate found RUSTSEC-2026-0285: rustls
# 0.23.44 accepting TLS 1.3 handshake messages across encryption-level boundaries, in a
# crate that ships inside the node through `futures-rustls` -> `libp2p-websocket` ->
# `libp2p` -> `sc-network`. It was unignored and unfixed, and every gate in the fast set
# was green with it present.
#
# Cargo cannot see an advisory that lives only in the GitHub Advisory Database -- those
# have no RustSec id and are covered by `scripts/check-advisory-scope.py`. The two gates
# are complementary, not redundant.
#
# The audit runs with `--no-fetch` so this gate is hermetic and takes about a second, and
# the age of the local RustSec database is checked separately, as a failure rather than a
# warning: a database nobody refreshed would turn this gate into exactly the kind of quiet
# false green the repository keeps finding.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DB="${RUSTSEC_DB:-$HOME/.cargo/advisory-db}"
MAX_AGE_DAYS="${RUSTSEC_DB_MAX_AGE_DAYS:-45}"

AUDIT="$(command -v cargo-audit || true)"
if [ -z "$AUDIT" ] && [ -x "$HOME/.cargo/bin/cargo-audit" ]; then
  AUDIT="$HOME/.cargo/bin/cargo-audit"
fi

if [ -z "$AUDIT" ]; then
  cat >&2 <<'EOF'
check-dependency-audit: cargo-audit is not installed, so the dependency ignore list
check-dependency-audit: in .cargo/audit.toml is unverified.

  cargo install cargo-audit --locked

  # or the prebuilt static binary (hosts with an older glibc need the musl build --
  # the *-unknown-linux-gnu one wants GLIBC_2.38+):
  #   gh release download -R rustsec/rustsec cargo-audit/v0.22.2 \
  #     -p 'cargo-audit-x86_64-unknown-linux-musl-*.tgz' -D /tmp/x3-audit
  #   tar -xzf /tmp/x3-audit/*.tgz -C /tmp/x3-audit
  #   install -m 755 /tmp/x3-audit/*/cargo-audit ~/.cargo/bin/cargo-audit

  # The binary is a cargo multicall, so it must be invoked as `cargo audit` (or
  # `cargo-audit audit`); running `cargo-audit` with no subcommand prints cargo's help.
EOF
  exit 1
fi

if [ ! -d "$DB/.git" ]; then
  echo "check-dependency-audit: no RustSec database at $DB." >&2
  echo "check-dependency-audit: run \`cargo audit\` once with network to fetch it, or set RUSTSEC_DB." >&2
  exit 1
fi

db_epoch="$(git -C "$DB" log -1 --format=%ct)"
db_date="$(git -C "$DB" log -1 --format=%cs)"
age_days=$(( ( $(date +%s) - db_epoch ) / 86400 ))

if [ "$age_days" -gt "$MAX_AGE_DAYS" ]; then
  echo "check-dependency-audit: the RustSec database at $DB is ${age_days} days old" >&2
  echo "check-dependency-audit: (last commit ${db_date}; limit ${MAX_AGE_DAYS} days)." >&2
  echo "check-dependency-audit: refresh it before this gate means anything:" >&2
  echo "  git -C $DB pull --ff-only      # or: cargo audit   (which fetches)" >&2
  exit 1
fi

echo "check-dependency-audit: RustSec database ${db_date} (${age_days}d old), audit --no-fetch"
cd "$ROOT"
"$AUDIT" audit --no-fetch
