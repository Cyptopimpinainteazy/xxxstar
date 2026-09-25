.PHONY: all setup start stop logs tps-bench

# Colors
GREEN=\033[0;32m
NC=\033[0m

all: setup start

setup:
	@echo "${GREEN}Setting up Swarm Orchestrator workspaces...${NC}"
	mkdir -p ./data
	mkdir -p ./logs
	chmod +x scripts/*.sh || true

start: setup
	@echo "${GREEN}Starting entire X3 Swarm Orchestra...${NC}"
	docker-compose up -d --build

stop:
	@echo "${GREEN}Stopping X3 Swarm Orchestra...${NC}"
	docker-compose down

logs:
	docker-compose logs -f

tps-bench:
	@echo "${GREEN}Benchmarking Multi-VM GPU TPS throughput...${NC}"
	docker-compose exec gpu-validator-swarm cargo run --bin x3_bench --release

arbitrage:
	@echo "${GREEN}Checking Arbitrage Strategies via quantum-swarm logs...${NC}"
	docker-compose logs -f quantum-swarm
