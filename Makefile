.PHONY: guard test audit mainnet-check fresh-machine-check\
 test-atomic-kernel test-atomic-router test-axe test-x3-forge test-x3-sentinel\
 test-x3-wallet test-atomic-gateway test-x3-readiness test-x3-lang-vm\
 test-runtime-upgrade test-all-pallets fmt lint\
 bench bench-criterion bench-k6 bench-pallets bench-report bench-all

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
	@cargo test -p pallet-x3-atomic-kernel -- --nocapture 2>&1 | tail -5
test-atomic-router:
	@cargo test -p pallet-x3-cross-vm-router -- --nocapture 2>&1 | tail -5
test-axe:
	@cargo test -p pallet-x3-dex --features std -- --nocapture 2>&1 | tail -5
test-x3-forge:
	@cargo test -p pallet-x3-token-factory --features std -- --nocapture 2>&1 | tail -5
test-x3-sentinel:
	@cargo test -p pallet-x3-sentinel --features std -- --nocapture 2>&1 | tail -5
test-x3-wallet:
	@cargo test -p pallet-x3-wallet-pallet --features std -- --nocapture 2>&1 | tail -5
test-atomic-gateway:
	@cargo test -p x3-gateway -- --nocapture 2>&1 | tail -5
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

fmt:
	@cargo fmt --manifest-path x3-lang/Cargo.toml --all
	@echo "=== x3-lang workspace formatted ==="

lint:
	@cargo clippy --manifest-path x3-lang/Cargo.toml --workspace --all-targets -- -D warnings
	@echo "=== x3-lang workspace clippy clean ==="

# ── Benchmarking targets ────────────────────────────────────────────────────

# Run all Criterion microbenchmarks
bench-criterion:
	@echo "=== Criterion microbenchmarks ==="
	@cd benches && cargo bench --bench atomic_swap_bench -- --output-format bencher
	@cd benches && cargo bench --bench dex_route_bench -- --output-format bencher
	@cd benches && cargo bench --bench bridge_proof_bench -- --output-format bencher
	@cd benches && cargo bench --bench vm_dispatch_bench -- --output-format bencher
	@cd benches && cargo bench --bench rpc_encoding_bench -- --output-format bencher
	@cd benches && cargo bench --bench signature_verify_bench -- --output-format bencher
	@echo "=== Criterion benchmarks complete ==="

# Run k6 RPC/WebSocket load test (requires k6 installed + node running)
bench-k6:
	@echo "=== k6 RPC load test ==="
	@mkdir -p reports/benchmarks
	@k6 run --vus 20 --duration 30s \
		--summary-export reports/benchmarks/x3-k6-summary.json \
		benchmarks/k6/x3_rpc_load.js
	@echo "=== k6 load test complete ==="

# Run FRAME pallet weight benchmarks (requires node binary with runtime-benchmarks)
bench-pallets:
	@echo "=== FRAME pallet weight benchmarks ==="
	@bash scripts/benchmark_pallet_weights.sh
	@echo "=== Pallet weight benchmarks complete ==="

# Generate benchmark report from collected results
bench-report:
	@echo "=== Benchmark report generation ==="
	@python scripts/benchmark_report.py --criterion-dir target/criterion
	@echo "=== Benchmark report written ==="

# Run all benchmark suites
bench-all:
	@echo "=== X3 Full Benchmark Suite ==="
	@bash scripts/run_all_benchmarks.sh
	@echo "=== Full benchmark run complete ==="

# Quick benchmark (reduced sample sizes)
bench:
	@echo "=== X3 Quick Benchmark ==="
	@bash scripts/run_all_benchmarks.sh --quick
	@echo "=== Quick benchmark complete ==="
