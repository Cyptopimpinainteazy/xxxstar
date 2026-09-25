#!/usr/bin/env bash
# Run the dependency-advisory gate this repository's configuration was written for.
#
# `.cargo/audit.toml` carries an ignore list with a reason per entry and `deny.toml`
# mirrors it, but no gate ever invoked cargo-audit or cargo-deny. An advisory could
# appear in the lockfile and nothing in this repository would say so.
#
# That is not hypothetical. The first run found RUSTSEC-2026-0285: rustls 0.23.44
# accepting TLS 1.3 handshake messages across encryption-level boundaries, in a crate
# that ships inside the node through `futures-rustls` -> `libp2p-websocket` ->
# `libp2p` -> `sc-network`. It was unignored and unfixed, and every gate in the fast
# set was green with it present.
#
# Both tools run, because they answer different halves of the same question:
#
#   cargo audit   audits every package in Cargo.lock against the RustSec database,
#                 honouring `.cargo/audit.toml`. This is what catches an advisory in a
#                 package the build does not actually reach.
#
#   cargo deny    builds a graph for the four targets in deny.toml's `[graph]` and,
#                 with `-D advisory-not-detected`, fails when an ignore entry matches
#                 nothing. That is the property that keeps the ignore lists from rotting:
#                 this repository had 51 entries in one file and 35 in the other, of
#                 which only 15 suppressed anything at all.
#
# Cargo cannot see an advisory that lives only in the GitHub Advisory Database, because
# those have no RustSec id; `scripts/check-advisory-scope.py` (the `advisory scope` gate)
# covers that half and cross-checks that the two ignore lists differ only where it says
# they may.
#
# Both audits run without fetching, so this gate is hermetic and takes a few seconds, and
# the age of the local RustSec database is checked separately as a failure rather than a
# warning: a database nobody refreshed would turn this gate into exactly the kind of quiet
# false green the repository keeps finding.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DB="${RUSTSEC_DB:-$HOME/.cargo/advisory-db}"
MAX_AGE_DAYS="${RUSTSEC_DB_MAX_AGE_DAYS:-45}"

find_tool() {
  command -v "$1" 2>/dev/null || { [ -x "$HOME/.cargo/bin/$1" ] && echo "$HOME/.cargo/bin/$1"; }
}

AUDIT="$(find_tool cargo-audit || true)"
DENY="$(find_tool cargo-deny || true)"

missing=0
if [ -z "$AUDIT" ]; then
  cat >&2 <<'EOF'
check-dependency-audit: cargo-audit is not installed, so the ignore list in
check-dependency-audit: .cargo/audit.toml is unverified.

  cargo install cargo-audit --locked

  # or the prebuilt static binary (hosts with an older glibc need the musl build --
  # the *-unknown-linux-gnu one wants GLIBC_2.38+):
  #   gh release download -R rustsec/rustsec cargo-audit/v0.22.2 \
  #     -p 'cargo-audit-x86_64-unknown-linux-musl-*.tgz' -D /tmp/x3-audit
  #   tar -xzf /tmp/x3-audit/*.tgz -C /tmp/x3-audit
  #   install -m 755 /tmp/x3-audit/*/cargo-audit ~/.cargo/bin/cargo-audit

  # The binary is a cargo multicall, so invoke it as `cargo audit` (or `cargo-audit audit`).
EOF
  missing=1
fi

if [ -z "$DENY" ]; then
  cat >&2 <<'EOF'
check-dependency-audit: cargo-deny is not installed, so nothing checks that the
check-dependency-audit: ignore lists in deny.toml and .cargo/audit.toml still match
check-dependency-audit: anything.

  cargo install cargo-deny --locked

  # or the prebuilt static binary:
  #   gh release download -R EmbarkStudios/cargo-deny 0.20.2 \
  #     -p 'cargo-deny-0.20.2-x86_64-unknown-linux-musl.tar.gz*' -D /tmp/x3-deny
  #   (cd /tmp/x3-deny && sha256sum -c *.sha256 && tar -xzf *.tar.gz)
  #   install -m 755 /tmp/x3-deny/*/cargo-deny ~/.cargo/bin/cargo-deny

  # Like cargo-audit, it is a cargo multicall: invoke it as `cargo deny ...`.
EOF
  missing=1
fi
[ "$missing" -eq 0 ] || exit 1

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

cd "$ROOT"

echo "check-dependency-audit: RustSec database ${db_date} (${age_days}d old)"
echo "check-dependency-audit: cargo audit --no-fetch (every package in Cargo.lock)"
"$AUDIT" audit --no-fetch

echo "check-dependency-audit: cargo deny check advisories -D advisory-not-detected (built graph)"
"$DENY" deny check advisories -D advisory-not-detected
