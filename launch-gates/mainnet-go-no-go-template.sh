#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════════════════════
# X3 MAINNET GO/NO-GO DECISION ENGINE
# ═══════════════════════════════════════════════════════════════════════════════
#
# Computes mainnet readiness from real ProofForge evidence:
# - claim statuses in proof/claims/registry.yml
# - structured claim receipts in proof/receipts/claims/*.json
#
# Output: launch-gates/reports/X3-MAINNET-GO-NO-GO-<timestamp>.md
#
# Usage: ./mainnet-go-no-go-template.sh
# ═══════════════════════════════════════════════════════════════════════════════

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
REPORTS_DIR="${REPO_ROOT}/launch-gates/reports"
REGISTRY_FILE="${REPO_ROOT}/proof/claims/registry.yml"
RECEIPTS_DIR="${REPO_ROOT}/proof/receipts/claims"
TIMESTAMP="$(date +%Y%m%d-%H%M%S)"
FRESH_HOURS="${FRESH_HOURS:-24}"

mkdir -p "${REPORTS_DIR}"
cd "${REPO_ROOT}"

if ! command -v jq >/dev/null 2>&1; then
  echo "ERROR: jq is required for go/no-go scoring." >&2
  exit 2
fi

if [[ ! -f "${REGISTRY_FILE}" ]]; then
  echo "ERROR: missing claims registry: ${REGISTRY_FILE}" >&2
  exit 2
fi

if [[ ! -d "${RECEIPTS_DIR}" ]]; then
  echo "ERROR: missing receipts directory: ${RECEIPTS_DIR}" >&2
  exit 2
fi

declare -A CATEGORY_WEIGHT=(
  ["Runtime / Pallets"]=12
  ["Consensus & Finality"]=12
  ["Universal Asset Kernel"]=15
  ["Atomic Cross-VM Execution"]=18
  ["Bridge Security"]=15
  ["DEX / Liquidity"]=8
  ["Governance / Launch Gates"]=6
  ["Validator Operations"]=6
  ["Observability"]=4
  ["Documentation / Code Drift"]=4
)

CATEGORY_ORDER=(
  "Runtime / Pallets"
  "Consensus & Finality"
  "Universal Asset Kernel"
  "Atomic Cross-VM Execution"
  "Bridge Security"
  "DEX / Liquidity"
  "Governance / Launch Gates"
  "Validator Operations"
  "Observability"
  "Documentation / Code Drift"
)

declare -A CATEGORY_SCORE_SUM
declare -A CATEGORY_CLAIM_COUNT
declare -A CATEGORY_AVG

status_to_score() {
  case "$1" in
    VERIFIED|verified|pass|passed) echo 100 ;;
    PARTIAL|partial) echo 60 ;;
    STALE|stale) echo 30 ;;
    UNVERIFIED|unverified) echo 20 ;;
    BLOCKED|blocked|FAILED|failed|REVOKED|revoked) echo 0 ;;
    *) echo 0 ;;
  esac
}

claim_to_category() {
  local claim="$1"
  case "${claim}" in
    x3.asset_kernel.*) echo "Universal Asset Kernel" ;;
    x3.bridge.*) echo "Bridge Security" ;;
    x3.atomic.*) echo "Atomic Cross-VM Execution" ;;
    x3.consensus.*) echo "Consensus & Finality" ;;
    x3.dex.*) echo "DEX / Liquidity" ;;
    x3.governance.*) echo "Governance / Launch Gates" ;;
    x3.gpu.*) echo "Validator Operations" ;;
    x3.observability.*) echo "Observability" ;;
    x3.flashloan.*|x3.x3vm.*|x3.x3lang.*|x3.contracts.*) echo "Runtime / Pallets" ;;
    x3.onboarding.*|x3.funding.*|x3.evolution.*) echo "Documentation / Code Drift" ;;
    *) echo "Runtime / Pallets" ;;
  esac
}

receipt_shape_valid() {
  local file="$1"
  jq -e '
    has("repo_commit_hash") and
    has("command_run") and
    has("artifact_hash") and
    has("policy_hash") and
    has("relevant_files") and
    has("timestamp") and
    has("result") and
    has("limitations") and
    has("binding_hash")
  ' "$file" >/dev/null 2>&1
}

# Extract binding_hash from receipt
receipt_binding_hash() {
  local file="$1"
  jq -r '.binding_hash // empty' "$file"
}

