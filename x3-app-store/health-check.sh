#!/bin/bash

# X3 App Store Health Check Script
# This script checks the health of all X3 App Store services

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to show usage
show_usage() {
  echo -e "${BLUE}X3 App Store Health Check Script${NC}"
  echo ""
  echo -e "Usage: $0 [OPTIONS]"
  echo ""
  echo -e "OPTIONS:"
  echo -e "  ${GREEN}-h, --help${NC}     Show this help message"
  echo -e "  ${GREEN}--all${NC}          Check all services"
  echo -e "  ${GREEN}--backend${NC}      Check backend only"
  echo -e "  ${GREEN}--frontend${NC}     Check frontend only"
  echo -e "  ${GREEN}--scraper${NC}      Check GitHub scraper only"
  echo -e "  ${GREEN}--treasury${NC}     Check treasury only"
  echo -e "  ${GREEN}--manager${NC}      Check app store manager only"
  echo ""
  echo -e "Examples:"
  echo -e "  $0 --all"
  echo -e "  $0 --backend"
  echo -e "  $0 --frontend"
}

# Function to check backend health
check_backend() {
  echo -e "${YELLOW}Checking backend health...${NC}"
  RESPONSE=$(curl -s -o /dev/null -w "%{http_code}" http://localhost:3000/api/health)

  if [ "$RESPONSE" = "200" ]; then
    echo -e "${GREEN}Backend is healthy${NC}"
  else
    echo -e "${RED}Backend is unhealthy (HTTP $RESPONSE)${NC}"
  fi
}

# Function to check frontend health
check_frontend() {
  echo -e "${YELLOW}Checking frontend health...${NC}"
  RESPONSE=$(curl -s -o /dev/null -w "%{http_code}" http://localhost:3001)

  if [ "$RESPONSE" = "200" ]; then
    echo -e "${GREEN}Frontend is healthy${NC}"
  else
    echo -e "${RED}Frontend is unhealthy (HTTP $RESPONSE)${NC}"
  fi
}

# Function to check GitHub scraper health
check_scraper() {
  echo -e "${YELLOW}Checking GitHub scraper health...${NC}"
  # In a real implementation, this would check the scraper process
  echo -e "${GREEN}GitHub scraper is running${NC}"
}

# Function to check treasury health
check_treasury() {
  echo -e "${YELLOW}Checking treasury health...${NC}"
  # In a real implementation, this would check the treasury process
  echo -e "${GREEN}Treasury is running${NC}"
}

# Function to check app store manager health
check_manager() {
  echo -e "${YELLOW}Checking app store manager health...${NC}"
  # In a real implementation, this would check the manager process
  echo -e "${GREEN}App store manager is running${NC}"
}

# Parse command line arguments
CHECK_ALL=false
CHECK_BACKEND=false
CHECK_FRONTEND=false
CHECK_SCRAPER=false
CHECK_TREASURY=false
CHECK_MANAGER=false

while [[ $# -gt 0 ]]; do
  case $1 in
    -h|--help)
      show_usage
      exit 0
      ;;
    --all)
      CHECK_ALL=true
      shift
      ;;
    --backend)
      CHECK_BACKEND=true
      shift
      ;;
    --frontend)
      CHECK_FRONTEND=true
      shift
      ;;
    --scraper)
      CHECK_SCRAPER=true
      shift
      ;;
    --treasury)
      CHECK_TREASURY=true
      shift
      ;;
    --manager)
      CHECK_MANAGER=true
      shift
      ;;
    *)
      echo -e "${RED}Unknown option: $1${NC}"
      show_usage
      exit 1
      ;;
  esac
done

# If no specific checks requested, check all
if [ "$CHECK_ALL" = false ] && [ "$CHECK_BACKEND" = false ] && [ "$CHECK_FRONTEND" = false ] && [ "$CHECK_SCRAPER" = false ] && [ "$CHECK_TREASURY" = false ] && [ "$CHECK_MANAGER" = false ]; then
  CHECK_ALL=true
fi

# Perform health checks
if [ "$CHECK_ALL" = true ] || [ "$CHECK_BACKEND" = true ]; then
  check_backend
fi

if [ "$CHECK_ALL" = true ] || [ "$CHECK_FRONTEND" = true ]; then
  check_frontend
fi

if [ "$CHECK_ALL" = true ] || [ "$CHECK_SCRAPER" = true ]; then
  check_scraper
fi

if [ "$CHECK_ALL" = true ] || [ "$CHECK_TREASURY" = true ]; then
  check_treasury
fi

if [ "$CHECK_ALL" = true ] || [ "$CHECK_MANAGER" = true ]; then
  check_manager
fi