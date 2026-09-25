#!/bin/bash

# X3 App Store Frontend Build Script
# This script builds the frontend application

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Building X3 App Store Frontend...${NC}"

# Check if package.json exists
if [ ! -f "package.json" ]; then
  echo -e "${RED}package.json not found. Please run this script from the frontend directory.${NC}"
  exit 1
fi

# Install dependencies
echo -e "${YELLOW}Installing dependencies...${NC}"
npm install > /dev/null 2>&1

if [ $? -ne 0 ]; then
  echo -e "${RED}Failed to install dependencies.${NC}"
  exit 1
fi

# Build the application
echo -e "${YELLOW}Building application...${NC}"
npm run build > /dev/null 2>&1

if [ $? -ne 0 ]; then
  echo -e "${RED}Build failed.${NC}"
  exit 1
fi

echo -e "${GREEN}Build completed successfully!${NC}"
echo -e "${YELLOW}Build output is in the 'dist' directory.${NC}"