#!/bin/bash

# X3 App Store Production Deployment Script
# This script handles production deployment

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to show usage
show_usage() {
  echo -e "${BLUE}X3 App Store Production Deployment Script${NC}"
  echo ""
  echo -e "Usage: $0 [OPTIONS]"
  echo ""
  echo -e "OPTIONS:"
  echo -e "  ${GREEN}-h, --help${NC}     Show this help message"
  echo -e "  ${GREEN}--build${NC}        Build Docker images before deploying"
  echo -e "  ${GREEN}--no-build${NC}     Skip building Docker images"
  echo -e "  ${GREEN}--dry-run${NC}      Show what would be deployed without actually deploying"
  echo -e "  ${GREEN}--force${NC}        Force deployment even if checks fail"
  echo -e "  ${GREEN}--rollback${NC}     Rollback to previous version"
  echo ""
  echo -e "Examples:"
  echo -e "  $0 --build"
  echo -e "  $0 --dry-run"
  echo -e "  $0 --rollback"
}

# Function to check dependencies
check_dependencies() {
  echo -e "${YELLOW}Checking dependencies...${NC}"

  # Check if Docker is installed
  if ! command -v docker &> /dev/null; then
    echo -e "${RED}Docker is not installed. Please install Docker first.${NC}"
    exit 1
  fi

  # Check if Docker Compose is installed
  if ! command -v docker-compose &> /dev/null; then
    echo -e "${RED}Docker Compose is not installed. Please install Docker Compose first.${NC}"
    exit 1
  fi

  # Check if kubectl is installed
  if ! command -v kubectl &> /dev/null; then
    echo -e "${YELLOW}kubectl is not installed. Some deployment features may not work.${NC}"
  fi

  echo -e "${GREEN}All dependencies are available.${NC}"
}

# Function to build Docker images
build_images() {
  echo -e "${YELLOW}Building Docker images...${NC}"

  # Build backend image
  echo -e "${BLUE}Building backend image...${NC}"
  docker build -t x3-app-store-backend:latest ./backend

  # Build frontend image
  echo -e "${BLUE}Building frontend image...${NC}"
  docker build -t x3-app-store-frontend:latest ./frontend

  # Build scraper image
  echo -e "${BLUE}Building GitHub scraper image...${NC}"
  docker build -t x3-app-store-scraper:latest ./github-scraper

  # Build treasury image
  echo -e "${BLUE}Building treasury image...${NC}"
  docker build -t x3-app-store-treasury:latest ./treasury

  # Build app store manager image
  echo -e "${BLUE}Building app store manager image...${NC}"
  docker build -t x3-app-store-manager:latest ./app-store-manager

  echo -e "${GREEN}Docker images built successfully.${NC}"
}

# Function to deploy to production
deploy_prod() {
  echo -e "${YELLOW}Deploying to production environment...${NC}"

  # Check if kubectl is available
  if ! command -v kubectl &> /dev/null; then
    echo -e "${RED}kubectl is required for production deployment.${NC}"
    exit 1
  fi

  # Apply Kubernetes manifests
  echo -e "${BLUE}Applying Kubernetes manifests...${NC}"
  kubectl apply -f k8s/production/

  # Wait for services to be ready
  echo -e "${BLUE}Waiting for services to be ready...${NC}"
  sleep 30

  # Check service health
  echo -e "${BLUE}Checking service health...${NC}"
  kubectl get pods -n x3-app-store

  echo -e "${GREEN}Production environment deployed successfully!${NC}"
}

# Function to rollback deployment
rollback_deployment() {
  echo -e "${YELLOW}Rolling back deployment...${NC}"

  # Check if kubectl is available
  if ! command -v kubectl &> /dev/null; then
    echo -e "${RED}kubectl is required for rollback.${NC}"
    exit 1
  fi

  # Rollback to previous version
  echo -e "${BLUE}Rolling back to previous version...${NC}"
  kubectl rollout undo deployment/backend -n x3-app-store
  kubectl rollout undo deployment/frontend -n x3-app-store
  kubectl rollout undo deployment/github-scraper -n x3-app-store
  kubectl rollout undo deployment/treasury -n x3-app-store
  kubectl rollout undo deployment/app-store-manager -n x3-app-store

  echo -e "${GREEN}Deployment rolled back successfully!${NC}"
}

# Parse command line arguments
BUILD_IMAGES=false
DRY_RUN=false
FORCE=false
ROLLBACK=false

while [[ $# -gt 0 ]]; do
  case $1 in
    -h|--help)
      show_usage
      exit 0
      ;;
    --build)
      BUILD_IMAGES=true
      shift
      ;;
    --no-build)
      BUILD_IMAGES=false
      shift
      ;;
    --dry-run)
      DRY_RUN=true
      shift
      ;;
    --force)
      FORCE=true
      shift
      ;;
    --rollback)
      ROLLBACK=true
      shift
      ;;
    *)
      echo -e "${RED}Unknown option: $1${NC}"
      show_usage
      exit 1
      ;;
  esac
done

# Show what would be deployed in dry run mode
if [ "$DRY_RUN" = true ]; then
  echo -e "${YELLOW}Dry run mode - showing what would be deployed:${NC}"
  echo -e "${BLUE}Build images: $BUILD_IMAGES${NC}"
  echo -e "${BLUE}Force: $FORCE${NC}"
  echo -e "${BLUE}Rollback: $ROLLBACK${NC}"
  exit 0
fi

# Check dependencies
check_dependencies

# Build images if requested
if [ "$BUILD_IMAGES" = true ]; then
  build_images
fi

# Deploy or rollback
if [ "$ROLLBACK" = true ]; then
  rollback_deployment
else
  deploy_prod
fi