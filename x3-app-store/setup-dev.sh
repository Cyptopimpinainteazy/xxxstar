#!/bin/bash

# X3 App Store Development Environment Setup Script
# This script sets up the development environment for X3 App Store

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to show usage
show_usage() {
  echo -e "${BLUE}X3 App Store Development Environment Setup${NC}"
  echo ""
  echo -e "Usage: $0 [OPTIONS]"
  echo ""
  echo -e "OPTIONS:"
  echo -e "  ${GREEN}-h, --help${NC}     Show this help message"
  echo -e "  ${GREEN}--install${NC}      Install all dependencies"
  echo -e "  ${GREEN}--setup-db${NC}     Set up MongoDB database"
  echo -e "  ${GREEN}--setup-env${NC}    Set up environment variables"
  echo -e "  ${GREEN}--all${NC}          Run all setup steps"
  echo ""
  echo -e "Examples:"
  echo -e "  $0 --all"
  echo -e "  $0 --install --setup-db"
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

  # Check if Docker is installed
  if ! command -v docker &> /dev/null; then
    echo -e "${YELLOW}Docker is not installed. Some features may not work.${NC}"
  fi

  # Check if MongoDB is installed
  if ! command -v mongod &> /dev/null; then
    echo -e "${YELLOW}MongoDB is not installed. Some features may not work.${NC}"
  fi

  echo -e "${GREEN}All dependencies are available.${NC}"
}

# Function to install dependencies
install_dependencies() {
  echo -e "${YELLOW}Installing dependencies...${NC}"

  # Install root dependencies
  echo -e "${BLUE}Installing root dependencies...${NC}"
  npm install > /dev/null 2>&1

  # Install backend dependencies
  echo -e "${BLUE}Installing backend dependencies...${NC}"
  cd backend
  npm install > /dev/null 2>&1
  cd ..

  # Install frontend dependencies
  echo -e "${BLUE}Installing frontend dependencies...${NC}"
  cd frontend
  npm install > /dev/null 2>&1
  cd ..

  # Install other service dependencies
  echo -e "${BLUE}Installing other service dependencies...${NC}"
  cd github-scraper
  npm install > /dev/null 2>&1
  cd ..

  cd treasury
  npm install > /dev/null 2>&1
  cd ..

  cd app-store-manager
  npm install > /dev/null 2>&1
  cd ..

  echo -e "${GREEN}Dependencies installed successfully.${NC}"
}

# Function to set up MongoDB database
setup_database() {
  echo -e "${YELLOW}Setting up MongoDB database...${NC}"

  # Check if MongoDB is installed
  if ! command -v mongod &> /dev/null; then
    echo -e "${RED}MongoDB is not installed. Please install MongoDB first.${NC}"
    exit 1
  fi

  # Start MongoDB if not running
  if ! pgrep "mongod" > /dev/null; then
    echo -e "${BLUE}Starting MongoDB...${NC}"
    mongod > /dev/null 2>&1 &
    MONGO_PID=$!
    sleep 5
  fi

  # Create database and collections
  echo -e "${BLUE}Creating database and collections...${NC}"
  mongo <<EOF
    use x3-app-store;
    db.createCollection('projects');
    db.createCollection('users');
    db.createCollection('rewards');
    db.createCollection('tokens');
    db.createCollection('transactions');
    db.createCollection('appStore');
    db.createCollection('sandbox');
    db.createCollection('logs');
    db.createCollection('metrics');
    db.createCollection('settings');
    db.createCollection('notifications');
    db.createCollection('audit');
    print('Database and collections created successfully');
EOF

  # Stop MongoDB if we started it
  if [ -n "$MONGO_PID" ]; then
    kill $MONGO_PID
  fi

  echo -e "${GREEN}Database setup completed.${NC}"
}

# Function to set up environment variables
setup_environment() {
  echo -e "${YELLOW}Setting up environment variables...${NC}"

  # Check if .env file exists
  if [ ! -f ".env" ]; then
    echo -e "${BLUE}Creating .env file...${NC}"
    cp .env.example .env

    # Generate secrets
    JWT_SECRET=$(openssl rand -hex 32)
    SESSION_SECRET=$(openssl rand -hex 32)

    # Update .env file
    echo -e "${BLUE}Updating .env file...${NC}"
    sed -i "s/your_jwt_secret_here/$JWT_SECRET/" .env
    sed -i "s/your_session_secret_here/$SESSION_SECRET/" .env

    echo -e "${GREEN}.env file created and configured.${NC}"
  else
    echo -e "${YELLOW}.env file already exists. Skipping creation.${NC}"
  fi
}

# Function to run all setup steps
setup_all() {
  check_dependencies
  install_dependencies
  setup_database
  setup_environment
}

# Parse command line arguments
INSTALL=false
SETUP_DB=false
SETUP_ENV=false

while [[ $# -gt 0 ]]; do
  case $1 in
    -h|--help)
      show_usage
      exit 0
      ;;
    --install)
      INSTALL=true
      shift
      ;;
    --setup-db)
      SETUP_DB=true
      shift
      ;;
    --setup-env)
      SETUP_ENV=true
      shift
      ;;
    --all)
      setup_all
      exit 0
      ;;
    *)
      echo -e "${RED}Unknown option: $1${NC}"
      show_usage
      exit 1
      ;;
  esac
done

# Run selected setup steps
if [ "$INSTALL" = true ]; then
  install_dependencies
fi

if [ "$SETUP_DB" = true ]; then
  setup_database
fi

if [ "$SETUP_ENV" = true ]; then
  setup_environment
fi