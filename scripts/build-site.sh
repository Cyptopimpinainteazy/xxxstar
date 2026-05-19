#!/usr/bin/env bash
# build-site.sh — Build all frontend apps and assemble the X3 static site.
# Usage: bash scripts/build-site.sh [--skip-install]
#
# Outputs to site/ at the repo root.
# After running, serve with: cd site && python3 -m http.server 8080

set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SITE="$REPO_ROOT/site"
SKIP_INSTALL="${1:-}"

log() { echo -e "\033[1;36m[build-site]\033[0m $*"; }
err() { echo -e "\033[1;31m[build-site] ERROR:\033[0m $*" >&2; }

npm_build() {
  local dir="$1" out="$2" dest="$3"
  log "Building $dir → site/$dest"
  pushd "$REPO_ROOT/$dir" > /dev/null
  [ "$SKIP_INSTALL" != "--skip-install" ] && npm ci --prefer-offline
  npm run build
  popd > /dev/null
  rm -rf "$SITE/$dest"
  cp -r "$REPO_ROOT/$dir/$out" "$SITE/$dest"
  log "  → site/$dest ✓"
}

static_copy() {
  local src="$1" dest="$2" file="${3:-}"
  log "Copying static: $src → site/$dest"
  mkdir -p "$SITE/$dest"
  if [ -n "$file" ]; then
    cp "$REPO_ROOT/$src/$file" "$SITE/$dest/index.html"
  else
    cp -r "$REPO_ROOT/$src/." "$SITE/$dest/"
  fi
  log "  → site/$dest ✓"
}

mkdir -p "$SITE"

# ── React/Vite apps ─────────────────────────────────────────────────────────
npm_build "apps/x3-intelligence"          "dist"  "intelligence"
npm_build "apps/inferstructor-dashboard"  "dist"  "inferstructor"
npm_build "infra-structure/dashboard"     "dist"  "infra-dashboard"
npm_build "apps/dashboard"                "dist"  "dashboard"
npm_build "apps/validators"               "dist"  "validators"

# ── Next.js apps (static export) ────────────────────────────────────────────
npm_build "apps/wallet"  "out"  "wallet"
npm_build "apps/dex"     "out"  "dex"

# ── x3fronend (root of site — copies dist/* into site/) ─────────────────────
log "Building x3fronend → site/ (root)"
pushd "$REPO_ROOT/x3fronend" > /dev/null
[ "$SKIP_INSTALL" != "--skip-install" ] && npm ci --prefer-offline
npm run build
popd > /dev/null
# Copy dist but don't overwrite our generated index.html
rsync -av --exclude="index.html" "$REPO_ROOT/x3fronend/dist/" "$SITE/"
log "  → site/ (root, excl index.html) ✓"

# ── Static HTML pages ───────────────────────────────────────────────────────
static_copy "infra-structure/services/blockchain-tps/public"  "tps"
static_copy "web/mainnet-progress"                             "mainnet-progress"
static_copy "apps/x3-extension"                                "extension"
static_copy "swarm_infrastructure/autonomic"                   "swarm-autonomic"  "dashboard.html"

# ── Jury Anchoring UI (TSX embed — copy build output if present) ────────────
if [ -d "$REPO_ROOT/packages/blockchain-adapter/dist" ]; then
  static_copy "packages/blockchain-adapter/dist" "jury"
else
  log "Skipping jury (no dist — run build in packages/blockchain-adapter first)"
fi

# ── Regenerate site/index.html ──────────────────────────────────────────────
log "Regenerating site/index.html from manifest…"
node "$REPO_ROOT/scripts/generate-site-index.js"

log ""
log "Build complete! Serve with:"
log "  cd site && python3 -m http.server 8080"
log "  open http://localhost:8080"
