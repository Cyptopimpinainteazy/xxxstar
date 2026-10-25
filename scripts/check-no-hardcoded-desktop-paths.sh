#!/usr/bin/env bash
# check-no-hardcoded-desktop-paths.sh — fail when a script on the bring-up path
# hardcodes an absolute /home/<user>/Desktop/ path.
#
# Why this exists: several scripts and systemd units named
# `/home/lojak/Desktop/X3_ATOMIC_STAR`, a directory that does not exist on this
# box. The failure mode is nasty — one script `mkdir -p`'d the missing root and
# then reported `manifest path tests/e2e/Cargo.toml does not exist`, which reads
# like a missing crate rather than a wrong root. The operator is about to bring
# up seven physical validator servers, so every entry point on that path has to
# work from a checkout at any path.
#
# Scope: tracked shell entry points under `scripts/` and at the repository root,
# plus the systemd units under `scripts/`. `%h/Desktop/...` is *not* flagged —
# that is systemd's portable home specifier, not a hardcoded user path.
#
# A line is allowed only when it carries one of the documented override tokens
# below; each allowance is printed on every run so it stays visible.
#
# Usage: scripts/check-no-hardcoded-desktop-paths.sh [--quiet]
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

QUIET=0
[ "${1:-}" = "--quiet" ] && QUIET=1

# Documented overrides. Each entry is <file-substring>:<token>; a line containing
# the token in that file is intentional and named here. Keep this list shrink-only.
ALLOW=(
  # A system unit (User=lojak) cannot use `%h` — that expands to the manager's
  # home for system units — and systemd does not expand $VAR in
  # WorkingDirectory=/Environment=, so it names this checkout explicitly.
  "scripts/testnet/x3-testnet-verify.service:Desktop/xxxstar-main"
)

# The defect: /home/<somebody>/Desktop/ in an executable line. The user segment is
# matched loosely so a wrong username (not just a wrong directory) is caught.
PATTERN='/home/[^/[:space:]"'"'"']*/Desktop/'

allow_for() { # allow_for <file>
  # Print the documented override token for this file, if any.
  local file="$1" entry
  for entry in "${ALLOW[@]}"; do
    case "$entry" in
      "${file}:"*) printf '%s' "${entry#*:}" ;;
    esac
  done
  return 0
}

is_candidate() { # is_candidate <path>
  local p="$1"
  case "$p" in
    scripts/*.sh|scripts/*.bash|scripts/*/*.sh|scripts/*/*.bash|scripts/*/*.service|scripts/*.service|scripts/*/*/*.sh|scripts/*/*/*.service) return 0 ;;
    *.sh|*.bash) case "$p" in */*) return 1 ;; *) return 0 ;; esac ;;
    *) return 1 ;;
  esac
}

failures=0
scanned=0

while IFS= read -r file; do
  [ -n "$file" ] || continue
  [ -f "$file" ] || continue
  is_candidate "$file" || continue
  scanned=$((scanned + 1))

  token="$(allow_for "$file")"

  while IFS= read -r hit; do
    [ -n "$hit" ] || continue
    lineno="${hit%%:*}"
    text="${hit#*:}"
    # Comment lines are documentation, not behavior.
    case "${text#"${text%%[![:space:]]*}"}" in '#'*) continue ;; esac
    if [ -n "$token" ] && printf '%s' "$text" | grep -qF -- "$token"; then
      [ "$QUIET" = 1 ] || printf '  allowed  %s:%s  (documented override: %s)\n' "$file" "$lineno" "$token"
      continue
    fi
    printf 'x hardcoded desktop path %s:%s\n    %s\n' "$file" "$lineno" "$text"
    failures=$((failures + 1))
  done < <(grep -nE -- "$PATTERN" "$file" 2>/dev/null || true)
done < <(git ls-files --cached --others --exclude-standard -- 'scripts' '*.sh' '*.bash' 2>/dev/null)

for entry in "${ALLOW[@]}"; do
  [ "$QUIET" = 1 ] || printf 'note: documented override %s\n' "$entry"
done

if [ "$failures" -gt 0 ]; then
  printf '\n%d hardcoded desktop path(s) on the bring-up path; derive the root from\n' "$failures"
  printf 'BASH_SOURCE or use a documented %%h/Environment override.\n'
  exit 1
fi

[ "$QUIET" = 1 ] || printf 'ok: no hardcoded desktop paths in %d scanned bring-up scripts/units\n' "$scanned"
