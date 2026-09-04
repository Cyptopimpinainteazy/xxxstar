#!/usr/bin/env bash
################################################################################
# RC5 CHAOS TESTING - QUICK REFERENCE CARD
# 
# Copy-paste commands to test different aspects of RC5 resilience
################################################################################

# ============================================================================
# 🚀 QUICK START (No Sudo Needed)
# ============================================================================

# Test 1: Check for RPC vulnerabilities (2 min)
bash scripts/mainnet/rc5_attack_vectors.sh

# Test 2: Run light orchestrated scenario (10 min, safe baseline)
bash scripts/mainnet/rc5_resilience_orchestrator.sh light

# Test 3: Check RC5 main harness status
tail -n 20 logs/rc5/rc5_72h.nohup.log | grep -E "Block|finalized"


# ============================================================================
# ⚡ NETWORK CHAOS (Tests latency/packet loss handling)
# ============================================================================

# Light network stress (50ms latency, 2% loss)
bash scripts/mainnet/rc5_chaos_harness.sh network 300 1

# Medium network chaos (100ms latency, 5% loss)
bash scripts/mainnet/rc5_chaos_harness.sh network 600 3

# Heavy network chaos (150ms latency, 8% loss)
bash scripts/mainnet/rc5_chaos_harness.sh network 900 5

# Extreme network chaos (200ms latency, 20% loss)
sudo bash scripts/mainnet/rc5_chaos_harness.sh network 1200 10


# ============================================================================
# 💾 DISK CHAOS (Tests I/O saturation & disk pressure)
# ============================================================================

# Light disk stress (background I/O)
bash scripts/mainnet/rc5_chaos_harness.sh disk 300 1

# Medium disk chaos (saturated I/O)
sudo bash scripts/mainnet/rc5_chaos_harness.sh disk 600 3

# Heavy disk pressure (fill 30-50% of disk)
sudo bash scripts/mainnet/rc5_chaos_harness.sh disk 900 5


# ============================================================================
# 🧠 MEMORY CHAOS (Tests memory pressure handling)
# ============================================================================

# Light memory stress (100-300MB pressure)
bash scripts/mainnet/rc5_chaos_harness.sh memory 300 1

# Medium memory pressure (400-600MB)
bash scripts/mainnet/rc5_chaos_harness.sh memory 600 4

# Heavy memory pressure (700-1000MB)
bash scripts/mainnet/rc5_chaos_harness.sh memory 900 7


# ============================================================================
# ⚔️ PROCESS CHAOS (Tests recovery from crashes)
# ============================================================================

# Light process stress (occasional crashes)
bash scripts/mainnet/rc5_chaos_harness.sh process 300 1

# Medium process chaos (multiple crashes + rapid restarts)
bash scripts/mainnet/rc5_chaos_harness.sh process 600 3

# Heavy process chaos (rapid crash cascade)
bash scripts/mainnet/rc5_chaos_harness.sh process 900 6

# Monitor: Watch validators restart
watch -n 1 'pgrep -f "x3-chain-node" | wc -l'


# ============================================================================
# 🔌 RPC CHAOS (Tests JSON-RPC robustness)
# ============================================================================

# Light RPC errors (5% error rate)
bash scripts/mainnet/rc5_chaos_harness.sh rpc 300 1

# Medium RPC chaos (10-15% error rate + timeouts)
bash scripts/mainnet/rc5_chaos_harness.sh rpc 600 3

# Heavy RPC chaos (high error rate + slow responses)
bash scripts/mainnet/rc5_chaos_harness.sh rpc 900 6


# ============================================================================
# 🗄️ DATABASE CHAOS (Tests lock contention & integrity)
# ============================================================================

# Light database contention
bash scripts/mainnet/rc5_chaos_harness.sh db 300 1

# Medium lock contention (may cause brief unresponsiveness)
bash scripts/mainnet/rc5_chaos_harness.sh db 600 3

# Heavy lock contention (significant stalling possible)
sudo bash scripts/mainnet/rc5_chaos_harness.sh db 900 5


# ============================================================================
# 🌪️ CASCADE CHAOS (All vectors simultaneously)
# ============================================================================

# Light cascade (network + process + memory)
bash scripts/mainnet/rc5_chaos_harness.sh cascade 300 1

# Medium cascade (moderate stress across all vectors)
sudo bash scripts/mainnet/rc5_chaos_harness.sh cascade 600 3

# Heavy cascade (aggressive stress on everything)
sudo bash scripts/mainnet/rc5_chaos_harness.sh cascade 900 6

# Monitor during cascade
tail -f logs/rc5-chaos/chaos_harness.log | grep -i "impact\|issue\|recovery"


# ============================================================================
# 📊 ATTACK VECTORS (Targeted vulnerability testing)
# ============================================================================

# Run all 10 attack vectors
bash scripts/mainnet/rc5_attack_vectors.sh

# View vulnerability summary
cat reports/rc5-attacks/attack_assessment_*.json | jq '.attack_assessment'

# See specific vulnerabilities found
cat reports/rc5-attacks/attack_assessment_*.json | jq '.vulnerabilities[]'


