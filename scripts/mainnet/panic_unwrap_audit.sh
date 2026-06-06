#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REPORT="$ROOT_DIR/reports/panic_unwrap_audit.md"
mkdir -p "$ROOT_DIR/reports"

cd "$ROOT_DIR"

mapfile -t TARGETS < <(find runtime node pallets crates -type f -name "*.rs" | sort)

{
  echo "# Panic Unwrap Audit"
  echo
  echo "Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo
  echo "## Classification Rules"
  echo
  echo "- test-only: path includes tests, fuzz, benchmarking, mock"
  echo "- startup fail-fast: path includes node/src/main or startup gate"
  echo "- compiler/tooling-only: path includes build.rs, macros, or generated tooling"
  echo "- runtime hook: path and context include on_initialize, on_finalize, offchain_worker"
  echo "- extrinsic path: path and context include pallet call functions"
  echo "- production hot path: runtime, pallet, or node execution code not covered above"
  echo
  echo "## Findings"
  echo
} > "$REPORT"

classify() {
  local file="$1"
  local line="$2"
  local text="$3"

  if [[ "$file" =~ (^|/)tests?(/|$)|/fuzz/|/benchmarking|/mock ]]; then
    echo "test-only"
    return
  fi
  if [[ "$file" =~ build\.rs$|/macros/|/codegen/ ]]; then
    echo "compiler/tooling-only"
    return
  fi
  if [[ "$file" =~ node/src/main|startup_gate ]]; then
    echo "startup fail-fast"
    return
  fi
  if [[ "$text" =~ on_initialize|on_finalize|offchain_worker ]]; then
    echo "runtime hook"
    return
  fi
  if [[ "$text" =~ "#[pallet::call]"|"pub fn " ]]; then
    echo "extrinsic path"
    return
  fi
  echo "production hot path"
}

hot_path_count=0
runtime_hook_count=0
extrinsic_count=0

for file in "${TARGETS[@]}"; do
  while IFS= read -r hit; do
    [[ -z "$hit" ]] && continue
    line_no="${hit%%:*}"
    line_txt="${hit#*:}"
    class="$(classify "$file" "$line_no" "$line_txt")"

    if [[ "$class" == "production hot path" ]]; then
      hot_path_count=$((hot_path_count + 1))
    fi
    if [[ "$class" == "runtime hook" ]]; then
      runtime_hook_count=$((runtime_hook_count + 1))
    fi
    if [[ "$class" == "extrinsic path" ]]; then
      extrinsic_count=$((extrinsic_count + 1))
    fi

    {
      echo "- [$class] $file:$line_no"
      echo "  - $line_txt"
    } >> "$REPORT"
  done < <(rg -n "panic!|unwrap\(|expect\(" "$file" || true)
done

{
  echo
  echo "## Summary"
  echo
  echo "- runtime-hook findings: $runtime_hook_count"
  echo "- extrinsic-path findings: $extrinsic_count"
  echo "- production hot-path findings: $hot_path_count"
  echo
  echo "## Gate Evaluation"
  echo
  if [[ "$runtime_hook_count" -eq 0 ]]; then
    echo "- runtime hook panic path gate: PASS"
  else
    echo "- runtime hook panic path gate: FAIL"
  fi

  if [[ "$hot_path_count" -eq 0 && "$extrinsic_count" -eq 0 ]]; then
    echo "- user-triggerable unwrap/expect gate: PASS"
  else
    echo "- user-triggerable unwrap/expect gate: FAIL"
  fi
} >> "$REPORT"

echo "panic_unwrap_audit: wrote $REPORT"
