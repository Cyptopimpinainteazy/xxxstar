#!/bin/bash

# X3 App Store Start Script
# This script starts the X3 App Store application

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to show usage
show_usage() {
  echo -e "${BLUE}X3 App Store Start Script${NC}"
  echo ""
  echo -e "Usage: $0 [OPTIONS]"
  echo ""
  echo -e "OPTIONS:"
  echo -e "  ${GREEN}-h, --help${NC}     Show this help message"
  echo -e "  ${GREEN}--dev${NC}          Start in development mode"
  echo -e "  ${GREEN}--prod${NC}         Start in production mode"
  echo -e "  ${GREEN}--docker${NC}       Start using Docker"
  echo -e "  ${GREEN}--build${NC}        Build before starting"
  echo -e "  ${GREEN}--no-build${NC}     Skip build"
  echo ""
  echo -e "Examples:"
  echo -e "  $0 --dev"
  echo -e "  $0 --prod"
  echo -e "  $0 --docker"
}

# Function to check dependencies
check_dependencies() {
  echo -e "${YELLOW}Checking dependencies...${NC}"

  # Check if Node.js is installed
  if ! command -v node &> /dev/null; then
    echo -e "${RED}Node.js is not installed. Please install Node.js first.${NC}"
    exit 1
  fi

  # Check if npm is installed
  if ! command -v npm &> /dev/null; then
    echo -e "${RED}npm is not installed. Please install npm first.${NC}"
    exit 1
  fi

  # Check if Docker is installed (for Docker mode)
  if [ "$DOCKER_MODE" = true ]; then
    if ! command -v docker &> /dev/null; then
      echo -e "${RED}Docker is not installed. Please install Docker first.${NC}"
      exit 1
    fi

    if ! command -v docker-compose &> /dev/null; then
      echo -e "${RED}Docker Compose is not installed. Please install Docker Compose first.${NC}"
      exit 1
    fi
  fi

  echo -e "${GREEN}All dependencies are available.${NC}"
}

# Function to build the application
build_app() {
  echo -e "${YELLOW}Building application...${NC}"

  # Build backend
  echo -e "${BLUE}Building backend...${NC}"
  cd backend
  npm install > /dev/null 2>&1
  npm run build > /dev/null 2>&1
  cd ..

  # Build frontend
  echo -e "${BLUE}Building frontend...${NC}"
  cd frontend
  npm install > /dev/null 2>&1
  npm run build > /dev/null 2>&1
  cd ..

  # Build other services
  echo -e "${BLUE}Building other services...${NC}"
  cd github-scraper
  npm install > /dev/null 2>&1
  cd ..

  cd treasury
  npm install > /dev/null 2>&1
  cd ..

  cd app-store-manager
  npm install > /dev/null 2>&1
  cd ..

  echo -e "${GREEN}Application built successfully.${NC}"
}

# Function to start in development mode
start_dev() {
  echo -e "${YELLOW}Starting X3 App Store in development mode...${NC}"

  # Start backend
  echo -e "${BLUE}Starting backend...${NC}"
  cd backend
  npm run dev > /dev/null 2>&1 &
  BACKEND_PID=$!
  cd ..

  # Start frontend
  echo -e "${BLUE}Starting frontend...${NC}"
  cd frontend
  npm run dev > /dev/null 2>&1 &
  FRONTEND_PID=$!
  cd ..

  # Start other services
  echo -e "${BLUE}Starting other services...${NC}"
  cd github-scraper
  node scraper.js > /dev/null 2>&1 &
  SCRAPER_PID=$!
  cd ..

  cd treasury
  node treasury.js > /dev/null 2>&1 &
  TREASURY_PID=$!
  cd ..

  cd app-store-manager
  node manager.js > /dev/null 2>&1 &
  MANAGER_PID=$!
  cd ..

  echo -e "${GREEN}X3 App Store started successfully!${NC}"
  echo -e "${YELLOW}Frontend: http://localhost:3001${NC}"
  echo -e "${YELLOW}Backend API: http://localhost:3000${NC}"

  # Wait for services to finish
  wait $BACKEND_PID $FRONTEND_PID $SCRAPER_PID $TREASURY_PID $MANAGER_PID
}

# Function to start in production mode
start_prod() {
  echo -e "${YELLOW}Starting X3 App Store in production mode...${NC}"

  # Start backend
  echo -e "${BLUE}Starting backend...${NC}"
  cd backend
  npm start > /dev/null 2>&1 &
  BACKEND_PID=$!
  cd ..

  # Start frontend
  echo -e "${BLUE}Starting frontend...${NC}"
  cd frontend
  npm start > /dev/null 2>&1 &
  FRONTEND_PID=$!
  cd ..

  echo -e "${GREEN}X3 App Store started successfully!${NC}"
  echo -e "${YELLOW}Frontend: http://localhost:3001${NC}"
  echo -e "${YELLOW}Backend API: http://localhost:3000${NC}"

  # Wait for services to finish
  wait $BACKEND_PID $FRONTEND_PID
}

# Function to start using Docker
start_docker() {
  echo -e "${YELLOW}Starting X3 App Store using Docker...${NC}"

  # Build and start services
  echo -e "${BLUE}Building and starting services...${NC}"
  docker-compose -f docker-compose.dev.yml up -d

  echo -e "${GREEN}X3 App Store started successfully!${NC}"
  echo -e "${YELLOW}Frontend: http://localhost:3001${NC}"
  echo -e "${YELLOW}Backend API: http://localhost:3000${NC}"
}

# Parse command line arguments
BUILD=false
DOCKER_MODE=false

while [[ $# -gt 0 ]]; do
  case $1 in
    -h|--help)
      show_usage
      exit 0
      ;;
    --dev)
      MODE="dev"
      shift
      ;;
    --prod)
      MODE="prod"
      shift
      ;;
    --docker)
      DOCKER_MODE=true
      shift
      ;;
    --build)
      BUILD=true
      shift
      ;;
    --no-build)
      BUILD=false
      shift
      ;;
    *)
      echo -e "${RED}Unknown option: $1${NC}"
      show_usage
      exit 1
      ;;
  esac
done

# Validate mode
if [ -z "$MODE" ] && [ "$DOCKER_MODE" = false ]; then
  echo -e "${RED}Mode is required unless using Docker.${NC}"
  show_usage
  exit 1
fi

# Check dependencies
check_dependencies

# Build if requested
if [ "$BUILD" = true ]; then
  build_app
fi

# Start based on mode
if [ "$DOCKER_MODE" = true ]; then
  start_docker
else
  case $MODE in
    dev)
      start_dev
      ;;
    prod)
      start_prod
      ;;
    *)
      echo -e "${RED}Unknown mode: $MODE${NC}"
      show_usage
      exit 1
      ;;
  esac
fi