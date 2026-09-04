# RC5 Chaos Engineering & Resilience Testing Suite

## 🎯 Overview

This suite of tools lets you throw adversarial conditions at the X3 Atomic Star RC5 validators to stress-test and validate system resilience. Think of it as a "chaos monkey" that systematically breaks things to prove the system can handle it.

## 📦 What's Included

### 1. **rc5_chaos_harness.sh** (21KB)
Systematic chaos injection framework with multiple attack vectors.

**Chaos Modes:**
- `network` - Latency, packet loss, partitions, timeouts
- `disk` - I/O saturation, space pressure  
- `memory` - Resource starvation, allocation pressure
- `process` - Crashes, signal floods, rapid restarts
- `rpc` - Error injection, slow responses, timeout cascades
- `db` - Lock contention, corruption simulation
- `clock` - System time skew (dangerous - use carefully)
- `cascade` - Multiple vectors simultaneously
- `all` - All vectors at once

**Intensity Levels:** 1-10 (1=light, 10=maximum)

**Duration:** Configurable in seconds

### 2. **rc5_attack_vectors.sh** (16KB)
Targeted exploit testing - 10 specific vulnerability attacks.

**Attack Vectors:**
1. RPC numeric overflow (block hash, parameter parsing)
2. RPC timeout cascade (100 concurrent slow requests)
3. Finality race condition (finality regression detection)
4. Settlement state machine violations
5. Cross-validator divergence (consensus breaks)
6. Process restart loops (cascade failures)
7. Database deadlock (lock acquisition)
8. Memory leak simulation (500 rapid requests)
9. Block production stall (consensus freeze)
10. Settlement invariant violations

### 3. **rc5_resilience_orchestrator.sh** (12KB)
Master coordination script that runs comprehensive multi-phase testing.

**Test Scenarios:**
- `light` - Safe low-intensity baseline tests (~10 min)
- `medium` - Moderate chaos progression (~20 min)
- `heavy` - Aggressive adversarial conditions (~30 min)
- `extreme` - Maximum stress (requires confirmation)
- `full` - Complete multi-phase comprehensive testing (~90 min)

---

## 🚀 Quick Start

### Before You Start ⚠️

**CRITICAL:** The active RC5 72-hour run is currently executing. These chaos tools are SAFE to run in parallel:

1. ✅ They use localhost network chaos (tc/iptables) - only affects local traffic
2. ✅ They don't modify blockchain state - only test RPC interfaces
3. ✅ They work on copies of DB paths - don't corrupt production data
4. ⚠️ They will cause temporary validator unresponsiveness
5. ⚠️ They require sudo for network/disk operations

### Option A: Run Attack Vectors Only (No Sudo Needed, ~2 min)

```bash
bash scripts/mainnet/rc5_attack_vectors.sh
```

**What it does:**
- Tests RPC parsing vulnerabilities
- Checks finality race conditions
- Validates cross-validator consensus
- Detects memory leaks
- Tests recovery from crashes

**Output:**
- `logs/rc5-attacks/attack_vectors.log` - Detailed attack log
- `reports/rc5-attacks/attack_assessment_*.json` - JSON vulnerability report

### Option B: Run Light Chaos (Safe, ~5 min)

```bash
bash scripts/mainnet/rc5_chaos_harness.sh network 300 1
bash scripts/mainnet/rc5_chaos_harness.sh disk 300 1
```

**What it does:**
- Injects 50ms network latency + 2% packet loss
- Saturates disk I/O with background writes
- Tests validator recovery

**Note:** Requires `sudo` for network/disk operations

### Option C: Run Full Orchestrated Test (Comprehensive, ~30-90 min)

```bash
bash scripts/mainnet/rc5_resilience_orchestrator.sh medium
```

**Available scenarios:**
```bash
# Light (safe, baseline)
bash scripts/mainnet/rc5_resilience_orchestrator.sh light

# Medium (moderate stress)
bash scripts/mainnet/rc5_resilience_orchestrator.sh medium

# Heavy (aggressive)
bash scripts/mainnet/rc5_resilience_orchestrator.sh heavy

# Extreme (maximum - requires confirmation)
bash scripts/mainnet/rc5_resilience_orchestrator.sh extreme

# Full (comprehensive multi-phase)
bash scripts/mainnet/rc5_resilience_orchestrator.sh full
```

---

## 📊 Specific Test Examples

### Example 1: Test Network Resilience

```bash
# Inject 100ms latency + 5% packet loss for 5 minutes
bash scripts/mainnet/rc5_chaos_harness.sh network 300 3

# Monitor results
tail -f logs/rc5-chaos/chaos_harness.log
cat reports/rc5-chaos/chaos_assessment_*.json | jq .
```

### Example 2: Test Process Recovery

```bash
# Crash validators 5 times in rapid succession
bash scripts/mainnet/rc5_chaos_harness.sh process 600 5

# Check if system recovers
tail logs/rc5-chaos/chaos_harness.log | grep -i "recovery\|recovered"
```

### Example 3: Test Memory Stability

