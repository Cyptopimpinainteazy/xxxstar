#!/bin/bash
#
# X3 Chain - IBM System x3550 M5 Setup Script
#
# Server Specs:
# - 2x Xeon E5-2620 v4 (16 cores / 32 threads)
# - 256GB RAM
# - New SSD to be added
#

set -e

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${BLUE}=====================================${NC}"
echo -e "${BLUE}IBM System x3550 M5 - Server Setup${NC}"
echo -e "${BLUE}=====================================${NC}"
echo ""

# Check we're on the right machine
echo -e "${YELLOW}Checking system specs...${NC}"
CPU_COUNT=$(nproc)
RAM_GB=$(free -g | awk '/^Mem:/{print $2}')

if [ "$CPU_COUNT" -lt 16 ]; then
    echo -e "${RED}Warning: Expected 16+ cores, found $CPU_COUNT${NC}"
fi

if [ "$RAM_GB" -lt 200 ]; then
    echo -e "${RED}Warning: Expected 256GB RAM, found ${RAM_GB}GB${NC}"
fi

echo -e "${GREEN}✓ CPU Cores: $CPU_COUNT${NC}"
echo -e "${GREEN}✓ RAM: ${RAM_GB}GB${NC}"
echo ""

# Detect new SSD
echo -e "${YELLOW}Detecting storage devices...${NC}"
echo ""
lsblk -o NAME,SIZE,TYPE,FSTYPE,MOUNTPOINT
echo ""

echo -e "${YELLOW}Available unmounted disks:${NC}"
UNMOUNTED=$(lsblk -rno NAME,SIZE,TYPE,MOUNTPOINT | awk '$3=="disk" && $4=="" {print "/dev/"$1" ("$2")"}')

if [ -z "$UNMOUNTED" ]; then
    echo -e "${RED}No unmounted disks found!${NC}"
    echo "If you just added an SSD, you may need to:"
    echo "  1. Partition it: sudo fdisk /dev/sdX"
    echo "  2. Format it: sudo mkfs.ext4 /dev/sdX1"
    echo "  3. Run this script again"
    exit 1
fi

echo "$UNMOUNTED"
echo ""

# Ask which disk to use
read -p "Enter device name for X3 Chain storage (e.g., sdb): " DISK_NAME
DISK="/dev/${DISK_NAME}"

if [ ! -b "$DISK" ]; then
    echo -e "${RED}Error: $DISK is not a block device!${NC}"
    exit 1
fi

echo ""
echo -e "${RED}WARNING: This will format $DISK!${NC}"
echo -e "${RED}ALL DATA ON $DISK WILL BE ERASED!${NC}"
echo ""
read -p "Type 'YES' to continue: " confirm

if [ "$confirm" != "YES" ]; then
    echo "Cancelled."
    exit 0
fi

# Partition and format
echo ""
echo -e "${YELLOW}Creating partition on $DISK...${NC}"
sudo parted -s "$DISK" mklabel gpt
sudo parted -s "$DISK" mkpart primary ext4 0% 100%
sleep 2

PARTITION="${DISK}1"
if [ ! -b "$PARTITION" ]; then
    PARTITION="${DISK}p1"  # NVMe naming
fi

echo -e "${YELLOW}Formatting ${PARTITION} as ext4...${NC}"
sudo mkfs.ext4 -F "$PARTITION"

# Create mount point
echo -e "${YELLOW}Creating mount point /var/lib/x3-chain...${NC}"
sudo mkdir -p /var/lib/x3-chain

# Mount
echo -e "${YELLOW}Mounting ${PARTITION}...${NC}"
sudo mount "$PARTITION" /var/lib/x3-chain

# Get UUID for fstab
UUID=$(sudo blkid -s UUID -o value "$PARTITION")

# Add to fstab
echo -e "${YELLOW}Adding to /etc/fstab for auto-mount...${NC}"
echo "UUID=$UUID /var/lib/x3-chain ext4 defaults,noatime 0 2" | sudo tee -a /etc/fstab

# Set permissions
sudo chown -R $USER:$USER /var/lib/x3-chain

echo ""
echo -e "${GREEN}✓ Storage setup complete!${NC}"
echo ""
df -h /var/lib/x3-chain
echo ""

# Install dependencies
echo -e "${YELLOW}Installing system dependencies...${NC}"
sudo apt update
sudo apt install -y curl jq net-tools

echo ""
echo -e "${BLUE}=====================================${NC}"
echo -e "${BLUE}🎉 Server Ready!${NC}"
echo -e "${BLUE}=====================================${NC}"
echo ""
echo "Server specs verified:"
echo "  CPU: $CPU_COUNT cores"
echo "  RAM: ${RAM_GB}GB"
echo "  Storage: Mounted at /var/lib/x3-chain"
echo ""
echo "Next steps:"
echo "  1. Copy x3-chain-node binary to this server"
echo "  2. Copy deployment files"
echo "  3. Run: ./deploy-local-testnet.sh"
echo ""
echo -e "${GREEN}Ready to deploy X3 Chain! 🚀${NC}"
