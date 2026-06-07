#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TAURI_DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/com.atlassphere.atlasdesktop/app-store"

printf "Verifying App Store packages (app-store -> Tauri app-data)\n"
printf "Repo: %s\nTauri app-store: %s\n\n" "$ROOT_DIR" "$TAURI_DATA_DIR"

printf "% -30s | % -8s | % -40s\n" "APP" "STATUS" "NOTES"
printf "%0.s-" {1..90}
printf "\n"

for d in "$ROOT_DIR"/*/; do
  app_name="$(basename "$d")"
  # skip non-app files
  case "$app_name" in
    COMPLETION_REPORT_*|INTEGRATION_COMPLETE.md|docs/root/README.md) continue ;;
  esac

  notes=()
  status="OK"

  repo_path="$ROOT_DIR/$app_name"
  data_path="$TAURI_DATA_DIR/$app_name"

  if [ -d "$data_path" ]; then
    notes+=("installed in appDataDir")
  else
    status="MISSING"
    notes+=("not copied to Tauri app-data (run setup-treasury-integration.sh)")
  fi

  # python app checks
  if [ -f "$repo_path/requirements.txt" ] || [ -f "$data_path/requirements.txt" ]; then
    if [ -d "$data_path/.venv" ]; then
      notes+=(".venv present")
      if "$data_path/.venv/bin/python" --version >/dev/null 2>&1; then
        :
      else
        status="WARN"
        notes+=(".venv python not runnable")
      fi
    else
      status="WARN"
      notes+=(".venv missing; dependencies may be global")
    fi
  fi

  # node app checks
  if [ -f "$repo_path/package.json" ] || [ -f "$data_path/package.json" ]; then
    pkgjson="$data_path/package.json"
    if [ -f "$pkgjson" ]; then
      if grep -q '"start"' "$pkgjson"; then
        notes+=("npm start script found")
      else
        notes+=("no npm start script")
      fi
    else
      notes+=("package.json not copied to appDataDir")
    fi
  fi

  # executable check (simple)
  if [ -f "$data_path/main.py" ]; then
    notes+=("entry: main.py")
  fi

  printf "% -30s | % -8s | % -40s\n" "$app_name" "$status" "$(IFS='; '; echo "${notes[*]}")"

done

printf "\nSummary: re-run 'apps/x3-desktop/app-store/setup-treasury-integration.sh' to install/copy apps into Tauri app-data.\n"
exit 0