```bash
# Apply memory pressure + execute 500 RPC requests
bash scripts/mainnet/rc5_chaos_harness.sh memory 600 5
bash scripts/mainnet/rc5_attack_vectors.sh

# Check memory growth
ps aux | grep x3-chain-node | awk '{print $6}' | tail -n 3
```

### Example 4: Cascade Failure (Everything at Once)

```bash
# Network + Process + Memory + Disk chaos simultaneously
bash scripts/mainnet/rc5_chaos_harness.sh cascade 900 7

# Monitor for cascade failures
grep -i "panic\|corruption\|failed" logs/rc5/rc5_72h.nohup.log | tail -20
```

### Example 5: Full Resilience Assessment

```bash
# Run complete multi-phase testing
bash scripts/mainnet/rc5_resilience_orchestrator.sh heavy

# Generate comprehensive report
cat reports/rc5-orchestrator/resilience_report_*.md
```

---

## 📈 Understanding Intensity Levels

Intensity scale: **1-10**

| Level | Effect | Risk Level |
|-------|--------|-----------|
| 1-2 | Minimal stress | ✅ Safe |
| 3-4 | Light chaos | ✅ Safe |
| 5-6 | Moderate stress | ⚠️ May cause brief unresponsiveness |
| 7-8 | Heavy chaos | ⚠️ Validators may temporarily fail |
| 9-10 | Maximum chaos | 🔴 System may become unresponsive |

---

## 📋 Log Files & Reports

### Chaos Harness Outputs

```
logs/rc5-chaos/
├── chaos_harness.log          # Main execution log
├── io_chaos_*.tmp             # Temporary I/O stress files (cleaned up)
└── disk_pressure.img          # Temporary disk space pressure file

reports/rc5-chaos/
└── chaos_assessment_*.json    # JSON report with metrics
```

### Attack Vector Outputs

```
logs/rc5-attacks/
└── attack_vectors.log         # Attack execution log with results

reports/rc5-attacks/
└── attack_assessment_*.json   # JSON report with vulnerabilities found
```

### Orchestrator Outputs

```
logs/rc5-orchestrator/
└── orchestrator.log           # Master coordination log

reports/rc5-orchestrator/
└── resilience_report_*.md     # Comprehensive markdown report
```

### Main Harness Monitoring

```
logs/rc5/
├── rc5_72h.nohup.log          # Main RC5 harness output (watch this!)
├── alice.log                  # Alice validator logs
├── bob.log                    # Bob validator logs
└── charlie.log                # Charlie validator logs

reports/rc5/
├── health_snapshots.jsonl     # Health metrics over time
├── settlement_snapshots.jsonl # Settlement cycle results
├── invariant_snapshots.jsonl  # Invariant violation checks
├── finality_snapshots.jsonl   # Finality progression
└── final_summary.json         # Final verdict (when RC5 completes)
```

---

## 🔍 What to Look For

### Successful Defense (Good)
```
[DEFENDED] ✅ System recovered from timeout cascade
[DEFENDED] ✅ Finality monotonicity maintained across 50 queries
[DEFENDED] ✅ All 3 validators recovered after 5 crash cycles
[DEFENDED] ✅ Memory usage stable: 1250KB growth is acceptable
[DEFENDED] ✅ No panics detected
[DEFENDED] ✅ Database integrity maintained
```

### Vulnerabilities Found (Red Flag)
```
[VULN FOUND] 🔴 Numeric parsing bypassed: 0x99999999... → result
[VULN FOUND] 🔴 System failed to recover from timeout cascade
[VULN FOUND] 🔴 Finality regression detected: block#35000 → block#34999
[VULN FOUND] 🔴 State divergence: Alice=0x123... Bob=0x456...
[VULN FOUND] 🔴 Potential memory leak detected: 75000KB growth
```

### Check for Panics

```bash
# Should return 0
grep -c "panic\|PANIC\|thread.*panicked" logs/rc5/rc5_72h.nohup.log

# Should return empty
grep -i "corrupt" logs/rc5/rc5_72h.nohup.log
```

---

## 🛠️ Advanced Usage

### Custom Duration & Intensity

```bash
# Network chaos for 2 hours at max intensity
bash scripts/mainnet/rc5_chaos_harness.sh network 7200 10

# Memory pressure for 1 hour at moderate intensity
bash scripts/mainnet/rc5_chaos_harness.sh memory 3600 6

# Database chaos for 30 minutes at low intensity
bash scripts/mainnet/rc5_chaos_harness.sh db 1800 2
```

### Monitoring During Tests

In separate terminals:

```bash
# Terminal 1: Watch chaos execution
tail -f logs/rc5-chaos/chaos_harness.log

# Terminal 2: Watch main harness for panics
tail -f logs/rc5/rc5_72h.nohup.log | grep -E "Panic|Error|FAIL"

# Terminal 3: Monitor validator processes
watch -n 1 'pgrep -f "x3-chain-node" | wc -l'

# Terminal 4: Check current block production
watch -n 5 'tail -n 1 logs/rc5/rc5_72h.nohup.log | grep -oE "#[0-9]+"'
```

### Extract Attack Metrics