# ============================================================================
# 🎯 ORCHESTRATED MULTI-PHASE TESTS
# ============================================================================

# Baseline light scenario (~10 min)
bash scripts/mainnet/rc5_resilience_orchestrator.sh light

# Moderate stress (~20 min)
bash scripts/mainnet/rc5_resilience_orchestrator.sh medium

# Aggressive testing (~30 min)
sudo bash scripts/mainnet/rc5_resilience_orchestrator.sh heavy

# Maximum stress (requires confirmation)
sudo bash scripts/mainnet/rc5_resilience_orchestrator.sh extreme

# Full comprehensive testing (~90 min, runs all phases)
sudo bash scripts/mainnet/rc5_resilience_orchestrator.sh full


# ============================================================================
# 🔍 MONITORING & DIAGNOSTICS
# ============================================================================

# Watch chaos execution in real-time
tail -f logs/rc5-chaos/chaos_harness.log

# Watch main RC5 harness for issues
tail -f logs/rc5/rc5_72h.nohup.log

# Monitor validator count (should stay at 3)
watch -n 1 'echo "Running validators: $(pgrep -f x3-chain-node | wc -l)"'

# Check for panics in main harness (should be 0)
grep -c "panic\|PANIC" logs/rc5/rc5_72h.nohup.log

# Check for database corruption (should be 0)
grep -c "corrupt\|rocksdb.*error" logs/rc5/rc5_72h.nohup.log

# Monitor memory usage of validators
watch -n 2 'ps aux | grep "[x]3-chain-node" | awk "{s+=$6} END {print \"Total validator memory: \" s \"KB\"}"'

# Check current block production
watch -n 5 'curl -s -X POST http://127.0.0.1:9964 -H "Content-Type: application/json" -d "{\"jsonrpc\":\"2.0\",\"method\":\"chain_getHeader\",\"params\":[],\"id\":1}" 2>/dev/null | jq ".result.number"'


# ============================================================================
# 📈 REPORT ANALYSIS
# ============================================================================

# View chaos assessment
cat reports/rc5-chaos/chaos_assessment_*.json | jq '.' | less

# View attack assessment
cat reports/rc5-attacks/attack_assessment_*.json | jq '.' | less

# View resilience report (markdown)
cat reports/rc5-orchestrator/resilience_report_*.md | less

# Get resilience rating
cat reports/rc5-orchestrator/resilience_report_*.md | grep "Defense Rating"


# ============================================================================
# 🧹 CLEANUP & RESET
# ============================================================================

# Clean up network chaos (if hung up)
sudo tc qdisc del dev lo root 2>/dev/null || echo "No qdisc to clean"
sudo iptables -F 2>/dev/null || echo "No iptables to clean"

# Kill any stuck flock processes
pkill -f "flock" || echo "No flock processes"

# Remove temporary files
rm -f logs/rc5-chaos/*.tmp logs/rc5-chaos/disk_pressure.img

# Check if validators are running after cleanup
pgrep -f "x3-chain-node" | wc -l

# Full emergency cleanup (if needed)
sudo tc qdisc del dev lo root 2>/dev/null || true
sudo iptables -F 2>/dev/null || true
pkill -f "stress-ng\|flock" || true
rm -f logs/rc5-chaos/*.tmp logs/rc5-chaos/disk_pressure.img
echo "Cleanup complete - validators should be running normally"


# ============================================================================
# 💡 DECISION TREE: Which test to run?
# ============================================================================

# I want to see if the system is stable
→ bash scripts/mainnet/rc5_resilience_orchestrator.sh light

# I want to test RPC robustness
→ bash scripts/mainnet/rc5_attack_vectors.sh

# I want to test network resilience
→ bash scripts/mainnet/rc5_chaos_harness.sh network 600 3

# I want to test crash recovery
→ bash scripts/mainnet/rc5_chaos_harness.sh process 600 3

# I want comprehensive testing
→ sudo bash scripts/mainnet/rc5_resilience_orchestrator.sh heavy

# I want to break it (stress test)
→ sudo bash scripts/mainnet/rc5_chaos_harness.sh cascade 1200 9

# I want to find all vulnerabilities
→ sudo bash scripts/mainnet/rc5_resilience_orchestrator.sh full


# ============================================================================
# ✅ EXPECTED OUTCOMES
# ============================================================================

# Good outcome:
# - Validators crash and recover within 30 seconds
# - No panics in logs
# - RPC eventually becomes responsive again
# - Block production resumes
# - No data corruption detected

# Bad outcome:
# - Validators don't recover
# - Panics found in logs
# - RPC permanently unresponsive
# - Block production completely stalled
# - Database corruption detected


# ============================================================================
# 📚 ADDITIONAL RESOURCES
# ============================================================================

# Full guide with detailed explanations
cat CHAOS_TESTING_GUIDE.md

# Check RC5 status during testing
bash scripts/mainnet/rc5_internal_alpha_72h.sh --status

# Check session memory for run details
cat /memories/session/rc5-active-run-status.md
