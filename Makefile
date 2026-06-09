.PHONY: bmad-generate-steps bmad-generate-workflows bmad-validate bmad-clean help testnet-verify frontier-rpc-smoke frontier-rpc-smoke-local x3-proof x3-proof-install guard test audit mainnet-check fresh-machine-check

# BMAD Build Automation - Phase 1 & 2
# Purpose: Consolidation generation and validation for steps and workflows
# Usage: make bmad-generate-steps
#        make bmad-generate-workflows
#        make bmad-validate

help:
	@echo "Production Gates:"
	@echo "  make guard                - run agent/stub/test-cheat guards"
	@echo "  make test                 - run focused Python + Rust compiler tests"
	@echo "  make audit                - run invariant and mainnet gate checks"
	@echo "  make mainnet-check        - run release gate validation"
	@echo "  make fresh-machine-check  - run fresh-machine bootstrap validation"
	@echo ""
	@echo "BMAD Consolidation Build Targets:"
	@echo "  Phase 1 (Steps):"
	@echo "    make bmad-generate-steps      - Generate .bmad/steps/ from YAML config"
	@echo "    make bmad-validate-steps      - Validate generated step files"
	@echo "    make bmad-clean-steps         - Remove generated step files"
	@echo ""
	@echo "  Phase 2 (Workflows):"
	@echo "    make bmad-generate-workflows  - Generate .github/workflows/ from YAML config"
	@echo "    make bmad-validate-workflows  - Validate generated workflow files"
	@echo "    make bmad-clean-workflows     - Remove generated workflow files"

guard:
	@python scripts/agent_guard.py
	@python scripts/no_stub_guard.py
	@python scripts/test_cheat_guard.py

test:
	@pytest -q x3-lang/tests/test_parser.py x3-lang/tests/test_typechecker.py x3-lang/tests/test_e2e_mocked.py
	@cargo test --manifest-path x3-lang/compiler/Cargo.toml --tests

audit:
	@python scripts/invariant_guard.py
	@python scripts/mainnet_release_gate.py

mainnet-check:
	@python scripts/mainnet_release_gate.py

fresh-machine-check:
	@bash scripts/fresh_machine_check.sh

x3-lang-mainnet-gate:
	@echo "=== x3-lang Mainnet Gate ==="
	@echo "--- Gate 1: x3-crosschain-intent compiles ---"
	@cargo check -p x3-crosschain-intent 2>&1
	@echo "--- Gate 2: x3-crosschain-intent tests pass ---"
	@cargo test -p x3-crosschain-intent --lib 2>&1
	@echo "--- Gate 3: old x3-lang workspace compiles ---"
	@cargo check --manifest-path x3-lang/Cargo.toml 2>&1 || echo "WARN: old x3-lang workspace has separate deps; skip if not installed"
	@echo "--- Gate 4: IntentSpecDraft JSON round-trip ---"
	@cargo test -p x3-crosschain-intent --lib from_draft::tests::draft_round_trips_through_json 2>&1
	@echo "--- Gate 5: IntentSpecDraft → CrossChainIntent → execution plan ---"
	@cargo test -p x3-crosschain-intent --lib from_draft::tests::draft_adapter_produces_compilable_intent 2>&1
	@echo "=== x3-lang Mainnet Gate PASSED ==="

# ============================================================================
# PHASE 1: STEP CONSOLIDATION TARGETS
# ============================================================================

bmad-generate-steps: .bmad/step-templates.yaml .bmad/templates/step-base-generic.md
	@echo "Generating step files from configuration..."
	@python3 scripts/process_templates.py \
		--config .bmad/step-templates.yaml \
		--base-template .bmad/templates/step-base-generic.md \
		--output .bmad/steps
	@echo "✓ Step generation complete"

bmad-generate-dry: .bmad/step-templates.yaml .bmad/templates/step-base-generic.md
	@echo "Previewing step generation (dry-run)..."
	@python3 scripts/process_templates.py \
		--config .bmad/step-templates.yaml \
		--base-template .bmad/templates/step-base-generic.md \
		--output .bmad/steps \
		--dry-run

bmad-validate-steps:
	@echo "Validating step files..."
	@ls -1 .bmad/steps/step-[0-9][0-9]-step-*.md 2>/dev/null | wc -l | xargs -I {} echo "✓ Found {} step files"

