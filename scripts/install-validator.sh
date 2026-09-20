#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# install-validator.sh — install the X3 validator binary and systemd service
#
# Intended for bare-metal or VPS validator hosts. Does NOT use Docker.
# Produces a systemd-managed validator.
#
# The binary comes from exactly one of two sources, and the script says which:
#
#   --binary <path>       a binary you built (the route
#                         launch-gates/VALIDATOR_ONBOARDING_RUNBOOK.md documents:
#                         `cargo build --release -p x3-chain-node`). Pass
#                         --sha256 <hex> to pin the artifact you intend to run.
#   --from-release [tag]  a **published** GitHub release. The matching
#                         `.sha256` asset is required and is verified; a missing
#                         one is an error, not a warning.
#
# With no source flag the script resolves the latest published release, and if
# the repository has none it stops and says so. (It previously downloaded from
# `releases/download/latest/...` unconditionally, which cannot work, and then
# printed "WARNING: No checksum file found. Skipping verification." while
# installing whatever it had.)
#
# A chain spec is always required, and it has to be a Live genesis with
# bootnodes: pointing a mainnet validator at a development genesis is refused
# unless --allow-non-live-chain is passed explicitly.
#
# Usage:
#   sudo bash scripts/install-validator.sh --binary ./target/release/x3-chain-node \
#        --sha256 <hex> --chain ./chain-specs/x3-mainnet-plain.json
#   sudo bash scripts/install-validator.sh --from-release v1.0.0 --chain ./x3-mainnet-plain.json
#
#   bash scripts/install-validator.sh --check --binary ... --chain ...   # validate only
#
# --check needs no root, changes nothing, and exits non-zero if anything the
# install would need is missing. It is what scripts/mainnet/validator_install_gate.sh runs.
#
# Path overrides (used by the gate, and by anyone installing to a staging
# prefix): X3_INSTALL_DIR, X3_DATA_DIR, X3_CONFIG_DIR, X3_LOG_DIR.
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

REPO="${REPO:-Cyptopimpinainteazy/xxxstar}"
BINARY="x3-chain-node"
INSTALL_DIR="${X3_INSTALL_DIR:-/usr/local/bin}"
DATA_DIR="${X3_DATA_DIR:-/var/lib/x3}"
CONFIG_DIR="${X3_CONFIG_DIR:-/etc/x3}"
LOG_DIR="${X3_LOG_DIR:-/var/log/x3}"
USER="${X3_VALIDATOR_USER:-x3}"

VERSION=""
FROM_RELEASE=0
LOCAL_BINARY=""
EXPECTED_SHA256=""
CHAIN_SPEC=""
ALLOW_NON_LIVE=0
CHECK_ONLY=0

usage() {
  sed -n '2,45p' "$0" | sed 's/^# \{0,1\}//'
}

die() {
  printf 'ERROR: %s\n' "$*" >&2
  exit 1
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version) VERSION="${2:?--version needs a tag}"; FROM_RELEASE=1; shift 2 ;;
    --from-release)
      FROM_RELEASE=1
      if [[ "${2:-}" != "" && "${2:0:1}" != "-" ]]; then VERSION="$2"; shift; fi
      shift ;;
    --binary) LOCAL_BINARY="${2:?--binary needs a path}"; shift 2 ;;
    --sha256) EXPECTED_SHA256="${2:?--sha256 needs a hex digest}"; shift 2 ;;
    --chain) CHAIN_SPEC="${2:?--chain needs a path}"; shift 2 ;;
    --allow-non-live-chain) ALLOW_NON_LIVE=1; shift ;;
    --check) CHECK_ONLY=1; shift ;;
    --help|-h) usage; exit 0 ;;
    *) die "unknown option: $1 (try --help)" ;;
  esac
done

[[ -n "$LOCAL_BINARY" && "$FROM_RELEASE" = 1 ]] \
  && die "--binary and --from-release are mutually exclusive"
[[ -z "$LOCAL_BINARY" && "$FROM_RELEASE" = 0 ]] \
  && die "no binary source: pass --binary <path> to install a binary you built, or --from-release [tag] for a published release"
[[ -n "$CHAIN_SPEC" ]] \
  || die "--chain <file> is required: a validator needs the genesis it is validating (scripts/mainnet/generate_mainnet_chain_spec.sh builds one)"

if [[ "$CHECK_ONLY" != 1 && $EUID -ne 0 ]]; then
  die "this script must be run as root (or pass --check to validate without installing)"
fi

