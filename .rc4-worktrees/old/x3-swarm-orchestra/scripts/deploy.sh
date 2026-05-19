#!/usr/bin/env bash
# Deploy script uniting the whole platform
set -e

echo "==============================================="
echo "🚀 INITIATING X3 SWARM ORCHESTRA DEPLOYMENT 🚀"
echo "==============================================="

# Ensure directories exist
mkdir -p data logs

# Step 1: Fire up the core blockchain / GPU validators first
echo "-> 1. Launching Multi-VM GPU Swarm Validators..."
docker-compose up -d gpu-validator-swarm
sleep 5

# Step 2: Boot Autonomic AI Agents for monitoring
echo "-> 2. Initializing Autonomic Health Monitoring..."
docker-compose up -d python-swarm-agent
sleep 3

# Step 3: Start Arbitrage & Quantum Engine 
echo "-> 3. Starting High-Frequency Arbitrage Scanners..."
docker-compose up -d quantum-swarm atomic-swap-orchestrator
sleep 3

# Step 4: Fire up User AI Features & Marketing Automation
echo "-> 4. Booting Marketing & Paid User Features (Coding/Video/PostAutomation)..."
docker-compose up -d marketing-post-automation swarm-media

echo "==============================================="
echo "✅ ORCHESTRATION ONLINE."
echo "Use 'make logs' to watch the swarm orchestrate."
echo "Use 'make tps-bench' to hit maximum blockchain output."
echo "==============================================="