# Extract receipt timestamp
receipt_timestamp() {
  local file="$1"
  jq -r '.timestamp // empty' "$file"
}

# Verify receipt integrity by recomputing binding hash and comparing
# This replicates the Rust Receipt::compute_binding_hash() logic from proof-forge/src/receipt.rs
receipt_integrity_valid() {
  local file="$1"
  local stored_hash
  stored_hash="$(receipt_binding_hash "$file")"

  # Check binding_hash exists and is non-empty
  if [[ -z "${stored_hash}" || "${stored_hash}" == "null" ]]; then
    return 1
  fi

  # Recompute binding hash from receipt fields
  local repo_commit command_run artifact_hash policy_hash timestamp
  local relevant_files result_json limitations

  repo_commit="$(jq -r '.repo_commit_hash // empty' "$file")"
  command_run="$(jq -r '.command_run // empty' "$file")"
  artifact_hash="$(jq -r '.artifact_hash // empty' "$file")"
  policy_hash="$(jq -r '.policy_hash // empty' "$file")"
  timestamp="$(jq -r '.timestamp // empty' "$file")"

  # Compute SHA256 of relevant_files list (in order)
  local files_hash=""
  files_hash="$(jq -r '.relevant_files[]? // empty' "$file" | while IFS= read -r f; do
    echo -n "$f"
  done | sha256sum | cut -d' ' -f1)"

  # Compute canonical result JSON (sorted keys, matching Rust canonicalize_json_value)
  result_json="$(jq -S '.result' "$file" | sha256sum | cut -d' ' -f1)"

  # Compute limitations hash
  local limit_hash=""
  limit_hash="$(jq -r '.limitations[]? // empty' "$file" | while IFS= read -r l; do
    echo -n "$l"
  done | sha256sum | cut -d' ' -f1)"

  # Recompute full binding hash
  local computed_hash
  computed_hash="$(echo -n "${repo_commit}${command_run}${artifact_hash}${policy_hash}${files_hash}$(echo -n "$timestamp" | sha256sum | cut -d' ' -f1)${result_json}${limit_hash}" | sha256sum | cut -d' ' -f1)"

  # Compare
  [[ "${computed_hash}" == "${stored_hash}" ]]
}

min_score() {
  local a="$1"
  local b="$2"
  if (( a < b )); then
    echo "$a"
  else
    echo "$b"
  fi
}

BLOCKERS_P0=()
BLOCKERS_P1=()
BLOCKERS_P2=()
CLAIM_ROWS=""
RECEIPT_BINDINGS=""

TOTAL_CLAIMS=0
S0_CLAIMS=0
S0_VERIFIED=0
RECEIPT_OK=0
RECEIPT_MISSING=0
RECEIPT_INVALID=0
RECEIPT_STALE=0

