#!/bin/bash
# Force PCIe bus rescan to detect 4th GPU
# Run with sudo

set -e

echo "=== Force PCIe Bus Rescan for 4th GPU ==="

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo "Please run as root (sudo)"
    exit 1
fi

echo "Current GPU status:"
nvidia-smi -L 2>/dev/null || true
echo ""

echo "Removing GPU 0000:09:00.0 from system..."
echo 1 > /sys/bus/pci/devices/0000:09:00.0/remove 2>/dev/null || echo "Remove failed - device may already be in bad state"
sleep 2

echo "Rescanning PCIe bus..."
echo 1 > /sys/bus/pci/rescan
sleep 3

echo ""
echo "Checking if GPU was rediscovered..."
if [ -d "/sys/bus/pci/devices/0000:09:00.0" ]; then
    echo "GPU 0000:09:00.0 rediscovered!"
    echo "Driver bound: $(basename $(readlink /sys/bus/pci/devices/0000:09:00.0/driver 2>/dev/null) 2>/dev/null || echo 'none')"
else
    echo "GPU 0000:09:00.0 NOT rediscovered - hardware issue likely"
fi

echo ""
echo "Current GPU status:"
nvidia-smi -L 2>/dev/null || echo "nvidia-smi failed"
echo ""

echo "Device nodes:"
ls -la /dev/nvidia* 2>/dev/null || echo "No device nodes"

echo ""
echo "=== Rescan Complete ==="