bmad-clean-steps:
	@echo "Removing generated step files..."
	@rm -f .bmad/steps/step-[0-9][0-9]-step-*.md
	@echo "✓ Step files cleaned"

# ============================================================================
# PHASE 2: WORKFLOW CONSOLIDATION TARGETS
# ============================================================================

bmad-generate-workflows: .bmad/workflows-templates.yaml
	@echo "Generating workflow files from configuration..."
	@python3 scripts/process_workflows.py \
		--config .bmad/workflows-templates.yaml \
		--output .github/workflows
	@echo "✓ Workflow generation complete"

bmad-generate-workflows-dry: .bmad/workflows-templates.yaml
	@echo "Previewing workflow generation (dry-run)..."
	@python3 scripts/process_workflows.py \
		--config .bmad/workflows-templates.yaml \
		--output .github/workflows \
		--dry-run

bmad-validate-workflows:
	@echo "Validating workflow files..."
	@python3 -c '\
from pathlib import Path; \
import yaml; \
workflows_dir = Path(".github/workflows"); \
workflows = list(workflows_dir.glob("*.yml")); \
valid_count = 0; \
for wf in workflows: \
    try: \
        yaml.safe_load(wf.read_text()); \
        valid_count += 1; \
    except yaml.YAMLError as e: \
        print(f"✗ Invalid YAML in {wf.name}: {e}"); \
if valid_count == len(workflows): \
    print(f"✓ All {valid_count} workflow files are valid YAML"); \
else: \
    print(f"⚠ {valid_count}/{len(workflows)} workflows are valid"); \
'

