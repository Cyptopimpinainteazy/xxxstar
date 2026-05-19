#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

require_pattern() {
  local file="$1"
  local pattern="$2"
  local message="$3"

  if ! rg -n --no-heading --fixed-strings "$pattern" "$file" >/dev/null; then
    echo "::error file=${file}::${message}"
    exit 1
  fi
}

echo "Checking node cross-VM safety gate wiring..."
require_pattern "node/src/service.rs" "struct CrossVmBridgeSafetyGate" "Missing CrossVmBridgeSafetyGate type"
require_pattern "node/src/service.rs" "fn preflight(" "Missing cross-VM preflight safety check"
require_pattern "node/src/service.rs" "fn postflight(" "Missing cross-VM postflight safety check"
require_pattern "node/src/service.rs" "fn open_dispute(" "Missing cross-VM dispute escalation path"
require_pattern "node/src/service.rs" "bridge_safety_gate.preflight(" "Cross-VM poller does not run preflight safety gate"
require_pattern "node/src/service.rs" "bridge_safety_gate.postflight(&results)" "Cross-VM poller does not run postflight safety gate"
require_pattern "node/src/service.rs" "b.pause();" "Cross-VM poller does not pause bridge on rejected batch"

echo "Checking node dependency wiring for safety crates..."
require_pattern "node/Cargo.toml" "x3-finality-oracle" "node missing x3-finality-oracle dependency"
require_pattern "node/Cargo.toml" "x3-verification-router" "node missing x3-verification-router dependency"
require_pattern "node/Cargo.toml" "x3-validator-attestation" "node missing x3-validator-attestation dependency"
require_pattern "node/Cargo.toml" "x3-proof-dispute" "node missing x3-proof-dispute dependency"
require_pattern "node/Cargo.toml" "x3-gateway-risk-engine" "node missing x3-gateway-risk-engine dependency"

echo "Checking relayer safety pipeline wiring..."
require_pattern "crates/x3-relayer/src/relayer.rs" "struct RelayerSafetyPipeline" "Missing RelayerSafetyPipeline"
require_pattern "crates/x3-relayer/src/relayer.rs" "evaluate_evm_proof(" "Missing relayer EVM proof safety evaluation"
require_pattern "crates/x3-relayer/src/relayer.rs" "evaluate_svm_proof(" "Missing relayer SVM proof safety evaluation"
require_pattern "crates/x3-relayer/src/relayer.rs" "self.safety_pipeline.evaluate_evm_proof(&proof, recent_failures)" "Relayer EVM submission path not safety-gated"
require_pattern "crates/x3-relayer/src/relayer.rs" "self.safety_pipeline.evaluate_svm_proof(&proof, recent_failures)" "Relayer SVM submission path not safety-gated"
require_pattern "crates/x3-relayer/src/relayer.rs" "safety_pipeline_accepts_evm_happy_path" "Missing relayer safety happy-path test"
require_pattern "crates/x3-relayer/src/relayer.rs" "safety_pipeline_rejects_unknown_domain_finality" "Missing relayer finality failure-path test"
require_pattern "crates/x3-relayer/src/relayer.rs" "safety_pipeline_rejects_svm_quorum_gap" "Missing relayer attestation failure-path test"

echo "Checking relayer dependency wiring for safety crates..."
require_pattern "crates/x3-relayer/Cargo.toml" "x3-finality-oracle" "relayer missing x3-finality-oracle dependency"
require_pattern "crates/x3-relayer/Cargo.toml" "x3-verification-router" "relayer missing x3-verification-router dependency"
require_pattern "crates/x3-relayer/Cargo.toml" "x3-validator-attestation" "relayer missing x3-validator-attestation dependency"
require_pattern "crates/x3-relayer/Cargo.toml" "x3-proof-dispute" "relayer missing x3-proof-dispute dependency"
require_pattern "crates/x3-relayer/Cargo.toml" "x3-gateway-risk-engine" "relayer missing x3-gateway-risk-engine dependency"

echo "Cross-VM safety wiring gate passed"
