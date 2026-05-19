.PHONY: build up stop simulate test test-htlc lint clean keygen deploy logs

# === BUILD ===
build:
	@echo "Building Docker images..."
	docker-compose build

keygen:
	@echo "Generating compiler keys..."
	./compiler/keygen.sh

# === RUN ===
up:
	@echo "Starting all services..."
	docker-compose up -d
	@echo "Waiting for services to initialize..."
	@sleep 5
	@echo "Services running:"
	@docker-compose ps

stop:
	@echo "Stopping all services..."
	docker-compose down

logs:
	docker-compose logs -f

logs-python:
	docker-compose logs -f python-service

logs-hardhat:
	docker-compose logs -f hardhat

# === DEPLOYMENT ===
deploy:
	@echo "Deploying contracts to local Hardhat network..."
	docker-compose exec hardhat npx hardhat run scripts/deploy.js --network localhost

deploy-local:
	cd hardhat && npx hardhat run scripts/deploy.js --network localhost

# === SIMULATION ===
simulate:
	@echo "Running full lifecycle simulation..."
	docker-compose exec python-service python /app/python/cli/simulate_lifecycle.py

simulate-local:
	python python/cli/simulate_lifecycle.py

# === TESTING ===
test: test-contracts test-python

test-contracts:
	@echo "Running Hardhat contract tests..."
	cd hardhat && npm test

test-python:
	@echo "Running Python tests..."
	python -m pytest python/tests/ -v

test-htlc:
	@echo "Running HTLC atomic swap integration test..."
	docker-compose exec python-service python /app/python/tests/test_htlc_integration.py

test-integration:
	@echo "Running full integration tests..."
	docker-compose exec python-service python /app/python/tests/test_integration.py

# === LINTING ===
lint: lint-python lint-solidity

lint-python:
	@echo "Linting Python code..."
	black python/ --check
	flake8 python/ --max-line-length=120

lint-solidity:
	@echo "Linting Solidity code..."
	cd hardhat && npx solhint contracts/**/*.sol

format:
	@echo "Formatting Python code..."
	black python/

# === CLEAN ===
clean:
	@echo "Cleaning build artifacts..."
	rm -rf hardhat/artifacts hardhat/cache
	rm -rf python/__pycache__ python/**/__pycache__
	rm -rf python/tmp/*
	find . -name "*.pyc" -delete

clean-all: clean
	@echo "Removing generated keys and data..."
	rm -f compiler/key.pem compiler/key.pub
	docker-compose down -v

# === UTILITIES ===
shell-python:
	docker-compose exec python-service /bin/bash

shell-hardhat:
	docker-compose exec hardhat /bin/sh

ipfs-id:
	docker-compose exec ipfs ipfs id

bitcoin-info:
	docker-compose exec bitcoin bitcoin-cli -regtest getblockchaininfo

# === HELP ===
help:
	@echo "botchain-tri-vm-genesis Makefile"
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@echo "Build:"
	@echo "  build         Build all Docker images"
	@echo "  keygen        Generate compiler signing keys"
	@echo ""
	@echo "Run:"
	@echo "  up            Start all services"
	@echo "  stop          Stop all services"
	@echo "  logs          Follow all service logs"
	@echo ""
	@echo "Deploy:"
	@echo "  deploy        Deploy contracts to local Hardhat"
	@echo ""
	@echo "Simulate:"
	@echo "  simulate      Run full lifecycle simulation"
	@echo ""
	@echo "Test:"
	@echo "  test          Run all tests"
	@echo "  test-contracts Run Hardhat tests"
	@echo "  test-python   Run Python tests"
	@echo "  test-htlc     Run HTLC integration test"
	@echo ""
	@echo "Lint:"
	@echo "  lint          Run all linters"
	@echo "  format        Auto-format Python code"
	@echo ""
	@echo "Clean:"
	@echo "  clean         Remove build artifacts"
	@echo "  clean-all     Remove all generated files"