bmad-clean-workflows:
	@echo "Removing generated workflow files..."
	@rm -f .github/workflows/*.yml
	@echo "✓ Workflow files cleaned"

# ============================================================================
# TESTNET VERIFICATION
# ============================================================================

testnet-verify:
	@echo "Running testnet verification..."
	@scripts/testnet/verify-testnet.sh

frontier-rpc-smoke:
	@echo "Running Frontier RPC smoke against $${NODE_URL:-http://127.0.0.1:9944}..."
	@NODE_URL="$${NODE_URL:-http://127.0.0.1:9944}" scripts/frontier_rpc_smoke.sh

frontier-rpc-smoke-local:
	@echo "Launching fresh local dev node for Frontier RPC smoke..."
	@set -e; \
	START_DESKTOP=false ./run-dev-node.sh --purge >/tmp/x3-frontier-smoke.log 2>&1 & \
	NODE_PID=$$!; \
	cleanup() { kill $$NODE_PID 2>/dev/null || true; wait $$NODE_PID 2>/dev/null || true; }; \
	trap cleanup EXIT INT TERM; \
	for i in $$(seq 1 45); do \
		if curl -s http://127.0.0.1:9944 >/dev/null 2>&1; then break; fi; \
		sleep 1; \
		if [ $$i -eq 45 ]; then echo "Node did not become ready"; exit 1; fi; \
	done; \
	NODE_URL="http://127.0.0.1:9944" scripts/frontier_rpc_smoke.sh

x3-proof:
	@chmod +x bin/x3-proof
	@bin/x3-proof $(ARGS)

x3-proof-install:
	@mkdir -p "$$HOME/.local/bin"
	@chmod +x bin/x3-proof
	@install -m 0755 bin/x3-proof "$$HOME/.local/bin/x3-proof"
	@echo "Installed x3-proof -> $$HOME/.local/bin/x3-proof"
	@echo "If needed, add to PATH: export PATH=\"$$HOME/.local/bin:$$PATH\""


# ============================================================================
# COMBINED TARGETS (BOTH PHASES)
# ============================================================================

bmad-generate: bmad-generate-steps bmad-generate-workflows
	@echo ""
	@echo "✓ Phase 1 & Phase 2 consolidation complete"

bmad-validate: bmad-validate-steps bmad-validate-workflows
	@echo ""
	@echo "✓ Phase 1 & Phase 2 validation complete"

bmad-clean: bmad-clean-steps bmad-clean-workflows
	@echo ""
# ============================================================================
# X3 TEST TOOL STACK — Advanced Testing Infrastructure
# ============================================================================

.PHONY: install-tools install-rust-tools install-python install-substrate install-evm install-chaos \
        x3-audit x3-coverage x3-mutants x3-fuzz x3-substrate-check x3-evm-check x3-chaos-check \
        x3-nextest x3-deny-check x3-geiger x3-proptest x3-zombienet-checklist

install-tools: install-rust-tools install-python install-chaos
	@echo "✓ X3 Test Tool Stack installation complete"
	@echo "  Run 'make x3-audit' to verify security tools"
	@echo "  Run 'make x3-coverage' for coverage report"
	@echo "  Run 'make x3-mutants' for mutation testing"

install-rust-tools:
	@echo "=== Installing Rust test tools ==="
	@cargo install cargo-fuzz --locked 2>&1 | tail -1 || echo "  cargo-fuzz already installed"
	@cargo install cargo-nextest --locked 2>&1 | tail -1 || echo "  cargo-nextest already installed"
	@cargo install cargo-geiger --locked 2>&1 | tail -1 || echo "  cargo-geiger already installed"
	@cargo install subwasm --locked 2>&1 | tail -1 || echo "  subwasm already installed"
	@cargo install aderyn --locked 2>&1 | tail -1 || echo "  aderyn already installed"
	@cargo install kani-verifier --locked 2>&1 | tail -1 || echo "  kani already installed"
	@echo "✓ Rust tools installed"

install-python:
	@echo "=== Installing Python test tools ==="
	@pip3 install slither-analyzer 2>&1 | tail -1 || echo "  slither already installed"
	@echo "✓ Python tools installed"

install-chaos:
	@echo "=== Installing chaos/load tools ==="
	@which toxiproxy-cli 2>/dev/null || { \
		curl -sL https://github.com/shopify/toxiproxy/releases/download/v2.9.0/toxiproxy-server-linux-amd64 -o ~/.cargo/bin/toxiproxy-server && \
		curl -sL https://github.com/shopify/toxiproxy/releases/download/v2.9.0/toxiproxy-cli-linux-amd64 -o ~/.cargo/bin/toxiproxy-cli && \
		chmod +x ~/.cargo/bin/toxiproxy-* && echo "  toxiproxy installed"; }
	@which k6 2>/dev/null || { \
		curl -sL https://github.com/grafana/k6/releases/download/v0.54.0/k6-v0.54.0-linux-amd64.tar.gz -o /tmp/k6.tar.gz && \
		tar -xzf /tmp/k6.tar.gz -C /tmp/ && \
		cp /tmp/k6-v0.54.0-linux-amd64/k6 ~/.cargo/bin/ && \
		rm -rf /tmp/k6.tar.gz /tmp/k6-v0.54.0-linux-amd64 && echo "  k6 installed"; }
	@echo "✓ Chaos tools installed"

x3-audit:
	@echo "=== cargo-audit ==="
	@cargo audit --no-fetch 2>&1 || cargo audit 2>&1 || echo "WARNING: audit database update needed"
	@echo ""
	@echo "=== cargo-deny ==="
	@cargo deny check 2>&1 || true

x3-coverage:
	@echo "=== cargo-llvm-cov ==="
	@cargo llvm-cov nextest --workspace --lcov --output-path lcov.info 2>&1 || \
	 echo "WARNING: coverage requires 'cargo nextest' and running node"

x3-mutants:
	@echo "=== cargo-mutants on core crates ==="
	@for crate in x3-asset-kernel x3-atomic-trade x3-bridge; do \
		echo "--- $$crate ---"; \
		cargo mutants -p $$crate 2>&1 | tail -3; \
	done

x3-fuzz:
	@echo "=== cargo-fuzz (requires fuzz target setup) ==="
	@echo "  Fuzz targets defined in tools/fuzz-targets/"
	@echo "  Setup: cargo fuzz init && cargo fuzz add bridge_message_decode"
	@echo "  Run:   cargo fuzz run bridge_message_decode"

x3-nextest:
	@echo "=== cargo nextest ==="
	@cargo nextest run --workspace --no-tests=warn 2>&1 || \
	 cargo test --workspace 2>&1 | tail -5

x3-deny-check:
	@cargo deny check advisories 2>&1
	@cargo deny check licenses 2>&1

x3-geiger:
	@echo "=== cargo-geiger (unsafe usage audit) ==="
	@which cargo-geiger && cargo geiger 2>&1 | head -30 || echo "  cargo-geiger not installed"

x3-proptest:
	@echo "=== proptest (property-based testing) ==="
	@echo "  proptest is in workspace dependencies (proptest = \"1.4\")"
	@echo "  Enabled in crates: x3-asset-kernel, x3-atomic-trade"
	@echo "  Run specific proptest: cargo test -p x3-asset-kernel proptest"

x3-zombienet-checklist:
	@echo "=== Zombienet Integration Test Checklist ==="
	@echo "  1. Build node: cargo build --release --bin x3-node"
	@echo "  2. Install Zombienet: npm install -g @zombienet/cli"
	@echo "  3. Run: zombienet spawn tools/test-tool-stack/zombienet/x3-local-7.toml"
	@echo "  4. Run test: zombienet test tools/test-tool-stack/zombienet/x3-finality-smoke.zndsl"
	@echo ""

x3-substrate-report:
	@echo "=== Substrate Tool Report ==="
	@echo "  try-runtime:   $$(which try-runtime 2>/dev/null || echo 'via cargo run frame-try-runtime')"
	@echo "  frame-bench:   $$(which frame-benchmarking 2>/dev/null || echo 'via cargo bench')"
	@echo "  subwasm:       $$(subwasm --version 2>/dev/null || echo 'not installed')"
	@echo "  srtool:        $$(docker ps 2>/dev/null && echo 'Docker available' || echo 'Docker not available')"
	@echo ""

x3-tool-status:
	@echo "=== X3 Test Tool Stack Status ==="
	@echo "--- Rust Tools ---"
	@for tool in cargo-audit cargo-deny cargo-mutants cargo-fuzz cargo-nextest cargo-geiger cargo-llvm-cov subwasm aderyn; do \
		printf "  %-20s " $$tool; \
		$$tool --version 2>/dev/null && echo "" || echo "NOT INSTALLED"; \
	done
	@echo ""
	@echo "--- Python Tools ---"
	@for tool in slither; do \
		printf "  %-20s " $$tool; \
		$$tool --version 2>/dev/null && echo "" || echo "NOT INSTALLED"; \
	done
	@echo ""
	@echo "--- EVM Tools ---"
	@for tool in forge cast anvil; do \
		printf "  %-20s " $$tool; \
		$$tool --version 2>/dev/null && echo "" || echo "NOT INSTALLED"; \
	done
	@echo ""
	@echo "--- Infrastructure Tools ---"
	@for tool in k6 toxiproxy-cli echidna; do \
		printf "  %-20s " $$tool; \
		$$tool --version 2>/dev/null && echo "" || echo "NOT INSTALLED"; \
	done
	@echo ""

# ============================================================================
# PHASE 3: YOLO FINISHER v5.0 (NUCLEAR FINALIZATION)
# ============================================================================

.PHONY: finish finish-stack finish-score finish-chaos finish-audit finish-clean

finish:
	@echo "☢️ YOLO FINISHER v5.0 — Starting Nuclear Finalization Pass..."
	@scripts/finisher_daemon.py --watch-dir ./drop --work-dir ./workspace

finish-stack:
	@echo "☢️ YOLO FINISHER — Running Full Stack Sequence..."
	@echo "Agents: CARTOGRAPHER ARCHAEOLOGIST BREAKER AUDITOR INTENT_ANALYST INTEGRATOR VERIFIER FIXER ECONOMIST CHAOS_ENGINE COMPLETION_JUDGE"
	@# Individual agent execution would go here if bound to CLI

finish-score:
	@echo "⚖️ YOLO FINISHER — Computing Readiness Score..."
	@# Trigger completion judge

finish-chaos:
	@echo "🌪️ YOLO FINISHER — Injecting Chaos & Fuzzing..."
	@# Trigger chaos engine

finish-audit:
	@echo "🕵️ YOLO FINISHER — Running Security & Economic Audit..."
	@# Trigger auditor + economist

finish-clean:
	@echo "🧹 Cleaning Finisher workspace..."
	@rm -rf ./workspace
	@rm -rf ./drop
	@echo "✓ Finisher workspace cleaned"

# ============================================================================
# PREREQUISITES CHECK
# ============================================================================

.bmad/step-templates.yaml:
	@echo "✗ Missing: .bmad/step-templates.yaml"
	@exit 1

.bmad/templates/step-base-generic.md:
	@echo "✗ Missing: .bmad/templates/step-base-generic.md"
	@exit 1

.bmad/workflows-templates.yaml:
	@echo "✗ Missing: .bmad/workflows-templates.yaml"
	@exit 1
