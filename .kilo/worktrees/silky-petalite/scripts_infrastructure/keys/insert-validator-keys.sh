#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  insert-validator-keys.sh --base-path <PATH> --chain-spec <PATH> [--suri <SURI>] [--password-file <PATH>]
  insert-validator-keys.sh --base-path <PATH> --chain-id <ID> [--suri <SURI>] [--password-file <PATH>]

Notes:
  - If --suri is not provided, you will be prompted (hidden input).
  - --password-file is optional. If provided, start the node with:
      --password-filename <PATH>
EOF
}

BASE_PATH=""
CHAIN_SPEC=""
CHAIN_ID=""
SURI=""
PASSWORD_FILE=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --base-path)
      BASE_PATH="$2"
      shift 2
      ;;
    --chain-spec)
      CHAIN_SPEC="$2"
      shift 2
      ;;
    --chain-id)
      CHAIN_ID="$2"
      shift 2
      ;;
    --suri)
      SURI="$2"
      shift 2
      ;;
    --password-file)
      PASSWORD_FILE="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1"
      usage
      exit 1
      ;;
  esac
done

if [[ -z "$BASE_PATH" ]]; then
  echo "Missing --base-path"
  usage
  exit 1
fi

if [[ -z "$CHAIN_ID" ]]; then
  if [[ -z "$CHAIN_SPEC" ]]; then
    echo "Missing --chain-spec or --chain-id"
    usage
    exit 1
  fi
  CHAIN_ID="$(CHAIN_SPEC="$CHAIN_SPEC" python - <<'PY'
import json
import os
from pathlib import Path

spec = json.loads(Path(os.environ["CHAIN_SPEC"]).read_text())
print(spec.get("id", ""))
PY
)"
fi

if [[ -z "$CHAIN_ID" ]]; then
  echo "Failed to resolve chain id"
  exit 1
fi

if [[ -z "$SURI" ]]; then
  read -r -s -p "Enter SURI (hidden): " SURI
  echo
  read -r -s -p "Confirm SURI: " SURI_CONFIRM
  echo
  if [[ "$SURI" != "$SURI_CONFIRM" ]]; then
    echo "SURI mismatch"
    exit 1
  fi
fi

if [[ -n "$PASSWORD_FILE" && ! -f "$PASSWORD_FILE" ]]; then
  echo "Password file not found: $PASSWORD_FILE"
  exit 1
fi

if ! command -v subkey >/dev/null 2>&1; then
  echo "subkey not found in PATH"
  exit 1
fi

KEYSTORE_DIR="${BASE_PATH}/chains/${CHAIN_ID}/keystore"
mkdir -p "$KEYSTORE_DIR"

aura_pub=$(subkey inspect --scheme sr25519 "$SURI" | awk '/Public key \(hex\):/ {print $4}')
gran_pub=$(subkey inspect --scheme ed25519 "$SURI" | awk '/Public key \(hex\):/ {print $4}')

if [[ -z "$aura_pub" || -z "$gran_pub" ]]; then
  echo "Failed to derive public keys"
  exit 1
fi

aura_file="${KEYSTORE_DIR}/61757261${aura_pub#0x}"
gran_file="${KEYSTORE_DIR}/6772616e${gran_pub#0x}"

SURI="$SURI" OUT="$aura_file" python - <<'PY'
import json
import os
from pathlib import Path

path = Path(os.environ["OUT"])
path.write_text(json.dumps(os.environ["SURI"]))
path.chmod(0o600)
PY

SURI="$SURI" OUT="$gran_file" python - <<'PY'
import json
import os
from pathlib import Path

path = Path(os.environ["OUT"])
path.write_text(json.dumps(os.environ["SURI"]))
path.chmod(0o600)
PY

unset SURI

echo "Inserted Aura + Grandpa keys into: ${KEYSTORE_DIR}"
if [[ -n "$PASSWORD_FILE" ]]; then
  echo "Start the node with: --password-filename ${PASSWORD_FILE}"
fi
