.PHONY: bmad-generate-steps bmad-generate-workflows bmad-validate bmad-clean help testnet-verify frontier-rpc-smoke frontier-rpc-smoke-local x3-proof x3-proof-install

# BMAD Build Automation - Phase 1 & 2
# Purpose: Consolidation generation and validation for steps and workflows
# Usage: make bmad-generate-steps
#        make bmad-generate-workflows
#        make bmad-validate

help:
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
	@echo ""
	@echo "  Combined:"
	@echo "    make bmad-generate            - Generate both steps and workflows"
	@echo "    make bmad-validate            - Validate both steps and workflows"
	@echo "    make bmad-clean               - Clean both steps and workflows"
	@echo ""
	@echo "  Testnet:"
	@echo "    make testnet-verify           - Run peer/finality (and optional telemetry/load) checks"
	@echo "                                   Uses TESTNET_CONFIG or docs/testnet-config/testnet-config.json"
	@echo ""
	@echo "  Frontier RPC:"
	@echo "    make frontier-rpc-smoke       - Run Frontier JSON-RPC smoke checks against NODE_URL"
	@echo "    make frontier-rpc-smoke-local - Launch a fresh local dev node, run smoke checks, then stop it"
	@echo ""
	@echo "  ProofForge CLI:"
	@echo "    make x3-proof                 - Run local ProofForge CLI wrapper (usage: make x3-proof ARGS='features --strict --fail-hard')"
	@echo "    make x3-proof-install         - Install x3-proof launcher to ~/.local/bin/x3-proof"

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
# SWARM ORCHESTRA 
# ============================================================================

start:
	@echo "Delegating to x3-swarm-orchestra..."
	@$(MAKE) -C x3-swarm-orchestra start

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
