#!/bin/bash
# GPU Recovery Script for 4th GPU (Bus ID 09:00.0)
# Run with sudo

set -e

GPU_BUS="0000:09:00.0"
GPU_DRIVER="nvidia"

echo "=== GPU Recovery Script ==="
echo "Target GPU: $GPU_BUS"
echo ""

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo "Please run as root (sudo)"
    exit 1
fi

echo "Step 1: Checking current GPU status..."
nvidia-smi -L 2>/dev/null || echo "nvidia-smi not responding normally"
echo ""

echo "Step 2: Unbinding GPU $GPU_BUS from nvidia driver..."
if [ -d "/sys/bus/pci/devices/$GPU_BUS/driver" ]; then
    echo "$GPU_BUS" > /sys/bus/pci/devices/$GPU_BUS/driver/unbind 2>/dev/null || echo "Unbind failed (may already be unbound)"
    echo "Unbound successfully"
else
    echo "Driver not bound to this device"
fi
echo ""

echo "Step 3: Resetting PCIe device..."
# Try secondary bus reset
if [ -f "/sys/bus/pci/devices/$GPU_BUS/reset" ]; then
    echo "1" > /sys/bus/pci/devices/$GPU_BUS/reset 2>/dev/null && echo "Device reset successful" || echo "Reset failed"
else
    echo "Reset file not found, trying secondary bus reset..."
    # Find the upstream bridge and reset
    BRIDGE=$(readlink -f /sys/bus/pci/devices/$GPU_BUS | xargs dirname | xargs basename)
    if [ -f "/sys/bus/pci/devices/0000:$BRIDGE/secondary_bus_reset" ]; then
        echo "1" > /sys/bus/pci/devices/0000:$BRIDGE/secondary_bus_reset 2>/dev/null || echo "Bridge reset failed"
    fi
fi
sleep 2
echo ""

echo "Step 4: Rebinding GPU to nvidia driver..."
echo "$GPU_BUS" > /sys/bus/pci/drivers/nvidia/bind 2>/dev/null && echo "Rebind successful" || echo "Rebind failed - may need manual intervention"
sleep 1
echo ""

echo "Step 5: Recreating device nodes..."
# Trigger udev to recreate device nodes
udevadm trigger --action=add /sys/bus/pci/devices/$GPU_BUS 2>/dev/null || true
sleep 1
echo ""

echo "Step 6: Checking GPU status after recovery..."
nvidia-smi -L 2>/dev/null || echo "nvidia-smi still not seeing all GPUs"
echo ""

echo "Step 7: Checking device nodes..."
ls -la /dev/nvidia* 2>/dev/null || echo "No nvidia device nodes found"
echo ""

echo "=== Recovery Complete ==="
echo ""
echo "If the GPU is still not visible, you may need to:"
echo "1. Check physical connections (power cables, PCIe slot)"
echo "2. Reboot the system"
echo "3. Check for hardware issues"
echo ""
echo "Current nvidia-smi output:"
nvidia-smi