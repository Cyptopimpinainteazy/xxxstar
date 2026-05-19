# GPU Validator Troubleshooting Guide

**Document Version:** 1.0  
**Last Updated:** 2026-04-21  
**Status:** Ready for Production Use  
**Target Audience:** GPU Validators, DevOps Engineers, Hardware Team

---

## Overview

This guide provides **comprehensive troubleshooting procedures for GPU-accelerated validators** on X3 mainnet. It covers:
- GPU initialization and detection
- CUDA error diagnosis
- Thermal management
- Memory pressure and OOM recovery
- Performance degradation troubleshooting
- Hardware failure recovery

### Quick Reference

**Common GPU Validation Commands:**

```bash
# Check GPU detection
nvidia-smi

# Check CUDA availability to validator
x3-validator --gpu-status

# Monitor GPU temperature
watch -n 1 'nvidia-smi --query-gpu=temperature.gpu --format=csv'

# Check GPU memory usage
nvidia-smi --query-gpu=memory.used,memory.free --format=csv

# Test GPU with workload
cuda-samples/bin/x86_64/linux/release/bandwidthTest
```
### Related Documents

**For GPU-related incidents:** See **MAINNET_INCIDENT_RESPONSE.md** (hardware failures during production)

**For validator lifecycle:** See **VALIDATOR_OPERATIONS.md** (removing failed validators, recovery procedures)