# ── Validate the chain spec ─────────────────────────────────────────────────
validate_chain_spec() {
  local spec="$1"
  [[ -f "$spec" ]] || die "chain spec not found: $spec"
  python3 - "$spec" "$ALLOW_NON_LIVE" <<'PY' || exit 1
import json
import sys

path, allow_non_live = sys.argv[1], sys.argv[2] == "1"
try:
    with open(path, encoding="utf-8") as handle:
        spec = json.load(handle)
except ValueError as error:
    print(f"ERROR: {path} is not valid JSON ({error})", file=sys.stderr)
    if "Expecting value" in str(error):
        print(
            "       a file that starts with a banner or blank line usually means "
            "stdout was not a clean spec: regenerate it with "
            "scripts/mainnet/generate_mainnet_chain_spec.sh",
            file=sys.stderr,
        )
    sys.exit(1)

for field in ("name", "id", "chainType"):
    if field not in spec:
        print(f"ERROR: {path} has no '{field}' field — not a chain spec", file=sys.stderr)
        sys.exit(1)

bootnodes = spec.get("bootNodes") or []
if not bootnodes:
    print(f"ERROR: {path} has no bootNodes: a validator would start with nobody to sync from", file=sys.stderr)
    sys.exit(1)

chain_type = spec["chainType"]
if chain_type != "Live" and not allow_non_live:
    print(
        f"ERROR: {path} is a '{chain_type}' chain, not 'Live'.\n"
        "       A mainnet validator must not be pointed at a development or local "
        "genesis by accident;\n       pass --allow-non-live-chain if this is a "
        "testnet host and that is intended.",
        file=sys.stderr,
    )
    sys.exit(1)

print(
    f"chain spec: {spec['name']} (id {spec['id']}, {chain_type}, "
    f"{len(bootnodes)} bootnode(s))"
)
PY
}

# ── Validate the binary ─────────────────────────────────────────────────────
binary_sha256() { sha256sum "$1" | awk '{print $1}'; }

validate_binary() {
  local bin="$1" expected="${2:-}"
  [[ -f "$bin" ]] || die "binary not found: $bin"
  [[ -x "$bin" ]] || die "binary is not executable: $bin"
  local reported
  reported="$("$bin" --version 2>/dev/null | head -1 || true)"
  # The node reports its human name ("X3 Chain Node 0.1.0"), not the file name,
  # so match on the project rather than on `x3-chain-node`.
  [[ -n "$reported" ]] \
    || die "$bin did not answer --version; is this an X3 node binary?"
  printf '%s' "$reported" | grep -qi 'x3' \
    || die "$bin does not look like an X3 node binary (--version said '$reported')"
  local digest
  digest="$(binary_sha256 "$bin")"
  if [[ -n "$expected" ]]; then
    [[ "$digest" == "$expected" ]] \
      || die "sha256 mismatch for $bin
    expected: $expected
    actual:   $digest"
    echo "binary: $bin ($reported) sha256 verified"
  else
    echo "binary: $bin ($reported) sha256 $digest (not pinned — pass --sha256 to pin it)"
  fi
}

# ── Resolve the source ──────────────────────────────────────────────────────
resolve_release_asset() {
  local asset="$1"
  # Drafts are invisible to the public download path, so `latest` has to be
  # resolved from the API and the resolved tag has to be a published release.
  if [[ -z "$VERSION" || "$VERSION" == "latest" ]]; then
    VERSION="$(curl -sSfL "https://api.github.com/repos/${REPO}/releases/latest" \
      | python3 -c 'import json,sys; print(json.load(sys.stdin)["tag_name"])' 2>/dev/null || true)"
    [[ -n "$VERSION" ]] || die "no published release to install from.
    The repository's only release is a draft, and drafts have no public assets.
    Build one instead:  cargo build --release -p x3-chain-node
    then:               $0 --binary target/release/$BINARY --chain <spec>"
  fi
  echo "https://github.com/${REPO}/releases/download/${VERSION}/${asset}"
}

# ── The plan ────────────────────────────────────────────────────────────────
echo "==> X3 validator install plan"
echo "    repository: ${REPO}"
echo "    binary:     ${INSTALL_DIR}/${BINARY}"
echo "    data:       ${DATA_DIR}"
echo "    config:     ${CONFIG_DIR}/chain-spec.json"
echo "    logs:       ${LOG_DIR}"
echo "    service:    packaging/systemd/x3-validator.service"
echo

validate_chain_spec "$CHAIN_SPEC"

