#!/usr/bin/env bash
# Register this host as a GitHub Actions GPU runner for the X3 swarm soak.
#
# Required:
#   GITHUB_RUNNER_TOKEN=<repo runner registration token>
#   or a gh token with repo runner registration-token permission.
#
# Optional:
#   GITHUB_REPO=Cyptopimpinainteazy/xxxstar
#   RUNNER_DIR=$HOME/actions-runner-x3-gpu
#   RUNNER_NAME=x3-gpu-$(hostname)
#   RUNNER_LABELS=self-hosted,linux,gpu
#   RUNNER_VERSION=2.329.0
#   RUNNER_MODE=run        # run | service
#
# Token source:
#   GitHub repo -> Settings -> Actions -> Runners -> New self-hosted runner

set -euo pipefail

REPO="${GITHUB_REPO:-Cyptopimpinainteazy/xxxstar}"
RUNNER_DIR="${RUNNER_DIR:-$HOME/actions-runner-x3-gpu}"
RUNNER_NAME="${RUNNER_NAME:-x3-gpu-$(hostname)}"
RUNNER_LABELS="${RUNNER_LABELS:-self-hosted,linux,gpu}"
RUNNER_VERSION="${RUNNER_VERSION:-2.329.0}"
RUNNER_TOKEN="${GITHUB_RUNNER_TOKEN:-}"
RUNNER_MODE="${RUNNER_MODE:-run}"
RUNNER_URL="https://github.com/${REPO}"

if [[ -z "$RUNNER_TOKEN" ]]; then
  if command -v gh >/dev/null 2>&1; then
    echo "GITHUB_RUNNER_TOKEN not set; trying gh api registration-token for ${REPO}..."
    set +e
    RUNNER_TOKEN="$(gh api -X POST "repos/${REPO}/actions/runners/registration-token" --jq .token 2>/dev/null)"
    status=$?
    set -e
    if [[ $status -ne 0 || -z "$RUNNER_TOKEN" ]]; then
      RUNNER_TOKEN=""
    fi
  fi
fi

if [[ -z "$RUNNER_TOKEN" ]]; then
  cat >&2 <<EOF
GITHUB_RUNNER_TOKEN is required.

Create one at:
  https://github.com/${REPO}/settings/actions/runners/new

Then run:
  GITHUB_RUNNER_TOKEN=<token> $0
EOF
  exit 2
fi

case "$RUNNER_MODE" in
  run|service) ;;
  *)
    echo "RUNNER_MODE must be 'run' or 'service'" >&2
    exit 2
    ;;
esac

if ! command -v curl >/dev/null 2>&1; then
  echo "curl is required" >&2
  exit 2
fi

if ! command -v tar >/dev/null 2>&1; then
  echo "tar is required" >&2
  exit 2
fi

if command -v nvidia-smi >/dev/null 2>&1; then
  echo "Detected GPU:"
  nvidia-smi --query-gpu=name,driver_version,memory.total --format=csv,noheader || true
else
  echo "nvidia-smi not found; continuing, but the gpu-labelled workflow may fail." >&2
fi

mkdir -p "$RUNNER_DIR"
cd "$RUNNER_DIR"

if [[ ! -x ./config.sh || ! -x ./run.sh ]]; then
  archive="actions-runner-linux-x64-${RUNNER_VERSION}.tar.gz"
  url="https://github.com/actions/runner/releases/download/v${RUNNER_VERSION}/${archive}"
  echo "Downloading $url"
  curl -fsSL "$url" -o "$archive"
  tar xzf "$archive"
fi

if [[ -f .runner ]]; then
  echo "Runner already configured in $RUNNER_DIR"
else
  ./config.sh \
    --unattended \
    --url "$RUNNER_URL" \
    --token "$RUNNER_TOKEN" \
    --name "$RUNNER_NAME" \
    --labels "$RUNNER_LABELS" \
    --work _work \
    --replace
fi

if [[ "$RUNNER_MODE" == "service" ]]; then
  if [[ -x ./svc.sh ]] && command -v sudo >/dev/null 2>&1; then
    echo "Installing runner service via svc.sh"
    sudo ./svc.sh install
    sudo ./svc.sh start
    echo "Runner service started."
    echo "Repo:   $RUNNER_URL"
    echo "Name:   $RUNNER_NAME"
    echo "Labels: $RUNNER_LABELS"
    exit 0
  fi

  if command -v systemctl >/dev/null 2>&1; then
    service_dir="${HOME}/.config/systemd/user"
    service_name="github-actions-${RUNNER_NAME}.service"
    mkdir -p "$service_dir"
    cat >"${service_dir}/${service_name}" <<EOF
[Unit]
Description=GitHub Actions runner ${RUNNER_NAME}
After=network-online.target
Wants=network-online.target

[Service]
WorkingDirectory=${RUNNER_DIR}
ExecStart=${RUNNER_DIR}/run.sh
Restart=always
RestartSec=5

[Install]
WantedBy=default.target
EOF
    systemctl --user daemon-reload
    systemctl --user enable --now "$service_name"
    echo "User service started: ${service_name}"
    echo "Repo:   $RUNNER_URL"
    echo "Name:   $RUNNER_NAME"
    echo "Labels: $RUNNER_LABELS"
    exit 0
  fi

  echo "RUNNER_MODE=service requested, but neither sudo svc.sh nor systemctl --user is available." >&2
  exit 2
fi

echo
echo "Starting runner. Leave this process running until GitHub picks up queued jobs."
echo "Repo:   $RUNNER_URL"
echo "Name:   $RUNNER_NAME"
echo "Labels: $RUNNER_LABELS"
echo
exec ./run.sh
