#!/bin/bash

# X3 App Store Backend Test Script
# This script runs tests for the backend application

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Running X3 App Store Backend Tests...${NC}"

# Check if package.json exists
if [ ! -f "package.json" ]; then
  echo -e "${RED}package.json not found. Please run this script from the backend directory.${NC}"
  exit 1
fi

# Install dependencies
echo -e "${YELLOW}Installing dependencies...${NC}"
npm install > /dev/null 2>&1

if [ $? -ne 0 ]; then
  echo -e "${RED}Failed to install dependencies.${NC}"
  exit 1
fi

# Run tests
echo -e "${YELLOW}Running tests...${NC}"
npm test > /dev/null 2>&1

if [ $? -ne 0 ]; then
  echo -e "${RED}Tests failed.${NC}"
  exit 1
fi

echo -e "${GREEN}All tests passed successfully!${NC}"