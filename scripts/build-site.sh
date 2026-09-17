#!/usr/bin/env bash
# build-site.sh — Build all frontend apps and assemble the X3 static site.
# Usage: bash scripts/build-site.sh [--skip-install]
#
# Outputs to site/ at the repo root.
# After running, serve with: cd site && python3 -m http.server 8080

set -uo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SITE="$REPO_ROOT/site"
SKIP_INSTALL="${1:-}"
FAILED=()

log() { echo -e "\033[1;36m[build-site]\033[0m $*"; }
err() { echo -e "\033[1;31m[build-site] ERROR:\033[0m $*" >&2; }

# One app's build failing (missing out/dist dir, broken deps, etc.) no longer
# kills the whole pipeline — it's logged and skipped so the rest still builds.
npm_build() {
  local dir="$1" out="$2" dest="$3"
  log "Building $dir → site/$dest"
  ( set -e
    cd "$REPO_ROOT/$dir"
    [ "$SKIP_INSTALL" != "--skip-install" ] && npm ci --prefer-offline
    npm run build
  )
  if [ $? -ne 0 ]; then
    err "$dir build failed — skipping site/$dest"
    FAILED+=("$dir (build failed)")
    return
  fi
  if [ ! -d "$REPO_ROOT/$dir/$out" ]; then
    err "$dir built but $out/ was never produced (check next.config.js output mode) — skipping site/$dest"
    FAILED+=("$dir (missing $out/ after build)")
    return
  fi
  rm -rf "$SITE/$dest"
  cp -r "$REPO_ROOT/$dir/$out" "$SITE/$dest"
  log "  → site/$dest ✓"
}

static_copy() {
  local src="$1" dest="$2" file="${3:-}"
  if [ ! -e "$REPO_ROOT/$src" ]; then
    err "$src does not exist — skipping site/$dest"
    FAILED+=("$src (source missing)")
    return
  fi
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

# ── x3fronend (investor/grant homepage — becomes site/ root, incl. index.html) ─
log "Building x3fronend → site/ (root)"
( set -e
  cd "$REPO_ROOT/x3fronend"
  [ "$SKIP_INSTALL" != "--skip-install" ] && npm install
  npm run build
)
if [ $? -eq 0 ] && [ -d "$REPO_ROOT/x3fronend/out" ]; then
  rsync -av "$REPO_ROOT/x3fronend/out/" "$SITE/"
  log "  → site/ (root) ✓"
else
  err "x3fronend build failed — site root will be missing the homepage"
  FAILED+=("x3fronend (build failed)")
fi

# ── React/Vite apps ─────────────────────────────────────────────────────────
npm_build "apps/x3-intelligence"          "dist"  "intelligence"
npm_build "apps/inferstructor-dashboard"  "dist"  "inferstructor"
npm_build "infra-structure/dashboard"     "dist"  "infra-dashboard"
npm_build "apps/dashboard"                "dist"  "dashboard"
npm_build "apps/validators"               "dist"  "validators"
npm_build "apps/x3-funding"               "dist"  "funding"
npm_build "apps/x3-transparency"          "dist"  "transparency"

# ── Next.js apps (static export) ────────────────────────────────────────────
npm_build "apps/wallet"  "out"  "wallet"
npm_build "apps/dex"     "out"  "dex"

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

# ── Regenerate site/apps/index.html (internal app directory) ───────────────
log "Regenerating site/apps/index.html from manifest…"
node "$REPO_ROOT/scripts/generate-site-index.js" || { err "generate-site-index.js failed"; FAILED+=("generate-site-index.js"); }

log ""
if [ "${#FAILED[@]}" -eq 0 ]; then
  log "Build complete, all apps succeeded! Serve with:"
else
  err "Build finished with ${#FAILED[@]} failure(s):"
  for f in "${FAILED[@]}"; do err "  - $f"; done
  log ""
  log "Everything else built. Serve with:"
fi
log "  cd site && python3 -m http.server 8080"
log "  open http://localhost:8080"

[ "${#FAILED[@]}" -eq 0 ]