SOURCE_BINARY=""
DOWNLOAD_DIR=""
if [[ -n "$LOCAL_BINARY" ]]; then
  validate_binary "$LOCAL_BINARY" "$EXPECTED_SHA256"
  SOURCE_BINARY="$LOCAL_BINARY"
else
  BINARY_URL="$(resolve_release_asset "$BINARY")"
  CHECKSUM_URL="${BINARY_URL}.sha256"
  echo "release: $VERSION"
  echo "asset:   $BINARY_URL"
  if [[ "$CHECK_ONLY" = 1 ]]; then
    curl -sSfIL "$BINARY_URL" >/dev/null \
      || die "release asset is not downloadable: $BINARY_URL"
    curl -sSfIL "$CHECKSUM_URL" >/dev/null \
      || die "release has no ${BINARY}.sha256 — refusing to install an unverifiable binary"
    echo "release asset and checksum are reachable"
  else
    DOWNLOAD_DIR="$(mktemp -d)"
    trap 'rm -rf "${DOWNLOAD_DIR:-}"' EXIT
    echo "==> downloading..."
    curl -sSfL -o "$DOWNLOAD_DIR/$BINARY" "$BINARY_URL" \
      || die "download failed: $BINARY_URL"
    curl -sSfL -o "$DOWNLOAD_DIR/$BINARY.sha256" "$CHECKSUM_URL" \
      || die "release has no ${BINARY}.sha256 — refusing to install an unverifiable binary"
    EXPECTED_SHA256="$(awk '{print $1}' "$DOWNLOAD_DIR/$BINARY.sha256")"
    [[ -n "$EXPECTED_SHA256" ]] || die "empty checksum file from $CHECKSUM_URL"
    validate_binary "$DOWNLOAD_DIR/$BINARY" "$EXPECTED_SHA256"
    SOURCE_BINARY="$DOWNLOAD_DIR/$BINARY"
  fi
fi

SERVICE_FILE="$(cd "$(dirname "$0")/.." && pwd)/packaging/systemd/x3-validator.service"
[[ -f "$SERVICE_FILE" ]] || die "systemd unit not found: $SERVICE_FILE"
echo "systemd: $SERVICE_FILE"

if [[ "$CHECK_ONLY" = 1 ]]; then
  echo
  echo "==> check only: everything the install needs is present and valid."
  echo "    Nothing was written. Re-run without --check as root to install."
  exit 0
fi

# ── Install ─────────────────────────────────────────────────────────────────
echo
echo "==> Installing X3 validator${VERSION:+ ${VERSION}}..."

if ! id -u "${USER}" &>/dev/null; then
  echo "==> Creating system user '${USER}'..."
  useradd --system --no-create-home --shell /sbin/nologin "${USER}"
fi

mkdir -p "${INSTALL_DIR}" "${DATA_DIR}" "${CONFIG_DIR}" "${LOG_DIR}"

install -m 0755 "$SOURCE_BINARY" "${INSTALL_DIR}/${BINARY}"
echo "    Binary installed: ${INSTALL_DIR}/${BINARY}"

install -m 0644 "$CHAIN_SPEC" "${CONFIG_DIR}/chain-spec.json"
echo "    Chain spec installed: ${CONFIG_DIR}/chain-spec.json"

cp "${SERVICE_FILE}" /etc/systemd/system/x3-validator.service
systemctl daemon-reload
echo "    Service installed: x3-validator.service"

chown -R "${USER}:${USER}" "${DATA_DIR}" "${CONFIG_DIR}" "${LOG_DIR}"

echo ""
echo "╔══════════════════════════════════════════════════╗"
echo "║  X3 Validator Installation Complete              ║"
echo "╠══════════════════════════════════════════════════╣"
echo "║  Binary:     ${INSTALL_DIR}/${BINARY}"
echo "║  Data:       ${DATA_DIR}"
echo "║  Config:     ${CONFIG_DIR}/chain-spec.json"
echo "║  Service:    x3-validator.service"
echo "╚══════════════════════════════════════════════════╝"
echo ""
echo "Before starting:"
echo "  1. Insert an authority key (this is the Aura/GRANDPA pair named in the genesis):"
echo "       ${INSTALL_DIR}/${BINARY} keys insert --key-type aura    --seed <suri>"
echo "       ${INSTALL_DIR}/${BINARY} keys insert --key-type grandpa --seed <suri>"
echo "     and check it with:"
echo "       ${INSTALL_DIR}/${BINARY} keys list"
echo "  2. Configure firewall: ufw allow 30333/tcp"
echo "  3. Mount NVMe storage at ${DATA_DIR} if available"