while IFS='|' read -r claim criticality registry_status; do
  [[ -z "${claim}" ]] && continue

  TOTAL_CLAIMS=$((TOTAL_CLAIMS + 1))
  if [[ "${criticality}" == "S0" ]]; then
    S0_CLAIMS=$((S0_CLAIMS + 1))
  fi

  category="$(claim_to_category "${claim}")"

  registry_score="$(status_to_score "${registry_status}")"
  receipt_file="${RECEIPTS_DIR}/${claim}.receipt.json"

  receipt_state="missing"
  receipt_score=25
  receipt_status="MISSING"
  age_hours="n/a"

  if [[ -f "${receipt_file}" ]]; then
    if receipt_shape_valid "${receipt_file}"; then
      # Verify receipt integrity (binding_hash must exist and be non-empty)
      if receipt_integrity_valid "${receipt_file}"; then
        RECEIPT_OK=$((RECEIPT_OK + 1))
        raw_receipt_status="$(jq -r '.result.status // .status // "unknown"' "${receipt_file}")"
        receipt_status="${raw_receipt_status^^}"
        receipt_score="$(status_to_score "${raw_receipt_status}")"
        receipt_state="ok"

        # Collect receipt metadata for immutability
        receipt_binding="$(receipt_binding_hash "${receipt_file}")"
        receipt_ts="$(receipt_timestamp "${receipt_file}")"
        RECEIPT_BINDINGS+="${claim}|${receipt_binding}|${receipt_ts}"$'\n'

        if [[ -n "${receipt_ts}" ]]; then
          now_epoch="$(date +%s)"
          receipt_epoch="$(date -d "${receipt_ts}" +%s 2>/dev/null || echo "")"
          if [[ -n "${receipt_epoch}" ]]; then
            age_hours="$(( (now_epoch - receipt_epoch) / 3600 ))"
            if (( age_hours > FRESH_HOURS )); then
              receipt_state="stale"
              RECEIPT_STALE=$((RECEIPT_STALE + 1))
              if (( receipt_score > 70 )); then
                receipt_score=70
              fi
              receipt_status="${receipt_status} (STALE)"
            fi
          fi
        fi
      else
        RECEIPT_INVALID=$((RECEIPT_INVALID + 1))
        receipt_state="invalid"
        receipt_status="INTEGRITY_FAILED"
        receipt_score=0
      fi
    else
      RECEIPT_INVALID=$((RECEIPT_INVALID + 1))
      receipt_state="invalid"
      receipt_status="INVALID_SHAPE"
      receipt_score=0
    fi
  else
    RECEIPT_MISSING=$((RECEIPT_MISSING + 1))
  fi

  effective_score="$(min_score "${registry_score}" "${receipt_score}")"
  CATEGORY_SCORE_SUM["${category}"]=$(( ${CATEGORY_SCORE_SUM["${category}"]:-0} + effective_score ))
  CATEGORY_CLAIM_COUNT["${category}"]=$(( ${CATEGORY_CLAIM_COUNT["${category}"]:-0} + 1 ))

  if [[ "${registry_status}" == "VERIFIED" && "${receipt_state}" == "ok" ]]; then
    if [[ "${criticality}" == "S0" ]]; then
      S0_VERIFIED=$((S0_VERIFIED + 1))
    fi
  fi

  if [[ "${criticality}" == "S0" ]]; then
    if [[ "${registry_status}" != "VERIFIED" ]]; then
      BLOCKERS_P0+=("${claim}: S0 status is ${registry_status}")
    fi
    if [[ "${receipt_state}" == "missing" || "${receipt_state}" == "invalid" ]]; then
      BLOCKERS_P0+=("${claim}: S0 receipt ${receipt_state}")
    fi
    if [[ "${receipt_state}" == "stale" ]]; then
      BLOCKERS_P1+=("${claim}: S0 receipt stale (${age_hours}h)")
    fi
  else
    if [[ "${receipt_state}" == "missing" || "${receipt_state}" == "invalid" ]]; then
      BLOCKERS_P2+=("${claim}: receipt ${receipt_state}")
    fi
  fi

  CLAIM_ROWS+="| ${claim} | ${criticality} | ${category} | ${registry_status} | ${receipt_status} | ${effective_score}% |\n"
done < <(
  awk '
    /^claims:/ { in_claims=1; next }
    in_claims && /^[^[:space:]]/ { in_claims=0 }
    in_claims && /^  x3\.[A-Za-z0-9_.-]+:/ {
      claim=$1
      sub(/:$/, "", claim)
      criticality=""
      status=""
    }
    in_claims && /^    criticality:/ { criticality=$2 }
    in_claims && /^    status:/ {
      status=$2
      if (claim != "") print claim "|" criticality "|" status
    }
  ' "${REGISTRY_FILE}"
)

for category in "${CATEGORY_ORDER[@]}"; do
  count="${CATEGORY_CLAIM_COUNT["${category}"]:-0}"
  if (( count > 0 )); then
    CATEGORY_AVG["${category}"]=$(( ${CATEGORY_SCORE_SUM["${category}"]} / count ))
  else
    CATEGORY_AVG["${category}"]=0
  fi
done

weighted_sum=0
for category in "${CATEGORY_ORDER[@]}"; do
  avg="${CATEGORY_AVG["${category}"]}"
  weight="${CATEGORY_WEIGHT["${category}"]}"
  weighted_sum=$(( weighted_sum + (avg * weight) ))
done
OVERALL_SCORE=$(( weighted_sum / 100 ))

