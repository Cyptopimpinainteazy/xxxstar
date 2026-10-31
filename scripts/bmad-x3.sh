#!/usr/bin/env bash
# X3 brownfield BMad installer. Never invoke in CI or on a release/validator node.
set -euo pipefail

usage() {
  cat <<'HELP'
Usage: bash scripts/bmad-x3.sh check|install|status
  check    Verify X3 checkout and BMad prerequisites, without network access.
  install  Install pinned stable BMad Method v6.12.4 for Codex on a clean feature branch.
  status   Inspect the BMad installation manifest (not a proof of execution).
BMad runs third-party npm code; review package provenance before install.
HELP
}
fail() { printf 'BLOCKED: %s\n' "$*" >&2; exit 2; }

if (( $# != 1 )); then usage; exit 2; fi
case "$1" in help|-h|--help) usage; exit 0 ;; esac

root="$(git rev-parse --show-toplevel 2>/dev/null)" || fail 'run inside an X3 git worktree'
[[ -f "$root/AGENTS.md" && -f "$root/Cargo.toml" && -d "$root/node" ]] || fail 'not an X3 source checkout'

check_prereqs() {
  for tool in git node npm python3 uv; do
    command -v "$tool" >/dev/null || fail "missing prerequisite: $tool"
  done
  node -e 'const [major,minor]=process.versions.node.split(".").map(Number); if (major<20 || (major===20 && minor<12)) process.exit(1)' || fail 'Node.js >=20.12 required'
  python3 -c 'import sys; assert sys.version_info >= (3, 10)' || fail 'Python >=3.10 required'
  printf 'PASS: prerequisites (Node %s, %s, %s)\n' "$(node -v)" "$(python3 --version)" "$(uv --version)"
}

case "$1" in
  check)
    check_prereqs
    printf 'Branch: %s\n' "$(git -C "$root" branch --show-current)"
    ;;
  status)
    if [[ -f "$root/_bmad/_config/manifest.yaml" ]]; then
      printf 'Manifest exists: %s\n' "$root/_bmad/_config/manifest.yaml"
      sed -n '1,50p' "$root/_bmad/_config/manifest.yaml"
    else
      printf 'NOT INSTALLED / NOT VERIFIED: no BMad manifest\n'
    fi
    ;;
  install)
    check_prereqs
    branch="$(git -C "$root" branch --show-current)"
    [[ -n "$branch" && "$branch" != master && "$branch" != main ]] || fail 'install only on a dedicated feature branch'
    [[ -z "$(git -C "$root" status --porcelain --untracked-files=normal)" ]] || fail 'dirty worktree: refuse to overwrite ongoing changes'
    [[ ! -e "$root/_bmad" ]] || fail '_bmad already exists: use the official update path'
    printf '%s\n' 'Installing pinned BMad v6.12.4; no deployment, release, or merge is performed.'
    (cd "$root" && npx --yes bmad-method@6.12.4 install --directory . --modules bmm --tools codex --yes)
    [[ -f "$root/_bmad/_config/manifest.yaml" ]] || fail 'installation returned without expected manifest'
    printf '%s\n' 'Manifest detected. Review generated files and git diff; adopt existing AGENTS.md before committing.'
    ;;
  *) usage; exit 2 ;;
esac
