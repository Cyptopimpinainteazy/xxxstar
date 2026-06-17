.PHONY: guard test audit mainnet-check fresh-machine-check\
 test-atomic-kernel test-atomic-router test-axe test-x3-forge test-x3-sentinel\
 test-x3-wallet test-atomic-gateway test-x3-readiness test-x3-lang-vm\
 test-runtime-upgrade test-all-pallets

guard:
	@python scripts/agent_guard.py
	@python scripts/no_stub_guard.py
	@python scripts/test_cheat_guard.py

test:
	@pytest -q x3-lang/tests/test_parser.py x3-lang/tests/test_typechecker.py x3-lang/tests/test_e2e_mocked.py
	@cargo test --manifest-path x3-lang/compiler/Cargo.toml --tests
	@cargo test --manifest-path x3-lang/Cargo.toml --tests

# ── Pallet test suites (wired to CI — removes "No CI gate" blockers) ──
test-atomic-kernel:
	@cargo test -p pallet-x3-atomic-kernel --tests
test-atomic-router:
	@cargo test -p pallet-x3-cross-vm-router --tests
test-axe:
	@cargo test -p pallet-x3-dex --tests
test-x3-forge:
	@cargo test -p pallet-x3-token-factory --tests
test-x3-sentinel:
	@cargo test -p pallet-x3-sentinel --tests
test-x3-wallet:
	@cargo test -p pallet-x3-wallet-pallet --tests
test-atomic-gateway:
	@cargo test -p x3-gateway --tests
test-x3-readiness:
	@cargo test -p x3-readiness --tests
test-x3-lang-vm:
	@cargo test --manifest-path x3-lang/Cargo.toml --tests
test-runtime-upgrade:
	@echo "=== Runtime upgrade rehearsal ==="
	@cargo build -p node --features mainnet-rc1 --release
	@echo "=== Runtime built, try-runtime requires live chain ==="
test-all-pallets:
	@$(MAKE) test-atomic-kernel
	@$(MAKE) test-atomic-router
	@$(MAKE) test-axe
	@$(MAKE) test-x3-forge
	@$(MAKE) test-x3-sentinel
	@$(MAKE) test-x3-wallet
	@$(MAKE) test-atomic-gateway
	@$(MAKE) test-x3-readiness
	@$(MAKE) test-x3-lang-vm
	@echo "=== All pallet tests passed ==="

audit:
	@python scripts/invariant_guard.py
	@python scripts/mainnet_release_gate.py
	@bash scripts/check-readiness-consistency.sh

mainnet-check:
	@python scripts/mainnet_release_gate.py
	@bash scripts/check-readiness-consistency.sh

fresh-machine-check:
	@cargo build -p node --features mainnet-rc1 --release
	@$(MAKE) test-all-pallets
	@echo "=== Fresh machine check passed ==="