P0_COUNT=${#BLOCKERS_P0[@]}
P1_COUNT=${#BLOCKERS_P1[@]}
P2_COUNT=${#BLOCKERS_P2[@]}

DECISION="GO"
if (( P0_COUNT > 0 )); then
  DECISION="NO-GO"
fi
if (( OVERALL_SCORE < 90 )); then
  DECISION="NO-GO"
fi
if (( S0_CLAIMS > 0 )) && (( S0_VERIFIED < S0_CLAIMS )); then
  DECISION="NO-GO"
fi

COMMIT_HASH="$(git rev-parse HEAD)"
SHORT_COMMIT="$(git rev-parse --short=16 HEAD)"
REPORT_FILE="${REPORTS_DIR}/X3-MAINNET-GO-NO-GO-${TIMESTAMP}.md"

{
  echo "# X3 MAINNET GO/NO-GO DECISION REPORT"
  echo
  echo "Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "Repository: ${REPO_ROOT}"
  echo "Commit: ${SHORT_COMMIT}"
  echo "Receipt freshness threshold: ${FRESH_HOURS}h"
  echo
  echo "## Executive Summary"
  echo
  echo "Decision: **${DECISION}**"
  echo "Overall Score: **${OVERALL_SCORE}%**"
  echo "S0 Verified: **${S0_VERIFIED}/${S0_CLAIMS}**"
  echo
  echo "Gate rules:"
  echo "- Score must be >= 90%"
  echo "- Zero P0 blockers"
  echo "- All S0 claims must be VERIFIED with valid receipts"
  echo
  echo "## Evidence Health"
  echo
  echo "- Claims in registry: ${TOTAL_CLAIMS}"
  echo "- Receipts valid shape: ${RECEIPT_OK}"
  echo "- Receipts missing: ${RECEIPT_MISSING}"
  echo "- Receipts invalid: ${RECEIPT_INVALID}"
  echo "- Receipts stale: ${RECEIPT_STALE}"
  echo
  echo "## Category Scores"
  echo
  echo "| Category | Weight | Claims | Computed Score | Weighted Contribution |"
  echo "|---|---:|---:|---:|---:|"
  for category in "${CATEGORY_ORDER[@]}"; do
    weight="${CATEGORY_WEIGHT["${category}"]}"
    count="${CATEGORY_CLAIM_COUNT["${category}"]:-0}"
    avg="${CATEGORY_AVG["${category}"]}"
    contribution=$(( avg * weight / 100 ))
    echo "| ${category} | ${weight}% | ${count} | ${avg}% | ${contribution}% |"
  done
  echo
  echo "## Claim-Level Scoring"
  echo
  echo "| Claim | Criticality | Category | Registry Status | Receipt Status | Effective Score |"
  echo "|---|---|---|---|---|---:|"
  printf '%b' "${CLAIM_ROWS}"
  echo
  echo "## Blockers"
  echo
  echo "### P0 (Launch Blocking): ${P0_COUNT}"
  if (( P0_COUNT == 0 )); then
    echo "- None"
  else
    for b in "${BLOCKERS_P0[@]}"; do
      echo "- ${b}"
    done
  fi
  echo
  echo "### P1 (Must Resolve Before Public Testnet): ${P1_COUNT}"
  if (( P1_COUNT == 0 )); then
    echo "- None"
  else
    for b in "${BLOCKERS_P1[@]}"; do
      echo "- ${b}"
    done
  fi
  echo
  echo "### P2 (Deferred / Non-Blocking): ${P2_COUNT}"
  if (( P2_COUNT == 0 )); then
    echo "- None"
  else
    for b in "${BLOCKERS_P2[@]}"; do
      echo "- ${b}"
    done
  fi
  echo
  echo "## Traceability"
  echo
  echo "- Repo commit hash: ${COMMIT_HASH}"
  echo "- Registry source: proof/claims/registry.yml"
  echo "- Receipt source: proof/receipts/claims/*.receipt.json"
  echo
  echo "## Receipt Bindings (for evidence immutability)"
  echo
  echo "| Claim | Binding Hash | Receipt Timestamp |"
  echo "|---|---|---|"
  if [[ -n "${RECEIPT_BINDINGS}" ]]; then
    echo -n "${RECEIPT_BINDINGS}" | while IFS='|' read -r claim binding timestamp; do
      if [[ -n "${claim}" && -n "${binding}" ]]; then
        echo "| ${claim} | ${binding} | ${timestamp} |"
      fi
    done
  fi
  echo
  echo "This report is machine-computed from current claim status + receipt integrity/freshness state."
  echo "Receipt binding hashes and timestamps are captured at report generation time for evidence traceability."
} > "${REPORT_FILE}"

echo "✅ Report generated: ${REPORT_FILE}"
echo "Decision: ${DECISION}"
echo "Overall score: ${OVERALL_SCORE}%"

if [[ "${DECISION}" == "GO" ]]; then
  exit 0
fi

exit 1