```bash
# How many vulnerabilities found?
cat reports/rc5-attacks/attack_assessment_*.json | jq '.attack_assessment.vulnerabilities_found'

# What specific vulnerabilities?
cat reports/rc5-attacks/attack_assessment_*.json | jq '.vulnerabilities[]'

# Chaos impact summary
cat reports/rc5-chaos/chaos_assessment_*.json | jq '.chaos_assessment'
```

---

## ⚡ Performance Tips

### Reduce Resource Usage
```bash
# Light network chaos only (no disk/memory)
bash scripts/mainnet/rc5_chaos_harness.sh network 600 2
```

### Run Minimal Tests First
```bash
# Just attack vectors (no root needed)
bash scripts/mainnet/rc5_attack_vectors.sh

# Then escalate
bash scripts/mainnet/rc5_chaos_harness.sh network 600 3
```

### Parallel Execution (Safe)
```bash
# These can run simultaneously without interference
bash scripts/mainnet/rc5_attack_vectors.sh &
bash scripts/mainnet/rc5_chaos_harness.sh network 600 3 &
wait
```

---

## 🚨 Troubleshooting

### "Permission denied" on network chaos
```bash
sudo bash scripts/mainnet/rc5_chaos_harness.sh network 600 3
```

### "iptables not found"
```bash
# Install iptables if missing (system-dependent)
# Ubuntu/Debian:
sudo apt-get install iptables

# Or use with sudo
sudo bash scripts/mainnet/rc5_chaos_harness.sh network 600 2
```

### "Process not found" warnings
These are normal - validators may have already recovered.

### Validators stuck unresponsive
Run cleanup manually:
```bash
sudo tc qdisc del dev lo root 2>/dev/null || true
sudo iptables -F 2>/dev/null || true
pkill -f "flock" || true
```

---

## 📊 Expected Results Summary

### Light Scenario
- ✅ All validators recover
- ✅ No panics
- ✅ RPC continues responding
- ✅ Finality advancing normally

### Medium Scenario
- ✅ Temporary RPC unresponsiveness
- ✅ Validators crash and restart
- ✅ System recovers within 30 seconds
- ✅ No data corruption
- ⚠️ May detect 0-2 minor vulnerabilities

### Heavy Scenario
- ⚠️ Extended RPC unavailability
- ⚠️ Validators may lag during restart
- ⚠️ May detect cascading failures
- ✅ System recovers within 60 seconds
- 📊 Comprehensive vulnerability report

### Extreme Scenario
- 🔴 System may become briefly unresponsive
- 🔴 Validators may need manual restart
- 📊 Extensive vulnerability mapping
- 📊 Recovery strategies identified

---

## 🎓 Learning from Results

### If Tests Pass
✅ **RC5 is solid!**
- System handles adversarial conditions
- Recovery is automatic and reliable
- Ready for continued hardening

### If Tests Find Issues
🔴 **Document and Fix**
1. Capture the exact attack that failed: `tail logs/rc5-chaos/chaos_harness.log`
2. Check main harness for error context: `grep -A5 "error\|panic" logs/rc5/rc5_72h.nohup.log`
3. Review attack assessment: `cat reports/rc5-attacks/attack_assessment_*.json | jq`
4. Implement fix (e.g., numeric parsing hardening)
5. Re-run same attack to verify fix

---

## 🔗 Integration with RC5 72h Test

These chaos tools run **alongside** the active RC5 72-hour burn-in:

```
RC5 72-Hour Burn-In (RUNNING) ━━━━━━━━━━━━━━━━━━━━━━━ 
   ├─ Block Production (#35071+) ✅
   ├─ Settlement Cycles (110+ cycles, all PASS) ✅
   ├─ Invariant Checks (113 cycles, all PASS) ✅
   └─ Boot ID Detection (monitoring) ✅

Chaos Testing (Can Run In Parallel)
   ├─ Attack Vectors (test RPC robustness)
   ├─ Network Chaos (test latency handling)
   ├─ Process Resilience (test recovery)
   └─ Orchestrated Scenarios (comprehensive)
```

**Key Point:** RC5 chaos tests don't interfere with the production run. They validate system components independently.

---

## 📞 Support

### Common Questions

**Q: Will this break my RC5 run?**
A: No. Chaos tools inject faults locally; they don't modify the blockchain state or corrupt critical databases.

**Q: Can I run multiple chaos tests simultaneously?**
A: Yes! They're designed to be parallelizable.

**Q: How do I know if a vulnerability is real?**
A: Reproduce it 3+ times. Check the panic/error logs in the main harness.

**Q: Should I run extreme chaos?**
A: Only after medium passes. Extreme is useful for finding edge cases but may require manual recovery.

---

## 📝 Next Steps

1. ✅ Run attack vectors: `bash scripts/mainnet/rc5_attack_vectors.sh`
2. ✅ Run light chaos: `bash scripts/mainnet/rc5_resilience_orchestrator.sh light`
3. 📊 Review reports in `reports/rc5-chaos/` and `reports/rc5-attacks/`
4. 🔧 Fix any vulnerabilities found
5. 🔄 Re-run to validate fixes
6. 🚀 When all pass: RC5 is production-hardened!

---

*Happy Chaos Testing! 🎉*