**For performance baselines:** See **MAINNET_PERFORMANCE_BASELINE.md** (GPU's impact on block production metrics)

**For launch timeline:** See **PHASE_13F_MAINNET_LAUNCH_RUNBOOK.md** (GPU validation windows pre-launch)

**For RPC failover:** See **RPC_FAILOVER_PROCEDURES.md** (ensuring RPC stays healthy during GPU issues)
---

## Section 1: GPU Initialization Issues

### GPU Not Detected

**Problem:** Validator cannot find GPU

**Diagnosis:**

```bash
#!/bin/bash

echo "=== GPU Detection Diagnosis ==="

# Step 1: Check if GPU hardware is present
echo "1. Checking hardware..."
lspci | grep -i "nvidia\|gpu"

# Expected output:
# 01:00.0 3D controller: NVIDIA Corporation GA100 [A100 PCIe 40GB]
# If no output: GPU not physically installed

# Step 2: Check if nvidia-smi can detect GPU
echo
echo "2. Checking nvidia-smi..."
nvidia-smi

# Expected output:
# +-----------------------------------------------------------------------------+
# | NVIDIA-SMI 525.125.06    Driver Version: 525.125.06    CUDA Version: 12.1 |
# | GPU  Name        Persistence-M| Bus-Id        Disp.A | Volatile Uncorr. ECC |
# | No running processes found  |
# +-----------------------------------------------------------------------------+

if [ $? -ne 0 ]; then
  echo "❌ nvidia-smi failed - driver issue"
fi

# Step 3: Check if CUDA toolkit is installed
echo
echo "3. Checking CUDA toolkit..."
nvcc --version

# Expected output: CUDA compilation tools, release 12.1, V12.1.0

if [ $? -ne 0 ]; then
  echo "❌ CUDA toolkit not found"
fi

# Step 4: Check if validator can detect GPU
echo
echo "4. Checking validator GPU detection..."
x3-validator --gpu-status

# Expected output:
# GPU 0: NVIDIA A100-PCIE-40GB
# Status: OK
# Memory: 40GB available

# Step 5: Check environment variables
echo
echo "5. Checking environment variables..."
echo "CUDA_HOME=$CUDA_HOME"
echo "LD_LIBRARY_PATH=$LD_LIBRARY_PATH" | grep -o "/usr/local/cuda[^:]*"
echo "PATH=$PATH" | grep -o "/usr/local/cuda[^:]*"

# Step 6: Check GPU PCIe connectivity
echo
echo "6. Checking PCIe connectivity..."
sudo setpci -s 01:00.0 4.w

# Expected: Returns hex value (e.g., 0147)
# If returns 0000 or fails: PCIe connectivity issue
```

**Recovery Steps:**

**If GPU not present in lspci:**
```bash
# Check BIOS settings
# 1. Reboot into BIOS
# 2. Check:
#    - PCIe slot is enabled
#    - X16 mode is set (not X1 or X4)
#    - GPU is in correct slot
# 3. Save and reboot
```

**If nvidia-smi fails:**
```bash
# Reinstall NVIDIA driver

# Step 1: Remove old driver
sudo apt-get remove --purge nvidia-*

# Step 2: Download correct driver
# Visit: https://www.nvidia.com/Download/driverDetails.aspx
# Download for your GPU and OS
wget https://us.download.nvidia.com/XFree86/Linux-x86_64/525.125.06/NVIDIA-Linux-x86_64-525.125.06.run

# Step 3: Install driver
sudo bash NVIDIA-Linux-x86_64-525.125.06.run

# Step 4: Verify installation
nvidia-smi

# Step 5: Restart validator
sudo systemctl restart x3-validator
```

**If CUDA toolkit missing:**
```bash
# Install CUDA toolkit

# Step 1: Download from NVIDIA
wget https://developer.download.nvidia.com/compute/cuda/12.1.0/local_installers/cuda_12.1.0_530.30.02_linux.run

# Step 2: Install
sudo bash cuda_12.1.0_530.30.02_linux.run

# Step 3: Add to PATH
echo 'export PATH=/usr/local/cuda/bin:$PATH' >> ~/.bashrc
echo 'export LD_LIBRARY_PATH=/usr/local/cuda/lib64:$LD_LIBRARY_PATH' >> ~/.bashrc
source ~/.bashrc

# Step 4: Verify
nvcc --version
```

### GPU Not Enabled for Validator

**Problem:** GPU detected but validator not using it

**Solution:**

```bash
# Check validator configuration
cat /etc/x3-validator/mainnet.yaml | grep -A 5 "gpu:"

# Expected:
# gpu:
#   enabled: true
#   device: 0
#   compute_capability: 8.0

# If not enabled, edit config
sudo nano /etc/x3-validator/mainnet.yaml

# Ensure:
gpu:
  enabled: true
  device: 0           # GPU device index
  cuda_visible_devices: 0  # Can be comma-separated for multiple GPUs

# Restart validator
sudo systemctl restart x3-validator

# Verify GPU is being used
x3-validator --gpu-status

# Check logs
sudo journalctl -u x3-validator | grep -i "gpu\|cuda"
# Should show: "GPU initialized: NVIDIA A100"
```

---

## Section 2: CUDA Error Diagnosis

### Common CUDA Errors

| Error | Meaning | Solution |
|-------|---------|----------|
| CUDA_ERROR_NO_DEVICE (100) | No GPU found | See GPU Not Detected |
| CUDA_ERROR_NOT_INITIALIZED (3) | CUDA not initialized | Reinstall CUDA toolkit |
| CUDA_ERROR_INVALID_DEVICE (101) | Wrong device index | Check device ID in config |
| CUDA_ERROR_OUT_OF_MEMORY (2) | GPU memory exhausted | Reduce cache size or add memory |
| CUDA_ERROR_LAUNCH_FAILED (719) | Kernel launch failed | Check CUDA version compatibility |
| CUDA_ERROR_PEER_ACCESS_NOT_ENABLED (217) | Multi-GPU issue | Enable peer access in config |

### CUDA Out of Memory (OOM)

**Problem:** CUDA_ERROR_OUT_OF_MEMORY during operation

```bash
#!/bin/bash

echo "=== CUDA Memory Diagnosis ==="

# Step 1: Check total GPU memory
echo "GPU Memory Status:"
nvidia-smi --query-gpu=memory.total,memory.used,memory.free --format=csv,noheader

# Expected:
# 40480 MiB, 15360 MiB, 25120 MiB

# Step 2: Monitor memory in real-time
watch -n 1 'nvidia-smi --query-gpu=memory.used,memory.free --format=csv'

# Step 3: Check what's using GPU memory
echo
echo "Processes using GPU memory:"
nvidia-smi | grep -E "PID|x3-validator"

# Step 4: Check if memory is fragmented
echo
echo "Memory utilization:"
nvidia-smi --query-gpu=utilization.memory --format=csv

# If memory fills but GPU isn't working hard:
# Likely fragmentation issue
```

**Recovery Steps:**

**Reduce cache size:**
```bash
sudo nano /etc/x3-validator/mainnet.yaml

# Find cache configuration
gpu_cache:
  max_size_gb: 8  # Was 16

# Restart validator
sudo systemctl restart x3-validator

# Monitor memory
watch -n 1 'nvidia-smi --query-gpu=memory.used,memory.free --format=csv'

# Memory should stabilize at lower level
```

**Add more GPU memory:**
```bash
# If you have multiple GPUs or expandable GPU memory:

# Check available memory on other GPUs
nvidia-smi

# Use multiple GPUs for validator
sudo nano /etc/x3-validator/mainnet.yaml

gpu:
  enabled: true
  devices: [0, 1]  # Use both GPU 0 and 1
  device_memory_split: [0.5, 0.5]  # 50-50 split

sudo systemctl restart x3-validator
```

**Restart validator to clear memory:**
```bash
# If memory keeps growing (leak), restart validator
sudo systemctl restart x3-validator

# Monitor that memory returns to baseline
watch -n 1 'nvidia-smi --query-gpu=memory.used --format=csv'

# If memory still grows, there's likely a CUDA memory leak
# See Section 4: Memory Pressure and OOM Recovery
```

### CUDA Version Mismatch

**Problem:** Validator compiled with different CUDA version than installed

```bash
# Check compilation CUDA version
strings /opt/x3-validator/x3-validator | grep "CUDA Version"

# Check installed CUDA version
nvcc --version

# If mismatch:
echo "Compiled for CUDA 12.0"
echo "Installed CUDA 11.8"

# Solution: Rebuild validator with correct CUDA version
cd /path/to/x3-validator
export CUDA_HOME=/usr/local/cuda-12.0
export PATH=$CUDA_HOME/bin:$PATH
export LD_LIBRARY_PATH=$CUDA_HOME/lib64:$LD_LIBRARY_PATH

cargo build --release --features cuda

# Or: Install matching CUDA version
# (See CUDA Out of Memory section above)
```

---

## Section 3: Thermal Management

### Temperature Monitoring

**Problem:** GPU running hot or throttling

```bash
#!/bin/bash

echo "=== GPU Temperature Diagnosis ==="

# Check current temperatures
echo "Current GPU temperatures:"
nvidia-smi --query-gpu=index,name,temperature.gpu,temperature.memory,power.draw,power.limit --format=csv

# Expected:
# 0, NVIDIA A100-PCIE-40GB, 42, 45, 150W, 250W

# Monitor continuously
echo
echo "Monitoring temperatures (Ctrl+C to stop)..."
watch -n 1 'nvidia-smi --query-gpu=temperature.gpu,temperature.memory --format=csv,noheader'

# Check thermal design power
nvidia-smi -pm 1

# Expected: Power management mode should be "ON" (not "N/A")
```

### Identifying Thermal Throttling

```bash
# Check if GPU is throttling
nvidia-smi --query-gpu=throttle_reason --format=csv

# Possible outputs:
# None
# HW Slowdown
# Thermal Slowdown
# Power Brake Slowdown

if nvidia-smi --query-gpu=throttle_reason --format=csv | grep -q "Thermal"; then
  echo "❌ GPU is thermally throttling - performance degraded"
fi
```

### Thermal Recovery Procedures

**If thermal throttling detected:**

```bash
#!/bin/bash

echo "=== Thermal Recovery Procedures ==="

CURRENT_TEMP=$(nvidia-smi --query-gpu=temperature.gpu --format=csv,noheader | head -1)
echo "Current GPU temperature: ${CURRENT_TEMP}°C"

if [ "$CURRENT_TEMP" -gt 80 ]; then
  echo "CRITICAL: GPU overheating!"
  
  # Step 1: Stop validator immediately
  sudo systemctl stop x3-validator
  echo "✓ Validator stopped"
  
  # Step 2: Monitor temperature drop
  echo "Waiting for cooling..."
  for i in {1..30}; do
    TEMP=$(nvidia-smi --query-gpu=temperature.gpu --format=csv,noheader | head -1)
    echo "  Temperature: ${TEMP}°C"
    if [ "$TEMP" -lt 50 ]; then
      echo "✓ GPU cooled to safe temperature"
      break
    fi
    sleep 10
  done
  
  # Step 3: Check cooling system
  echo
  echo "Checking cooling system..."
  # Check fan speeds
  nvidia-smi -pm 1
  nvidia-smi -lgc  # Lock graphics clock
  
  # Step 4: Increase fan speed
  # If using nvidia-settings:
  DISPLAY=:0 nvidia-settings -a GPUFanControlState=1
  DISPLAY=:0 nvidia-settings -a GPUTargetFanSpeed=100
  
  # Step 5: Restart validator with reduced load
  # Edit config to reduce GPU utilization
  sudo nano /etc/x3-validator/mainnet.yaml
  
  # Add:
  gpu:
    power_limit_percent: 70  # Reduce power to 70%
  
  # Step 6: Start validator again
  sudo systemctl start x3-validator
  
  # Step 7: Monitor temperature
  watch -n 5 'echo "Temp: $(nvidia-smi --query-gpu=temperature.gpu --format=csv,noheader | head -1)°C"'
  
elif [ "$CURRENT_TEMP" -gt 70 ]; then
  echo "⚠️  GPU running warm, monitor carefully"
  
  # Increase fan speed
  DISPLAY=:0 nvidia-settings -a GPUFanControlState=1
  DISPLAY=:0 nvidia-settings -a GPUTargetFanSpeed=75
  
else
  echo "✅ GPU temperature normal"
fi
```

### Physical Cooling Solutions

**If thermal throttling persists:**

```bash
# Hardware solutions:
1. Improve case airflow:
   - Ensure intake fans working
   - Check for dust buildup
   - Verify hot air exhaust path

2. Increase cooling:
   - Add case fans
   - Add GPU cooler
   - Replace thermal pads
   - Consider liquid cooling

3. Reduce load:
   - Disable other GPUs if not needed
   - Reduce validator stake
   - Run on different GPU

# Monitor thermal improvements
nvidia-smi --query-gpu=temperature.gpu --format=csv --loop=1 --loop-ms=1000
```

---

## Section 4: Memory Pressure and OOM Recovery

### Memory Leak Detection

```bash
#!/bin/bash

echo "=== Memory Leak Detection ==="

# Monitor GPU memory over time
echo "Sampling GPU memory every 30 seconds for 5 minutes..."
for i in {1..10}; do
  TIMESTAMP=$(date '+%H:%M:%S')
  MEMORY=$(nvidia-smi --query-gpu=memory.used --format=csv,noheader | head -1 | sed 's/ MiB//')
  echo "$TIMESTAMP: $MEMORY MB"
  sleep 30
done

# Calculate growth rate
echo
echo "If memory increased consistently, there may be a leak"
```

**Identifying Memory Leak Source:**

```bash
# Check which process is using GPU memory
nvidia-smi | grep "x3-validator" | awk '{print $3}'

# Detailed memory breakdown
nvidia-smi -q -d MEMORY | grep -A 20 "Process Memory"

# If validator is accumulating memory:
# 1. Check validator logs for CUDA errors
sudo journalctl -u x3-validator | grep -i "cuda\|memory"

# 2. Check for CUDA_ERROR_OUT_OF_MEMORY
sudo journalctl -u x3-validator | grep "CUDA_ERROR_OUT_OF_MEMORY"

# 3. Identify if it's:
#    - Gradual leak (memory grows slowly)
#    - Sudden jump (configuration issue)
#    - Spike and stabilize (normal caching)
```

### OOM Event Recovery

**When GPU runs out of memory:**

```bash
#!/bin/bash

echo "=== GPU OOM Recovery ==="

# Step 1: Verify OOM occurred
MEMORY_AVAILABLE=$(nvidia-smi --query-gpu=memory.free --format=csv,noheader | head -1)
echo "GPU memory available: $MEMORY_AVAILABLE MiB"

if [ $MEMORY_AVAILABLE -lt 100 ]; then
  echo "❌ GPU memory critically low (< 100 MiB free)"
  
  # Step 2: Stop validator
  sudo systemctl stop x3-validator
  echo "✓ Validator stopped"
  
  # Step 3: Clear GPU memory
  nvidia-smi --gpu-reset
  echo "✓ GPU memory cleared"
  
  # Step 4: Verify memory cleared
  sleep 5
  MEMORY_CLEARED=$(nvidia-smi --query-gpu=memory.free --format=csv,noheader | head -1)
  echo "GPU memory after reset: $MEMORY_CLEARED MiB"
  
  # Step 5: Adjust configuration
  sudo nano /etc/x3-validator/mainnet.yaml
  
  # Reduce:
  # - Cache size
  # - Batch size
  # - Max workers
  
  # Add memory limit:
  gpu:
    memory_limit_mb: 20000  # 20GB of 40GB capacity
  
  # Step 6: Restart validator
  sudo systemctl start x3-validator
  echo "✓ Validator restarted with reduced memory footprint"
  
  # Step 7: Monitor
  watch -n 10 'nvidia-smi --query-gpu=memory.used,memory.free --format=csv'
  
  # Memory should stabilize below 20GB
fi
```

---

## Section 5: Performance Degradation Troubleshooting

### GPU Underperforming

**Problem:** GPU not achieving expected throughput

```bash
#!/bin/bash

echo "=== GPU Performance Diagnosis ==="

# Check clock speeds
echo "GPU Clock Speeds:"
nvidia-smi --query-gpu=clocks.current.graphics,clocks.current.memory,clocks.current.sm,clocks.current.memory --format=csv

# Expected: Close to max clocks (e.g., 1410 MHz for A100)

# Check power draw
echo
echo "GPU Power Usage:"
nvidia-smi --query-gpu=power.draw,power.limit,power.state --format=csv

# Expected: Power draw close to limit (not limited)

# Check utilization
echo
echo "GPU Utilization:"
nvidia-smi --query-gpu=utilization.gpu,utilization.memory,temperature.gpu,clocks.current.graphics --format=csv

# Expected:
# GPU: > 80%
# Memory: > 80%
# Temp: < 75°C
# Clock: At max

# Check for throttling
echo
echo "Throttling Status:"
nvidia-smi --query-gpu=throttle_reason --format=csv

# Should show: None
```

**Performance Recovery:**

```bash
# If clocks are low:
# 1. Check power limit
nvidia-smi -i 0 -pm 1  # Enable persistence mode
nvidia-smi -i 0 -plr 250  # Set power limit to 250W (for A100)

# If utilization is low (< 50%):
# 1. Increase work batch size
sudo nano /etc/x3-validator/mainnet.yaml
# Increase: batch_size, worker_count

# If temperature is high (> 75°C):
# See Section 3: Thermal Management

# If power is limited:
# 1. Check power supply capacity
# 2. Reduce other GPU power limits
# 3. Stagger GPU utilization
```

### Synchronous vs Asynchronous Issues

```bash
# Check if GPU operations are blocking (slow) or async (fast)

# Test synchronous operation
time (nvidia-smi --query-gpu=memory.used --format=csv > /dev/null)

# Test async operation
time (nvidia-smi --query-gpu=memory.used --format=csv &)

# If sync is significantly slower, GPU might be
# blocked waiting for CPU or host-device transfers

# Check PCIe bandwidth
nvidia-smi dmon | head -20

# PCIe bandwidth should be utilized if large transfers occur
```

---

## Section 6: Hardware Failure Recovery

### GPU Hardware Failure Signs

| Symptom | Likely Issue | Action |
|---------|-------------|--------|
| Consistent CUDA_ERROR_ECC_UNCORRECTABLE | Memory/ECC failure | Replace GPU |
| Temperature stuck at 0°C | Sensor failure | Replace GPU |
| Sudden loss of GPU detection | PCIe slot/interface failure | Check PCIe connection |
| Random kernel panics | Power delivery issue | Check power supply |
| Corrupt computation results | Die/logic failure | Replace GPU |

### Detecting Hardware Failure

```bash
#!/bin/bash

echo "=== GPU Hardware Health Check ==="

# Run NVIDIA's diagnostic
nvidia-smi topo --matrix  # Topology test
nvidia-smi test --pstate  # Power state test

# Run CUDA samples test (if available)
/usr/local/cuda/samples/bin/x86_64/linux/release/deviceQuery

# Check ECC errors
nvidia-smi --query-gpu=ecc.errors.corrected.volatile.total,ecc.errors.uncorrected.volatile.total --format=csv

# Expected: All zeros (no errors)
# If > 0: Potential hardware issue developing

# Monitor for growing errors
echo "Monitoring for ECC errors (1 hour)..."
for i in {1..60}; do
  nvidia-smi --query-gpu=ecc.errors.uncorrected.volatile.total --format=csv,noheader
  sleep 60
done | awk '{if(prev!="" && $1>prev) print "Error count increased!"; prev=$1}'
```

### Hardware Failure Recovery

**If GPU hardware failure detected:**

```bash
# Step 1: Stop validator immediately
sudo systemctl stop x3-validator

# Step 2: Document failure
cat > /tmp/gpu-failure-report.txt << 'EOF'
Date: $(date)
GPU Model: $(nvidia-smi --query-gpu=name --format=csv,noheader)
Error: $(nvidia-smi --query-gpu=ecc.errors.uncorrected.volatile.total --format=csv,noheader)
Temperature: $(nvidia-smi --query-gpu=temperature.gpu --format=csv,noheader)
Power Draw: $(nvidia-smi --query-gpu=power.draw --format=csv,noheader)

Symptoms:
- [List symptoms]

Actions Taken:
- Stopped validator
- Documented failure
EOF

# Step 3: Disable GPU in config (temporary)
sudo nano /etc/x3-validator/mainnet.yaml
# Set: gpu.enabled = false

# Step 4: Start validator on CPU (degraded performance)
sudo systemctl start x3-validator
echo "Validator running on CPU (degraded performance)"

# Step 5: Order replacement GPU
# Contact: [GPU supplier/IT team]
# Model needed: [Same model as failed GPU]
# Warranty: [Check warranty status]

# Step 6: Schedule GPU replacement
# Maintenance window: [Day/Time]

# Step 7: Install replacement GPU
# See GPU Installation section below

# Step 8: Re-enable GPU in config
sudo nano /etc/x3-validator/mainnet.yaml
# Set: gpu.enabled = true

# Step 9: Restart validator
sudo systemctl restart x3-validator
```

### GPU Installation/Replacement

```bash
#!/bin/bash

echo "=== GPU Installation Procedure ==="

# Step 1: Shut down system completely
sudo shutdown -h now
echo "System shutting down. Wait for complete shutdown."
sleep 30

# Power off system at PSU

# Step 2: Ensure proper grounding
# Wear ESD wrist strap
# Touch grounded metal part of case

# Step 3: Remove old GPU
# Release PCIe slot retention clip
# Remove power connectors
# Carefully extract GPU

# Step 4: Install new GPU
# Align GPU with PCIe x16 slot
# Insert firmly until click heard (and retention clip engages)
# Connect power cables (should click)

# Step 5: Power on system
# Switch PSU on
# Press power button

# Step 6: Verify GPU detection
nvidia-smi

# Expected:
# GPU 0: NVIDIA A100-PCIE-40GB

# Step 7: Install/update drivers if needed
# See GPU Not Detected section above

# Step 8: Run diagnostics
/usr/local/cuda/samples/bin/x86_64/linux/release/deviceQuery

# Step 9: Restart validator
sudo systemctl restart x3-validator

# Step 10: Monitor
x3-validator --gpu-status

# Should show: OK
```

---

## Appendix: Quick Diagnostics

### 5-Minute GPU Health Check

```bash
#!/bin/bash

echo "=== 5-Minute GPU Health Check ==="

echo "1. GPU Detection:"
nvidia-smi -L
[ $? -eq 0 ] && echo "   ✅ GPU detected" || echo "   ❌ GPU not detected"

echo "2. Driver:"
nvidia-smi --query-gpu=driver_version --format=csv,noheader
echo "   ✅ Driver OK"

echo "3. Temperature:"
TEMP=$(nvidia-smi --query-gpu=temperature.gpu --format=csv,noheader | head -1)
echo "   Temperature: ${TEMP}°C"
[ $TEMP -lt 80 ] && echo "   ✅ Safe" || echo "   ⚠️  Hot"

echo "4. Memory:"
nvidia-smi --query-gpu=memory.total,memory.used,memory.free --format=csv,noheader | head -1
echo "   ✅ Memory OK"

echo "5. Power:"
nvidia-smi --query-gpu=power.draw,power.limit --format=csv,noheader | head -1
echo "   ✅ Power OK"

echo "6. Validator Integration:"
x3-validator --gpu-status
[ $? -eq 0 ] && echo "   ✅ Validator sees GPU" || echo "   ❌ Validator can't access GPU"

echo
echo "=== Check Complete ==="
```

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2026-04-21 | Initial GPU troubleshooting guide |

---

**Questions?** Contact: [gpu-support-team-email]
